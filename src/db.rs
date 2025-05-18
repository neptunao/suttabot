use sqlx::{Sqlite, SqlitePool, Transaction};

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
        sqlx::query_as!(SubscriptionDto, r#"SELECT id "id!: i64", chat_id as "chat_id!: i64", is_enabled, created_at, updated_at FROM subscription WHERE chat_id = ?"#, chat_id)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn create_subscription(
        &self,
        chat_id: i64,
        is_enabled: i32,
        timestamp: i64,
    ) -> Result<(), sqlx::Error> {
        let mut transaction = self.pool.begin().await?;

        sqlx::query(
            "INSERT INTO subscription (chat_id, is_enabled, created_at, updated_at) VALUES (?, ?, ?, ?)",
        )
        .bind(chat_id.to_string())
        .bind(is_enabled)
        .bind(timestamp)
        .bind(timestamp)
        .execute(&mut *transaction)
        .await?;

        let subscription_row: (i64,) = sqlx::query_as("SELECT last_insert_rowid()")
            .fetch_one(&mut *transaction)
            .await?;
        let subscription_id = subscription_row.0;

        let default_utc_send_minute = 300;
        sqlx::query("INSERT INTO sendout_times (subscription_id, sendout_time) VALUES (?, ?)")
            .bind(subscription_id)
            .bind(default_utc_send_minute)
            .execute(&mut *transaction)
            .await?;

        transaction.commit().await?;
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

    pub async fn set_sendout_times(
        &self,
        subscription_id: i64,
        times_utc_minutes: &[i32],
    ) -> Result<(), sqlx::Error> {
        let mut transaction: sqlx::Transaction<'_, Sqlite> = self.pool.begin().await?;

        sqlx::query("DELETE FROM sendout_times WHERE subscription_id = ?")
            .bind(subscription_id)
            .execute(&mut *transaction)
            .await?;

        for time_utc_minute in times_utc_minutes {
            sqlx::query("INSERT INTO sendout_times (subscription_id, sendout_time) VALUES (?, ?)")
                .bind(subscription_id)
                .bind(time_utc_minute)
                .execute(&mut *transaction)
                .await?;
        }

        transaction.commit().await?;

        Ok(())
    }

    pub async fn get_all_active_schedules(&self) -> Result<Vec<(i64, i32)>, sqlx::Error> {
        let results = sqlx::query_as::<_, (i64, i32)>(
            r#"
            SELECT s.chat_id, st.sendout_time
            FROM subscription s
            JOIN sendout_times st ON s.id = st.subscription_id
            WHERE s.is_enabled = 1
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(results)
    }
}
