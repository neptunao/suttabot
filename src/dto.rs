#[derive(sqlx::FromRow)]
pub struct SubscriptionDto {
    pub id: i32,
    pub chat_id: i32,
    pub is_enabled: i32,
    pub created_at: i64,
    pub updated_at: i64,
}
