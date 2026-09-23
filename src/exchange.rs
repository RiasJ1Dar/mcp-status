//! Optional agent-exchange board probe.
//!
//! Default URL: `http://127.0.0.1:9750/` (agent-exchange / exchange-ui read-only
//! HTML). Override with env `MCP_STATUS_EXCHANGE_URL`. Empty string disables.
//!
//! A successful TCP+HTTP GET (any 2xx/3xx) ⇒ live. Connection errors ⇒ dead.
//! This is a stub-friendly health check, not a full HTML scrape.

use crate::ping::Health;
use std::env;
use std::time::Duration;

const DEFAULT_URL: &str = "http://127.0.0.1:9750/";
const TIMEOUT: Duration = Duration::from_secs(2);

/// Resolved exchange probe target, if enabled.
pub fn configured_url() -> Option<String> {
    match env::var("MCP_STATUS_EXCHANGE_URL") {
        Ok(v) if v.trim().is_empty() => None,
        Ok(v) => Some(v),
        Err(_) => Some(DEFAULT_URL.to_string()),
    }
}

/// GET the exchange health/board URL.
pub fn check() -> Option<(String, Health)> {
    let url = configured_url()?;
    let health = http_get_ok(&url);
    Some((url, health))
}

fn http_get_ok(url: &str) -> Health {
    let agent = ureq::AgentBuilder::new().timeout(TIMEOUT).build();
    match agent.get(url).call() {
        Ok(resp) => {
            let code = resp.status();
            if (200..400).contains(&code) {
                Health::Live
            } else {
                Health::Dead
            }
        }
        Err(ureq::Error::Status(code, _)) if (200..400).contains(&code) => Health::Live,
        Err(_) => Health::Dead,
    }
}
