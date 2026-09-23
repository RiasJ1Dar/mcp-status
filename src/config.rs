//! Discover and parse MCP server entries from common client configs.
//!
//! Scanned locations are merged by name (first wins). Diagnostics list every
//! candidate path (found or missing).
//!
//! | Client | Windows | Linux / macOS |
//! |---|---|---|
//! | Cursor (global) | `%USERPROFILE%\.cursor\mcp.json` | `~/.cursor/mcp.json` |
//! | Cursor (legacy) | `%APPDATA%\Cursor\User\globalStorage\cursor.mcp\settings.json` | `~/.config/Cursor/.../cursor.mcp/settings.json` |
//! | Claude Code | `%USERPROFILE%\.claude.json`, `%USERPROFILE%\.claude\settings.json` | same under `$HOME` |
//! | Claude Desktop | `%APPDATA%\Claude\claude_desktop_config.json` | Application Support / `.config/Claude` |
//! | Jan | `%APPDATA%\Jan\data\mcp_config.json` | `~/.config/Jan/data/mcp_config.json` |
//! | AnythingLLM | `%APPDATA%\anythingllm-desktop\storage\plugins\anythingllm_mcp_servers.json` | under config dir |
//!
//! Grok Bot account MCP plugins live in the cloud catalog, not a local
//! `mcpServers` JSON — they are not discoverable from disk today.
//! Project-local `.cursor/mcp.json` is **not** scanned (no workspace root).

use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// How an MCP server is launched / reached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Transport {
    /// HTTP or SSE URL from config `url` field.
    Url { url: String },
    /// Local process via `command` (+ optional `args`).
    Stdio {
        command: String,
        args: Vec<String>,
    },
    /// Present in JSON but neither `url` nor `command`.
    Unknown,
}

/// One named MCP server from a config file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpServer {
    pub name: String,
    pub transport: Transport,
    pub source: PathBuf,
}

#[derive(Debug, Deserialize)]
struct Root {
    #[serde(rename = "mcpServers", default)]
    mcp_servers: BTreeMap<String, ServerEntry>,
}

#[derive(Debug, Deserialize)]
struct ServerEntry {
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    args: Option<Vec<String>>,
}

/// Candidate config file paths for this OS (order = priority for duplicate names).
pub fn candidate_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(home) = dirs::home_dir() {
        out.push(home.join(".cursor").join("mcp.json"));
        // Claude Code stores mcpServers in the user JSON (not only Desktop).
        out.push(home.join(".claude.json"));
        out.push(home.join(".claude").join("settings.json"));
    }
    if let Some(config) = dirs::config_dir() {
        out.push(
            config
                .join("Cursor")
                .join("User")
                .join("globalStorage")
                .join("cursor.mcp")
                .join("settings.json"),
        );
        out.push(config.join("Claude").join("claude_desktop_config.json"));
        out.push(config.join("Claude Code").join("settings.json"));
        out.push(config.join("Jan").join("data").join("mcp_config.json"));
        out.push(
            config
                .join("anythingllm-desktop")
                .join("storage")
                .join("plugins")
                .join("anythingllm_mcp_servers.json"),
        );
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = dirs::home_dir() {
            out.push(
                home.join("Library")
                    .join("Application Support")
                    .join("Claude")
                    .join("claude_desktop_config.json"),
            );
            out.push(
                home.join("Library")
                    .join("Application Support")
                    .join("Claude Code")
                    .join("settings.json"),
            );
            out.push(
                home.join("Library")
                    .join("Application Support")
                    .join("Jan")
                    .join("data")
                    .join("mcp_config.json"),
            );
        }
    }
    out
}

/// Load and merge all readable configs. Duplicate names: first wins.
pub fn load_all() -> Vec<McpServer> {
    let mut by_name: BTreeMap<String, McpServer> = BTreeMap::new();
    for path in candidate_paths() {
        if !path.is_file() {
            continue;
        }
        match parse_file(&path) {
            Ok(servers) => {
                for s in servers {
                    by_name.entry(s.name.clone()).or_insert(s);
                }
            }
            Err(e) => {
                eprintln!("mcp-status: skip {}: {e}", path.display());
            }
        }
    }
    by_name.into_values().collect()
}

/// Parse one JSON file that contains `mcpServers`.
pub fn parse_file(path: &Path) -> Result<Vec<McpServer>, String> {
    let mut text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    // Windows editors / PowerShell often write UTF-8 with BOM.
    if text.starts_with('\u{feff}') {
        text = text.trim_start_matches('\u{feff}').to_string();
    }
    parse_json(&text, path)
}

fn parse_json(text: &str, source: &Path) -> Result<Vec<McpServer>, String> {
    // Prefer `{ "mcpServers": { ... } }`. A bare map of servers is accepted only
    // when every value is an object (legacy). Files without mcpServers and with
    // unrelated string fields (e.g. Claude Desktop prefs) return empty, not error.
    let value: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let root: Root = if value.get("mcpServers").is_some() {
        // Deserialize mcpServers map entry-by-entry so one bad value cannot
        // drop the whole file (and so string fields next to mcpServers are fine).
        let map = value
            .get("mcpServers")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();
        let mut mcp_servers = BTreeMap::new();
        for (name, ent) in map {
            match serde_json::from_value::<ServerEntry>(ent) {
                Ok(e) => {
                    mcp_servers.insert(name, e);
                }
                Err(err) => {
                    eprintln!(
                        "mcp-status: skip entry «{name}» in {}: {err}",
                        source.display()
                    );
                }
            }
        }
        Root { mcp_servers }
    } else if value.is_object()
        && value
            .as_object()
            .map(|m| !m.is_empty() && m.values().all(|v| v.is_object()))
            .unwrap_or(false)
    {
        Root {
            mcp_servers: serde_json::from_value(value).map_err(|e| e.to_string())?,
        }
    } else {
        Root {
            mcp_servers: BTreeMap::new(),
        }
    };

    let mut out = Vec::new();
    for (name, entry) in root.mcp_servers {
        let url = entry
            .url
            .filter(|u| !u.trim().is_empty());
        let command = entry
            .command
            .filter(|c| !c.trim().is_empty());
        let transport = if let Some(url) = url {
            Transport::Url { url }
        } else if let Some(command) = command {
            Transport::Stdio {
                command,
                args: entry.args.unwrap_or_default(),
            }
        } else {
            Transport::Unknown
        };
        out.push(McpServer {
            name,
            transport,
            source: source.to_path_buf(),
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_claude_prefs_without_mcp_servers() {
        let json = r#"{
            "coworkUserFilesPath": "C:\\Users\\Viktor\\Claude",
            "preferences": { "sidebarMode": "chat" }
        }"#;
        let servers = parse_json(json, Path::new("claude_desktop_config.json")).unwrap();
        assert!(servers.is_empty());
    }

    #[test]
    fn parses_claude_code_user_json() {
        let json = r#"{
            "numStartups": 3,
            "mcpServers": {
                "codebase-memory-mcp": {
                    "command": "C:/Users/Viktor/AppData/Local/Programs/codebase-memory-mcp/codebase-memory-mcp.exe",
                    "args": []
                }
            },
            "theme": "dark"
        }"#;
        let servers = parse_json(json, Path::new(".claude.json")).unwrap();
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].name, "codebase-memory-mcp");
        assert!(matches!(servers[0].transport, Transport::Stdio { .. }));
    }

    #[test]
    fn prefers_url_when_command_empty() {
        let json = r#"{
            "mcpServers": {
                "exa": { "type": "http", "url": "https://mcp.exa.ai/mcp", "command": "", "args": [] }
            }
        }"#;
        let servers = parse_json(json, Path::new("mcp_config.json")).unwrap();
        assert_eq!(servers.len(), 1);
        match &servers[0].transport {
            Transport::Url { url } => assert!(url.contains("exa.ai")),
            _ => panic!("expected url"),
        }
    }

    #[test]
    fn parses_url_and_stdio() {
        let json = r#"{
            "mcpServers": {
                "remote": { "url": "http://127.0.0.1:3000/mcp" },
                "local": { "command": "npx", "args": ["-y", "foo"] }
            }
        }"#;
        let servers = parse_json(json, Path::new("test.json")).unwrap();
        assert_eq!(servers.len(), 2);
        let remote = servers.iter().find(|s| s.name == "remote").unwrap();
        assert!(matches!(remote.transport, Transport::Url { .. }));
        let local = servers.iter().find(|s| s.name == "local").unwrap();
        match &local.transport {
            Transport::Stdio { command, args } => {
                assert_eq!(command, "npx");
                assert_eq!(args, &["-y", "foo"]);
            }
            _ => panic!("expected stdio"),
        }
    }
}
