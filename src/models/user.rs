use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub id: String,
    pub email: String,
    pub password: String,
    pub business_type: String,
    pub created_at: String,
    pub full_name: Option<String>,
    pub nickname: Option<String>,
    pub phone: Option<String>,
    pub country: Option<String>,
    pub gender: Option<String>,
    pub profile_picture: Option<String>,
    pub telegram_username: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthRequest {
    pub email: String,
    pub password: String,
    pub business_type: Option<String>,
    pub full_name: Option<String>,
    pub nickname: Option<String>,
    pub phone: Option<String>,
    pub country: Option<String>,
    pub gender: Option<String>,
    pub profile_picture: Option<String>,
    pub telegram_username: Option<String>,
}