use sleepguard_core::config_path;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const UNIT_NAME: &str = "sleepguard-monitor.service";
const DESKTOP_NAME: &str = "sleepguard-monitor.desktop";

#[cfg(target_os = "windows")]
const WINDOWS_TASK: &str = "SleepGuard-Monitor";

const DESKTOP_TEMPLATE: &str = r#"[Desktop Entry]
Type=Application
Version=1.0
Name=SleepGuard
Comment=SleepGuard session monitor
Exec=@APP_BIN@ --background
Icon=sleepguard-app
Terminal=false
Categories=Utility;
X-GNOME-Autostart-enabled=true
StartupNotify=false
"#;

pub fn set_enabled(enabled: bool) -> Result<(), String> {
    if enabled {
        enable()
    } else {
        disable()
    }
}

fn enable() -> Result<(), String> {
    let Some(app_bin) = resolve_autostart_binary() else {
        // `tauri dev` without a production binary — skip quietly.
        return Ok(());
    };

    #[cfg(target_os = "linux")]
    return enable_linux(&app_bin);

    #[cfg(target_os = "windows")]
    return enable_windows(&app_bin);

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = app_bin;
        Err("Autostart del monitor no soportado en este SO".into())
    }
}

/// Prefer a production binary for login autostart (never `target/debug`).
fn resolve_autostart_binary() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let exe = exe.canonicalize().unwrap_or(exe);

    if is_production_path(&exe) {
        return Some(exe);
    }

    // From `tauri dev`, register the release binary if it was built with `tauri build`.
    let release = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/sleepguard-app");
    if is_production_path(&release) {
        return Some(release.canonicalize().unwrap_or(release));
    }
    None
}

fn is_production_path(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    let s = path.to_string_lossy();
    !s.contains("/target/debug/") && !s.contains("\\target\\debug\\")
}

fn disable() -> Result<(), String> {
    #[cfg(target_os = "linux")]
    return disable_linux();

    #[cfg(target_os = "windows")]
    return disable_windows();

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    Ok(())
}

#[cfg(target_os = "linux")]
fn user_unit_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("No se pudo resolver el directorio home")?;
    Ok(home.join(".config/systemd/user").join(UNIT_NAME))
}

#[cfg(target_os = "linux")]
fn autostart_desktop_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("No se pudo resolver el directorio home")?;
    Ok(home.join(".config/autostart").join(DESKTOP_NAME))
}

#[cfg(target_os = "linux")]
fn enable_linux(app_bin: &Path) -> Result<(), String> {
    let _ = config_path(); // ensure config dir exists for session use
    let app_str = app_bin.to_string_lossy();

    // One launcher only: XDG autostart. Remove leftover systemd unit (double-start).
    let _ = Command::new("systemctl")
        .args(["--user", "disable", "--now", "--quiet", UNIT_NAME])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    if let Ok(unit_path) = user_unit_path() {
        let _ = fs::remove_file(unit_path);
    }
    let _ = Command::new("systemctl")
        .args(["--user", "daemon-reload", "--quiet"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    let desktop_path = autostart_desktop_path()?;
    if let Some(parent) = desktop_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let desktop_body = DESKTOP_TEMPLATE.replace("@APP_BIN@", &desktop_exec_escape(&app_str));
    fs::write(&desktop_path, desktop_body).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn disable_linux() -> Result<(), String> {
    let _ = Command::new("systemctl")
        .args(["--user", "disable", "--now", "--quiet", UNIT_NAME])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    if let Ok(unit_path) = user_unit_path() {
        let _ = fs::remove_file(unit_path);
    }
    if let Ok(desktop_path) = autostart_desktop_path() {
        let _ = fs::remove_file(desktop_path);
    }
    let _ = Command::new("systemctl")
        .args(["--user", "daemon-reload", "--quiet"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    Ok(())
}

#[cfg(target_os = "linux")]
fn desktop_exec_escape(value: &str) -> String {
    if value.chars().any(|c| c.is_whitespace() || c == '"') {
        format!("\"{}\"", value.replace('"', "\\\""))
    } else {
        value.to_string()
    }
}

#[cfg(target_os = "windows")]
fn enable_windows(app_bin: &Path) -> Result<(), String> {
    let app_path = app_bin.to_string_lossy().replace('\'', "''");

    let script = format!(
        r#"
$ErrorActionPreference = 'Stop'
Unregister-ScheduledTask -TaskName '{task}' -Confirm:$false -ErrorAction SilentlyContinue
$action = New-ScheduledTaskAction -Execute '{app}' -Argument '--background'
$trigger = New-ScheduledTaskTrigger -AtLogOn
$settings = New-ScheduledTaskSettingsSet `
    -AllowStartIfOnBatteries `
    -DontStopIfGoingOnBatteries `
    -StartWhenAvailable `
    -RestartCount 3 `
    -RestartInterval (New-TimeSpan -Minutes 1) `
    -ExecutionTimeLimit (New-TimeSpan -Days 0)
$principal = New-ScheduledTaskPrincipal -UserId $env:USERNAME -LogonType Interactive
Register-ScheduledTask `
    -TaskName '{task}' `
    -Action $action `
    -Trigger $trigger `
    -Settings $settings `
    -Principal $principal `
    -Description 'SleepGuard monitor at logon (Restart on failure)' | Out-Null
"#,
        task = WINDOWS_TASK,
        app = app_path,
    );

    let status = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &script,
        ])
        .status()
        .map_err(|e| format!("No se pudo ejecutar PowerShell: {e}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "No se pudo registrar la tarea {WINDOWS_TASK} (código {status})"
        ))
    }
}

#[cfg(target_os = "windows")]
fn disable_windows() -> Result<(), String> {
    let script = format!(
        "Unregister-ScheduledTask -TaskName '{WINDOWS_TASK}' -Confirm:$false -ErrorAction SilentlyContinue"
    );
    let _ = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &script,
        ])
        .status();
    Ok(())
}
