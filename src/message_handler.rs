use crate::db::DbService;
use crate::helpers::{list_files, MAX_RETRY_COUNT, MAX_SENDOUT_TIMES};
use crate::make_keyboard;
use crate::sender::send_file_text_to_chat;
use anyhow::{anyhow, Result};
use chrono::Utc;
use log::{info, warn};
use rand::seq::IteratorRandom;
use regex::Regex;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::Me;
use teloxide::utils::command::BotCommands;

#[derive(BotCommands)]
#[command(rename_rule = "lowercase")]
enum Command {
    #[command(description = "показать это сообщение")]
    Help,
    #[command(description = "начать работу с ботом")]
    Start,
    #[command(description = "отписаться от рассылки")]
    Unsubscribe,
    #[command(description = "подписаться на рассылку")]
    Subscribe,
    #[command(description = "получить случайную сутту")]
    Random,
    #[command(description = "установить время рассылки в формате 6:00 8:18 19:31")]
    SetTime,
    #[command(description = "найти сутту по номеру, например: /get МН 65")]
    Get(String),
    #[command(description = "информация о поддержке Дхамма-центров")]
    Dana,
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

async fn handle_dana_command(
    bot: Bot,
    msg: Message,
    db: Arc<DbService>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    use std::time::{SystemTime, UNIX_EPOCH};
    use crate::helpers::DONATION_FILE_PATH;

    // Send donation info WITHOUT a sutta
    let donation_file = PathBuf::from(DONATION_FILE_PATH);

    if let Err(e) = send_file_text_to_chat(&bot, msg.chat.id.0, donation_file).await {
        log::error!("Failed to send donation info to chat_id={}: {:?}", msg.chat.id, e);
        return Err(Box::new(e));
    }

    // Bump reminder counter and timestamp (UTC)
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| anyhow::anyhow!("System time error: {}", e))?
        .as_secs() as i64;

    if let Err(e) = db.update_donation_reminder(msg.chat.id.0, timestamp).await {
        log::error!("Failed to update donation reminder for chat_id={}: {:?}", msg.chat.id, e);
    }

    info!("Chat id={} handled dana command", msg.chat.id.0);
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
    donation_period: i64,
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
            // Initialize sendout_count to DONATION_MESSAGE_PERIOD - 1 so first donation occurs after first sent sutta
            let initial_sendout = if donation_period > 0 { donation_period - 1 } else { 0 };

            db.create_subscription(chat_id, 1, Utc::now().timestamp(), initial_sendout)
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

    while let Err(e) = send_file_text_to_chat(&bot, msg.chat.id.0, random_file.path()).await {
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

async fn handle_set_time_command(
    bot: Bot,
    msg: Message,
    db: Arc<DbService>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let chat_id = msg.chat.id.0;

    // format is 6:00 8:00 18:00
    let times_str = msg
        .text()
        .unwrap_or("8:00")
        .split_whitespace()
        .skip(1)
        .collect::<Vec<&str>>();

    // now we need to translate time into integer like 08:00 is 800 and 0:00 is 0 and 1:15 is 115, returning result of vec
    let times = times_str
        .iter()
        .map(|time| {
            let time_parts = time.split(':').collect::<Vec<&str>>();
            let hours = time_parts[0].parse::<i32>()?;
            let minutes = time_parts[1].parse::<i32>()?;
            Ok::<i32, anyhow::Error>(hours * 100 + minutes)
        })
        .collect::<Result<Vec<i32>, anyhow::Error>>()?;

    if times.is_empty() {
        bot.send_message(
            msg.chat.id,
            "Укажите время рассылки в формате 6:00 8:18 19:31",
        )
        .await?;

        return Ok(());
    }

    if times.len() > MAX_SENDOUT_TIMES {
        bot.send_message(
            msg.chat.id,
            format!(
                "Максимальное количество времени рассылки - {} раз в сутки.",
                MAX_SENDOUT_TIMES
            ),
        )
        .await?;

        return Ok(());
    }

    let existing_subscription = db.get_subscription_by_chat_id(chat_id).await?;
    match existing_subscription {
        Some(subscription) => {
            if subscription.is_enabled == 0 {
                info!(
                    "handle_set_time_command: Chat id={} title='{}' is not subscribed, doing nothing",
                    chat_id,
                    msg.chat.title().unwrap_or("")
                );
                bot.send_message(msg.chat.id, "Вы не подписаны на рассылку")
                    .await?;
            } else {
                db.set_sendout_times(subscription.id, &times).await?;
                info!(
                    "handle_set_time_command: Chat id={} title='{}' set time to {:?}",
                    chat_id,
                    msg.chat.title().unwrap_or(""),
                    &times
                );
                bot.send_message(
                    msg.chat.id,
                    "Время рассылки изменено. Вы будете получать новую сутту каждый день в указанное время",
                )
                .await?;
            }
        }
        None => {
            info!(
                "handle_set_time_command: Chat id={} title='{}' is not subscribed, doing nothing",
                chat_id,
                msg.chat.title().unwrap_or("")
            );
            bot.send_message(msg.chat.id, "Вы не подписаны на рассылку")
                .await?;
        }
    }

    Ok(())
}

// Function to find a sutta file based on a search query
fn find_sutta_file(data_dir: &Path, query: &str) -> Result<Option<PathBuf>, anyhow::Error> {
    let files = list_files(data_dir)?;

    // Normalize the query by removing spaces and converting to lowercase
    let normalized_query = query.to_lowercase().replace(" ", "");

    // Try to extract collection code and number
    // This regex handles both formats: mn65 and sn1.10
    let re = Regex::new(r"(?i)(mn|sn|an|dn|vv|ud|thag|thig|snp|iti|мн|сн|ан|дн|вв|уд|тхаг|тхиг|снп|ити)\s*(\d+)(?:\.(\d+))?").unwrap();

    if let Some(caps) = re.captures(&normalized_query) {
        let collection = caps.get(1).map_or("", |m| m.as_str()).to_lowercase();
        let main_number: i32 = caps
            .get(2)
            .ok_or(anyhow!("Main number not provided"))?
            .as_str()
            .parse()
            .map_err(|_| anyhow!("Failed to convert main number to integer"))?;

        let sub_number: Option<i32> = caps
            .get(3)
            .map(|m| {
                m.as_str()
                    .parse()
                    .map_err(|_| anyhow!("Failed to convert sub number to integer"))
            })
            .transpose()?;

        let collection_en = ru_code_to_en(&collection);

        // Create different possible formats for the filename
        let patterns = filename_patterns(main_number, sub_number, &collection_en);

        // First, try exact matches
        for file in &files {
            let filename = file.file_name().to_string_lossy().to_lowercase();

            for pattern in &patterns {
                if filename.contains(pattern) {
                    return Ok(Some(file.path()));
                }
            }

            if let Some(value) =
                find_in_ranged_suttas(main_number, sub_number, &collection_en, file, filename)?
            {
                return Ok(Some(value));
            }
        }

        // If no exact match, try fuzzy match
        if let Some(value) = fuzzy_search_sutta(files, main_number, sub_number, collection_en)? {
            return Ok(Some(value));
        }
    }

    Ok(None)
}

fn filename_patterns(
    main_number: i32,
    sub_number: Option<i32>,
    collection_en: &String,
) -> Vec<String> {
    let mut patterns = Vec::new();

    match sub_number {
        None => {
            patterns.push(format!("{}{}", collection_en, main_number)); // e.g. mn65
        }
        Some(sub_num) => {
            // e.g. sn1.10
            patterns.push(format!("{}{}.{}", collection_en, main_number, sub_num));
            // e.g. sn1_10
            patterns.push(format!("{}{}_{}", collection_en, main_number, sub_num));
        }
    }

    patterns
}

fn fuzzy_search_sutta(
    files: Vec<std::fs::DirEntry>,
    main_number: i32,
    sub_number: Option<i32>,
    collection_en: String,
) -> Result<Option<PathBuf>, anyhow::Error> {
    // If no exact match, try fuzzy match
    for file in &files {
        let filename = file.file_name().to_string_lossy().to_lowercase();

        // Skip files that don't contain the collection code
        if !filename.contains(&collection_en) {
            continue;
        }

        // Skip files that don't contain the main number
        if !filename.contains(&main_number.to_string()) {
            continue;
        }

        // If there is no sub-number, return immediately, e.g. mn65
        if sub_number.is_none() {
            return Ok(Some(file.path()));
        }

        // For files with sub-number, check if it's present
        if let Some(sub_num) = sub_number {
            if filename.contains(&sub_num.to_string()) {
                return Ok(Some(file.path()));
            }
        }
    }

    Ok(None)
}

fn find_in_ranged_suttas(
    main_number: i32,
    sub_number: Option<i32>,
    collection_en: &String,
    file: &std::fs::DirEntry,
    filename: String,
) -> Result<Option<PathBuf>, anyhow::Error> {
    let sub_num = match sub_number {
        Some(num) => num,
        None => return Ok(None),
    };

    // Check for range files like sn35.195-197.md
    // Create regex to match patterns like "sn35.195-197" or "sn35_195-197"
    let range_pattern = format!(r"{}{}[\.-_](\d+)-(\d+)", collection_en, main_number);

    let range_re =
        Regex::new(&range_pattern).map_err(|e| anyhow!("Error creating range regex: {}", e))?;

    // Try to find and extract the range from the filename
    let range_caps = match range_re.captures(&filename) {
        Some(caps) => caps,
        None => return Ok(None),
    };

    // Parse the start and end numbers of the range
    let start_num: i32 = range_caps
        .get(1)
        .map_or("0", |m| m.as_str())
        .parse()
        .unwrap_or(0);

    let end_num: i32 = range_caps
        .get(2)
        .map_or("0", |m| m.as_str())
        .parse()
        .unwrap_or(0);

    // Check if the sub_num is within the range
    if sub_num >= start_num && sub_num <= end_num {
        return Ok(Some(file.path()));
    }

    Ok(None)
}

fn ru_code_to_en(collection: &str) -> String {
    let collection_en = match collection {
        "мн" => "mn",
        "сн" => "sn",
        "ан" => "an",
        "дн" => "dn",
        "вв" => "vv",
        "уд" => "ud",
        "тхаг" => "thag",
        "тхиг" => "thig",
        "снп" => "snp",
        "ити" => "iti",
        _ => collection,
    };

    collection_en.to_string()
}

async fn handle_get_command(
    bot: Bot,
    msg: Message,
    query: String,
    data_dir: PathBuf,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if query.is_empty() {
        bot.send_message(
            msg.chat.id,
            "Пожалуйста, укажите название сутты для поиска, например: /get МН 65",
        )
        .await?;
        return Ok(());
    }

    match find_sutta_file(&data_dir, &query) {
        Ok(Some(file_path)) => {
            info!(
                "Chat id={} title='{}' found and sending sutta with filename={} by query={}",
                msg.chat.id.0,
                msg.chat.title().unwrap_or(""),
                file_path.file_name().unwrap_or_default().to_string_lossy(),
                query
            );

            send_file_text_to_chat(&bot, msg.chat.id.0, file_path.clone()).await?;
        }
        Ok(None) => {
            bot.send_message(msg.chat.id, format!("Сутта '{}' не найдена.", query))
                .await?;
        }
        Err(e) => {
            warn!("Error searching for sutta: {}", e);
            bot.send_message(msg.chat.id, "Произошла ошибка при поиске сутты.")
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
    data_dir: PathBuf,
    donation_period: i64,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(text) = msg.text() {
        match BotCommands::parse(text, me.username()) {
            Ok(Command::Help) => handle_help_command(bot.clone(), msg.clone()).await?,
            Ok(Command::Start) => handle_start_command(bot.clone(), msg.clone()).await?,
            Ok(Command::Unsubscribe) => {
                handle_unsubscribe_command(bot.clone(), msg.clone(), db.clone()).await?
            }
            Ok(Command::Subscribe) => {
                handle_subscribe_command(bot.clone(), msg.clone(), db.clone(), donation_period).await?
            }
            Ok(Command::Random) => {
                handle_random_command(bot.clone(), msg.clone(), data_dir.clone()).await?
            }
            Ok(Command::SetTime) => {
                handle_set_time_command(bot.clone(), msg.clone(), db.clone()).await?
            }
            Ok(Command::Get(query)) => {
                handle_get_command(bot.clone(), msg.clone(), query, data_dir.clone()).await?
            }
            Ok(Command::Dana) => {
                handle_dana_command(bot.clone(), msg.clone(), db.clone()).await?
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
