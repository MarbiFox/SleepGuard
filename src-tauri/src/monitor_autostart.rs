use sleepguard_core::config_path;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const UNIT_NAME: &str = "sleepguard-monitor.service";

#[cfg(target_os = "windows")]
const WINDOWS_TASK: &str = "SleepGuard-Monitor";

const MONITOR_UNIT_TEMPLATE: &str = r#"[Unit]
Description=SleepGuard monitor (user session)
After=graphical-session.target
PartOf=graphical-session.target

[Service]
Type=simple
ExecStart=@APP_BIN@
Restart=always
RestartSec=5
Environment=SLEEPGUARD_CONFIG=@CONFIG_PATH@

[Install]
WantedBy=default.target
"#;

pub fn set_enabled(enabled: bool) -> Result<(), String> {
    if enabled {
        enable()
    } else {
        disable()
    }
}

fn enable() -> Result<(), String> {
    #[cfg(target_os = "linux")]
    return enable_linux();

    #[cfg(target_os = "windows")]
    return enable_windows();

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    Err("Autostart del monitor no soportado en este SO".into())
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
fn enable_linux() -> Result<(), String> {
    let app_bin = std::env::current_exe().map_err(|e| e.to_string())?;
    let config = config_path();
    let unit_body = MONITOR_UNIT_TEMPLATE
        .replace("@APP_BIN@", &shell_escape(&app_bin.to_string_lossy()))
        .replace("@CONFIG_PATH@", &shell_escape(&config.to_string_lossy()));

    let unit_path = user_unit_path()?;
    if let Some(parent) = unit_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&unit_path, unit_body).map_err(|e| e.to_string())?;

    run_systemctl(&["--user", "daemon-reload"])?;
    run_systemctl(&["--user", "enable", UNIT_NAME])?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn disable_linux() -> Result<(), String> {
    let _ = run_systemctl(&["--user", "disable", "--now", UNIT_NAME]);
    if let Ok(unit_path) = user_unit_path() {
        let _ = fs::remove_file(unit_path);
    }
    let _ = run_systemctl(&["--user", "daemon-reload"]);
    Ok(())
}

#[cfg(target_os = "linux")]
fn run_systemctl(args: &[&str]) -> Result<(), String> {
    let status = Command::new("systemctl")
        .args(args)
        .status()
        .map_err(|e| format!("No se pudo ejecutar systemctl: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("systemctl {} falló con código {}", args.join(" "), status))
    }
}

#[cfg(target_os = "linux")]
fn shell_escape(value: &str) -> String {
    if value.contains(' ') || value.contains('"') {
        format!("\"{}\"", value.replace('"', "\\\""))
    } else {
        value.to_string()
    }
}

#[cfg(target_os = "windows")]
fn enable_windows() -> Result<(), String> {
    let app_bin = std::env::current_exe().map_err(|e| e.to_string())?;
    let app_path = app_bin.to_string_lossy().replace('\'', "''");

    let script = format!(
        r#"
$ErrorActionPreference = 'Stop'
Unregister-ScheduledTask -TaskName '{task}' -Confirm:$false -ErrorAction SilentlyContinue
$action = New-ScheduledTaskAction -Execute '{app}'
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
