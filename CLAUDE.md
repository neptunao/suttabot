# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

A Telegram bot written in Rust that sends daily Buddhist suttas (from [theravada.ru](https://www.theravada.ru/) and SuttaCentral) to subscribers. Users subscribe/unsubscribe via inline keyboard buttons or bot commands. Suttas are plain `.md` files served from `DATA_DIR`. Only Russian language is supported.

## Build and run commands

```bash
# Build
cargo build

# Run (requires env vars set)
cargo run

# Build release
cargo build --release

# Check compilation without building
cargo check

# Run clippy linter
cargo clippy

# Run tests
cargo test

# Run a single test
cargo test <test_name>
```

sqlx uses offline mode — the `.sqlx/` directory contains cached query metadata. If you change SQL queries, run `cargo sqlx prepare` to regenerate it (requires `DATABASE_URL` to be set pointing to a real database).

## Required environment variables

| Variable | Required | Default | Notes |
|---|---|---|---|
| `TELOXIDE_TOKEN` | Yes | — | Telegram bot token |
| `DATABASE_URL` | Yes | — | `sqlite:///path/to/suttabot.db` |
| `DATA_DIR` | Yes | — | Directory with `.md` sutta files |
| `MESSAGE_INTERVAL` | No | `86400` | Seconds between daily sends |
| `DONATION_MESSAGE_PERIOD` | No | `15` | Send donation info every N suttas |

## Architecture

The bot runs two concurrent tasks via `tokio::spawn`:

1. **Daily message task** (`send_daily_messages` in `main.rs`) — fires once per day at 05:00 UTC (= 08:00 Moscow). Iterates all enabled subscribers, picks a random `.md` file from `DATA_DIR`, and sends it. Every N sends (controlled by `DONATION_MESSAGE_PERIOD`), also sends `data/donation_info.md`. Retries failed sends with exponential backoff up to 5 attempts.

2. **Telegram dispatcher** (`Dispatcher` in `main.rs`) — handles incoming messages and callback queries (inline button presses).

### Source modules

- `main.rs` — entrypoint, wires up both tasks, defines inline keyboard builders and `callback_handler` for subscribe/unsubscribe button presses
- `message_handler.rs` — parses bot commands (`/start`, `/subscribe`, `/unsubscribe`, `/random`, `/get`, `/settime`, `/dana`, `/help`) and dispatches to per-command handlers; also contains sutta file search logic (`find_sutta_file`) supporting both Latin and Cyrillic collection codes (e.g. `МН 65` → `mn65`)
- `sender.rs` — reads a `.md` file, escapes it with `telegram_escape::tg_escape`, splits it into ≤4096-char chunks (without breaking escape sequences), and sends via MarkdownV2. Defines `TgMessageSendError` for typed retry logic.
- `db.rs` — `DbService` wrapping `SqlitePool`; all DB operations live here
- `dto.rs` — `SubscriptionDto` struct mapping the `subscription` table
- `helpers.rs` — constants and `list_files`

### Database

SQLite via sqlx with migrations in `db/migrations/`. The single table `subscription` tracks: `chat_id`, `is_enabled`, timestamps, `sendout_count` (total suttas sent), `last_donation_reminder`, and `donation_reminder_count`. There is also a `sendout_times` table (unused in current scheduling logic — the daily task fires at a fixed UTC time).

### Data / scripts

Sutta `.md` files are **not stored in the repo**. The `scripts/` directory has Python utilities to convert source data into the expected format:
- `bilara2md.py` — converts SuttaCentral Bilara JSON exports to Markdown
- `suttacentral2md.py` / `bilarify.py` — additional conversion helpers

Scripts use Poetry (`scripts/pyproject.toml`); run `poetry install` inside `scripts/` before using them.

## Docker

```bash
docker build -t suttabot .
docker run -d --name=suttabot \
  -e RUST_LOG=info \
  -e TELOXIDE_TOKEN="<token>" \
  -e DATABASE_URL="sqlite:///db/suttabot.db" \
  -e DATA_DIR="/data" \
  -v "<LOCAL_DB_PATH>:/db/suttabot.db" \
  -v "<LOCAL_DATA_PATH>:/data" \
  suttabot
```
