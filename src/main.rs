//! mcp-status — tray monitor for local MCP servers (Windows MVP).
//!
//! Linux/macOS: headless stdout loop so CI `cargo check` / `--once` works.

mod autostart;
mod config;
mod exchange;
mod ping;
mod toast;
mod tray;

use ping::Health;
use std::collections::HashMap;
use std::env;
use std::time::Duration;
use sysinfo::System;
use toast::ToastGate;
use tray::{SharedRows, StatusRow};

const DEFAULT_INTERVAL_SECS: u64 = 10;
const APP_NAME: &str = "mcp-status";

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print_help();
        return;
    }

    if args.iter().any(|a| a == "--autostart-on") {
        match env::current_exe() {
            Ok(exe) => match autostart::enable(&exe, "") {
                Ok(()) => println!("autostart enabled"),
                Err(e) => {
                    eprintln!("autostart-on: {e}");
                    std::process::exit(1);
                }
            },
            Err(e) => {
                eprintln!("current_exe: {e}");
                std::process::exit(1);
            }
        }
        return;
    }
    if args.iter().any(|a| a == "--autostart-off") {
        match autostart::disable() {
            Ok(()) => println!("autostart disabled"),
            Err(e) => {
                eprintln!("autostart-off: {e}");
                std::process::exit(1);
            }
        }
        return;
    }
    if args.iter().any(|a| a == "--autostart-status") {
        match autostart::is_enabled() {
            Ok(v) => println!("autostart: {}", if v { "on" } else { "off" }),
            Err(e) => {
                eprintln!("autostart-status: {e}");
                std::process::exit(1);
            }
        }
        return;
    }

    let once = args.iter().any(|a| a == "--once");
    let interval = Duration::from_secs(
        env::var("MCP_STATUS_INTERVAL_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_INTERVAL_SECS),
    );
    let suppress = env::var("MCP_STATUS_TOAST_SUPPRESS_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .map(Duration::from_secs)
        .unwrap_or(toast::STARTUP_SUPPRESS);

    let rows = tray::new_shared();

    if once {
        let mut prev = HashMap::new();
        let mut sys = System::new();
        let gate = ToastGate::new(Duration::from_secs(3600));
        poll_once(&rows, &mut prev, &gate, &mut sys);
        if let Ok(guard) = rows.lock() {
            for row in guard.iter() {
                println!("{}", tray::format_row(row));
            }
            if guard.is_empty() {
                println!("(no MCP servers found in config paths)");
            }
        }
        println!("candidates:");
        for p in config::candidate_paths() {
            let mark = if p.is_file() { "✓" } else { "·" };
            println!("  {mark} {}", p.display());
        }
        return;
    }

    if env::var_os("MCP_STATUS_AUTOSTART").is_some() {
        if let Ok(exe) = env::current_exe() {
            let _ = autostart::enable(&exe, "");
        }
    }

    eprintln!(
        "{APP_NAME}: polling every {}s (toast suppress {}s)",
        interval.as_secs(),
        suppress.as_secs()
    );

    let gate = ToastGate::new(suppress);
    let mut prev: HashMap<String, Health> = HashMap::new();
    let mut sys = System::new();
    let rows_for_loop = rows.clone();
    if let Err(e) = tray::run_loop(rows, interval, move || {
        poll_once(&rows_for_loop, &mut prev, &gate, &mut sys);
    }) {
        eprintln!("{APP_NAME}: tray error: {e}");
        std::process::exit(1);
    }
}

fn poll_once(
    rows: &SharedRows,
    prev: &mut HashMap<String, Health>,
    gate: &ToastGate,
    sys: &mut System,
) {
    let servers = config::load_all();
    let mut next_rows: Vec<StatusRow> = Vec::new();

    for s in &servers {
        let health = ping::check(s, sys);
        let detail = match &s.transport {
            config::Transport::Url { url } => format!("tcp {url}"),
            config::Transport::Stdio { command, .. } => format!("stdio {command}"),
            config::Transport::Unknown => "unknown transport".into(),
        };
        let old = prev.insert(s.name.clone(), health);
        gate.on_change(&s.name, old, health);
        next_rows.push(StatusRow {
            name: s.name.clone(),
            health,
            detail,
        });
    }

    if let Some((url, health)) = exchange::check() {
        let name = "agent-exchange".to_string();
        let old = prev.insert(name.clone(), health);
        gate.on_change(&name, old, health);
        next_rows.push(StatusRow {
            name,
            health,
            detail: format!("GET {url}"),
        });
    }

    if let Ok(mut guard) = rows.lock() {
        *guard = next_rows;
    }
}

fn print_help() {
    println!(
        "\
{APP_NAME} — live/dead tray for local MCP servers

Usage:
  {APP_NAME}              Run tray (Windows) or headless stdout loop
  {APP_NAME} --once       Scan + ping once, print, exit (CI-friendly)
  {APP_NAME} --autostart-on|off|status
  {APP_NAME} -h | --help

Env:
  MCP_STATUS_INTERVAL_SECS       Poll interval (default 10)
  MCP_STATUS_TOAST_SUPPRESS_SECS Startup toast mute (default 15)
  MCP_STATUS_EXCHANGE_URL        agent-exchange URL (default http://127.0.0.1:9750/; empty=off)
  MCP_STATUS_AUTOSTART           If set, register Windows Run key on launch
  MCP_STATUS_TOAST_LOG           Non-Windows: print toast lines to stderr
"
    );
}
