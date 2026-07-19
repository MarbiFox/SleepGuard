use chrono::{DateTime, Duration, Local};
use serde::Serialize;
use sleepguard_core::{
    config_exists, config_path, day_key, execute_shutdown_delayed, execute_shutdown_now as core_shutdown_now,
    format_hhmm, load_config as core_load, pending_shutdown_lockscreen, resolve_activation,
    save_config as core_save, today_shutdown_target, AppConfig,
};
use std::sync::Mutex;
use std::thread;
use std::time::Duration as StdDuration;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, State, WindowEvent,
};
use tauri_plugin_notification::NotificationExt;

mod monitor_autostart;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum LaunchMode {
    Normal,
    Guard { activation: String },
}

#[derive(Debug, Clone, Serialize)]
struct ShutdownLockscreenPayload {
    activation_time: String,
    countdown_secs: u32,
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

#[tauri::command]
fn ensure_monitor_autostart(enabled: bool) -> Result<(), String> {
    monitor_autostart::set_enabled(enabled)
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let open_item = MenuItem::with_id(app, "open", "Abrir", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Salir", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_item, &quit_item])?;

    let icon = app
        .default_window_icon()
        .ok_or("No hay icono de aplicación para la bandeja")?
        .clone();

    TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .tooltip("SleepGuard")
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

fn setup_close_to_tray(app: &tauri::App) {
    if let Some(window) = app.get_webview_window("main") {
        window.clone().on_window_event(move |event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        });
    }
}

fn monitor_loop(app: AppHandle) {
    let mut notified_for: Option<DateTime<Local>> = None;
    let mut fired_for: Option<DateTime<Local>> = None;
    let mut sleep_secs: u64 = 30;

    loop {
        thread::sleep(StdDuration::from_secs(sleep_secs));

        let cfg = match core_load(&config_path()) {
            Ok(c) => c,
            Err(_) => {
                sleep_secs = 30;
                continue;
            }
        };

        let now = Local::now();
        let Some(target) = today_shutdown_target(&cfg, now) else {
            sleep_secs = 30;
            continue;
        };

        let remaining = target - now;
        if remaining > Duration::zero()
            && remaining <= Duration::minutes(15)
            && notified_for != Some(target)
        {
            let body = format!(
                "SleepGuard: el equipo se apagará a las {}",
                format_hhmm(target.time())
            );
            let _ = app
                .notification()
                .builder()
                .title("SleepGuard")
                .body(body)
                .show();
            notified_for = Some(target);
        }

        if let Some(trigger) = pending_shutdown_lockscreen(&cfg, now) {
            if fired_for == Some(trigger.shutdown) {
                sleep_secs = 30;
                continue;
            }

            harden_window(&app);
            let _ = app.emit(
                "show-shutdown-lockscreen",
                ShutdownLockscreenPayload {
                    activation_time: format_hhmm(trigger.next_activation.time()),
                    countdown_secs: 30,
                },
            );
            fired_for = Some(trigger.shutdown);
            sleep_secs = 30;
            continue;
        }

        sleep_secs = if remaining > Duration::zero() && remaining <= Duration::minutes(1)
        {
            1
        } else {
            30
        };
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

fn harden_window(app: &impl Manager<tauri::Wry>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_fullscreen(true);
        let _ = window.set_always_on_top(true);
        let _ = window.set_decorations(false);
        let _ = window.set_closable(false);
        let _ = window.set_skip_taskbar(true);
        let _ = window.set_title("SleepGuard");
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn harden_guard_window(app: &tauri::App) {
    harden_window(app);
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
                setup_tray(app).expect("failed to setup system tray");
                setup_close_to_tray(app);

                let handle = app.handle().clone();
                thread::spawn(move || {
                    monitor_loop(handle);
                });

                if let Ok(cfg) = core_load(&config_path()) {
                    let _ = monitor_autostart::set_enabled(cfg.enabled);
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            execute_shutdown,
            execute_shutdown_now,
            get_launch_mode,
            is_first_launch,
            ensure_monitor_autostart
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
