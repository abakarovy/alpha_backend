use actix_web::{web, HttpRequest, HttpResponse};
use actix_multipart::Multipart;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;
use futures_util::TryStreamExt;
use std::collections::HashMap;

use crate::i18n::{self, Locale};
use crate::state::AppState;
use crate::services::{telegram, fcm};

#[derive(Deserialize, Serialize)]
pub struct SupportMessage {
    pub user_id: String,
    pub message: String,
    pub user_name: Option<String>,
    pub photo_url: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct SupportResponse {
    pub success: bool,
    pub message_id: Option<i64>,
    pub photo_url: Option<String>,
    pub photo_count: Option<usize>,
}

#[derive(Deserialize, Serialize)]
pub struct DeviceRegistration {
    pub user_id: String,
    pub fcm_token: String,
    pub platform: Option<String>,
    pub device_id: Option<String>,
}

#[derive(Serialize)]
pub struct MessageHistoryResponse {
    pub success: bool,
    pub messages: Vec<MessageHistoryItem>,
    pub greeting_sent: bool,
}

#[derive(Serialize)]
pub struct MessageHistoryItem {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub photo_url: Option<String>,
    pub direction: String,
    pub created_at: String,
}

#[derive(Deserialize)]
struct TelegramWebhookUpdate {
    message: Option<TelegramMessage>,
}

#[derive(Deserialize)]
struct TelegramMessage {
    message_id: i64,
    chat: TelegramChat,
    text: Option<String>,
    reply_to_message: Option<TelegramReplyToMessage>,
}

#[derive(Deserialize)]
struct TelegramChat {
    id: i64,
}

#[derive(Deserialize)]
struct TelegramReplyToMessage {
    message_id: i64,
}

// Send support message with optional photo
pub async fn send_support_message(
    req: HttpRequest,
    data: web::Json<SupportMessage>,
    state: web::Data<AppState>,
) -> HttpResponse {
    let locale = i18n::detect_locale(&req);
    let pool = &state.pool;
    let msg_data = data.into_inner();
    
    let bot = match telegram::TelegramBot::new() {
        Ok(bot) => bot,
        Err(e) => {
            eprintln!("Failed to initialize Telegram bot: {}", e);
            let error_msg = match locale {
                Locale::Ru => "Ошибка инициализации бота",
                Locale::En => "Failed to initialize bot",
            };
            return HttpResponse::InternalServerError().json(json!({
                "error": error_msg
            }));
        }
    };

    let message_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let mut telegram_msg_id: Option<i64> = None;
    let mut photo_url: Option<String> = None;
    let mut photo_count = 0;

    // Send message to Telegram
    if let Some(ref photo) = msg_data.photo_url {
        photo_url = Some(photo.clone());
        photo_count = 1;
        match bot.send_photo(photo, Some(&msg_data.message), msg_data.user_name.as_deref()).await {
            Ok(tg_id) => {
                telegram_msg_id = Some(tg_id);
            }
            Err(e) => {
                eprintln!("Failed to send photo to Telegram: {}", e);
            }
        }
    } else {
        match bot.send_message(&msg_data.message, msg_data.user_name.as_deref()).await {
            Ok(tg_id) => {
                telegram_msg_id = Some(tg_id);
            }
            Err(e) => {
                eprintln!("Failed to send message to Telegram: {}", e);
                let error_msg = match locale {
                    Locale::Ru => "Ошибка отправки сообщения в Telegram",
                    Locale::En => "Failed to send message to Telegram",
                };
                return HttpResponse::InternalServerError().json(json!({
                    "error": error_msg
                }));
            }
        }
    }

    // Save message to database
    let _ = sqlx::query(
        "INSERT INTO support_messages (id, user_id, message, photo_url, direction, telegram_message_id, created_at) VALUES (?, ?, ?, ?, 'user', ?, ?)"
    )
    .bind(&message_id)
    .bind(&msg_data.user_id)
    .bind(&msg_data.message)
    .bind(&photo_url)
    .bind(&telegram_msg_id)
    .bind(&now)
    .execute(pool)
    .await;

    // Save message mapping if we have telegram message ID
    if let Some(tg_id) = telegram_msg_id {
        let mapping_id = Uuid::new_v4().to_string();
        let _ = sqlx::query(
            "INSERT INTO message_mapping (id, telegram_message_id, user_id, support_message_id, created_at) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&mapping_id)
        .bind(tg_id)
        .bind(&msg_data.user_id)
        .bind(&message_id)
        .bind(&now)
        .execute(pool)
        .await;
    }

    HttpResponse::Ok().json(SupportResponse {
        success: true,
        message_id: telegram_msg_id,
        photo_url,
        photo_count: if photo_count > 0 { Some(photo_count) } else { None },
    })
}

// Handle multipart form data with photos
pub async fn send_support_message_multipart(
    req: HttpRequest,
    mut payload: Multipart,
    state: web::Data<AppState>,
) -> HttpResponse {
    let locale = i18n::detect_locale(&req);
    let pool = &state.pool;
    
    let bot = match telegram::TelegramBot::new() {
        Ok(bot) => bot,
        Err(e) => {
            eprintln!("Failed to initialize Telegram bot: {}", e);
            let error_msg = match locale {
                Locale::Ru => "Ошибка инициализации бота",
                Locale::En => "Failed to initialize bot",
            };
            return HttpResponse::InternalServerError().json(json!({
                "error": error_msg
            }));
        }
    };

    let mut user_id: Option<String> = None;
    let mut message: Option<String> = None;
    let mut user_name: Option<String> = None;
    let mut photos: Vec<(Vec<u8>, String)> = Vec::new();

    // Parse multipart form
    while let Ok(Some(mut field)) = payload.try_next().await {
        let name = field.name().to_string();
        
        if name == "user_id" {
            let mut data = Vec::new();
            while let Ok(Some(chunk)) = field.try_next().await {
                data.extend_from_slice(&chunk);
            }
            user_id = String::from_utf8(data).ok();
        } else if name == "message" {
            let mut data = Vec::new();
            while let Ok(Some(chunk)) = field.try_next().await {
                data.extend_from_slice(&chunk);
            }
            message = String::from_utf8(data).ok();
        } else if name == "user_name" {
            let mut data = Vec::new();
            while let Ok(Some(chunk)) = field.try_next().await {
                data.extend_from_slice(&chunk);
            }
            user_name = String::from_utf8(data).ok();
        } else if name == "photo" {
            let filename = field.content_disposition().get_filename()
                .unwrap_or("photo.jpg")
                .to_string();
            let mut data = Vec::new();
            while let Ok(Some(chunk)) = field.try_next().await {
                data.extend_from_slice(&chunk);
            }
            if !data.is_empty() {
                photos.push((data, filename));
            }
        }
    }

    let user_id = match user_id {
        Some(id) => id,
        None => {
            let error_msg = match locale {
                Locale::Ru => "Требуется user_id",
                Locale::En => "user_id is required",
            };
            return HttpResponse::BadRequest().json(json!({
                "error": error_msg
            }));
        }
    };

    let message_text = message.unwrap_or_else(|| String::new());
    let message_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // Send photos and/or message to Telegram
    let mut telegram_msg_id: Option<i64> = None;
    let mut photo_url: Option<String> = None;
    
    if !photos.is_empty() {
        // Send first photo with caption
        if let Ok(tg_id) = bot.send_photo_multipart(
            photos[0].0.clone(),
            &photos[0].1,
            if !message_text.is_empty() { Some(&message_text) } else { None },
            user_name.as_deref(),
        ).await {
            telegram_msg_id = Some(tg_id);
            photo_url = Some(format!("/uploads/{}", photos[0].1));
        }
        
        // Send remaining photos without caption
        for (photo_data, filename) in photos.iter().skip(1) {
            let _ = bot.send_photo_multipart(
                photo_data.clone(),
                filename,
                None,
                None,
            ).await;
        }
    } else if !message_text.is_empty() {
        match bot.send_message(&message_text, user_name.as_deref()).await {
            Ok(tg_id) => {
                telegram_msg_id = Some(tg_id);
            }
            Err(e) => {
                eprintln!("Failed to send message: {}", e);
            }
        }
    }

    // Save to database
    let _ = sqlx::query(
        "INSERT INTO support_messages (id, user_id, message, photo_url, direction, telegram_message_id, created_at) VALUES (?, ?, ?, ?, 'user', ?, ?)"
    )
    .bind(&message_id)
    .bind(&user_id)
    .bind(&message_text)
    .bind(&photo_url)
    .bind(&telegram_msg_id)
    .bind(&now)
    .execute(pool)
    .await;

    if let Some(tg_id) = telegram_msg_id {
        let mapping_id = Uuid::new_v4().to_string();
        let _ = sqlx::query(
            "INSERT INTO message_mapping (id, telegram_message_id, user_id, support_message_id, created_at) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&mapping_id)
        .bind(tg_id)
        .bind(&user_id)
        .bind(&message_id)
        .bind(&now)
        .execute(pool)
        .await;
    }

    HttpResponse::Ok().json(SupportResponse {
        success: true,
        message_id: telegram_msg_id,
        photo_url,
        photo_count: if photos.is_empty() { None } else { Some(photos.len()) },
    })
}

// Register FCM device token
pub async fn register_device(
    req: HttpRequest,
    data: web::Json<DeviceRegistration>,
    state: web::Data<AppState>,
) -> HttpResponse {
    let locale = i18n::detect_locale(&req);
    let pool = &state.pool;
    let device_data = data.into_inner();

    let device_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // Check if device already exists
    let existing = sqlx::query_scalar::<_, Option<String>>(
        "SELECT id FROM device_tokens WHERE user_id = ? AND fcm_token = ?"
    )
    .bind(&device_data.user_id)
    .bind(&device_data.fcm_token)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    let result = if existing.is_some() {
        // Update existing device
        sqlx::query(
            "UPDATE device_tokens SET platform = ?, device_id = ? WHERE user_id = ? AND fcm_token = ?"
        )
        .bind(&device_data.platform)
        .bind(&device_data.device_id)
        .bind(&device_data.user_id)
        .bind(&device_data.fcm_token)
        .execute(pool)
        .await
    } else {
        // Insert new device
        sqlx::query(
            "INSERT INTO device_tokens (id, user_id, fcm_token, platform, device_id, created_at) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(&device_id)
        .bind(&device_data.user_id)
        .bind(&device_data.fcm_token)
        .bind(&device_data.platform)
        .bind(&device_data.device_id)
        .bind(&now)
        .execute(pool)
        .await
    };

    match result {
        Ok(_) => {
            // Get device count
            let count_result = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM device_tokens WHERE user_id = ?"
            )
            .bind(&device_data.user_id)
            .fetch_one(pool)
            .await;

            let device_count = count_result.unwrap_or(0) as usize;

            HttpResponse::Ok().json(json!({
                "success": true,
                "message": match locale {
                    Locale::Ru => "Устройство зарегистрировано успешно",
                    Locale::En => "Device registered successfully",
                },
                "device_count": device_count
            }))
        }
        Err(e) => {
            eprintln!("Error registering device: {}", e);
            let error_msg = match locale {
                Locale::Ru => "Ошибка регистрации устройства",
                Locale::En => "Failed to register device",
            };
            HttpResponse::InternalServerError().json(json!({
                "error": error_msg
            }))
        }
    }
}

// Get message history
pub async fn get_support_history(
    req: HttpRequest,
    path: web::Path<String>,
    query: web::Query<HashMap<String, String>>,
    state: web::Data<AppState>,
) -> HttpResponse {
    let locale = i18n::detect_locale(&req);
    let user_id = path.into_inner();
    let pool = &state.pool;
    
    let limit: i64 = query.get("limit")
        .and_then(|l| l.parse().ok())
        .unwrap_or(50);

    // Check if greeting was sent today
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let greeting_sent = sqlx::query_scalar::<_, Option<String>>(
        "SELECT date FROM greetings_sent WHERE user_id = ? AND date = ?"
    )
    .bind(&user_id)
    .bind(&today)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .is_some();

    // Send greeting if not sent today
    if !greeting_sent {
        let user_name = query.get("user_name");
        let greeting = if let Some(name) = user_name {
            format!("Здравствуйте, {}!", name)
        } else {
            "Здравствуйте!".to_string()
        };

        let greeting_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        
        let _ = sqlx::query(
            "INSERT INTO support_messages (id, user_id, message, direction, created_at) VALUES (?, ?, ?, 'support', ?)"
        )
        .bind(&greeting_id)
        .bind(&user_id)
        .bind(&greeting)
        .bind(&now)
        .execute(pool)
        .await;

        let _ = sqlx::query(
            "INSERT OR IGNORE INTO greetings_sent (user_id, date, created_at) VALUES (?, ?, ?)"
        )
        .bind(&user_id)
        .bind(&today)
        .bind(&now)
        .execute(pool)
        .await;
    }

    // Get messages
    let messages_result = sqlx::query(
        "SELECT message, photo_url, direction, created_at FROM support_messages 
         WHERE user_id = ? 
         ORDER BY created_at ASC 
         LIMIT ?"
    )
    .bind(&user_id)
    .bind(limit)
    .fetch_all(pool)
    .await;

    match messages_result {
        Ok(rows) => {
            let messages: Vec<MessageHistoryItem> = rows.into_iter().map(|r| {
                MessageHistoryItem {
                    message: r.get("message"),
                    photo_url: r.try_get::<Option<String>, _>("photo_url").ok().flatten(),
                    direction: r.get("direction"),
                    created_at: r.get("created_at"),
                }
            }).collect();

            let has_messages = !messages.is_empty();

            HttpResponse::Ok().json(MessageHistoryResponse {
                success: true,
                messages,
                greeting_sent: greeting_sent || has_messages,
            })
        }
        Err(e) => {
            eprintln!("Error getting history: {}", e);
            let error_msg = match locale {
                Locale::Ru => "Ошибка получения истории",
                Locale::En => "Failed to get history",
            };
            HttpResponse::InternalServerError().json(json!({
                "error": error_msg
            }))
        }
    }
}

// Check device registration
pub async fn check_device(
    req: HttpRequest,
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> HttpResponse {
    let _locale = i18n::detect_locale(&req);
    let user_id = path.into_inner();
    let pool = &state.pool;

    let count_result = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM device_tokens WHERE user_id = ?"
    )
    .bind(&user_id)
    .fetch_one(pool)
    .await;

    let device_count = count_result.unwrap_or(0) as usize;

    let has_tokens = device_count > 0;

    HttpResponse::Ok().json(json!({
        "user_id": user_id,
        "registered": has_tokens,
        "device_count": device_count,
        "has_tokens": has_tokens
    }))
}

// Telegram webhook to receive replies from support team
pub async fn telegram_webhook(
    data: web::Json<serde_json::Value>,
    state: web::Data<AppState>,
) -> HttpResponse {
    let pool = &state.pool;
    
    // Extract message from webhook update
    if let Some(update) = data.get("message") {
        if let (Some(msg_id), Some(_chat_id), Some(text)) = (
            update.get("message_id").and_then(|v| v.as_i64()),
            update.get("chat").and_then(|c| c.get("id")).and_then(|v| v.as_i64()),
            update.get("text").and_then(|v| v.as_str()),
        ) {
            // Check if this is a reply to a user message
            if let Some(reply_to) = update.get("reply_to_message") {
                if let Some(original_msg_id) = reply_to.get("message_id").and_then(|v| v.as_i64()) {
                    // Find the user_id from message_mapping
                    if let Ok(Some(user_id_row)) = sqlx::query(
                        "SELECT user_id FROM message_mapping WHERE telegram_message_id = ?"
                    )
                    .bind(original_msg_id)
                    .fetch_optional(pool)
                    .await
                    {
                        let user_id: String = user_id_row.get("user_id");
                        // Save support reply to database
                        let message_id = Uuid::new_v4().to_string();
                        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
                        
                        let _ = sqlx::query(
                            "INSERT INTO support_messages (id, user_id, message, direction, telegram_message_id, created_at) VALUES (?, ?, ?, 'support', ?, ?)"
                        )
                        .bind(&message_id)
                        .bind(&user_id)
                        .bind(text)
                        .bind(msg_id)
                        .bind(&now)
                        .execute(pool)
                        .await;

                        // Send push notification
                        if let Ok(tokens_result) = sqlx::query(
                            "SELECT fcm_token FROM device_tokens WHERE user_id = ?"
                        )
                        .bind(&user_id)
                        .fetch_all(pool)
                        .await
                        {
                            let tokens: Vec<String> = tokens_result
                                .into_iter()
                                .filter_map(|row| row.try_get::<String, _>("fcm_token").ok())
                                .collect();

                            if !tokens.is_empty() {
                                if let Ok(fcm_service) = fcm::FcmService::new() {
                                    let _ = fcm_service.send_notification(
                                        tokens,
                                        "Новое сообщение от поддержки",
                                        text,
                                        None,
                                    ).await;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    HttpResponse::Ok().json(json!({"ok": true}))
}
