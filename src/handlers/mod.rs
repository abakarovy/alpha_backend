pub mod auth;
pub mod chat;
pub mod business;
pub mod analytics;
pub mod legal;
pub mod files;

use actix_web::HttpResponse;
use serde_json::json;

pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().json(json!({
        "status": "OK",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "version": "1.0.0"
    }))
}