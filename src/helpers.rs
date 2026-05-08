use anyhow::Result;
use std::path::{Path, PathBuf};

pub const TELEGRAM_TEXT_MAX_LENGTH: usize = 4096;
pub const MAX_RETRY_COUNT: usize = 5;
pub const MAX_SENDOUT_TIMES: usize = 10;
pub const DONATION_FILE_PATH: &str = "data/donation_info.md";
pub const CONFIG_PATH: &str = "config.yaml";

pub fn news_dir() -> PathBuf {
    let base = std::env::var("DATA_DIR").unwrap_or_else(|_| "data/ru".to_string());
    PathBuf::from(base).join("news")
}

pub fn list_files(dir: &Path) -> Result<Vec<std::fs::DirEntry>> {
    let files = dir
        .read_dir()?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|ft| ft.is_file()).unwrap_or(false))
        .collect::<Vec<_>>();

    Ok(files)
}
