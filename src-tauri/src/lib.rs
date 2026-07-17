use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;
use chrono::{Local, Datelike};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub os: String,
    pub enabled: bool,
    pub schedule: ScheduleConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScheduleConfig {
    pub shutdown_default: String,
    pub activation_default: String,
    pub overrides: HashMap<String, OverrideConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct OverrideConfig {
    pub shutdown: String,
    pub activation: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        let mut overrides = HashMap::new();
        // Initialize with empty strings or default values based on frontend state
        let days = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];
        for day in days {
            if day == "fri" {
                overrides.insert(day.to_string(), OverrideConfig { shutdown: "00:30".to_string(), activation: "08:30".to_string() });
            } else if day == "sat" || day == "sun" {
                overrides.insert(day.to_string(), OverrideConfig { shutdown: "".to_string(), activation: "".to_string() });
            } else {
                overrides.insert(day.to_string(), OverrideConfig { shutdown: "23:30".to_string(), activation: "07:00".to_string() });
            }
        }

        Self {
            os: if cfg!(windows) { "windows".to_string() } else { "linux".to_string() },
            enabled: true,
            schedule: ScheduleConfig {
                shutdown_default: "23:30".to_string(),
                activation_default: "07:00".to_string(),
                overrides,
            },
        }
    }
}

fn get_config_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("sleepguard");
    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    path.push("config.json");
    path
}

#[tauri::command]
fn load_config() -> Result<AppConfig, String> {
    let path = get_config_path();
    if path.exists() {
        let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
        let config: AppConfig = serde_json::from_str(&content).map_err(|e| e.to_string())?;
        Ok(config)
    } else {
        Ok(AppConfig::default())
    }
}

#[tauri::command]
fn save_config(config: AppConfig) -> Result<(), String> {
    let path = get_config_path();
    let content = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn execute_shutdown() -> Result<(), String> {
    println!("Ejecutando shutdown...");
    #[cfg(target_os = "windows")]
    {
        Command::new("shutdown")
            .args(["/s", "/t", "60"])
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        // Require pkexec or sudo to shutdown, or if it runs as a service it has rights
        // As a fallback for tests, just use shutdown directly (which works in many modern distros via polkit)
        Command::new("shutdown")
            .args(["-h", "+1"])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        println!("Shutdown no soportado en este SO");
    }

    Ok(())
}

fn monitor_loop() {
    loop {
        thread::sleep(Duration::from_secs(60));
        
        let path = get_config_path();
        if !path.exists() {
            continue;
        }

        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        
        let config: AppConfig = match serde_json::from_str(&content) {
            Ok(c) => c,
            Err(_) => continue,
        };

        if !config.enabled {
            continue;
        }

        let now = Local::now();
        let day_index = now.weekday().num_days_from_monday(); // 0 = Mon, 6 = Sun
        let days = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];
        let today_key = days[day_index as usize];

        let mut shutdown_time = config.schedule.shutdown_default.clone();
        if let Some(override_cfg) = config.schedule.overrides.get(today_key) {
            if !override_cfg.shutdown.is_empty() {
                shutdown_time = override_cfg.shutdown.clone();
            }
        }

        let current_time_str = now.format("%H:%M").to_string();
        
        // Very basic exact minute match
        if current_time_str == shutdown_time {
            let _ = execute_shutdown();
            // Sleep an extra minute to avoid double trigger
            thread::sleep(Duration::from_secs(65));
        }
    }
}

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // Fuerza la asignación del ícono a la ventana principal (Especialmente útil en Linux/Wayland durante dev)
            if let Some(window) = app.get_webview_window("main") {
                if let Some(icon) = app.default_window_icon() {
                    let _ = window.set_icon(icon.clone());
                }
            }

            thread::spawn(|| {
                monitor_loop();
            });
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![load_config, save_config, execute_shutdown])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
