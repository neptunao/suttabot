use anyhow::anyhow;
use anyhow::Result;
use chrono::Duration;
use chrono::{Local, Utc};
use log::info;
use log::warn;
use sqlx::{migrate, SqlitePool};
use std::{env, error::Error, sync::Arc};
use teloxide::{
    dispatching::dialogue::GetChatId,
    payloads::SendMessageSetters,
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, Me},
    utils::command::BotCommands,
};
use tokio::signal::unix::{signal, SignalKind};
use tokio::time::interval;
use tokio::time::{interval_at, Instant};

/// These commands are supported:
#[derive(BotCommands)]
#[command(rename_rule = "lowercase")]
enum Command {
    /// Display this text
    Help,
    /// Start
    Start,
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

async fn callback_handler(
    pool: Arc<SqlitePool>,
    bot: Bot,
    q: CallbackQuery,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(chosen_action) = &q.data {
        if let Some(chat_id) = q.chat_id() {
            let now = Utc::now();
            let timestamp = now.timestamp();

            match chosen_action.as_str() {
                "Подписаться" => {
                    sqlx::query("INSERT INTO subscription (chat_id, is_enabled, created_at, updated_at) VALUES (?, ?, ?, ?)")
                        .bind(chat_id.to_string())
                        .bind(1)
                        .bind(timestamp)
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
                    sqlx::query(
                        "UPDATE subscription SET is_enabled = 0, updated_at = ? WHERE chat_id = ?",
                    )
                    .bind(timestamp)
                    .bind(chat_id.to_string())
                    .execute(pool.as_ref())
                    .await?;

                    let text = "Вы отписались от рассылки";

                    bot.answer_callback_query(q.id).await?;

                    if let Some(Message { id, chat, .. }) = q.message {
                        bot.edit_message_text(chat.id, id, text).await?;
                    }
                }
                _ => {
                    warn!("Unknown action: {}", chosen_action);
                }
            }
        }
    }

    Ok(())
}

async fn send_daily_message(
    bot: Bot,
    pool: Arc<SqlitePool>,
    mut shutdown_signal: tokio::sync::mpsc::Receiver<()>,
) -> anyhow::Result<()> {
    let now = Instant::now();
    let duration = duration_until(5, 0)?; // 8:00 Moscow time is 5:00 UTC
    let start_time = now + duration;
    // let mut interval = interval_at(
    //     start_time,
    //     Duration::try_days(1)
    //         .ok_or(anyhow!("Invalid time"))?
    //         .to_std()?,
    // );
    let mut interval = interval(Duration::try_seconds(5).ok_or(anyhow!("123"))?.to_std()?);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let chat_ids = sqlx::query_as::<_, (i64,)>("SELECT chat_id FROM subscription WHERE is_enabled = 1")
                    .fetch_all(pool.as_ref())
                    .await?
                    .into_iter()
                    .map(|(chat_id,)| chat_id)
                    .collect::<Vec<i64>>();

                for chat_id in chat_ids {
                    bot.send_message(ChatId(chat_id), "Сутта дня").await?;
                }
            }
            _ = shutdown_signal.recv() => {
                println!("Shutting down daily message task");
                break;
            }
        }
    }

    Ok(())
}

fn duration_until(hour: u32, min: u32) -> Result<std::time::Duration, anyhow::Error> {
    let now = Local::now().naive_utc();

    let eight_am = now
        .date()
        .and_hms_opt(hour, min, 0)
        .ok_or(anyhow!("Invalid time"))?;

    let res_duration = if now < eight_am {
        eight_am - now
    } else {
        let hours24_delta = Duration::try_hours(24).ok_or(anyhow!("Invalid time"))?;
        hours24_delta - (now - eight_am)
    };

    Ok(res_duration.to_std()?)
}

async fn migrate_db(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::migrate!("./db/migrations").run(pool).await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // TODO anyhow::Result
    pretty_env_logger::init();

    let pool = Arc::new(SqlitePool::connect(&env::var("DATABASE_URL")?).await?);

    info!("Migrating database...");
    migrate_db(pool.as_ref()).await?;
    info!("Database migrated");

    info!("Starting buttons bot...");
    let bot = Bot::from_env();

    let send_bot = bot.clone();
    let send_pool = pool.clone();
    let (shutdown_send, shutdown_recv) = tokio::sync::mpsc::channel(1);

    let send_daily_message_task = tokio::spawn(async move {
        match send_daily_message(send_bot.clone(), send_pool.clone(), shutdown_recv).await {
            Ok(_) => (),
            Err(e) => log::error!("Failed to send daily message: {}", e),
        }
    });

    let receiver_pool = pool.clone();
    let handler = dptree::entry()
        .branch(Update::filter_message().endpoint(message_handler))
        .branch(
            Update::filter_callback_query().endpoint(move |bot: Bot, q| {
                callback_handler(receiver_pool.clone(), bot.clone(), q)
            }),
        );

    Dispatcher::builder(bot.clone(), handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    let mut stream = signal(SignalKind::interrupt())?;
    tokio::spawn(async move {
        stream.recv().await;
        let _ = shutdown_send.send(());
    });

    send_daily_message_task.await?;

    Ok(())
}
