//! System tray UI.
//!
//! **Windows:** `tray-icon` + `tao` event loop; menu rebuilt each poll with
//! live/dead marks. **Other OS:** headless stdout loop so `cargo check` / CI
//! work without GUI deps.

use crate::ping::Health;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// One row shown in the tray / console.
#[derive(Debug, Clone)]
pub struct StatusRow {
    pub name: String,
    pub health: Health,
    pub detail: String,
}

/// Shared snapshot updated by the poller.
pub type SharedRows = Arc<Mutex<Vec<StatusRow>>>;

pub fn new_shared() -> SharedRows {
    Arc::new(Mutex::new(Vec::new()))
}

/// Format a console line (also used by Linux headless mode).
pub fn format_row(row: &StatusRow) -> String {
    format!(
        "{} {}  ({})",
        row.health.menu_mark(),
        row.name,
        if row.detail.is_empty() {
            row.health.label().to_string()
        } else {
            row.detail.clone()
        }
    )
}

#[cfg(windows)]
mod windows_impl {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Instant;
    use tao::event_loop::{ControlFlow, EventLoopBuilder};
    use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
    use tray_icon::{Icon, TrayIcon, TrayIconBuilder, TrayIconEvent};

    static QUIT_REQUESTED: AtomicBool = AtomicBool::new(false);

    fn icon_rgba() -> Icon {
        let size = 16u32;
        let mut rgba = Vec::with_capacity((size * size * 4) as usize);
        for y in 0..size {
            for x in 0..size {
                let edge = x == 0 || y == 0 || x == size - 1 || y == size - 1;
                if edge {
                    rgba.extend_from_slice(&[30, 64, 175, 255]);
                } else {
                    rgba.extend_from_slice(&[59, 130, 246, 255]);
                }
            }
        }
        Icon::from_rgba(rgba, size, size).expect("icon")
    }

    struct MenuBits {
        menu: Menu,
        quit_id: String,
    }

    fn build_menu(rows: &[StatusRow]) -> Result<MenuBits, String> {
        let menu = Menu::new();
        if rows.is_empty() {
            let item = MenuItem::new("(немає MCP у конфігах)", false, None);
            menu.append(&item).map_err(|e| e.to_string())?;
        } else {
            for row in rows {
                let label = format!(
                    "{} {} — {}",
                    row.health.menu_mark(),
                    row.name,
                    row.health.label()
                );
                let item = MenuItem::new(&label, false, None);
                menu.append(&item).map_err(|e| e.to_string())?;
            }
        }
        menu.append(&PredefinedMenuItem::separator())
            .map_err(|e| e.to_string())?;
        let quit = MenuItem::new("Вийти / Quit", true, None);
        let quit_id = quit.id().as_ref().to_string();
        menu.append(&quit).map_err(|e| e.to_string())?;
        Ok(MenuBits { menu, quit_id })
    }

    /// Blocking tray loop. `on_tick` runs about every `interval`.
    pub fn run_loop<F>(rows: SharedRows, interval: Duration, mut on_tick: F) -> Result<(), String>
    where
        F: 'static + FnMut(),
    {
        let icon = icon_rgba();
        let event_loop = EventLoopBuilder::new().build();
        let menu_channel = MenuEvent::receiver();
        let tray_channel = TrayIconEvent::receiver();

        on_tick();
        let bits = {
            let guard = rows.lock().map_err(|e| e.to_string())?;
            build_menu(&guard)?
        };
        let mut quit_id = bits.quit_id;
        let tray: TrayIcon = TrayIconBuilder::new()
            .with_menu(Box::new(bits.menu))
            .with_tooltip("MCP Status")
            .with_icon(icon)
            .build()
            .map_err(|e| e.to_string())?;

        let mut next = Instant::now() + interval;

        event_loop.run(move |_event, _, control_flow| {
            if QUIT_REQUESTED.load(Ordering::SeqCst) {
                *control_flow = ControlFlow::Exit;
                return;
            }

            let now = Instant::now();
            if now >= next {
                on_tick();
                if let Ok(guard) = rows.lock() {
                    if let Ok(bits) = build_menu(&guard) {
                        quit_id = bits.quit_id;
                        let _ = tray.set_menu(Some(Box::new(bits.menu)));
                        let live = guard.iter().filter(|r| r.health == Health::Live).count();
                        let _ = tray.set_tooltip(Some(&format!(
                            "MCP Status — {live}/{} live",
                            guard.len()
                        )));
                    }
                }
                next = Instant::now() + interval;
            }
            *control_flow = ControlFlow::WaitUntil(next);

            if let Ok(event) = menu_channel.try_recv() {
                if event.id.as_ref() == quit_id {
                    QUIT_REQUESTED.store(true, Ordering::SeqCst);
                    *control_flow = ControlFlow::Exit;
                }
            }
            let _ = tray_channel.try_recv();
        });
    }
}

#[cfg(windows)]
pub use windows_impl::run_loop;

/// Headless monitor for non-Windows.
#[cfg(not(windows))]
pub fn run_loop<F>(rows: SharedRows, interval: Duration, mut on_tick: F) -> Result<(), String>
where
    F: 'static + FnMut(),
{
    loop {
        on_tick();
        if let Ok(guard) = rows.lock() {
            println!("--- mcp-status ---");
            for row in guard.iter() {
                println!("{}", format_row(row));
            }
            if guard.is_empty() {
                println!("(no MCP servers found in config paths)");
            }
        }
        std::thread::sleep(interval);
    }
}
