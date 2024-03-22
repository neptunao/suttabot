use anyhow::anyhow;
use anyhow::Result;
use chrono::{Duration, Local, Utc};
use log::{debug, error, info, warn};
use rand::seq::IteratorRandom;
use sqlx::SqlitePool;
use std::{env, error::Error, fs, path::Path, sync::Arc};
use teloxide::{
    dispatching::dialogue::GetChatId,
    payloads::SendMessageSetters,
    prelude::*,
    types::ParseMode,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, Me},
    utils::command::BotCommands,
};
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::mpsc;
use tokio::time::{interval_at, Instant};

mod dto;

#[derive(BotCommands)]
#[command(rename_rule = "lowercase")]
enum Command {
    Help,
    Start,
}

const TELEGRAM_TEXT_MAX_LENGTH: usize = 4096;

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

fn make_unsubscribe_keyboard() -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];
    let subscribe =
        InlineKeyboardButton::callback("Отписаться".to_owned(), "Отписаться".to_owned());

    keyboard.push(vec![subscribe]);

    InlineKeyboardMarkup::new(keyboard)
}

fn make_subscribe_keyboard() -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];
    let subscribe =
        InlineKeyboardButton::callback("Подписаться".to_owned(), "Подписаться".to_owned());

    keyboard.push(vec![subscribe]);

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
                    let existing_subscription = sqlx::query_as::<_, dto::SubscriptionDto>(
                        "SELECT * FROM subscription WHERE chat_id = ?",
                    )
                    .bind(chat_id.to_string())
                    .fetch_optional(pool.as_ref())
                    .await?;

                    match existing_subscription {
                        Some(subscription) => {
                            if subscription.is_enabled == 1 {
                                info!("Chat_id: {} already subscribed, doing nothing", chat_id);

                                let text = "Вы уже подписаны на рассылку";
                                answer_message_with_replace(
                                    &bot,
                                    q.id,
                                    q.message,
                                    text,
                                    make_unsubscribe_keyboard(),
                                )
                                .await?;

                                return Ok(());
                            }

                            info!(
                                "Updating subscription {} for chat_id: {}",
                                subscription.id, chat_id
                            );

                            sqlx::query(
                                    "UPDATE subscription SET is_enabled = 1, updated_at = ? WHERE id = ?",
                                )
                                .bind(timestamp)
                                .bind(subscription.id)
                                .execute(pool.as_ref())
                                .await?;

                            let text = "Спасибо! Вы будете получать новую сутту каждый день в 8:00 по Москве";
                            answer_message_with_replace(
                                &bot,
                                q.id,
                                q.message,
                                text,
                                make_unsubscribe_keyboard(),
                            )
                            .await?;

                            return Ok(());
                        }
                        None => {
                            info!("Inserting new subscription for chat_id: {}", chat_id);

                            sqlx::query("INSERT INTO subscription (chat_id, is_enabled, created_at, updated_at) VALUES (?, ?, ?, ?)")
                                .bind(chat_id.to_string())
                                .bind(1)
                                .bind(timestamp)
                                .bind(timestamp)
                                .execute(pool.as_ref())
                                .await?;

                            let text = "Спасибо! Вы будете получать новую сутту каждый день в 8:00 по Москве";

                            answer_message_with_replace(
                                &bot,
                                q.id,
                                q.message,
                                text,
                                make_unsubscribe_keyboard(),
                            )
                            .await?;

                            return Ok(());
                        }
                    }
                }
                "Отписаться" => {
                    info!("Disabling subscription for chat_id: {}", chat_id);

                    sqlx::query(
                        "UPDATE subscription SET is_enabled = 0, updated_at = ? WHERE chat_id = ?",
                    )
                    .bind(timestamp)
                    .bind(chat_id.to_string())
                    .execute(pool.as_ref())
                    .await?;

                    let text = "Вы отписались от рассылки";

                    answer_message_with_replace(
                        &bot,
                        q.id,
                        q.message,
                        text,
                        make_subscribe_keyboard(),
                    )
                    .await?;

                    return Ok(());
                }
                _ => {
                    warn!("Unknown action: {}", chosen_action);
                }
            }
        }
    }

    Ok(())
}

async fn answer_message_with_replace(
    bot: &Bot,
    callback_query_id: String,
    message: Option<Message>,
    text: &str,
    keyboard: InlineKeyboardMarkup,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    bot.answer_callback_query(callback_query_id).await?;

    if let Some(Message { id, chat, .. }) = message {
        bot.edit_message_text(chat.id, id, text)
            .reply_markup(keyboard)
            .await?;
    }

    Ok(())
}

async fn send_daily_message(
    bot: Bot,
    pool: Arc<SqlitePool>,
    interval_sec: i64,
    data_dir: &Path,
    mut shutdown_signal: mpsc::Receiver<()>,
) -> anyhow::Result<()> {
    let now = Instant::now();
    // TODO input for daily message time
    let duration = duration_until(5, 0)?; // 8:00 Moscow time is 5:00 UTC
    let start_time = now + duration;
    let mut interval = interval_at(
        start_time,
        Duration::try_seconds(interval_sec)
            .ok_or(anyhow!("Invalid time"))?
            .to_std()?,
    );
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    info!("Reading files from data dir");
    let files = data_dir
        .read_dir()?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|ft| ft.is_file()).unwrap_or(false))
        .collect::<Vec<_>>();

    info!(
        "starting daily message task with interval: {}s and {} files",
        interval_sec,
        files.len()
    );

    loop {
        tokio::select! {
            _ = interval.tick() => {
                info!("Sending daily message");
                debug!("Querying chat_ids");

                let chat_ids = sqlx::query_as::<_, (i64,)>("SELECT chat_id FROM subscription WHERE is_enabled = 1")
                    .fetch_all(pool.as_ref())
                    .await?
                    .into_iter()
                    .map(|(chat_id,)| chat_id)
                    .collect::<Vec<i64>>();

                debug!("Got {} chat_ids", chat_ids.len());

                for chat_id in chat_ids {
                    let file = files
                        .iter()
                        .choose(&mut rand::thread_rng()) // Use the choose method on the iterator
                        .ok_or(anyhow!("No files in data dir"))?;

                    let texts = fs::read_to_string(file.path())?
                        .chars()
                        .collect::<Vec<char>>()
                        .chunks(TELEGRAM_TEXT_MAX_LENGTH)
                        .map(|chunk| chunk.iter().collect::<String>())
                        .collect::<Vec<String>>();

                    info!("Sending daily message to chat_id: {}, filename: {}", chat_id, file.file_name().to_string_lossy());

                    for (i, text) in texts.iter().enumerate() {
                        let mut send_msg = bot.send_message(ChatId(chat_id), text).parse_mode(ParseMode::MarkdownV2);

                        if i == texts.len() - 1 {
                            send_msg = send_msg.reply_markup(make_unsubscribe_keyboard()); // TODO bug: last message will be replaced with keyboard if unsubscribe is clicked
                        }

                        if let Err(e) = send_msg.await {
                            error!("Failed to send message to chat_id: {}, error: {}", chat_id, e);
                        }
                    }
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
async fn main() -> Result<(), anyhow::Error> {
    pretty_env_logger::init();

    let db_url = &env::var("DATABASE_URL")?;
    let data_dir_str = env::var("DATA_DIR")?;
    let interval: i64 = env::var("MESSAGE_INTERVAL")
        .unwrap_or("86400".to_string()) // in seconds
        .parse()?;

    if !Path::new(&data_dir_str).is_dir() {
        Err(anyhow!("DATA_DIR is not a directory"))?;
    }

    let pool = Arc::new(SqlitePool::connect(db_url).await?);

    info!("Migrating database...");
    migrate_db(pool.as_ref()).await?;
    info!("Database migrated");

    info!("Starting buttons bot...");
    let bot = Bot::from_env();

    let send_bot = bot.clone();
    let send_pool = pool.clone();
    let (shutdown_send, shutdown_recv) = mpsc::channel(1);

    let send_daily_message_task = tokio::spawn(async move {
        let data_dir = Path::new(&data_dir_str);

        let send_result = send_daily_message(
            send_bot.clone(),
            send_pool.clone(),
            interval,
            data_dir,
            shutdown_recv,
        )
        .await;

        match send_result {
            Ok(_) => (),
            Err(e) => error!("Failed to send daily message: {}", e),
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

        shutdown_send.send(()).await
    });

    send_daily_message_task.await?;

    Ok(())
}
