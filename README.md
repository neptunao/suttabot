# suttabot

This project is a Telegram bot written in Rust, using the [teloxide](https://github.com/teloxide/teloxide) library. The bot allows users to subscribe or unsubscribe from daily suttas (ancient buddhist scriptures) in Russian translation of the [Theravada](https://en.wikipedia.org/wiki/Theravada) branch of buddhism. The messages are sent from a collection of files in a specified directory, with one file randomly chosen each day. The bot uses SQLite for storing subscription data.

Note that this project currently support **only Russian language**.

The data is not stored in the repo to avoid conflict with authors. All suttas are taken from site [theravada.ru](https://www.theravada.ru/Teaching/canon.htm), which is one of the biggest sites with Russian translations of Theravada teachings and suttas.

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

## Contributing

Contributions are welcome! If you have any suggestions, bug reports, or feature requests, please open an issue or submit a pull request.

## License

This project is licensed under the [MIT License](LICENSE).

All suttas and texts are taken from [theravada.ru](https://www.theravada.ru/) website.
