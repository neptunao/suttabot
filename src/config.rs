use anyhow::Result;
use serde::Deserialize;
use std::fs;
use teloxide::types::User;

use crate::helpers::CONFIG_PATH;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub admins: Vec<AdminEntry>,
}

#[derive(Debug, Deserialize)]
pub struct AdminEntry {
    pub user_id: Option<i64>,
    pub username: Option<String>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let content = fs::read_to_string(CONFIG_PATH)?;
        let config: Config = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    pub fn is_admin(&self, user: &User) -> bool {
        self.admins.iter().any(|admin| {
            let id_match = admin.user_id == Some(user.id.0 as i64);
            let username_match = admin
                .username
                .as_deref()
                .zip(user.username.as_deref())
                .is_some_and(|(a, b)| a.eq_ignore_ascii_case(b));
            id_match || username_match
        })
    }
}
