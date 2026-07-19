use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::{AppHandle, Manager};

/// Register the early-boot / logon activation guard (elevated, once).
pub fn ensure_boot_guard(app: &AppHandle) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        return ensure_boot_guard_linux(app);
    }

    #[cfg(target_os = "windows")]
    {
        return ensure_boot_guard_windows(app);
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = app;
        Err("Agente de arranque no soportado en este SO".into())
    }
}

#[cfg(target_os = "linux")]
fn ensure_boot_guard_linux(app: &AppHandle) -> Result<(), String> {
    let script = find_linux_install_script(app)?;
    let guard_bin = find_guard_binary(app)?;
    let app_bin = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .canonicalize()
        .unwrap_or_else(|_| std::env::current_exe().unwrap());

    // Absolute paths — pkexec resets cwd, so relative paths break in `tauri dev`.
    let status = Command::new("pkexec")
        .arg("env")
        .arg(format!("GUARD_SRC={}", guard_bin.display()))
        .arg(format!("APP_BIN={}", app_bin.display()))
        .arg("bash")
        .arg(script.as_os_str())
        .arg("--guard-only")
        .status()
        .map_err(|e| {
            format!(
                "No se pudo lanzar pkexec ({e}). Instala polkit o ejecuta manualmente:\n  sudo GUARD_SRC={} {} --guard-only",
                guard_bin.display(),
                script.display()
            )
        })?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "Instalación del agente de arranque cancelada o fallida (código {status})"
        ))
    }
}

#[cfg(target_os = "windows")]
fn ensure_boot_guard_windows(app: &AppHandle) -> Result<(), String> {
    let script = find_windows_register_script(app)?;
    let app_bin = std::env::current_exe().map_err(|e| e.to_string())?;
    let app_path = app_bin.to_string_lossy().replace('\'', "''");
    let script_path = script.to_string_lossy().replace('\'', "''");

    let elevate = format!(
        r#"Start-Process -FilePath powershell -Verb RunAs -Wait -ArgumentList '-NoProfile','-ExecutionPolicy','Bypass','-File','{script}','-AppPath','{app}','-GuardOnly'"#,
        script = script_path,
        app = app_path,
    );

    let status = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &elevate,
        ])
        .status()
        .map_err(|e| format!("No se pudo elevar PowerShell: {e}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "Registro del agente de arranque cancelado o fallido (código {status})"
        ))
    }
}

/// `src-tauri` at compile time — used to resolve repo paths in `tauri dev`.
fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn project_root() -> PathBuf {
    manifest_dir()
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(manifest_dir)
}

#[cfg(target_os = "linux")]
fn find_linux_install_script(app: &AppHandle) -> Result<PathBuf, String> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Ok(p) = app
        .path()
        .resolve("install-guard.sh", tauri::path::BaseDirectory::Resource)
    {
        candidates.push(p);
    }
    if let Ok(p) = app
        .path()
        .resolve("linux/install-guard.sh", tauri::path::BaseDirectory::Resource)
    {
        candidates.push(p);
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("install-guard.sh"));
            candidates.push(dir.join("installer/linux/install-guard.sh"));
        }
    }

    candidates.push(project_root().join("installer/linux/install-guard.sh"));
    candidates.push(manifest_dir().join("../installer/linux/install-guard.sh"));

    absolute_existing_file(&candidates).ok_or_else(|| {
        "No se encontró install-guard.sh. Ejecuta desde el repo o reinstala el paquete.".into()
    })
}

#[cfg(target_os = "windows")]
fn find_windows_register_script(app: &AppHandle) -> Result<PathBuf, String> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Ok(p) = app
        .path()
        .resolve("register-guard.ps1", tauri::path::BaseDirectory::Resource)
    {
        candidates.push(p);
    }
    if let Ok(p) = app
        .path()
        .resolve("windows/register-guard.ps1", tauri::path::BaseDirectory::Resource)
    {
        candidates.push(p);
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("register-guard.ps1"));
            candidates.push(dir.join("installer/windows/register-guard.ps1"));
        }
    }

    candidates.push(project_root().join("installer/windows/register-guard.ps1"));
    candidates.push(manifest_dir().join("../installer/windows/register-guard.ps1"));

    absolute_existing_file(&candidates).ok_or_else(|| {
        "No se encontró register-guard.ps1. Ejecuta desde el repo o reinstala el paquete.".into()
    })
}

#[cfg(target_os = "linux")]
fn find_guard_binary(app: &AppHandle) -> Result<PathBuf, String> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("sleepguard-guard"));
        }
    }

    if let Ok(p) = app
        .path()
        .resolve("sleepguard-guard", tauri::path::BaseDirectory::Resource)
    {
        candidates.push(p);
    }

    // Dev / local builds (prefer same profile as the running app).
    candidates.push(manifest_dir().join("target/debug/sleepguard-guard"));
    candidates.push(manifest_dir().join("target/release/sleepguard-guard"));
    candidates.push(project_root().join("installer/linux/sleepguard-guard"));
    candidates.push(PathBuf::from("/usr/local/bin/sleepguard-guard"));
    candidates.push(PathBuf::from("/usr/bin/sleepguard-guard"));

    absolute_existing_file(&candidates).ok_or_else(|| {
        "No se encontró sleepguard-guard. En desarrollo compílalo con:\n  cargo build -p sleepguard-guard --manifest-path src-tauri/Cargo.toml".into()
    })
}

fn absolute_existing_file(candidates: &[PathBuf]) -> Option<PathBuf> {
    for p in candidates {
        if !p.is_file() {
            continue;
        }
        return Some(p.canonicalize().unwrap_or_else(|_| p.clone()));
    }
    None
}
