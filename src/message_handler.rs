use crate::db::DbService;
use crate::helpers::{list_files, MAX_RETRY_COUNT};
use crate::make_keyboard;
use crate::sender::send_message;
use anyhow::{anyhow, Result};
use chrono::Utc;
use log::{debug, error, info, warn};
use rand::seq::IteratorRandom;
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::Me;
use teloxide::utils::command::BotCommands;

#[derive(BotCommands)]
#[command(rename_rule = "lowercase")]
enum Command {
    Help,
    Start,
    Unsubscribe,
    Subscribe,
    Random,
}

async fn handle_help_command(bot: Bot, msg: Message) -> Result<(), Box<dyn Error + Send + Sync>> {
    bot.send_message(msg.chat.id, Command::descriptions().to_string())
        .await?;

    info!(
        "Chat id={} title='{}' handled help command",
        msg.chat.id.0,
        msg.chat.title().unwrap_or("")
    );

    Ok(())
}

async fn handle_start_command(bot: Bot, msg: Message) -> Result<(), Box<dyn Error + Send + Sync>> {
    bot.send_message(
        msg.chat.id,
        "Нажмите подписаться чтобы получать каждый день сутту из сайта theravada.ru",
    )
    .await?;

    let keyboard = make_keyboard();
    bot.send_message(msg.chat.id, "Выберите действие:")
        .reply_markup(keyboard)
        .await?;

    info!(
        "Chat id={} title='{}' handled start command",
        msg.chat.id.0,
        msg.chat.title().unwrap_or("")
    );

    Ok(())
}

async fn handle_unsubscribe_command(
    bot: Bot,
    msg: Message,
    db: Arc<DbService>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let chat_id = msg.chat.id.0;
    let existing_subscription = db.get_subscription_by_chat_id(chat_id).await?;

    match existing_subscription {
        Some(subscription) => {
            if subscription.is_enabled == 0 {
                info!(
                    "Chat id={} title='{}' already unsubscribed, doing nothing",
                    chat_id,
                    msg.chat.title().unwrap_or("")
                );
                bot.send_message(msg.chat.id, "Вы уже отписаны от рассылки")
                    .await?;
            } else {
                db.set_subscription_enabled(chat_id, 0, Utc::now().timestamp())
                    .await?;
                info!(
                    "Chat id={} title='{}' unsubscribed",
                    chat_id,
                    msg.chat.title().unwrap_or("")
                );
                bot.send_message(msg.chat.id, "Вы отписались от рассылки")
                    .await?;
            }
        }
        None => {
            info!(
                "Chat id={} title='{}' is not subscribed, doing nothing",
                chat_id,
                msg.chat.title().unwrap_or("")
            );
            bot.send_message(msg.chat.id, "Вы не подписаны на рассылку")
                .await?;
        }
    }
    Ok(())
}

async fn handle_subscribe_command(
    bot: Bot,
    msg: Message,
    db: Arc<DbService>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let chat_id = msg.chat.id.0;
    let existing_subscription = db.get_subscription_by_chat_id(chat_id).await?;

    match existing_subscription {
        Some(subscription) => {
            if subscription.is_enabled == 1 {
                info!(
                    "Chat id={} title='{}' already subscribed, doing nothing",
                    chat_id,
                    msg.chat.title().unwrap_or("")
                );
                bot.send_message(msg.chat.id, "Вы уже подписаны на рассылку")
                    .await?;
            } else {
                db.set_subscription_enabled(chat_id, 1, Utc::now().timestamp())
                    .await?;
                info!(
                    "Chat id={} title='{}' resubscribed",
                    chat_id,
                    msg.chat.title().unwrap_or("")
                );
                bot.send_message(
                    msg.chat.id,
                    "Спасибо! Вы будете получать новую сутту каждый день в 8:00 по Москве",
                )
                .await?;
            }
        }
        None => {
            db.create_subscription(chat_id, 1, Utc::now().timestamp())
                .await?;
            info!(
                "Chat id={} title='{}' subscribed",
                chat_id,
                msg.chat.title().unwrap_or("")
            );
            bot.send_message(
                msg.chat.id,
                "Спасибо! Вы будете получать новую сутту каждый день в 8:00 по Москве",
            )
            .await?;
        }
    }
    Ok(())
}

async fn handle_random_command(
    bot: Bot,
    msg: Message,
    data_dir: PathBuf,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let files = list_files(&data_dir)?;
    // TODO refactor duplication here
    let mut random_file = files
        .iter()
        .choose(&mut rand::thread_rng())
        .ok_or(anyhow!("No files in data dir"))?;
    let mut retry_count = 0;

    while let Err(e) = send_message(&bot, msg.chat.id.0, random_file, make_keyboard()).await {
        warn!("Error sending message: {}", e);
        retry_count += 1;
        // TODO refactor duplication here
        random_file = files
            .iter()
            .choose(&mut rand::thread_rng())
            .ok_or(anyhow!("No files in data dir"))?;

        if retry_count >= MAX_RETRY_COUNT {
            return Err(Box::new(e));
        }
    }

    info!(
        "Chat id={} title='{}' sent random message with filename={}",
        msg.chat.id.0,
        msg.chat.title().unwrap_or(""),
        random_file
            .file_name()
            .to_str()
            .unwrap_or("can't get filename")
    );

    Ok(())
}

pub async fn message_handler(
    bot: Bot,
    msg: Message,
    me: Me,
    db: Arc<DbService>,
    data_dir: PathBuf,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(text) = msg.text() {
        match BotCommands::parse(text, me.username()) {
            Ok(Command::Help) => handle_help_command(bot.clone(), msg.clone()).await?,
            Ok(Command::Start) => handle_start_command(bot.clone(), msg.clone()).await?,
            Ok(Command::Unsubscribe) => {
                handle_unsubscribe_command(bot.clone(), msg.clone(), db.clone()).await?
            }
            Ok(Command::Subscribe) => {
                handle_subscribe_command(bot.clone(), msg.clone(), db.clone()).await?
            }
            Ok(Command::Random) => {
                handle_random_command(bot.clone(), msg.clone(), data_dir).await?
            }
            Err(_) => {
                if text.starts_with('/') {
                    log::info!("Unknown command '{}' in chat {}", text, msg.chat.id.0);
                } else {
                    log::debug!("Unknown command '{}' in chat {}", text, msg.chat.id.0);
                }
            }
        }
    }

    Ok(())
}
