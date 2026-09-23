**Українська** · [English](README.en.md)

# mcp-status

Windows-трей застосунок (Rust, один бінарник), який показує **live/dead** стан
локальних MCP-серверів з конфігів Cursor, Claude Code/Desktop, Jan та ін. і шле toast лише при
зміні стану.

MVP 0.1 — робочий каркас: парсинг конфігів, TCP/stdio-перевірки, трей (Windows),
автозапуск, toast з глушінням на старті. На Linux/macOS GUI-залежності
відключені (`cfg(windows)`), працює headless/`--once` для CI.

## Що робить

1. Читає `mcpServers` з типових JSON-конфігів.
2. Періодично перевіряє кожен сервер:
   - **HTTP/SSE (`url`)** — TCP connect на host:port (не MCP-протокол).
   - **stdio (`command`)** — евристика за іменем процесу; якщо процесу немає →
     `unknown` (сконфігуровано, але не підтверджено), не `dead`.
3. Трей-меню з індикатором ● / ○ / ? для кожного MCP.
4. Windows toast **лише на зміну** стану; перші N секунд після старту — тиша
   (за замовчуванням 15 с), щоб не спамити при запуску.
5. Опційно GET на agent-exchange UI (`http://127.0.0.1:9750/` або
   `MCP_STATUS_EXCHANGE_URL`).
6. Автозапуск Windows через `HKCU\...\Run` (як у desktop-remote-kit).

## Шляхи конфігів

| Клієнт | Windows | Linux / macOS |
|---|---|---|
| Cursor | `%USERPROFILE%\.cursor\mcp.json` | `~/.cursor/mcp.json` |
| Cursor (legacy) | `%APPDATA%\Cursor\...\cursor.mcp\settings.json` | `~/.config/Cursor/...` |
| Claude Code | `%USERPROFILE%\.claude.json`, `.claude\settings.json` | те саме під `$HOME` |
| Claude Desktop | `%APPDATA%\Claude\claude_desktop_config.json` | Application Support / `.config/Claude` |
| Jan | `%APPDATA%\Jan\data\mcp_config.json` | `~/.config/Jan/data/mcp_config.json` |
| AnythingLLM | `%APPDATA%\anythingllm-desktop\...\anythingllm_mcp_servers.json` | під config dir |

Плагіни **Grok Bot** (хмарний каталог конекторів) **не** лежать у локальному `mcpServers` — з диска їх зараз не видно.
Проєктні `.cursor/mcp.json` **не** скануються.


## Обмеження stdio-евристики

- Спільні імена (`node`, `python`, `npx`) дають хибні спрацьовування.
- Короткоживучі дочірні процеси можуть «миготіти» між опитуваннями.
- Порівнюється лише basename / file stem команди, не повний командний рядок.

## Збірка

```bash
cargo build --release
```

Бінарник: `target/release/mcp-status` (на Windows — `.exe`).

```bash
cargo run -- --once          # один прохід, зручно для CI / Linux
cargo test
cargo check                  # має проходити на Linux без Windows-deps
```

### Прапорці

| Прапорець | Дія |
|---|---|
| `--once` | Скан + ping, друк, вихід |
| `--autostart-on` / `--off` / `--status` | Run-ключ Windows |
| `-h` / `--help` | Довідка |

### Змінні середовища

| Змінна | За замовчуванням | Сенс |
|---|---|---|
| `MCP_STATUS_INTERVAL_SECS` | `10` | Інтервал опитування |
| `MCP_STATUS_TOAST_SUPPRESS_SECS` | `15` | Тиша toast після старту |
| `MCP_STATUS_EXCHANGE_URL` | `http://127.0.0.1:9750/` | Порожній рядок = вимкнути |
| `MCP_STATUS_AUTOSTART` | — | Якщо задано — увімкнути Run при старті |
| `MCP_STATUS_TOAST_LOG` | — | На non-Windows друкувати «toast» у stderr |

## Що реалізовано / заглушки

| Частина | Стан |
|---|---|
| Парсинг `mcpServers` | ✅ |
| TCP ping для `url` | ✅ |
| stdio process heuristic | ✅ (з обмеженнями вище) |
| Трей + меню | ✅ Windows; headless на інших ОС |
| Toast на зміну + suppress | ✅ Windows (`winrt-notification`); stub лог elsewhere |
| agent-exchange GET | ✅ (простий HTTP GET) |
| Автозапуск Windows | ✅; elsewhere `Unsupported` |
| Project-level `.cursor/mcp.json` | ❌ не сканується |
| Повний MCP handshake | ❌ навмисно лише TCP / process |

## Ліцензія

MIT
