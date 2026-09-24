**Українська** · [English](README.en.md)

# mcp-status

Windows-трей застосунок на Rust, який збирає локальні `mcpServers` із конфігів
популярних клієнтів, показує їхній стан і надсилає toast лише при зміні стану.
Один бінарник, без окремого runtime.

Покрокове встановлення: **[ІНСТРУКЦІЯ.md](ІНСТРУКЦІЯ.md)**.
English setup: [INSTRUCTIONS.md](INSTRUCTIONS.md).

## Що перевіряється

| Конфіг | Перевірка | Результат |
|---|---|---|
| `url` з `http(s)` або `ws(s)` | TCP connect до host:port, timeout 2 с | `live` або `dead` |
| `command` для stdio | Пошук процесу за basename / stem команди | `live` або `unknown` |
| запис без `url` і `command` | Перевірити транспорт неможливо | `unknown` |
| agent-exchange URL | HTTP GET, 2xx/3xx | `live` або `dead` |

TCP-перевірка не виконує MCP handshake і не перевіряє конкретний HTTP route.
Відкритий порт означає лише те, що endpoint доступний на транспортному рівні.

Позначки в треї:

- `●` — live;
- `○` — dead;
- `?` — сконфігуровано, але стан не підтверджено.

## Встановлення

Потрібен Rust toolchain. Для Windows-збірки MSVC потрібні Visual Studio Build
Tools із компонентом C++.

```bash
cargo install --git https://github.com/RiasJ1Dar/mcp-status
```

Або з клону:

```bash
git clone https://github.com/RiasJ1Dar/mcp-status.git
cd mcp-status
cargo build --release
```

Бінарник: `target/release/mcp-status.exe` на Windows або
`target/release/mcp-status` на інших ОС.

## Швидкий старт

Перевірити, які конфіги знайдено, без запуску трея:

```bash
mcp-status --once
```

Команда друкує сервери, їхній стан і всі candidate paths із позначкою, чи
існує файл.

Запустити трей на Windows:

```bash
mcp-status
```

Перші 15 секунд toast приглушені, щоб запуск не створював серію сповіщень.
Далі toast з'являється лише при зміні health state.

## Команди

| Прапорець | Дія |
|---|---|
| `--once` | Один scan + ping, друк результату й вихід |
| `--autostart-on` | Додати поточний exe у Windows Run |
| `--autostart-off` | Прибрати запис із Windows Run |
| `--autostart-status` | Показати стан автозапуску |
| `-h`, `--help` | Надрукувати довідку |

На Linux/macOS без `--once` працює headless loop; Windows GUI-залежності
підключаються лише через `cfg(windows)`.

## Джерела конфігів

Підтримується об'єкт `mcpServers` у JSON. Також приймається legacy bare map,
якщо всі його значення — об'єкти.

| Клієнт | Windows | Linux / macOS |
|---|---|---|
| Cursor | `%USERPROFILE%\.cursor\mcp.json` | `~/.cursor/mcp.json` |
| Cursor legacy | `%APPDATA%\Cursor\User\globalStorage\cursor.mcp\settings.json` | відповідний config dir |
| Claude Code | `%USERPROFILE%\.claude.json`, `%USERPROFILE%\.claude\settings.json` | те саме під `$HOME` |
| Claude Desktop | `%APPDATA%\Claude\claude_desktop_config.json` | Application Support / `.config/Claude` |
| Jan | `%APPDATA%\Jan\data\mcp_config.json` | відповідний config dir |
| AnythingLLM | `%APPDATA%\anythingllm-desktop\storage\plugins\anythingllm_mcp_servers.json` | відповідний config dir |

Проєктні `.cursor/mcp.json` не скануються: застосунок не має workspace root.

## Ручний список і пріоритет

Для Grok Bot або іншого MCP, якого немає в локальному конфігу клієнта, створи:

- шлях із `MCP_STATUS_CONFIG`;
- `%USERPROFILE%\.mcp-status.json`;
- `%APPDATA%\mcp-status\mcp-status.json`.

Приклад: [mcp-status.example.json](mcp-status.example.json).

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

Файли читаються в порядку пріоритету; при однаковому імені перший запис
перемагає. Спочатку йдуть ручні файли, потім конфіги клієнтів. Один
пошкоджений entry пропускається з повідомленням у stderr, а не приховує решту
файла. UTF-8 BOM підтримується.

## Змінні середовища

| Змінна | За замовчуванням | Призначення |
|---|---|---|
| `MCP_STATUS_INTERVAL_SECS` | `10` | Інтервал опитування |
| `MCP_STATUS_TOAST_SUPPRESS_SECS` | `15` | Тиша toast після старту |
| `MCP_STATUS_EXCHANGE_URL` | `http://127.0.0.1:9750/` | URL agent-exchange; порожнє значення вимикає probe |
| `MCP_STATUS_CONFIG` | — | Додатковий JSON найвищого пріоритету |
| `MCP_STATUS_AUTOSTART` | — | Якщо змінна існує, зареєструвати Windows Run під час запуску |
| `MCP_STATUS_TOAST_LOG` | — | На non-Windows друкувати toast-події у stderr |

Якщо agent-exchange не використовується, вимкни його probe:

```powershell
$env:MCP_STATUS_EXCHANGE_URL = ""
mcp-status --once
```

## Обмеження

- `node`, `python`, `npx` та інші спільні імена процесів можуть дати
  хибний `live`.
- Короткоживучий stdio-процес може зникнути між опитуваннями.
- Для stdio не аналізується повний command line або `args`.
- Для URL не виконується MCP handshake.
- Застосунок спостерігає; він не запускає, не зупиняє й не перезапускає MCP.

## Розробка

```bash
cargo fmt --check
cargo test
cargo check
cargo clippy --all-targets -- -D warnings
```

CI перевіряє Linux-збірку без Windows GUI-залежностей.

## Ліцензія

MIT