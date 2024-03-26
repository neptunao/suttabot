use anyhow::anyhow;
use anyhow::Result;
use log::{error, info};
use rand::seq::IteratorRandom;
use std::fs;
use std::fs::DirEntry;
use teloxide::payloads::SendMessageSetters;
use teloxide::requests::Requester;
use teloxide::types::ChatId;
use teloxide::types::InlineKeyboardMarkup;
use teloxide::types::ParseMode;

use teloxide::Bot;

const TELEGRAM_TEXT_MAX_LENGTH: usize = 4096;

pub async fn send_daily_message(
    bot: &Bot,
    chat_id: i64,
    files: &[DirEntry],
    keyboard: InlineKeyboardMarkup,
) -> Result<(), anyhow::Error> {
    let file = files
        .iter()
        .choose(&mut rand::thread_rng())
        .ok_or(anyhow!("No files in data dir"))?;

    let texts = fs::read_to_string(file.path())?
        .chars()
        .collect::<Vec<char>>()
        .chunks(TELEGRAM_TEXT_MAX_LENGTH)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect::<Vec<String>>();

    info!(
        "Sending daily message to chat_id: {}, filename: {}",
        chat_id,
        file.file_name().to_string_lossy()
    );

    for (i, text) in texts.iter().enumerate() {
        let mut send_msg = bot
            .send_message(ChatId(chat_id), text)
            .parse_mode(ParseMode::MarkdownV2);

        if i == texts.len() - 1 {
            send_msg = send_msg.reply_markup(keyboard.clone()); // TODO bug: last message will be replaced with keyboard if unsubscribe is clicked
        }

        //TODO remove previous message if second failed to send
        if let Err(e) = send_msg.await {
            error!(
                "Failed to send message to chat_id: {} filename: {} error: {}",
                chat_id,
                file.file_name().to_string_lossy(),
                e
            );
        }
    }

    Ok(())
}
