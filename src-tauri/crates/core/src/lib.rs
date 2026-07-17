use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, NaiveTime, Timelike};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownMode {
    Delayed,
    Immediate,
}

impl Default for AppConfig {
    fn default() -> Self {
        let mut overrides = HashMap::new();
        let days = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];
        for day in days {
            if day == "fri" {
                overrides.insert(
                    day.to_string(),
                    OverrideConfig {
                        shutdown: "00:30".to_string(),
                        activation: "08:30".to_string(),
                    },
                );
            } else if day == "sat" || day == "sun" {
                overrides.insert(
                    day.to_string(),
                    OverrideConfig {
                        shutdown: String::new(),
                        activation: String::new(),
                    },
                );
            } else {
                overrides.insert(
                    day.to_string(),
                    OverrideConfig {
                        shutdown: "23:30".to_string(),
                        activation: "07:00".to_string(),
                    },
                );
            }
        }

        Self {
            os: if cfg!(windows) {
                "windows".to_string()
            } else {
                "linux".to_string()
            },
            enabled: true,
            schedule: ScheduleConfig {
                shutdown_default: "23:30".to_string(),
                activation_default: "07:00".to_string(),
                overrides,
            },
        }
    }
}

/// Default config path, or `SLEEPGUARD_CONFIG` when set (agent/testing).
pub fn config_path() -> PathBuf {
    if let Ok(custom) = std::env::var("SLEEPGUARD_CONFIG") {
        if !custom.is_empty() {
            return PathBuf::from(custom);
        }
    }

    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("sleepguard");
    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    path.push("config.json");
    path
}

pub fn load_config(path: &Path) -> Result<AppConfig, String> {
    if path.exists() {
        let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| e.to_string())
    } else {
        Ok(AppConfig::default())
    }
}

pub fn save_config(path: &Path, config: &AppConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())?;
    apply_restrictive_permissions(path);
    Ok(())
}

fn apply_restrictive_permissions(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        // Windows: ACL inherited from the user profile is sufficient for v1.0.
    }
}

pub fn config_exists(path: &Path) -> bool {
    path.exists()
}

/// Day key using short form: `"mon"` … `"sun"`.
pub fn day_key(date: NaiveDate) -> &'static str {
    match date.weekday().num_days_from_monday() {
        0 => "mon",
        1 => "tue",
        2 => "wed",
        3 => "thu",
        4 => "fri",
        5 => "sat",
        _ => "sun",
    }
}

fn parse_hhmm(value: &str) -> Option<NaiveTime> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    NaiveTime::parse_from_str(trimmed, "%H:%M").ok()
}

fn resolve_field(cfg: &AppConfig, day: &str, field: impl Fn(&OverrideConfig) -> &str, default: &str) -> Option<NaiveTime> {
    if let Some(override_cfg) = cfg.schedule.overrides.get(day) {
        let value = field(override_cfg);
        if !value.trim().is_empty() {
            return parse_hhmm(value);
        }
    }
    parse_hhmm(default)
}

pub fn resolve_shutdown(cfg: &AppConfig, day: &str) -> Option<NaiveTime> {
    resolve_field(cfg, day, |o| &o.shutdown, &cfg.schedule.shutdown_default)
}

pub fn resolve_activation(cfg: &AppConfig, day: &str) -> Option<NaiveTime> {
    resolve_field(cfg, day, |o| &o.activation, &cfg.schedule.activation_default)
}

/// Next shutdown instant at or after `now` (today's time if still upcoming, else tomorrow's).
pub fn next_shutdown_event(cfg: &AppConfig, now: DateTime<Local>) -> Option<DateTime<Local>> {
    if !cfg.enabled {
        return None;
    }

    let today = now.date_naive();
    let today_key = day_key(today);
    if let Some(time) = resolve_shutdown(cfg, today_key) {
        if let Some(event) = today.and_time(time).and_local_timezone(Local).single() {
            if event >= now {
                return Some(event);
            }
        }
    }

    let tomorrow = today + Duration::days(1);
    let tomorrow_key = day_key(tomorrow);
    let time = resolve_shutdown(cfg, tomorrow_key)?;
    tomorrow
        .and_time(time)
        .and_local_timezone(Local)
        .single()
}

pub fn is_dry_run() -> bool {
    matches!(
        std::env::var("SLEEPGUARD_DRY_RUN").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes") | Ok("YES")
    )
}

pub fn execute_shutdown(mode: ShutdownMode) -> Result<(), String> {
    if is_dry_run() {
        println!(
            "[DRY-RUN] shutdown ({})",
            match mode {
                ShutdownMode::Delayed => "delayed",
                ShutdownMode::Immediate => "immediate",
            }
        );
        return Ok(());
    }

    println!(
        "Ejecutando shutdown ({})...",
        match mode {
            ShutdownMode::Delayed => "delayed",
            ShutdownMode::Immediate => "immediate",
        }
    );

    #[cfg(target_os = "windows")]
    {
        let t = match mode {
            ShutdownMode::Delayed => "60",
            ShutdownMode::Immediate => "0",
        };
        Command::new("shutdown")
            .args(["/s", "/t", t])
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        let args: &[&str] = match mode {
            ShutdownMode::Delayed => &["-h", "+1"],
            ShutdownMode::Immediate => &["-h", "now"],
        };
        Command::new("shutdown")
            .args(args)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        let _ = mode;
        println!("Shutdown no soportado en este SO");
    }

    Ok(())
}

pub fn execute_shutdown_delayed() -> Result<(), String> {
    execute_shutdown(ShutdownMode::Delayed)
}

pub fn execute_shutdown_now() -> Result<(), String> {
    execute_shutdown(ShutdownMode::Immediate)
}

/// Format `HH:MM` for display from a NaiveTime.
pub fn format_hhmm(time: NaiveTime) -> String {
    format!("{:02}:{:02}", time.hour(), time.minute())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn cfg_with(
        shutdown_default: &str,
        activation_default: &str,
        overrides: HashMap<String, OverrideConfig>,
    ) -> AppConfig {
        AppConfig {
            os: "linux".into(),
            enabled: true,
            schedule: ScheduleConfig {
                shutdown_default: shutdown_default.into(),
                activation_default: activation_default.into(),
                overrides,
            },
        }
    }

    #[test]
    fn resolve_uses_override_when_non_empty() {
        let mut overrides = HashMap::new();
        overrides.insert(
            "fri".into(),
            OverrideConfig {
                shutdown: "00:30".into(),
                activation: "08:30".into(),
            },
        );
        let cfg = cfg_with("23:30", "07:00", overrides);

        assert_eq!(
            resolve_shutdown(&cfg, "fri").unwrap(),
            NaiveTime::from_hms_opt(0, 30, 0).unwrap()
        );
        assert_eq!(
            resolve_activation(&cfg, "fri").unwrap(),
            NaiveTime::from_hms_opt(8, 30, 0).unwrap()
        );
    }

    #[test]
    fn resolve_falls_back_on_empty_override() {
        let mut overrides = HashMap::new();
        overrides.insert(
            "sat".into(),
            OverrideConfig {
                shutdown: "".into(),
                activation: "".into(),
            },
        );
        let cfg = cfg_with("23:30", "07:00", overrides);

        assert_eq!(
            resolve_shutdown(&cfg, "sat").unwrap(),
            NaiveTime::from_hms_opt(23, 30, 0).unwrap()
        );
        assert_eq!(
            resolve_activation(&cfg, "sat").unwrap(),
            NaiveTime::from_hms_opt(7, 0, 0).unwrap()
        );
    }

    #[test]
    fn next_shutdown_today_when_still_upcoming() {
        let cfg = cfg_with("23:30", "07:00", HashMap::new());
        // Pick a fixed local datetime: a Monday 20:00
        let now = Local
            .with_ymd_and_hms(2026, 7, 13, 20, 0, 0)
            .single()
            .expect("valid local time");
        assert_eq!(day_key(now.date_naive()), "mon");

        let event = next_shutdown_event(&cfg, now).unwrap();
        assert_eq!(event.date_naive(), now.date_naive());
        assert_eq!(event.time(), NaiveTime::from_hms_opt(23, 30, 0).unwrap());
    }

    #[test]
    fn next_shutdown_tomorrow_when_today_passed() {
        let cfg = cfg_with("23:30", "07:00", HashMap::new());
        let now = Local
            .with_ymd_and_hms(2026, 7, 13, 23, 45, 0)
            .single()
            .expect("valid local time");

        let event = next_shutdown_event(&cfg, now).unwrap();
        assert_eq!(event.date_naive(), now.date_naive() + Duration::days(1));
        assert_eq!(day_key(event.date_naive()), "tue");
        assert_eq!(event.time(), NaiveTime::from_hms_opt(23, 30, 0).unwrap());
    }

    #[test]
    fn next_shutdown_crosses_midnight_with_override() {
        let mut overrides = HashMap::new();
        overrides.insert(
            "tue".into(),
            OverrideConfig {
                shutdown: "00:05".into(),
                activation: "".into(),
            },
        );
        let cfg = cfg_with("23:30", "07:00", overrides);
        // Monday 23:50 → next event is Tuesday 00:05
        let now = Local
            .with_ymd_and_hms(2026, 7, 13, 23, 50, 0)
            .single()
            .expect("valid local time");

        let event = next_shutdown_event(&cfg, now).unwrap();
        assert_eq!(day_key(event.date_naive()), "tue");
        assert_eq!(event.time(), NaiveTime::from_hms_opt(0, 5, 0).unwrap());

        let notify_at = event - Duration::minutes(15);
        assert_eq!(notify_at.time(), NaiveTime::from_hms_opt(23, 50, 0).unwrap());
        assert_eq!(day_key(notify_at.date_naive()), "mon");
    }

    #[test]
    fn next_shutdown_none_when_disabled() {
        let mut cfg = cfg_with("23:30", "07:00", HashMap::new());
        cfg.enabled = false;
        let now = Local
            .with_ymd_and_hms(2026, 7, 13, 20, 0, 0)
            .single()
            .expect("valid local time");
        assert!(next_shutdown_event(&cfg, now).is_none());
    }

    #[test]
    fn save_config_applies_restrictive_permissions() {
        let dir = std::env::temp_dir().join(format!("sg_perm_{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("config.json");
        let _ = fs::remove_file(&path);

        save_config(&path, &AppConfig::default()).unwrap();
        assert!(path.exists());

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }

        let _ = fs::remove_dir_all(&dir);
    }
}
