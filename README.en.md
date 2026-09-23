[Українська](README.md) · **English**

# mcp-status

Windows tray app (Rust, single binary) that shows **live/dead** status for local
MCP servers from Cursor / Claude Desktop configs and fires toasts only on state
change.

MVP 0.1 scaffold: config parse, TCP/stdio checks, tray (Windows), autostart,
startup-suppressed toasts. On Linux/macOS GUI crates are `cfg(windows)`-gated;
headless / `--once` works for CI.

## Behaviour

1. Reads `mcpServers` from common JSON configs.
2. Periodically checks each server:
   - **HTTP/SSE (`url`)** — TCP connect to host:port (not MCP protocol).
   - **stdio (`command`)** — process-name heuristic; no match → `unknown`, not `dead`.
3. Tray menu with ● / ○ / ? per MCP.
4. Windows toast **only on change**; first N seconds after launch are silent
   (default 15s).
5. Optional GET to agent-exchange UI (`http://127.0.0.1:9750/` or
   `MCP_STATUS_EXCHANGE_URL`).
6. Windows autostart via `HKCU\...\Run` (same idea as desktop-remote-kit).

## Config paths

| Client | Windows | Linux / macOS |
|---|---|---|
| Cursor | `%USERPROFILE%\.cursor\mcp.json` | `~/.cursor/mcp.json` |
| Cursor (legacy) | `%APPDATA%\Cursor\User\globalStorage\cursor.mcp\settings.json` | `~/.config/Cursor/.../cursor.mcp/settings.json` |
| Claude Desktop | `%APPDATA%\Claude\claude_desktop_config.json` | macOS Application Support / Linux `~/.config/Claude/...` |

Project-local `.cursor/mcp.json` is **not** scanned.

## stdio heuristic limits

- Shared names (`node`, `python`, `npx`) false-positive easily.
- Short-lived children may be missed between polls.
- Only command basename / stem is matched.

## Build

```bash
cargo build --release
cargo run -- --once
cargo test
cargo check
```

See the Ukrainian README for flags and environment variables.

## License

MIT
