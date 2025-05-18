use anyhow::anyhow;
use anyhow::Result;
use chrono::{Duration as ChronoDuration, Local, Utc, Timelike};
use db::DbService;
use helpers::list_files;
use log::{debug, error, info, warn};
use std::path::PathBuf;
use std::{env, error::Error, path::Path, sync::Arc, collections::HashMap, fs::DirEntry};
use teloxide::{
    dispatching::dialogue::GetChatId,
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, Me, ParseMode},
    utils::command::BotCommands,
};
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval_at, Instant, interval, sleep, Duration as TokioDuration};

mod time_utils;
mod scheduler;
mod keyboards;

use crate::message_handler::message_handler;
use crate::sender::{send_daily_message, TgMessageSendError};
use crate::scheduler::{ScheduleManager, run_schedule_manager_refresh_loop, scheduler_loop};

mod db;
mod dto;
mod helpers;
mod message_handler;
mod sender;

async fn callback_handler(
    db: Arc<DbService>,
    bot: Bot,
    q: CallbackQuery,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(chosen_action) = &q.data {
        if let Some(chat_id_teloxide) = q.chat_id() {
            let chat_id = chat_id_teloxide.0;
            let now = Utc::now();
            let timestamp = now.timestamp();

            match chosen_action.as_str() {
                "Подписаться" => {
                    let existing_subscription = db.get_subscription_by_chat_id(chat_id).await?;
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
                                    keyboards::make_unsubscribe_keyboard(),
                                )
                                .await?;
                            } else {
                                info!(
                                    "Updating subscription {} for chat_id: {}",
                                    subscription.id, chat_id
                                );
                                db.set_subscription_enabled(chat_id, 1, timestamp).await?;
                                let text = "Вы снова подписаны на рассылку.";
                                answer_message_with_replace(
                                    &bot,
                                    q.id,
                                    q.message,
                                    text,
                                    keyboards::make_unsubscribe_keyboard(),
                                )
                                .await?;
                            }
                        }
                        None => {
                            info!("Inserting new subscription for chat_id: {}", chat_id);
                            db.create_subscription(chat_id, 1, timestamp).await?;
                            let text = "Спасибо! Вы будете получать новую сутту каждый день в 8:00 по Москве";
                            answer_message_with_replace(
                                &bot,
                                q.id,
                                q.message,
                                text,
                                keyboards::make_unsubscribe_keyboard(),
                            )
                            .await?;
                        }
                    }
                }
                "Отписаться" => {
                    info!("Disabling subscription for chat_id: {}", chat_id);
                    db.set_subscription_enabled(chat_id, 0, timestamp).await?;
                    let text = "Вы отписались от рассылки";
                    answer_message_with_replace(
                        &bot,
                        q.id,
                        q.message,
                        text,
                        keyboards::make_subscribe_keyboard(),
                    )
                    .await?;
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

fn get_data_dir() -> Result<PathBuf> {
    let data_dir_str = env::var("DATA_DIR")?;
    let path = Path::new(&data_dir_str);
    if !path.is_dir() {
        return Err(anyhow!("DATA_DIR '{}' is not a directory", data_dir_str));
    }
    Ok(PathBuf::from(data_dir_str))
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    pretty_env_logger::init();

    let db_url = &env::var("DATABASE_URL")?;
    let data_dir = get_data_dir()?;

    let db_service = Arc::new(DbService::new_sqlite(db_url).await?);

    info!("Migrating database...");
    db_service.migrate().await?;
    info!("Database migrated");

    info!("Starting bot...");
    let bot = Bot::from_env();

    let schedule_manager = Arc::new(ScheduleManager::new(db_service.clone()));

    if let Err(e) = schedule_manager.refresh().await {
        error!("Initial schedule refresh failed: {}. Exiting.", e);
        return Err(e.into());
    }

    let sm_refresh_clone = schedule_manager.clone();
    tokio::spawn(async move {
        run_schedule_manager_refresh_loop(sm_refresh_clone).await;
    });

    let sutta_files_list = match list_files(&data_dir) {
        Ok(files) => Arc::new(files),
        Err(e) => {
            error!("Failed to list sutta files from data_dir '{}': {}. Exiting.", data_dir.display(), e);
            return Err(e.into());
        }
    };
    info!("Loaded {} sutta files for scheduler.", sutta_files_list.len());

    let bot_scheduler_clone = bot.clone();
    let schedule_data_for_loop = schedule_manager.get_schedule_data_arc();
    let data_dir_scheduler_clone = data_dir.clone();
    tokio::spawn(async move {
        scheduler_loop(
            bot_scheduler_clone,
            schedule_data_for_loop,
            data_dir_scheduler_clone,
            sutta_files_list
        ).await;
    });

    let dispatcher_db = db_service.clone();
    let dispatcher_schedule_manager = schedule_manager.clone();
    let dispatcher_data_dir = data_dir.clone();

    let message_handler_fn = move |bot_instance: Bot, msg: Message, me: Me| {
        message_handler(
            bot_instance,
            msg,
            me,
            dispatcher_db.clone(),
            dispatcher_data_dir.clone(),
            dispatcher_schedule_manager.clone(),
        )
    };

    let callback_db_clone = db_service.clone();

    let handler = dptree::entry()
        .branch(Update::filter_message().endpoint(message_handler_fn))
        .branch(Update::filter_callback_query().endpoint(
            move |bot_instance: Bot, q: CallbackQuery| {
                callback_handler(
                    callback_db_clone.clone(),
                    bot_instance,
                    q
                )
            },
        ));

    info!("Dispatcher starting...");
    Dispatcher::builder(bot.clone(), handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    let (shutdown_send, mut shutdown_recv) = mpsc::channel::<()>(1);
    let mut term_signal = signal(SignalKind::terminate())?;
    let mut int_signal = signal(SignalKind::interrupt())?;

    tokio::spawn(async move {
        tokio::select! {
            _ = term_signal.recv() => info!("Received SIGTERM, initiating shutdown..."),
            _ = int_signal.recv() => info!("Received SIGINT (Ctrl+C), initiating shutdown..."),
        }
    });

    info!("Bot shut down gracefully.");
    Ok(())
}
