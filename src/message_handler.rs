use crate::db::DbService;
use crate::make_keyboard;
use chrono::Utc;
use std::error::Error;
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
}

async fn handle_help_command(bot: Bot, msg: Message) -> Result<(), Box<dyn Error + Send + Sync>> {
    bot.send_message(msg.chat.id, Command::descriptions().to_string())
        .await?;
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
                bot.send_message(msg.chat.id, "Вы уже отписаны от рассылки")
                    .await?;
            } else {
                db.set_subscription_enabled(chat_id, 0, Utc::now().timestamp())
                    .await?;
                bot.send_message(msg.chat.id, "Вы отписались от рассылки")
                    .await?;
            }
        }
        None => {
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
                bot.send_message(msg.chat.id, "Вы уже подписаны на рассылку")
                    .await?;
            } else {
                db.set_subscription_enabled(chat_id, 1, Utc::now().timestamp())
                    .await?;
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
            bot.send_message(
                msg.chat.id,
                "Спасибо! Вы будете получать новую сутту каждый день в 8:00 по Москве",
            )
            .await?;
        }
    }
    Ok(())
}

pub async fn message_handler(
    bot: Bot,
    msg: Message,
    me: Me,
    db: Arc<DbService>,
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
            Err(_) => {
                bot.send_message(msg.chat.id, "Command not found!").await?;
            }
        }
    }

    Ok(())
}
