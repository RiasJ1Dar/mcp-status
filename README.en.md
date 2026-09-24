[Українська](README.md) · **English**

# mcp-status

A Rust Windows tray application that collects local `mcpServers` entries from
common client configurations, displays their state, and sends a toast only when
that state changes. It builds as one binary with no separate runtime.

Step-by-step setup: **[INSTRUCTIONS.md](INSTRUCTIONS.md)**.
Ukrainian setup: [ІНСТРУКЦІЯ.md](ІНСТРУКЦІЯ.md).

## What is checked

| Configuration | Probe | Result |
|---|---|---|
| `http(s)` or `ws(s)` `url` | TCP connect to host:port, 2-second timeout | `live` or `dead` |
| stdio `command` | Process search by command basename / stem | `live` or `unknown` |
| entry without `url` or `command` | Transport cannot be checked | `unknown` |
| agent-exchange URL | HTTP GET, 2xx/3xx | `live` or `dead` |

The TCP probe does not perform an MCP handshake or validate a specific HTTP
route. An open port only proves transport-level reachability.

Tray marks:

- `●` — live;
- `○` — dead;
- `?` — configured, but not confirmed.

## Installation

A Rust toolchain is required. Windows MSVC builds also need Visual Studio Build
Tools with the C++ component.

```bash
cargo install --git https://github.com/RiasJ1Dar/mcp-status
```

Or build from a clone:

```bash
git clone https://github.com/RiasJ1Dar/mcp-status.git
cd mcp-status
cargo build --release
```

The binary is `target/release/mcp-status.exe` on Windows and
`target/release/mcp-status` elsewhere.

## Quick start

Inspect discovered configs without starting the tray:

```bash
mcp-status --once
```

The command prints servers, health states, and every candidate path with a mark
showing whether the file exists.

Run the tray on Windows:

```bash
mcp-status
```

Toasts are suppressed for the first 15 seconds to avoid a burst at startup.
After that, a toast is emitted only when a health state changes.

## Commands

| Flag | Action |
|---|---|
| `--once` | Scan and probe once, print, then exit |
| `--autostart-on` | Add the current executable to Windows Run |
| `--autostart-off` | Remove the Windows Run entry |
| `--autostart-status` | Print autostart status |
| `-h`, `--help` | Print help |

On Linux/macOS, running without `--once` uses a headless loop. Windows GUI
dependencies are gated with `cfg(windows)`.

## Configuration sources

The parser accepts a JSON `mcpServers` object. A legacy bare map is also
accepted when every value is an object.

| Client | Windows | Linux / macOS |
|---|---|---|
| Cursor | `%USERPROFILE%\.cursor\mcp.json` | `~/.cursor/mcp.json` |
| Cursor legacy | `%APPDATA%\Cursor\User\globalStorage\cursor.mcp\settings.json` | matching config directory |
| Claude Code | `%USERPROFILE%\.claude.json`, `%USERPROFILE%\.claude\settings.json` | same paths under `$HOME` |
| Claude Desktop | `%APPDATA%\Claude\claude_desktop_config.json` | Application Support / `.config/Claude` |
| Jan | `%APPDATA%\Jan\data\mcp_config.json` | matching config directory |
| AnythingLLM | `%APPDATA%\anythingllm-desktop\storage\plugins\anythingllm_mcp_servers.json` | matching config directory |

Project-local `.cursor/mcp.json` files are not scanned because the application
has no workspace root.

## Manual list and precedence

For Grok Bot or another MCP absent from local client configs, create one of:

- the path in `MCP_STATUS_CONFIG`;
- `%USERPROFILE%\.mcp-status.json`;
- `%APPDATA%\mcp-status\mcp-status.json`.

See [mcp-status.example.json](mcp-status.example.json).

```json
{
  "mcpServers": {
    "local-http": {
      "url": "http://127.0.0.1:3000/mcp"
    },
    "local-stdio": {
      "command": "C:/Tools/exchange-mcp.exe",
      "args": []
    },
    "catalog-only": {}
  }
}
```

Files are read in priority order; the first duplicate name wins. Manual files
come before client configurations. One malformed entry is reported to stderr
and skipped without hiding the rest of the file. UTF-8 BOM is supported.

## Environment variables

| Variable | Default | Purpose |
|---|---|---|
| `MCP_STATUS_INTERVAL_SECS` | `10` | Poll interval |
| `MCP_STATUS_TOAST_SUPPRESS_SECS` | `15` | Toast mute period after launch |
| `MCP_STATUS_EXCHANGE_URL` | `http://127.0.0.1:9750/` | agent-exchange URL; empty disables the probe |
| `MCP_STATUS_CONFIG` | — | Highest-priority additional JSON file |
| `MCP_STATUS_AUTOSTART` | — | If present, register Windows Run during launch |
| `MCP_STATUS_TOAST_LOG` | — | On non-Windows, print toast events to stderr |

Disable the agent-exchange probe when it is not used:

```powershell
$env:MCP_STATUS_EXCHANGE_URL = ""
mcp-status --once
```

## Limitations

- Shared process names such as `node`, `python`, and `npx` can produce a
  false `live`.
- A short-lived stdio process can disappear between polls.
- The stdio check does not inspect the complete command line or `args`.
- URL checks do not perform an MCP handshake.
- The application observes servers; it does not start, stop, or restart them.

## Development

```bash
cargo fmt --check
cargo test
cargo check
cargo clippy --all-targets -- -D warnings
```

CI checks the Linux build without Windows GUI dependencies.

## License

MIT