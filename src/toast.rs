//! Windows toast on MCP state change.
//!
//! Rules:
//! - Notify **only** on transition after the first observation (baseline).
//! - Suppress all toasts for [`STARTUP_SUPPRESS`] after process start (avoids
//!   a flood when many servers are first observed).
//! - Non-Windows: no-op unless `MCP_STATUS_TOAST_LOG=1` (eprintln).

use crate::ping::Health;
use std::time::{Duration, Instant};

/// Default grace period after launch before toasts are allowed.
pub const STARTUP_SUPPRESS: Duration = Duration::from_secs(15);

/// Toast gate + last-known map helper.
pub struct ToastGate {
    started: Instant,
    suppress: Duration,
}

impl ToastGate {
    pub fn new(suppress: Duration) -> Self {
        Self {
            started: Instant::now(),
            suppress,
        }
    }

    pub fn allow(&self) -> bool {
        self.started.elapsed() >= self.suppress
    }

    /// Fire a toast if allowed and health actually changed.
    pub fn on_change(&self, name: &str, from: Option<Health>, to: Health) {
        // First observation establishes baseline — no toast.
        let Some(from) = from else {
            return;
        };
        if from == to {
            return;
        }
        if !self.allow() {
            return;
        }
        let title = "MCP Status";
        let body = format!("{name}: {} → {}", from.label(), to.label());
        show(title, &body);
    }
}

fn show(title: &str, body: &str) {
    #[cfg(windows)]
    {
        use winrt_notification::Toast;
        let _ = Toast::new(Toast::POWERSHELL_APP_ID)
            .title(title)
            .text1(body)
            .show();
    }
    #[cfg(not(windows))]
    {
        if std::env::var_os("MCP_STATUS_TOAST_LOG").is_some() {
            eprintln!("toast: {title}: {body}");
        }
    }
}
