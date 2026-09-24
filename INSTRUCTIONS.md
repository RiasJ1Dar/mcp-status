# Instructions: mcp-status

Step-by-step: build, run, auto-discovered configs, manual list for **Grok Bot**, indicators, toast, and autostart.

Product overview: [README.en.md](README.en.md). Українська: [ІНСТРУКЦІЯ.md](ІНСТРУКЦІЯ.md).

---

## 1. Requirements

- Windows 10/11 (tray, toast, autostart).
- [Rust](https://rustup.rs/) (`cargo` on PATH).
- Linux/macOS: `cargo test` / `mcp-status --once` only (no tray).

---

## 2. Build

```powershell
git clone https://github.com/RiasJ1Dar/mcp-status.git
cd mcp-status
cargo build --release
```

Binary: `target\release\mcp-status.exe`.

```powershell
.\target\release\mcp-status.exe --once
```

---

## 3. Run on Windows

| Action | Command |
|---|---|
| Tray (stay running) | `.\target\release\mcp-status.exe` |
| One console snapshot | `.\target\release\mcp-status.exe --once` |
| Enable autostart | `.\target\release\mcp-status.exe --autostart-on` |
| Disable autostart | `.\target\release\mcp-status.exe --autostart-off` |
| Autostart status | `.\target\release\mcp-status.exe --autostart-status` |

Or set `MCP_STATUS_AUTOSTART=1` before a normal launch to register the Run key.

---

## 4. Indicators

| Mark | Meaning |
|---|---|
| ● | live — TCP to `url` OK |
| ○ | dead — TCP failed / exchange down |
| ? | unknown — stdio process not found, or entry with neither `url` nor `command` |

Toasts only on **state change**. First ~15s after start are muted (`MCP_STATUS_TOAST_SUPPRESS_SECS`).

---

## 5. Auto-discovered paths (Windows)

Cursor, Claude Code (`.claude.json`), Claude Desktop, Jan, AnythingLLM — see README. `--once` prints `candidates:` (✓ present, · missing).

Project-local `.cursor\mcp.json` is **not** scanned.

---

## 6. Grok Bot manual list

Grok connectors are **cloud-side**. There is no local `mcpServers` under `%APPDATA%\Grok Bot`.

```powershell
copy mcp-status.example.json $env:USERPROFILE\.mcp-status.json
```

Edit `%USERPROFILE%\.mcp-status.json`:

```json
{
  "mcpServers": {
    "grok-github": {},
    "grok-gmail": { "url": "https://your-endpoint-if-known" }
  }
}
```

- `{}` → tray shows **?**
- `"url"` → TCP ● / ○
- `"command"` + `"args"` → stdio heuristic

Also: `MCP_STATUS_CONFIG`, or `%APPDATA%\mcp-status\mcp-status.json`. Manual file is read **first** (overrides same names from auto-scan).

---

## 7. Environment

| Variable | Default | Purpose |
|---|---|---|
| `MCP_STATUS_INTERVAL_SECS` | `10` | Poll interval |
| `MCP_STATUS_TOAST_SUPPRESS_SECS` | `15` | Startup toast mute |
| `MCP_STATUS_EXCHANGE_URL` | `http://127.0.0.1:9750/` | agent-exchange; empty = off |
| `MCP_STATUS_CONFIG` | — | Extra / override JSON path |
| `MCP_STATUS_AUTOSTART` | — | Register Run on launch |
| `MCP_STATUS_TOAST_LOG` | — | Log toast lines on non-Windows |

---

## License

MIT — [LICENSE](LICENSE).
