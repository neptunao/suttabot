# suttabot

This project is a Telegram bot written in Rust, using the [teloxide](https://github.com/teloxide/teloxide) library. The bot allows users to subscribe or unsubscribe from daily suttas (ancient buddhist scriptures) in Russian translation of the [Theravada](https://en.wikipedia.org/wiki/Theravada) branch of buddhism. The messages are sent from a collection of files in a specified directory, with one file randomly chosen each day. The bot uses SQLite for storing subscription data.

Note that this project currently support **only Russian language**.

The data is not stored in the repo to avoid conflict with authors. All suttas are taken from site [theravada.ru](https://www.theravada.ru/Teaching/canon.htm), which is one of the biggest sites with Russian translations of Theravada teachings and suttas.

Inspired by [Reading Faithfully](https://readingfaithfully.org/).

## Environment Variables

| Variable | Description | Required | Default |
| --- | --- | --- | --- |
| `TELOXIDE_TOKEN` | The telegram bot token. | Yes | None |
| `DATABASE_URL` | The URL of the SQLite database. | Yes | None |
| `DATA_DIR` | The directory where the message files are stored. | Yes | None |
| `MESSAGE_INTERVAL` | The interval in seconds between each daily message. | No | 86400 (24 hours) |

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

### suttacentral2md.py

Converts HTML files downloaded from SuttaCentral to Markdown.

```bash
python suttacentral2md.py <source_folder> <target_folder>
```

## Contributing

Contributions are welcome! If you have any suggestions, bug reports, or feature requests, please open an issue or submit a pull request.

## License

This project is licensed under the [MIT License](LICENSE).

All suttas and texts are taken from [theravada.ru](https://www.theravada.ru/) website and [SuttaCentral](https://suttacentral.net/) [repository](https://github.com/suttacentral/sc-data).
