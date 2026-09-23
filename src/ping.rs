//! Liveness checks for configured MCP servers.
//!
//! ## HTTP / SSE (`url`)
//! Parse host + port from the URL and attempt a **TCP connect** with a short
//! timeout. This does **not** speak MCP or HTTP — open port ⇒ “live”.
//!
//! ## stdio (`command`)
//! Heuristic: look for a running process whose name matches the command basename
//! (e.g. `npx`, `node`, `exchange-mcp`). Limitations:
//! - Shared names (`node`, `python`, `npx`) collide with unrelated processes.
//! - Short-lived MCP children may not be visible between polls.
//! - Path-qualified commands are matched by file stem only.
//! When no matching process is found we return [`Health::Unknown`] (configured,
//! not confirmed), not [`Health::Dead`].

use crate::config::{McpServer, Transport};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;
use sysinfo::{ProcessesToUpdate, System};

/// Observed liveness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Health {
    Live,
    Dead,
    /// Config present; cannot confirm (stdio with no process match, or bad URL).
    Unknown,
}

impl Health {
    pub fn label(self) -> &'static str {
        match self {
            Health::Live => "live",
            Health::Dead => "dead",
            Health::Unknown => "unknown",
        }
    }

    pub fn menu_mark(self) -> &'static str {
        match self {
            Health::Live => "●",
            Health::Dead => "○",
            Health::Unknown => "?",
        }
    }
}

const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);

/// Ping one server.
pub fn check(server: &McpServer, sys: &mut System) -> Health {
    match &server.transport {
        Transport::Url { url } => ping_url(url),
        Transport::Stdio { command, .. } => ping_stdio(command, sys),
        Transport::Unknown => Health::Unknown,
    }
}

fn ping_url(url: &str) -> Health {
    let Some((host, port)) = host_port(url) else {
        return Health::Unknown;
    };
    let addr = format!("{host}:{port}");
    match addr.to_socket_addrs() {
        Ok(addrs) => {
            for a in addrs {
                if try_connect(a) {
                    return Health::Live;
                }
            }
            Health::Dead
        }
        Err(_) => Health::Dead,
    }
}

/// Minimal host:port extraction from http(s)/ws(s) URLs without an extra crate.
fn host_port(url: &str) -> Option<(String, u16)> {
    let url = url.trim();
    let rest = url
        .strip_prefix("https://")
        .map(|r| (r, 443u16))
        .or_else(|| url.strip_prefix("http://").map(|r| (r, 80)))
        .or_else(|| url.strip_prefix("wss://").map(|r| (r, 443)))
        .or_else(|| url.strip_prefix("ws://").map(|r| (r, 80)))?;
    let (after_scheme, default_port) = rest;
    let authority = after_scheme.split(['/', '?', '#']).next().unwrap_or("");
    if authority.is_empty() {
        return None;
    }
    // [IPv6]:port or host:port or host
    if let Some(inner) = authority.strip_prefix('[') {
        let (host, rem) = inner.split_once(']')?;
        let port = if let Some(p) = rem.strip_prefix(':') {
            p.parse().ok()?
        } else {
            default_port
        };
        return Some((host.to_string(), port));
    }
    if let Some((host, port_s)) = authority.rsplit_once(':') {
        if !host.is_empty() && port_s.chars().all(|c| c.is_ascii_digit()) {
            let port: u16 = port_s.parse().ok()?;
            return Some((host.to_string(), port));
        }
    }
    Some((authority.to_string(), default_port))
}

fn try_connect(addr: SocketAddr) -> bool {
    TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT).is_ok()
}

fn ping_stdio(command: &str, sys: &mut System) -> Health {
    let base = basename(command);
    let needle = file_stem(&base).to_ascii_lowercase();
    if needle.is_empty() {
        return Health::Unknown;
    }
    sys.refresh_processes(ProcessesToUpdate::All, true);
    for proc in sys.processes().values() {
        let name = proc.name().to_string_lossy().to_ascii_lowercase();
        let name = file_stem(&name);
        if name == needle || name.contains(&needle) {
            return Health::Live;
        }
        if let Some(exe) = proc.exe() {
            if let Some(stem) = exe.file_stem() {
                let stem = stem.to_string_lossy().to_ascii_lowercase();
                if stem == needle {
                    return Health::Live;
                }
            }
        }
    }
    Health::Unknown
}

fn basename(command: &str) -> String {
    let t = command.trim().trim_matches('"');
    t.rsplit(['/', '\\'])
        .next()
        .unwrap_or(t)
        .split_whitespace()
        .next()
        .unwrap_or(t)
        .to_string()
}

fn file_stem(name: &str) -> &str {
    name.strip_suffix(".exe")
        .or_else(|| name.strip_suffix(".EXE"))
        .unwrap_or(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_host_port() {
        assert_eq!(
            host_port("http://127.0.0.1:3000/mcp"),
            Some(("127.0.0.1".into(), 3000))
        );
        assert_eq!(
            host_port("https://example.com/sse"),
            Some(("example.com".into(), 443))
        );
        assert_eq!(
            host_port("http://localhost/mcp"),
            Some(("localhost".into(), 80))
        );
    }

    #[test]
    fn basename_windows_path() {
        assert_eq!(basename(r"C:\Tools\exchange-mcp.exe"), "exchange-mcp.exe");
        assert_eq!(basename("npx"), "npx");
        assert_eq!(file_stem("exchange-mcp.exe"), "exchange-mcp");
    }
}
