use actix_web::{web, HttpResponse};
use serde_json::json;
use uuid::Uuid;

use crate::models::{ChatRequest, ChatResponse, QuickAdviceRequest, MessageRecord, ConversationSummary};
use crate::state::AppState;
use crate::services::openai;
use sqlx::Row;

pub async fn send_message(
    data: web::Json<ChatRequest>,
    state: web::Data<AppState>,
) -> HttpResponse {
    let chat_req = data.into_inner();
    
    if chat_req.message.is_empty() || chat_req.user_id.is_empty() {
        return HttpResponse::BadRequest().json(json!({
            "error": "Message and user_id are required"
        }));
    }

    let ai_response = match openai::generate_response(
        &chat_req.message,
        chat_req.category.as_deref().unwrap_or("general"),
        chat_req.business_type.as_deref().unwrap_or("общий бизнес"),
        &state,
        &chat_req.user_id
    ).await {
        Ok(response) => response,
        Err(_) => "Извините, произошла ошибка при обработке запроса".to_string()
    };

    // Ensure conversation exists or create new
    let pool = &state.pool;
    let conversation_id = if let Some(cid) = chat_req.conversation_id.clone() {
        // Validate conversation belongs to user
        let exists: Option<i64> = sqlx::query_scalar(
            "SELECT CASE WHEN EXISTS(SELECT 1 FROM conversations WHERE id = ? AND user_id = ?) THEN 1 ELSE 0 END"
        )
        .bind(&cid)
        .bind(&chat_req.user_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();
        match exists {
            Some(1) => cid,
            _ => {
                let new_id = Uuid::new_v4().to_string();
                let now = chrono::Utc::now().to_rfc3339();
                let title = chat_req.category.clone();
                let _ = sqlx::query(
                    "INSERT INTO conversations (id, user_id, title, created_at) VALUES (?, ?, ?, ?)"
                )
                .bind(&new_id)
                .bind(&chat_req.user_id)
                .bind(&title)
                .bind(&now)
                .execute(pool)
                .await;
                new_id
            }
        }
    } else {
        let new_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let title = chat_req.category.clone();
        let _ = sqlx::query(
            "INSERT INTO conversations (id, user_id, title, created_at) VALUES (?, ?, ?, ?)"
        )
        .bind(&new_id)
        .bind(&chat_req.user_id)
        .bind(&title)
        .bind(&now)
        .execute(pool)
        .await;
        new_id
    };

    let user_msg_id = Uuid::new_v4().to_string();
    let now1 = chrono::Utc::now().to_rfc3339();
    let _ = sqlx::query(
        "INSERT INTO messages (id, conversation_id, user_id, role, content, timestamp) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(&user_msg_id)
    .bind(&conversation_id)
    .bind(&chat_req.user_id)
    .bind("user")
    .bind(&chat_req.message)
    .bind(&now1)
    .execute(pool)
    .await;

    let asst_msg_id = Uuid::new_v4().to_string();
    let now2 = chrono::Utc::now().to_rfc3339();
    let _ = sqlx::query(
        "INSERT INTO messages (id, conversation_id, user_id, role, content, timestamp) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(&asst_msg_id)
    .bind(&conversation_id)
    .bind(&chat_req.user_id)
    .bind("assistant")
    .bind(&ai_response)
    .bind(&now2)
    .execute(pool)
    .await;

    HttpResponse::Ok().json(ChatResponse {
        response: ai_response,
        message_id: Uuid::new_v4().to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        conversation_id,
    })
}

pub async fn get_quick_advice(
    query: web::Query<QuickAdviceRequest>,
) -> HttpResponse {
    let advice = openai::generate_quick_advice(&query.category, &query.business_type).await;
    
    HttpResponse::Ok().json(json!({
        "category": query.category,
        "business_type": query.business_type,
        "advice": advice,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

pub async fn list_conversations(
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> HttpResponse {
    let user_id = path.into_inner();
    let pool = &state.pool;
    let rows = sqlx::query(
        "SELECT id, user_id, title, created_at FROM conversations WHERE user_id = ? ORDER BY datetime(created_at) DESC"
    )
    .fetch_all(pool)
    .await;

    match rows {
        Ok(rs) => {
            let list: Vec<ConversationSummary> = rs.into_iter().map(|r| ConversationSummary {
                id: r.get::<String, _>("id"),
                user_id: r.get::<String, _>("user_id"),
                title: r.try_get::<Option<String>, _>("title").unwrap_or(None),
                created_at: r.get::<String, _>("created_at"),
            }).collect();
            HttpResponse::Ok().json(json!({"user_id": user_id, "conversations": list}))
        }
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

pub async fn get_conversation_history(
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> HttpResponse {
    let conversation_id = path.into_inner();
    let pool = &state.pool;
    let rows = sqlx::query(
        "SELECT id, role, content, timestamp FROM messages WHERE conversation_id = ? ORDER BY datetime(timestamp) ASC"
    )
    .bind(&conversation_id)
    .fetch_all(pool)
    .await;

    match rows {
        Ok(rs) => {
            let messages: Vec<MessageRecord> = rs.into_iter().map(|r| MessageRecord {
                id: r.get::<String, _>("id"),
                role: r.get::<String, _>("role"),
                content: r.get::<String, _>("content"),
                timestamp: r.get::<String, _>("timestamp"),
            }).collect();
            HttpResponse::Ok().json(json!({
                "conversation_id": conversation_id,
                "messages": messages,
                "count": messages.len()
            }))
        }
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}