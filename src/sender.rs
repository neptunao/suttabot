use anyhow::anyhow;
use anyhow::Result;
use log::warn;
use log::{debug, error, info};
use rand::seq::IteratorRandom;
use std::fs;
use std::fs::DirEntry;
use teloxide::payloads::SendMessageSetters;
use teloxide::requests::Requester;
use teloxide::types::ChatId;
use teloxide::types::InlineKeyboardMarkup;
use teloxide::types::ParseMode;
use teloxide::RequestError;
use thiserror::Error;

use teloxide::Bot;

const TELEGRAM_TEXT_MAX_LENGTH: usize = 4096;

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
    keyboard: InlineKeyboardMarkup,
) -> Result<(), TgMessageSendError> {
    let file = files
        .iter()
        .choose(&mut rand::thread_rng())
        .ok_or(anyhow!("No files in data dir"))
        .map_err(TgMessageSendError::UnknownError)?;

    let texts = fs::read_to_string(file.path())
        .map_err(|err| anyhow!("Failed to read file: {:?} error: {}", file.path(), err))
        .map_err(TgMessageSendError::UnknownError)?
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
        map_send_error(send_msg.await)?;
    }

    Ok(())
}
