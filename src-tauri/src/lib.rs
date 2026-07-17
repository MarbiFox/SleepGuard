use chrono::{DateTime, Duration, Local};
use serde::Serialize;
use sleepguard_core::{
    config_exists, config_path, day_key, execute_shutdown_delayed, execute_shutdown_now as core_shutdown_now,
    format_hhmm, load_config as core_load, next_shutdown_event, resolve_activation, save_config as core_save,
    AppConfig,
};
use std::sync::Mutex;
use std::thread;
use std::time::Duration as StdDuration;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_notification::NotificationExt;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum LaunchMode {
    Normal,
    Guard { activation: String },
}

struct AppState {
    launch_mode: LaunchMode,
}

#[tauri::command]
fn load_config() -> Result<AppConfig, String> {
    core_load(&config_path())
}

#[tauri::command]
fn save_config(config: AppConfig) -> Result<(), String> {
    core_save(&config_path(), &config)
}

#[tauri::command]
fn execute_shutdown() -> Result<(), String> {
    execute_shutdown_delayed()
}

#[tauri::command]
fn execute_shutdown_now() -> Result<(), String> {
    core_shutdown_now()
}

#[tauri::command]
fn get_launch_mode(state: State<'_, Mutex<AppState>>) -> Result<LaunchMode, String> {
    Ok(state.lock().map_err(|e| e.to_string())?.launch_mode.clone())
}

#[tauri::command]
fn is_first_launch() -> bool {
    !config_exists(&config_path())
}

fn monitor_loop(app: AppHandle) {
    let mut notified_for: Option<DateTime<Local>> = None;
    let mut fired_for: Option<DateTime<Local>> = None;

    loop {
        thread::sleep(StdDuration::from_secs(30));

        let cfg = match core_load(&config_path()) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let now = Local::now();
        let Some(event) = next_shutdown_event(&cfg, now) else {
            continue;
        };

        // Config change → new event → fresh notify/fire guards
        if notified_for.is_some_and(|n| n != event) {
            notified_for = None;
        }
        if fired_for.is_some_and(|f| f != event) {
            fired_for = None;
        }

        let remaining = event - now;

        if remaining > Duration::zero()
            && remaining <= Duration::minutes(15)
            && notified_for != Some(event)
        {
            let body = format!(
                "SleepGuard: el equipo se apagará a las {}",
                format_hhmm(event.time())
            );
            let _ = app
                .notification()
                .builder()
                .title("SleepGuard")
                .body(body)
                .show();
            notified_for = Some(event);
        }

        if remaining <= Duration::zero() && fired_for != Some(event) {
            let _ = execute_shutdown_delayed();
            fired_for = Some(event);
        }
    }
}

fn parse_guard_arg() -> bool {
    std::env::args().any(|a| a == "--guard")
}

fn resolve_guard_mode() -> LaunchMode {
    let cfg = match core_load(&config_path()) {
        Ok(c) => c,
        Err(_) => return LaunchMode::Normal,
    };

    if !cfg.enabled {
        std::process::exit(0);
    }

    let now = Local::now();
    let today = day_key(now.date_naive());
    let Some(activation) = resolve_activation(&cfg, today) else {
        std::process::exit(0);
    };

    if now.time() >= activation {
        std::process::exit(0);
    }

    LaunchMode::Guard {
        activation: format_hhmm(activation),
    }
}

fn harden_guard_window(app: &tauri::App) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_fullscreen(true);
        let _ = window.set_always_on_top(true);
        let _ = window.set_decorations(false);
        let _ = window.set_closable(false);
        let _ = window.set_skip_taskbar(true);
        let _ = window.set_title("SleepGuard");
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let launch_mode = if parse_guard_arg() {
        resolve_guard_mode()
    } else {
        LaunchMode::Normal
    };
    let is_guard = matches!(launch_mode, LaunchMode::Guard { .. });

    tauri::Builder::default()
        .manage(Mutex::new(AppState { launch_mode }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .setup(move |app| {
            if let Some(window) = app.get_webview_window("main") {
                if let Some(icon) = app.default_window_icon() {
                    let _ = window.set_icon(icon.clone());
                }
            }

            if is_guard {
                harden_guard_window(app);
            } else {
                let handle = app.handle().clone();
                thread::spawn(move || {
                    monitor_loop(handle);
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            execute_shutdown,
            execute_shutdown_now,
            get_launch_mode,
            is_first_launch
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
