use chrono::{Local, NaiveTime};
use clap::Parser;
use sleepguard_core::{
    config_path, day_key, execute_shutdown_now, format_hhmm, load_config, resolve_activation,
};
use std::io::{self, Write};
use std::process::Command;
use std::thread;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(
    name = "sleepguard-guard",
    about = "SleepGuard early-boot activation guard (fail-open)"
)]
struct Args {
    /// Path to config.json (overrides SLEEPGUARD_CONFIG)
    #[arg(long)]
    config: Option<String>,

    /// Log shutdown instead of executing (also via SLEEPGUARD_DRY_RUN=1)
    #[arg(long)]
    dry_run: bool,

    /// Override "now" as HH:MM for testing
    #[arg(long)]
    now: Option<String>,
}

fn main() {
    let args = Args::parse();

    if args.dry_run {
        std::env::set_var("SLEEPGUARD_DRY_RUN", "1");
    }

    let path = args
        .config
        .map(std::path::PathBuf::from)
        .unwrap_or_else(config_path);

    let cfg = match load_config(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("sleepguard-guard: cannot load config ({e}); fail-open");
            std::process::exit(0);
        }
    };

    if !cfg.enabled {
        println!("sleepguard-guard: disabled; exit 0");
        std::process::exit(0);
    }

    let now = Local::now();
    let current_time = if let Some(ref hhmm) = args.now {
        match NaiveTime::parse_from_str(hhmm.trim(), "%H:%M") {
            Ok(t) => t,
            Err(_) => {
                eprintln!("sleepguard-guard: invalid --now HH:MM");
                std::process::exit(1);
            }
        }
    } else {
        now.time()
    };

    let today = day_key(now.date_naive());
    let activation = match resolve_activation(&cfg, today) {
        Some(t) => t,
        None => {
            println!("sleepguard-guard: no activation time; exit 0");
            std::process::exit(0);
        }
    };

    if current_time >= activation {
        println!(
            "sleepguard-guard: current {} >= activation {}; exit 0",
            format_hhmm(current_time),
            format_hhmm(activation)
        );
        std::process::exit(0);
    }

    let msg = format!(
        "Este equipo estará disponible a las {}",
        format_hhmm(activation)
    );
    println!("{msg}");
    let _ = io::stdout().flush();
    plymouth_message(&msg);

    for remaining in (1..=30).rev() {
        println!("sleepguard-guard: apagando en {remaining}s...");
        let _ = io::stdout().flush();
        thread::sleep(Duration::from_secs(1));
    }

    if let Err(e) = execute_shutdown_now() {
        eprintln!("sleepguard-guard: shutdown failed: {e}");
        // Fail-open: do not hang boot forever
        std::process::exit(0);
    }
}

fn plymouth_message(msg: &str) {
    let _ = Command::new("plymouth")
        .args(["display-message", "--text", msg])
        .status();
}
