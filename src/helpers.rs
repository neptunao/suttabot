use std::path::Path;
use anyhow::Result;

pub const TELEGRAM_TEXT_MAX_LENGTH: usize = 4096;
pub const MAX_RETRY_COUNT: usize = 5;

pub fn list_files(dir: &Path) -> Result<Vec<std::fs::DirEntry>> {
    let files = dir
        .read_dir()?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|ft| ft.is_file()).unwrap_or(false))
        .collect::<Vec<_>>();

    Ok(files)
}
