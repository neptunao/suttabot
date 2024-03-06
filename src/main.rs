use chrono::Utc;
use sqlx::SqlitePool;
use std::{env, error::Error, sync::Arc};
use teloxide::{
    dispatching::dialogue::GetChatId,
    payloads::SendMessageSetters,
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, Me},
    utils::command::BotCommands,
};

/// These commands are supported:
#[derive(BotCommands)]
#[command(rename_rule = "lowercase")]
enum Command {
    /// Display this text
    Help,
    /// Start
    Start,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();
    log::info!("Starting buttons bot...");

    let bot = Bot::from_env();
    let pool = Arc::new(SqlitePool::connect(&env::var("DATABASE_URL")?).await?);

    let handler = dptree::entry()
        .branch(Update::filter_message().endpoint(message_handler))
        .branch(
            Update::filter_callback_query()
                .endpoint(move |bot, q| callback_handler(pool.clone(), bot, q)),
        );

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}

/// Creates a keyboard made by buttons in a big column.
fn make_keyboard() -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];

    let possible_actions = ["Подписаться", "Отписаться"];

    for actions in possible_actions.chunks(3) {
        let row = actions
            .iter()
            .map(|&action| InlineKeyboardButton::callback(action.to_owned(), action.to_owned()))
            .collect();

        keyboard.push(row);
    }

    InlineKeyboardMarkup::new(keyboard)
}

/// Parse the text wrote on Telegram and check if that text is a valid command
/// or not, then match the command. If the command is `/start` it writes a
/// markup with the `InlineKeyboardMarkup`.
async fn message_handler(
    bot: Bot,
    msg: Message,
    me: Me,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(text) = msg.text() {
        match BotCommands::parse(text, me.username()) {
            Ok(Command::Help) => {
                // Just send the description of all commands.
                bot.send_message(msg.chat.id, Command::descriptions().to_string())
                    .await?;
            }
            Ok(Command::Start) => {
                bot.send_message(
                    msg.chat.id,
                    "Нажмите подписаться чтобы получать каждый день сутту из сайта theravada.ru",
                )
                .await?;

                // Create a list of buttons and send them.
                let keyboard = make_keyboard();
                bot.send_message(msg.chat.id, "Выберите действие:")
                    .reply_markup(keyboard)
                    .await?;
            }

            Err(_) => {
                bot.send_message(msg.chat.id, "Command not found!").await?;
            }
        }
    }

    Ok(())
}

/// When it receives a callback from a button it edits the message with all
/// those buttons writing a text with the selected Debian version.
///
/// **IMPORTANT**: do not send privacy-sensitive data this way!!!
/// Anyone can read data stored in the callback button.
async fn callback_handler(
    pool: Arc<SqlitePool>,
    bot: Bot,
    q: CallbackQuery,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(chosen_action) = &q.data {
        if let Some(chat_id) = q.chat_id() {
            match chosen_action.as_str() {
                "Подписаться" => {
                    let now = Utc::now();
                    let timestamp = now.timestamp();

                    sqlx::query("INSERT INTO subscriptions (chat_id, subscribed, ) VALUES (?, ?)")
                        .bind(chat_id.to_string())
                        .bind(true)
                        .bind(timestamp)
                        .execute(pool.as_ref())
                        .await?;

                    let text =
                        "Спасибо! Вы будете получать новую сутту каждый день в 8:00 по Москве";

                    // Tell telegram that we've seen this query, to remove 🕑 icons from the
                    // clients. You could also use `answer_callback_query`'s optional
                    // parameters to tweak what happens on the client side.
                    bot.answer_callback_query(q.id).await?;

                    // Edit text of the message to which the buttons were attached
                    if let Some(Message { id, chat, .. }) = q.message {
                        bot.edit_message_text(chat.id, id, text).await?;
                    }
                }
                "Отписаться" => {
                    todo!("implement unsubscribe");
                }
                _ => {
                    log::warn!("Unknown action: {}", chosen_action);
                }
            }

            log::info!("You chose: {}", chosen_action);
        }
    }

    Ok(())
}