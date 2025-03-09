use anyhow::anyhow;
use anyhow::Result;
use log::{debug, error, info};
use rand::seq::IteratorRandom;
use std::fs;
use std::fs::DirEntry;
use std::path::PathBuf;
use teloxide::payloads::SendMessageSetters;
use teloxide::requests::Requester;
use teloxide::types::ChatId;
use teloxide::types::ParseMode;
use teloxide::RequestError;
use thiserror::Error;

use teloxide::Bot;

use crate::helpers::TELEGRAM_TEXT_MAX_LENGTH;

#[derive(Error, Debug)]
pub enum TgMessageSendError {
    #[error("TgMessageSendError.RetryAfter: {0:?}")]
    RetryAfter(std::time::Duration),
    #[error("TgMessageSendError.BotBlocked")]
    BotBlocked,
    #[error("TgMessageSendError.TeloxideError: {0}")]
    TeloxideError(teloxide::RequestError),
    #[error("TgMessageSendError.UnknownError: {0}")]
    UnknownError(anyhow::Error),
}

fn map_send_error<T>(send_result: Result<T, RequestError>) -> Result<(), TgMessageSendError> {
    match send_result {
        Ok(_) => Ok(()),
        Err(e) => {
            match e {
                // ignoring this error due to teloxide bug
                teloxide::RequestError::InvalidJson { source: _, raw: _ } => {
                    debug!("Ignoring InvalidJson error: {}", e);
                    Ok(())
                }
                teloxide::RequestError::RetryAfter(duration) => {
                    Err(TgMessageSendError::RetryAfter(duration))
                }
                teloxide::RequestError::Api(api_error) => match api_error {
                    teloxide::ApiError::BotBlocked => Err(TgMessageSendError::BotBlocked),
                    _ => Err(TgMessageSendError::TeloxideError(
                        teloxide::RequestError::Api(api_error.clone()),
                    )),
                },

                _ => Err(TgMessageSendError::TeloxideError(e)),
            }
        }
    }
}

pub async fn send_daily_message(
    bot: &Bot,
    chat_id: i64,
    files: &[DirEntry],
) -> Result<(), TgMessageSendError> {
    let file = files
        .iter()
        .choose(&mut rand::thread_rng())
        .ok_or(anyhow!("No files in data dir"))
        .map_err(TgMessageSendError::UnknownError)?;

    send_file_text_to_chat(bot, chat_id, file.path()).await
}

pub async fn send_file_text_to_chat(
    bot: &Bot,
    chat_id: i64,
    file: PathBuf,
) -> Result<(), TgMessageSendError> {
    let content = fs::read_to_string(file.clone())
        .map_err(|err| anyhow!("Failed to read file: {:?} error: {}", file.clone(), err))
        .map_err(TgMessageSendError::UnknownError)?;

    let texts = chunk_text(content);

    info!(
        "Sending message to chat_id: {}, filename: {}",
        chat_id,
        file.file_name().unwrap_or_default().to_string_lossy()
    );

    for text in texts.iter() {
        let send_msg = bot
            .send_message(ChatId(chat_id), text)
            .parse_mode(ParseMode::MarkdownV2);

        //TODO remove previous message if second failed to send
        map_send_error(send_msg.await)?;
    }

    Ok(())
}

fn chunk_text(content: String) -> Vec<String> {
    // Create chunks that don't break escape sequences
    let mut texts = Vec::new();
    let mut current_chunk = String::new();

    for c in content.chars() {
        // If adding this character would exceed the limit
        if current_chunk.len() + 1 > TELEGRAM_TEXT_MAX_LENGTH {
            // If the last character is a backslash, remove it and save it for the next chunk
            if current_chunk.ends_with('\\') {
                let len = current_chunk.len();
                let without_backslash = &current_chunk[..len - 1];
                texts.push(without_backslash.to_string());
                current_chunk = String::from('\\');
            } else {
                texts.push(current_chunk);
                current_chunk = String::new();
            }
        }

        current_chunk.push(c);
    }

    // Don't forget the last chunk
    if !current_chunk.is_empty() {
        texts.push(current_chunk);
    }
    texts
}
