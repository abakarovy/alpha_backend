pub mod auth;
pub mod chat;
pub mod business;
pub mod analytics;
pub mod legal;
pub mod files;
pub mod support;

use actix_web::HttpResponse;
use serde_json::json;

pub async fn main() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(include_str!("../../assets/index.html"))
}

pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().json(json!({
        "status": "OK",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "version": "1.0.0"
    }))
}