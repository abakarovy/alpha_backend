mod models;
mod handlers;
mod services;
mod state;
mod db;
mod i18n;

use actix_web::{web, App, HttpServer};
use actix_web::middleware::NormalizePath;
use actix_cors::Cors;
use state::AppState;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .unwrap_or(8080);
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://app.db".to_string());
    
    println!("
-------------------@@@@@@@@@@@@@@@@+------------------------------------------------------------------@@@@@-----
------------------%@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@---------------------------------------------@@@@@@@@@@@-----
------------------@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@------------------------------@@@@@@@@@@@@@@@@=-----
-----------------@@@@@@@@@@@@@@@@@@@@@@@@@@@@--@@@--@@@@@@@-------------------------@@@@@@@@@@@@@-@@@@@@@@------
----------------@@@---+@@@@@------@@--@@@@@@----@@--@@@@@@--------------------@@@@@@@@@@@@@-------@@@@@@@@------
---------------@@@@----@@@@@---@--@@--@@@@-------@--@@@@@---------------@@@@@@@@@@@@@-@@----------@@@@@@@-------
--------------@@@@-----@@@@@--@@--@@-----@--@--@-@--@@@@@---------@@@@@@@@@@@@@-------@@----@@@@@@@@@@@@@-------
-------------+@@@@-----@@@@@--@@--@@--@--@--@--------@@@----%@@@@@@@@@@@@---@@------@@@@----@@@@@@@@@@@@@-------
-------------@@@@---@---@@@---@@--@*--@--@------@--@-@@@@@@@@@@@@@--@---@---@@----@@@@@@----@@---@@@@@@@--------
------------@@@@@---@---@@@---@%--@=--@--@@@@-=@@@@@@@@@@@@@-@=-%---@---@---@@----@@@-@@---------@@@@@@@--------
-----------@@@@@---=@---@@@---@#--@-----@@@@@@@@@@@@@@----@--@--@---@---@---@@--------@@------=@@@@@@@@#--------
----------@@@@@@---------@----@=--@--@@@@@@@@@@@@-@--@-@--@--@--@---@---@---@@------@@@@----@@@@@@@@@@@---------
---------@@@@@@@---------@---@@@@@@@@@@@@@@@@---@-@--@-@--@@----@---@---@---@@----@@@@@@----@@@@@@@@@@@---------
---------@@@@@@---*@@@--@@@@@@@@@@@@@@@@--@@@-@@@---@@-@--@@---@@---@---@---@@----@@@@@@----@@%---@@@@----------
--------@@@@@@@---@@@@@@@@@@@@@@@@-------@@@@---@@--@@-@--@@=--@@---@---@---@@--------%@----------@@@@----------
-------@@@@@@@@@@@@@@@@@@@@#-------------@@@@-@-@@--@@-@--@@#--@@------------@--------%@------=@@@@@@@----------
------@@@@@@@@@@@@@@@-------------------@@@@@-=-@--*@-----=@--@@@------%@@---@@@@@@@@@@@@@@@@@@@@@@@@-----------
-----@@@@@@@@@@-------------------------@@@@@--@@-+@@--@@-=@@@@@@@@@@@@@@@---@@@@@@@@@@@@@@@@@@@@@@@@-----------
----#@@@@------------------------------#%@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@-----------
");
    
    let pool = db::init_pool(&database_url)
        .await
        .expect("Failed to initialize SQLite pool");
    let app_state = web::Data::new(AppState::new(pool));
    
    HttpServer::new(move || {
        App::new()
            .wrap(NormalizePath::trim())
            .wrap(Cors::permissive())
            .app_data(app_state.clone())
            .route("/", web::get().to(handlers::main))
            .route("/health", web::get().to(handlers::health_check))
            
            .route("/api/chat/message", web::post().to(handlers::chat::send_message))
            .route("/api/chat/conversations", web::post().to(handlers::chat::create_conversation))
            .route("/api/chat/conversations/{user_id}", web::get().to(handlers::chat::list_conversations))
            .route("/api/chat/conversations/{conversation_id}", web::delete().to(handlers::chat::delete_conversation))
            .route("/api/chat/conversations/{conversation_id}/title", web::put().to(handlers::chat::update_conversation_title))
            .route("/api/chat/conversations/{conversation_id}/context", web::put().to(handlers::chat::update_conversation_context))
            .route("/api/chat/history/{conversation_id}", web::get().to(handlers::chat::get_conversation_history))
            
            .route("/api/auth/register", web::post().to(handlers::auth::register))
            .route("/api/auth/login", web::post().to(handlers::auth::login))
            .route("/api/auth/check-user", web::get().to(handlers::auth::email_exists))
            .route("/api/auth/check-token", web::get().to(handlers::auth::check_token))
            .route("/api/auth/profile/{user_id}", web::get().to(handlers::auth::get_profile))
            .route("/api/auth/profile", web::put().to(handlers::auth::update_profile))

            .route("/api/analytics/weekly-trends", web::get().to(handlers::analytics::get_weekly_trends))
            .route("/api/analytics/weekly-trends", web::post().to(handlers::analytics::upsert_weekly_trends))
            .route("/api/analytics/ai-analytics", web::get().to(handlers::analytics::get_ai_analytics))
            .route("/api/analytics/ai-analytics", web::post().to(handlers::analytics::upsert_ai_analytics))
            .route("/api/analytics/niches-month", web::get().to(handlers::analytics::get_niches_month))
            .route("/api/analytics/niches-month", web::post().to(handlers::analytics::upsert_niches_month))

            .route("/api/analytics/top-trend", web::get().to(handlers::analytics::get_top_trend))
            .route("/api/analytics/top-trend", web::post().to(handlers::analytics::upsert_top_trend))
            .route("/api/analytics/popularity", web::get().to(handlers::analytics::get_popularity_trends))
            .route("/api/analytics/popularity", web::post().to(handlers::analytics::upsert_popularity_trend))
            
            .route("/privacy-policy", web::get().to(handlers::legal::privacy_policy))
            .route("/api/files/{id}", web::get().to(handlers::files::download_file))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}