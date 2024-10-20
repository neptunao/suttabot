use anyhow::anyhow;
use anyhow::Result;
use chrono::{Duration, Local, Utc};
use db::DbService;
use helpers::list_files;
use log::{debug, error, info, warn};
use std::path::PathBuf;
use std::{env, error::Error, path::Path, sync::Arc};
use teloxide::{
    dispatching::dialogue::GetChatId,
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, Me},
};
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::mpsc;
use tokio::time::{interval_at, Instant};

use crate::message_handler::message_handler;
use crate::sender::send_daily_message;
use crate::sender::TgMessageSendError;

mod db;
mod dto;
mod helpers;
mod message_handler;
mod sender;

const RETRY_LIMIT: u8 = 5;
const RETRY_INTERVAL_SEC: u64 = 5;

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

async fn callback_handler(
    db: Arc<DbService>,
    bot: Bot,
    q: CallbackQuery,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(chosen_action) = &q.data {
        if let Some(chat_id) = q.chat_id() {
            let now = Utc::now();
            let timestamp = now.timestamp();

            match chosen_action.as_str() {
                "Подписаться" => {
                    let existing_subscription = db.get_subscription_by_chat_id(chat_id.0).await?;

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

                            db.set_subscription_enabled(chat_id.0, 1, timestamp).await?;

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

                            db.create_subscription(chat_id.0, 1, timestamp).await?;

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

                    db.set_subscription_enabled(chat_id.0, 0, timestamp).await?;

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

fn can_retry_error_with_interval(e: &TgMessageSendError) -> (bool, std::time::Duration) {
    match e {
        TgMessageSendError::RetryAfter(duration) => (true, *duration),
        TgMessageSendError::TeloxideError(_e) => {
            (true, std::time::Duration::from_secs(RETRY_INTERVAL_SEC))
        }
        TgMessageSendError::UnknownError(_e) => {
            (false, std::time::Duration::from_secs(RETRY_INTERVAL_SEC))
        }
        TgMessageSendError::BotBlocked => {
            (false, std::time::Duration::from_secs(RETRY_INTERVAL_SEC))
        }
    }
}

async fn send_daily_messages(
    bot: Bot,
    db: Arc<DbService>,
    interval_sec: i64,
    data_dir: PathBuf,
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
    let files = list_files(&data_dir)?;

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

                let chat_ids = db.get_enabled_chat_ids().await?;
                debug!("Got {} chat_ids", chat_ids.len());

                for chat_id in chat_ids {
                    match send_daily_message(&bot, chat_id, &files).await {
                        Ok(_) => {
                            info!("Sent daily message to chat_id: {}", chat_id);
                        },
                        Err(send_err) => {
                            let (is_recoverable, mut retry_interval) = can_retry_error_with_interval(&send_err);
                            if !is_recoverable {
                                error!("Failed to send message to chat_id: {}, error: {:?}", chat_id, send_err);
                                continue;
                            }

                            let mut retry_count = 0;
                            let mut success = false;

                            while !success && retry_count < RETRY_LIMIT {
                                retry_count += 1;
                                error!("Failed to send message to chat_id: {}, error: {:?}, retry attempt: {}", chat_id, send_err, retry_count);

                                tokio::time::sleep(retry_interval).await; 
                                success = match send_daily_message(&bot, chat_id, &files).await {
                                    Ok(_) => true,
                                    Err(err) => {
                                        error!("Failed to send message to chat_id: {}, error: {:?}", chat_id, err);
                                        false
                                    }
                                };
                                retry_interval *= 2; // exponential backoff

                                if retry_count == RETRY_LIMIT {
                                    error!("Failed to send message to chat_id: {} after {} attempts", chat_id, RETRY_LIMIT);
                                }
                            }
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

fn get_data_dir() -> Result<PathBuf> {
    let data_dir_str = env::var("DATA_DIR")?;

    if !Path::new(&data_dir_str).is_dir() {
        Err(anyhow!("DATA_DIR is not a directory"))?;
    }

    Ok(PathBuf::from(data_dir_str))
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    pretty_env_logger::init();

    let db_url = &env::var("DATABASE_URL")?;
    let data_dir = get_data_dir()?;
    let interval: i64 = env::var("MESSAGE_INTERVAL")
        .unwrap_or("86400".to_string()) // in seconds
        .parse()?;

    let db_service = Arc::new(DbService::new_sqlite(db_url).await?);

    info!("Migrating database...");
    db_service.migrate().await?;
    info!("Database migrated");

    info!("Starting bot...");
    let bot = Bot::from_env();

    let send_bot = bot.clone();
    let send_db = db_service.clone();
    let (shutdown_send, shutdown_recv) = mpsc::channel(1);

    let send_task_data_dir = data_dir.clone();
    let send_daily_message_task = tokio::spawn(async move {
        let send_result = send_daily_messages(
            send_bot.clone(),
            send_db.clone(),
            interval,
            send_task_data_dir,
            shutdown_recv,
        )
        .await;

        match send_result {
            Ok(_) => (),
            Err(e) => error!("Failed to send daily message: {}", e),
        }
    });

    let recv_db = db_service.clone();
    let handler_db = db_service.clone();

    let message_handler_data_dir = data_dir.clone();
    let message_handler_fn = move |bot: Bot, msg: Message, me: Me| {
        message_handler(
            bot,
            msg,
            me,
            handler_db.clone(),
            message_handler_data_dir.clone(),
        )
    };

    let handler = dptree::entry()
        .branch(Update::filter_message().endpoint(message_handler_fn))
        .branch(
            Update::filter_callback_query()
                .endpoint(move |bot: Bot, q| callback_handler(recv_db.clone(), bot.clone(), q)),
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
