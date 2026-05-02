use sqlx::{Sqlite, SqlitePool};

use crate::dto::{NewsBroadcastDto, SubscriptionDto};

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
            "INSERT INTO subscription (chat_id, is_enabled, created_at, updated_at, last_donation_reminder, donation_reminder_count, sendout_count, news_onboarded) VALUES (?, ?, ?, ?, ?, ?, ?, 0)"
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

    // Returns (announcements_enabled, news_onboarded) for a given chat_id.
    pub async fn get_subscription_news_status(
        &self,
        chat_id: i64,
    ) -> Result<Option<(i64, i64)>, sqlx::Error> {
        let row: Option<(i64, i64)> = sqlx::query_as(
            "SELECT announcements_enabled, news_onboarded FROM subscription WHERE chat_id = ?"
        )
        .bind(chat_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn set_news_onboarded(&self, chat_id: i64) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE subscription SET news_onboarded = 1 WHERE chat_id = ?")
            .bind(chat_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_announcement_recipients(&self) -> Result<Vec<i64>, sqlx::Error> {
        let rows: Vec<(i64,)> = sqlx::query_as(
            "SELECT chat_id FROM subscription WHERE is_enabled = 1 AND announcements_enabled = 1"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    pub async fn set_announcements_enabled(
        &self,
        chat_id: i64,
        enabled: bool,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE subscription SET announcements_enabled = ? WHERE chat_id = ?"
        )
        .bind(if enabled { 1i64 } else { 0i64 })
        .bind(chat_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn record_news_broadcast(
        &self,
        slug: &str,
        recipient_count: i64,
        triggered_by: i64,
        version: &str,
    ) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT OR REPLACE INTO news_broadcast (slug, broadcast_at, recipient_count, triggered_by, version) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(slug)
        .bind(now)
        .bind(recipient_count)
        .bind(triggered_by)
        .bind(version)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_news_broadcast(
        &self,
        slug: &str,
    ) -> Result<Option<NewsBroadcastDto>, sqlx::Error> {
        sqlx::query_as::<_, NewsBroadcastDto>(
            "SELECT slug, broadcast_at, recipient_count, triggered_by, version FROM news_broadcast WHERE slug = ?"
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn get_news_broadcast_latest(&self) -> Result<Option<NewsBroadcastDto>, sqlx::Error> {
        sqlx::query_as::<_, NewsBroadcastDto>(
            "SELECT slug, broadcast_at, recipient_count, triggered_by, version FROM news_broadcast ORDER BY broadcast_at DESC LIMIT 1"
        )
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn get_news_broadcasts_all(&self) -> Result<Vec<NewsBroadcastDto>, sqlx::Error> {
        sqlx::query_as::<_, NewsBroadcastDto>(
            "SELECT slug, broadcast_at, recipient_count, triggered_by, version FROM news_broadcast ORDER BY broadcast_at DESC"
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get_news_broadcasts_by_version(
        &self,
        version: &str,
    ) -> Result<Vec<NewsBroadcastDto>, sqlx::Error> {
        let normalized = version.trim_start_matches('v');
        sqlx::query_as::<_, NewsBroadcastDto>(
            "SELECT slug, broadcast_at, recipient_count, triggered_by, version FROM news_broadcast WHERE version = ? OR version = ? ORDER BY broadcast_at DESC"
        )
        .bind(normalized)
        .bind(format!("v{}", normalized))
        .fetch_all(&self.pool)
        .await
    }
}
