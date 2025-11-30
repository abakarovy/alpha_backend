use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
    pub timestamp: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatRequest {
    pub message: String,
    pub category: Option<String>,
    pub user_id: String,
    pub business_type: Option<String>,
    pub conversation_id: Option<String>,
    pub output_format: Option<String>, // e.g. "xlsx" | "csv"
    pub table: Option<TableSpec>,
    pub language: Option<String>, // e.g. "en" | "ru"
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatResponse {
    pub response: String,
    pub message_id: String,
    pub timestamp: String,
    pub conversation_id: String,
    pub files: Option<Vec<FileAttachment>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QuickAdviceRequest {
    pub category: String,
    pub business_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConversationSummary {
    pub id: String,
    pub user_id: String,
    pub title: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MessageRecord {
    pub id: String,
    pub role: String,
    pub content: String,
    pub timestamp: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TableSpec {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileAttachment {
    pub id: Option<String>,
    pub filename: String,
    pub mime: String,
    pub size: usize,
    pub content_base64: Option<String>,
    pub download_url: Option<String>,
}