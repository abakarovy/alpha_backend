use actix_web::{Error, HttpRequest, HttpResponse, web};
use serde_json::json;
use uuid::Uuid;
use sqlx::Row;

use crate::models::{CreateTelegramUserRequest, TelegramUserResponse};
use crate::state::AppState;
use crate::i18n::{self, Locale};

pub async fn create_or_get_telegram_user(
    req: HttpRequest,
    data: web::Json<CreateTelegramUserRequest>,
    state: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    let locale = i18n::detect_locale(&req);
    let telegram_req = data.into_inner();
    let pool = &state.pool;

    // Check if user already exists
    let existing = sqlx::query(
        "SELECT id, telegram_user_id, telegram_username, first_name, last_name, created_at, user_id
         FROM telegram_users
         WHERE telegram_user_id = ?
         LIMIT 1"
    )
    .bind(telegram_req.telegram_user_id)
    .fetch_optional(pool)
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    if let Some(row) = existing {
        // User exists, return existing user
        let response = TelegramUserResponse {
            id: row.get::<String, _>("id"),
            telegram_user_id: row.get::<i64, _>("telegram_user_id"),
            telegram_username: row.try_get::<Option<String>, _>("telegram_username").unwrap_or(None),
            first_name: row.try_get::<Option<String>, _>("first_name").unwrap_or(None),
            last_name: row.try_get::<Option<String>, _>("last_name").unwrap_or(None),
            created_at: row.get::<String, _>("created_at"),
            user_id: row.try_get::<Option<String>, _>("user_id").unwrap_or(None),
        };
        return Ok(HttpResponse::Ok().json(response));
    }

    // Create new user
    let id = Uuid::new_v4().to_string();
    let created_at = chrono::Utc::now().to_rfc3339();

    // Normalize empty strings to None
    let telegram_username_value = telegram_req.telegram_username.as_ref()
        .and_then(|s| if s.is_empty() { None } else { Some(s.as_str()) });
    let first_name_value = telegram_req.first_name.as_ref()
        .and_then(|s| if s.is_empty() { None } else { Some(s.as_str()) });
    let last_name_value = telegram_req.last_name.as_ref()
        .and_then(|s| if s.is_empty() { None } else { Some(s.as_str()) });

    let result = sqlx::query(
        "INSERT INTO telegram_users (id, telegram_user_id, telegram_username, first_name, last_name, created_at)
         VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(telegram_req.telegram_user_id)
    .bind(telegram_username_value)
    .bind(first_name_value)
    .bind(last_name_value)
    .bind(&created_at)
    .execute(pool)
    .await;

    match result {
        Ok(_) => {
            let response = TelegramUserResponse {
                id,
                telegram_user_id: telegram_req.telegram_user_id,
                telegram_username: telegram_username_value.map(|s| s.to_string()),
                first_name: first_name_value.map(|s| s.to_string()),
                last_name: last_name_value.map(|s| s.to_string()),
                created_at,
                user_id: None,
            };
            Ok(HttpResponse::Created().json(response))
        }
        Err(_) => {
            let error_msg = match locale {
                Locale::Ru => "Не удалось создать пользователя Telegram",
                Locale::En => "Failed to create Telegram user",
            };
            Ok(HttpResponse::InternalServerError().json(json!({
                "error": error_msg
            })))
        }
    }
}

pub async fn get_telegram_user_by_id(
    _req: HttpRequest,
    path: web::Path<i64>,
    state: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    let telegram_user_id = path.into_inner();
    let pool = &state.pool;

    let row = sqlx::query(
        "SELECT id, telegram_user_id, telegram_username, first_name, last_name, created_at, user_id
         FROM telegram_users
         WHERE telegram_user_id = ?
         LIMIT 1"
    )
    .bind(telegram_user_id)
    .fetch_optional(pool)
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    match row {
        Some(r) => {
            let response = TelegramUserResponse {
                id: r.get::<String, _>("id"),
                telegram_user_id: r.get::<i64, _>("telegram_user_id"),
                telegram_username: r.try_get::<Option<String>, _>("telegram_username").unwrap_or(None),
                first_name: r.try_get::<Option<String>, _>("first_name").unwrap_or(None),
                last_name: r.try_get::<Option<String>, _>("last_name").unwrap_or(None),
                created_at: r.get::<String, _>("created_at"),
                user_id: r.try_get::<Option<String>, _>("user_id").unwrap_or(None),
            };
            Ok(HttpResponse::Ok().json(response))
        }
        None => {
            Ok(HttpResponse::NotFound().json(json!({
                "error": "Telegram user not found"
            })))
        }
    }
}

pub async fn link_telegram_user_to_account(
    req: HttpRequest,
    path: web::Path<i64>,
    data: web::Json<serde_json::Value>,
    state: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    let locale = i18n::detect_locale(&req);
    let telegram_user_id = path.into_inner();
    let pool = &state.pool;

    let user_id = match data.get("user_id") {
        Some(serde_json::Value::String(uid)) => uid.clone(),
        _ => {
            let error_msg = match locale {
                Locale::Ru => "user_id обязателен",
                Locale::En => "user_id is required",
            };
            return Ok(HttpResponse::BadRequest().json(json!({
                "error": error_msg
            })));
        }
    };

    // Check if telegram user exists
    let telegram_user_exists = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(1) FROM telegram_users WHERE telegram_user_id = ?"
    )
    .bind(telegram_user_id)
    .fetch_one(pool)
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    if telegram_user_exists == 0 {
        let error_msg = match locale {
            Locale::Ru => "Пользователь Telegram не найден",
            Locale::En => "Telegram user not found",
        };
        return Ok(HttpResponse::NotFound().json(json!({
            "error": error_msg
        })));
    }

    // Check if main user exists
    let user_exists = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(1) FROM users WHERE id = ?"
    )
    .bind(&user_id)
    .fetch_one(pool)
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    if user_exists == 0 {
        let error_msg = match locale {
            Locale::Ru => "Пользователь не найден",
            Locale::En => "User not found",
        };
        return Ok(HttpResponse::NotFound().json(json!({
            "error": error_msg
        })));
    }

    // Link telegram user to main user account
    let result = sqlx::query(
        "UPDATE telegram_users SET user_id = ? WHERE telegram_user_id = ?"
    )
    .bind(&user_id)
    .bind(telegram_user_id)
    .execute(pool)
    .await;

    match result {
        Ok(_) => {
            Ok(HttpResponse::Ok().json(json!({
                "message": "Telegram user linked successfully"
            })))
        }
        Err(_) => {
            let error_msg = match locale {
                Locale::Ru => "Ошибка при связывании пользователей",
                Locale::En => "Failed to link users",
            };
            Ok(HttpResponse::InternalServerError().json(json!({
                "error": error_msg
            })))
        }
    }
}

