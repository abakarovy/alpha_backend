mod models;
mod handlers;
mod services;
mod state;
mod db;

use actix_web::{web, App, HttpServer};
use state::AppState;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .unwrap_or(3000);
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://app.db".to_string());
    
    println!("🚀 Starting Business Assistant Server on port {}", port);
    
    let pool = db::init_pool(&database_url)
        .await
        .expect("Failed to initialize SQLite pool");
    let app_state = web::Data::new(AppState::new(pool));
    
    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .route("/health", web::get().to(handlers::health_check))
            .route("/api/chat/message", web::post().to(handlers::chat::send_message))
            .route("/api/chat/quick-advice", web::get().to(handlers::chat::get_quick_advice))
            .route("/api/chat/history/{user_id}", web::get().to(handlers::chat::get_conversation_history))
            .route("/api/auth/register", web::post().to(handlers::auth::register))
            .route("/api/auth/login", web::post().to(handlers::auth::login))
            .route("/api/auth/check-user", web::get().to(handlers::auth::email_exists))
            .route("/api/auth/check-token", web::get().to(handlers::auth::check_token))
            .route("/api/business/categories", web::get().to(handlers::business::get_categories))
            .route("/api/business/resources/{category}", web::get().to(handlers::business::get_resources))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}