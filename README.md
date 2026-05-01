# suttabot

This project is a Telegram bot written in Rust, using the [teloxide](https://github.com/teloxide/teloxide) library. The bot allows users to subscribe or unsubscribe from daily suttas (ancient Buddhist scriptures) in Russian translation of the [Theravada](https://en.wikipedia.org/wiki/Theravada) branch of Buddhism. The messages are sent from a collection of files in a specified directory, with one file randomly chosen each day. The bot uses SQLite for storing subscription data.

Note that this project currently supports **only Russian language**.

Inspired by [Reading Faithfully](https://readingfaithfully.org/).

Sutta texts are included in the `data/ru/` directory.

## Environment Variables

| Variable | Description | Required | Default |
| --- | --- | --- | --- |
| `TELOXIDE_TOKEN` | The telegram bot token. | Yes | None |
| `DATABASE_URL` | The URL of the SQLite database. | Yes | None |
| `DATA_DIR` | The directory where the message files are stored. | Yes | None |
| `MESSAGE_INTERVAL` | The interval in seconds between each daily message. | No | 86400 (24 hours) |
| `DONATION_MESSAGE_PERIOD` | Send a donation reminder every N suttas delivered to a subscriber. | No | 15 |

## Running Locally

### Without Docker

1. Set the required environment variables
2. Run the project: `cargo run`

### With Docker

1. Install Docker: https://docs.docker.com/get-docker/
2. Build the Docker image: `docker build -t suttabot .`
3. Run the Docker container, passing in the environment variables:

```bash
docker run -d --name=suttabot -e RUST_LOG=info -e TELOXIDE_TOKEN="<TELOXIDE_TOKEN>" -e DATABASE_URL="sqlite:///db/suttabot.db" -e DATA_DIR="/data" -v "<LOCAL_DB_PATH>:/db/suttabot.db" -v "<LOCAL_DATA_PATH>:/data" suttabot
```

## Bot Commands

| Command | Description |
| --- | --- |
| `/start` | Greet the user and show the subscribe/unsubscribe keyboard |
| `/subscribe` | Subscribe to the daily sutta mailing |
| `/unsubscribe` | Unsubscribe from the daily sutta mailing |
| `/random` | Receive a random sutta immediately |
| `/get <id>` | Find and send a sutta by number, e.g. `/get МН 65` or `/get mn65`. Supports Latin and Cyrillic collection codes (MN, SN, AN, DN, and their Russian equivalents). |
| `/settime <times>` | Set daily delivery times, e.g. `/settime 6:00 8:18 19:31`. Up to 10 times per day. Requires an active subscription. (**not yet implemented** — times are saved but ignored; delivery is fixed at 08:00 Moscow time) |
| `/dana` | Show donation information for Dhamma centres |
| `/help` | List all available commands |

## Data Scripts

Scripts live in `scripts/`. Install dependencies first:

```bash
cd scripts && poetry install
```

### bilarify.py

Pre-processes Bilara source JSON files in-place: normalises quotes, replaces `...` with `…`, and strips square brackets from text values.

```bash
python bilarify.py <source_dir>
```

Run this on the Bilara source directory before `bilara2md.py`.

### bilara2md.py

Converts Bilara JSON source + HTML-format files to Markdown files ready for the bot.

```bash
python bilara2md.py <source_folder> <format_folder> <target_folder>
```

| Argument | Description |
| --- | --- |
| `source_folder` | Bilara translation JSON files (e.g. `mn1_translation-ru-sv.json`) |
| `format_folder` | Bilara HTML-format JSON files (e.g. `mn1_html.json`) — same directory structure |
| `target_folder` | Output directory for `.md` files |
| `-r`, `--recursive` | Traverse subfolders and write all output flat into `target_folder` (default: on) |
| `--filename-format` | `full` keeps source stem `mn1_translation-ru-sv.md` (default); `numerical` uses just `mn1.md` |
| `--overwrite` | Delete the alternative-format file for the same sutta if present, and overwrite the file if it already exists. Without this flag, existing files are skipped. (default: off) |

### suttacentral2md.py

Converts HTML files downloaded from SuttaCentral to Markdown.

```bash
python suttacentral2md.py <source_folder> <target_folder>
```

## Contributing

Contributions are welcome! If you have any suggestions, bug reports, or feature requests, please open an issue or submit a pull request.

## License

This project is licensed under the [MIT License](LICENSE).

Sutta texts are sourced from [theravada.ru](https://www.theravada.ru/) and the [SuttaCentral](https://suttacentral.net/) [repository](https://github.com/suttacentral/sc-data). All original material created by SuttaCentral is dedicated to the Public Domain via [CC0 1.0 Universal](https://creativecommons.org/publicdomain/zero/1.0/).
