use crate::db::DbService;
use crate::helpers::{list_files, MAX_RETRY_COUNT, MAX_SENDOUT_TIMES};
use crate::make_keyboard;
use crate::sender::{send_file_text_to_chat, send_audio_file_to_chat};
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
use std::ffi::OsStr;

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
    #[command(description = "получить аудиофайл сутты, например: /audio МН 1")]
    Audio(String),
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

#[derive(Debug, PartialEq)]
struct SuttaIdentifier {
    collection: String,
    main_number: i32,
    sub_number: Option<String>, // Keep as string to preserve exact format
}

fn parse_sutta_identifier(query: &str) -> Result<Option<SuttaIdentifier>, anyhow::Error> {
    let normalized_query = query.to_lowercase().replace(" ", "");
    let re = Regex::new(r"(?i)(mn|sn|an|dn|vv|ud|thag|thig|snp|iti|мн|сн|ан|дн|вв|уд|тхаг|тхиг|снп|ити)\s*(\d+)(?:\.(\d+))?")?;

    let caps = match re.captures(&normalized_query) {
        Some(c) => c,
        None => return Ok(None),
    };

    let collection = caps.get(1).map_or("", |m| m.as_str()).to_lowercase();
    let main_number: i32 = caps
        .get(2)
        .map(|m| m.as_str().parse().unwrap_or(0))
        .unwrap_or(0);
    let sub_number = caps.get(3).map(|m| m.as_str().to_string());
    let collection_en = ru_code_to_en(&collection);

    Ok(Some(SuttaIdentifier {
        collection: collection_en,
        main_number,
        sub_number,
    }))
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

fn matches_sutta_identifier(filename: &str, sutta: &SuttaIdentifier) -> bool {
    let filename = filename.to_lowercase();
    let audio_extensions = ["mp3", "ogg", "wav", "m4a", "flac"];
    let filename_no_ext = match filename.rsplit_once('.') {
        Some((name, ext)) if audio_extensions.contains(&ext) => name,
        _ => filename.as_str(),
    };
    let pattern = match &sutta.sub_number {
        Some(sub) => format!("{}{}.{}", sutta.collection, sutta.main_number, sub),
        None => format!("{}{}", sutta.collection, sutta.main_number),
    };
    filename_no_ext == pattern
}

fn find_audio_by_sutta_name(
    audio_files: &[std::fs::DirEntry],
    query: &str,
) -> Result<Option<PathBuf>, anyhow::Error> {
    let sutta = match parse_sutta_identifier(query)? {
        Some(s) => s,
        None => return Ok(None),
    };

    for file in audio_files {
        let filename = file.file_name().to_string_lossy().to_string();
        if matches_sutta_identifier(&filename, &sutta) {
            return Ok(Some(file.path()));
        }
    }

    Ok(None)
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

fn get_audio_files(audio_dir: &Path) -> Result<Vec<std::fs::DirEntry>, anyhow::Error> {
    let audio_extensions = ["mp3", "ogg", "wav", "m4a", "flac"];
    let files = list_files(audio_dir)?;

    // Helper to check extension
    let is_audio = |file: &std::fs::DirEntry| {
        file.path()
            .extension()
            .and_then(OsStr::to_str)
            .map(|ext| audio_extensions.contains(&ext))
            .unwrap_or(false)
    };

    Ok(files.into_iter().filter(|f| is_audio(f)).collect())
}

fn get_random_audio_file(audio_files: &[std::fs::DirEntry]) -> Option<PathBuf> {
    audio_files
        .iter()
        .choose(&mut rand::rng())
        .map(|f| f.path())
}

async fn handle_audio_command(
    bot: Bot,
    msg: Message,
    query: String,
    data_dir: PathBuf,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let audio_dir = data_dir.join("audio");

    // Get list of audio files
    let audio_files = match get_audio_files(&audio_dir) {
        Ok(files) => files,
        Err(e) => {
            log::error!("Error listing audio files: {}", e);
            bot.send_message(msg.chat.id, "Ошибка при отправке аудиофайла.").await?;
            return Ok(());
        }
    };

    if audio_files.is_empty() {
        log::warn!("Audio files dir is empty");
        bot.send_message(msg.chat.id, "Ошибка при отправке аудиофайла.").await?;
        return Ok(());
    }

    // Find audio file to send
    let file_to_send = if query.trim().is_empty() {
        get_random_audio_file(&audio_files)
    } else {
        match find_audio_by_sutta_name(&audio_files, &query) {
            Ok(Some(path)) => Some(path),
            Ok(None) => None,
            Err(e) => {
                log::error!("Error finding audio file: {}", e);
                None
            }
        }
    };

    // Send the audio file
    match file_to_send {
        Some(path) => {
            info!(
                "Chat id={} title='{}' sending audio with filename={}",
                msg.chat.id.0,
                msg.chat.title().unwrap_or(""),
                path.file_name().unwrap_or_default().to_string_lossy()
            );
            if let Err(e) = send_audio_file_to_chat(&bot, msg.chat.id.0, path.clone()).await {
                warn!("Error sending audio: {}", e);
                bot.send_message(msg.chat.id, "Ошибка при отправке аудиофайла.").await?;
            }
        }
        None => {
            bot.send_message(msg.chat.id, "Аудиофайл не найден.").await?;
        }
    }

    Ok(())
}

fn find_sutta_file(data_dir: &Path, query: &str) -> Result<Option<PathBuf>, anyhow::Error> {
    let files = list_files(data_dir)?;
    let sutta = match parse_sutta_identifier(query)? {
        Some(s) => s,
        None => return Ok(None),
    };

    for file in &files {
        let filename = file.file_name().to_string_lossy().to_string();
        if matches_sutta_identifier(&filename, &sutta) {
            return Ok(Some(file.path()));
        }
    }

    Ok(None)
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
                handle_random_command(bot.clone(), msg.clone(), data_dir.clone()).await?
            }
            Ok(Command::SetTime) => {
                handle_set_time_command(bot.clone(), msg.clone(), db.clone()).await?
            }
            Ok(Command::Get(query)) => {
                handle_get_command(bot.clone(), msg.clone(), query, data_dir.clone()).await?
            }
            Ok(Command::Audio(query)) => {
                handle_audio_command(bot.clone(), msg.clone(), query, data_dir.clone()).await?
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sutta_identifier() -> Result<(), anyhow::Error> {
        // Test basic parsing
        assert_eq!(
            parse_sutta_identifier("an6.5")?,
            Some(SuttaIdentifier {
                collection: "an".to_string(),
                main_number: 6,
                sub_number: Some("5".to_string()),
            })
        );

        // Test with spaces
        assert_eq!(
            parse_sutta_identifier("an 6.5")?,
            Some(SuttaIdentifier {
                collection: "an".to_string(),
                main_number: 6,
                sub_number: Some("5".to_string()),
            })
        );

        // Test without sub-number
        assert_eq!(
            parse_sutta_identifier("mn1")?,
            Some(SuttaIdentifier {
                collection: "mn".to_string(),
                main_number: 1,
                sub_number: None,
            })
        );

        // Test Russian collection codes
        assert_eq!(
            parse_sutta_identifier("ан6.5")?,
            Some(SuttaIdentifier {
                collection: "an".to_string(),
                main_number: 6,
                sub_number: Some("5".to_string()),
            })
        );

        // Test invalid formats
        assert_eq!(parse_sutta_identifier("invalid")?, None);
        assert_eq!(parse_sutta_identifier("an")?, None);
        assert_eq!(parse_sutta_identifier("an.")?, None);

        Ok(())
    }

    #[test]
    fn test_matches_sutta_identifier() {
        // Test AN6.5
        let an6_5 = SuttaIdentifier {
            collection: "an".to_string(),
            main_number: 6,
            sub_number: Some("5".to_string()),
        };
        assert!(matches_sutta_identifier("an6.5", &an6_5));
        assert!(matches_sutta_identifier("an6.5.mp3", &an6_5));
        assert!(!matches_sutta_identifier("an6.50", &an6_5));  // Different sub-number
        assert!(!matches_sutta_identifier("an6.51", &an6_5));  // Different sub-number

        // Test MN1 (no sub-number)
        let mn1 = SuttaIdentifier {
            collection: "mn".to_string(),
            main_number: 1,
            sub_number: None,
        };
        assert!(matches_sutta_identifier("mn1", &mn1));
        assert!(matches_sutta_identifier("mn1.mp3", &mn1));
        assert!(!matches_sutta_identifier("mn1.1", &mn1));  // Different sutta (has sub-number)
        assert!(!matches_sutta_identifier("mn1.1.mp3", &mn1));  // Different sutta (has sub-number)

        // Test SN1.10
        let sn1_10 = SuttaIdentifier {
            collection: "sn".to_string(),
            main_number: 1,
            sub_number: Some("10".to_string()),
        };
        assert!(matches_sutta_identifier("sn1.10", &sn1_10));
        assert!(matches_sutta_identifier("sn1.10.mp3", &sn1_10));
        assert!(!matches_sutta_identifier("an1.10", &sn1_10));  // Different collection
        assert!(!matches_sutta_identifier("mn1.10", &sn1_10));  // Different collection

        // Test case insensitivity
        assert!(matches_sutta_identifier("AN6.5", &an6_5));
        assert!(matches_sutta_identifier("An6.5", &an6_5));
        assert!(matches_sutta_identifier("an6.5", &an6_5));
    }
}
