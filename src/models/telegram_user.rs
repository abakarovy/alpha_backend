use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TelegramUser {
    pub id: String,
    pub telegram_user_id: i64,
    pub telegram_username: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub created_at: String,
    pub user_id: Option<String>, // Link to main users table if user registered
}

#[derive(Deserialize)]
pub struct CreateTelegramUserRequest {
    pub telegram_user_id: i64,
    pub telegram_username: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

#[derive(Serialize)]
pub struct TelegramUserResponse {
    pub id: String,
    pub telegram_user_id: i64,
    pub telegram_username: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub created_at: String,
    pub user_id: Option<String>,
}

