use sqlx::{Sqlite, SqlitePool};

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
        sqlx::query_as!(SubscriptionDto, r#"SELECT id "id!: i64", chat_id as "chat_id!: i64", is_enabled, created_at, updated_at, last_donation_reminder, donation_reminder_count, sendout_count FROM subscription WHERE chat_id = ?"#, chat_id)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn create_subscription(
        &self,
        chat_id: i64,
        is_enabled: i32,
        timestamp: i64,
        initial_sendout: i64,
    ) -> Result<(), sqlx::Error> {
        // Set last_donation_reminder to 14 days ago (UTC) for tracking
        let last_donation_reminder = timestamp - 1_209_600; // 14 days in seconds

        // initial_sendout passed by caller (donation_period - 1)
        sqlx::query(
            "INSERT INTO subscription (chat_id, is_enabled, created_at, updated_at, last_donation_reminder, donation_reminder_count, sendout_count) VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(chat_id.to_string())
        .bind(is_enabled)
        .bind(timestamp)
        .bind(timestamp)
        .bind(last_donation_reminder)
        .bind(0)
        .bind(initial_sendout)
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

    pub async fn set_sendout_times(
        &self,
        subscription_id: i64,
        times: &[i32],
    ) -> Result<(), sqlx::Error> {
        let mut transaction: sqlx::Transaction<'_, Sqlite> = self.pool.begin().await?;

        // first we need to delete all existing times
        sqlx::query("DELETE FROM sendout_times WHERE subscription_id = ?")
            .bind(subscription_id)
            .execute(&mut *transaction)
            .await?;

        // then we need to insert new times
        for time in times {
            sqlx::query("INSERT INTO sendout_times (subscription_id, sendout_time) VALUES (?, ?)")
                .bind(subscription_id)
                .bind(time)
                .execute(&mut *transaction)
                .await?;
        }

        transaction.commit().await?;

        Ok(())
    }

    pub async fn update_donation_reminder(
        &self,
        chat_id: i64,
        timestamp: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE subscription SET last_donation_reminder = ?, donation_reminder_count = donation_reminder_count + 1, updated_at = ? WHERE chat_id = ?"
        )
        .bind(timestamp)
        .bind(timestamp)
        .bind(chat_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_subscriptions_for_donation_reminder(
        &self,
    ) -> Result<Vec<SubscriptionDto>, sqlx::Error> {
        sqlx::query_as!(
            SubscriptionDto,
            r#"SELECT id as "id!: i64", chat_id as "chat_id!: i64", is_enabled, created_at, updated_at, last_donation_reminder, donation_reminder_count, sendout_count
            FROM subscription
            WHERE is_enabled = 1 AND (strftime('%s', 'now') - last_donation_reminder) >= 1296000"#
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn increment_sendout_count(
        &self,
        chat_id: i64,
    ) -> Result<i64, sqlx::Error> {
        // Increment and return the new value
        sqlx::query_scalar::<_, i64>(
            "UPDATE subscription SET sendout_count = sendout_count + 1 WHERE chat_id = ? RETURNING sendout_count"
        )
        .bind(chat_id.to_string())
        .fetch_one(&self.pool)
        .await
    }

    pub async fn get_subscriptions_for_donation_by_count(
        &self,
        period: i64,
    ) -> Result<Vec<SubscriptionDto>, sqlx::Error> {
        sqlx::query_as!(
            SubscriptionDto,
            r#"SELECT id as "id!: i64", chat_id as "chat_id!: i64", is_enabled, created_at, updated_at, last_donation_reminder, donation_reminder_count, sendout_count
            FROM subscription
            WHERE is_enabled = 1 AND sendout_count % ? = 0"#,
            period
        )
        .fetch_all(&self.pool)
        .await
    }
}
