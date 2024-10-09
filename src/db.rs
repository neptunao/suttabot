use sqlx::SqlitePool;

use crate::dto::SubscriptionDto;

pub struct DbService {
    pool: SqlitePool,
}

impl DbService {
    pub async fn new_sqlite(url: &str) -> Result<Self, sqlx::Error> {
        let pool = SqlitePool::connect(url).await?;

        Ok(Self { pool })
    }

    pub async fn migrate(&self) -> Result<(), sqlx::Error> {
        sqlx::migrate!("./db/migrations").run(&self.pool).await?;

        Ok(())
    }

    pub async fn get_subscription_by_chat_id(
        &self,
        chat_id: i64,
    ) -> Result<Option<SubscriptionDto>, sqlx::Error> {
        sqlx::query_as!(SubscriptionDto, r#"SELECT id, chat_id as "chat_id!: i64", is_enabled, created_at, updated_at FROM subscription WHERE chat_id = ?"#, chat_id)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn create_subscription(
        &self,
        chat_id: i64,
        is_enabled: i32,
        timestamp: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("INSERT INTO subscription (chat_id, is_enabled, created_at, updated_at) VALUES (?, ?, ?, ?)")
        .bind(chat_id.to_string())
        .bind(is_enabled)
        .bind(timestamp)
        .bind(timestamp)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn set_subscription_enabled(
        &self,
        chat_id: i64,
        is_enabled: i32,
        timestamp: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE subscription SET is_enabled = ?, updated_at = ? WHERE chat_id = ?")
            .bind(is_enabled)
            .bind(timestamp)
            .bind(chat_id.to_string())
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn get_enabled_chat_ids(&self) -> Result<Vec<i64>, sqlx::Error> {
        let chat_ids = sqlx::query!(
            r#"SELECT chat_id as "chat_id!: i64" FROM subscription WHERE is_enabled = 1"#
        )
        .fetch_all(&self.pool)
        .await?;

        let res = chat_ids.into_iter().map(|r| r.chat_id).collect();

        Ok(res)
    }
}
