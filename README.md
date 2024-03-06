# suttabot

This is a Telegram bot implemented in Rust using the Teloxide library.

## Project Structure

```
suttabot
├── src
│   ├── main.rs
│   ├── commands
│   │   └── mod.rs
│   └── handlers
│       └── mod.rs
├── Cargo.toml
└── README.md
```

## Files

- `src/main.rs`: This file is the entry point of the application. It contains the main function that initializes the Telegram bot using the Teloxide library and sets up the bot's behavior.

- `src/commands/mod.rs`: This file exports the modules for different bot commands. You can define separate modules for each command and implement the logic for handling those commands.

- `src/handlers/mod.rs`: This file exports the modules for different event handlers. You can define separate modules for handling different types of events, such as message events, callback query events, etc.

- `Cargo.toml`: This file is the configuration file for Cargo, the package manager and build system for Rust. It lists the dependencies and other project metadata.

## Usage

To use this Telegram bot, follow these steps:

1. Clone the repository: `git clone https://github.com/your-username/suttabot.git`
2. Navigate to the project directory: `cd suttabot`
3. Build the project: `cargo build`
4. Run the bot: `cargo run`

Make sure to set up your Telegram bot token and other configuration options in the `main.rs` file before running the bot.

## Contributing

Contributions are welcome! If you have any suggestions, bug reports, or feature requests, please open an issue or submit a pull request.

## License

This project is licensed under the [MIT License](LICENSE).