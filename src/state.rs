use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::models::{Message};
use sqlx::SqlitePool;

pub type UserId = String;
pub type ConversationHistory = Arc<Mutex<HashMap<UserId, Vec<Message>>>>;

#[derive(Clone)]
pub struct AppState {
    pub conversations: ConversationHistory,
    pub pool: SqlitePool,
}

impl AppState {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            conversations: Arc::new(Mutex::new(HashMap::new())),
            pool,
        }
    }
}