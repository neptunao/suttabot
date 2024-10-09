#[derive(sqlx::FromRow)]
pub struct SubscriptionDto {
    pub id: i64,
    pub chat_id: i64,
    pub is_enabled: i64,
    pub created_at: i64,
    pub updated_at: i64,
}
