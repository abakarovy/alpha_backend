use actix_web::{web, HttpResponse};
use serde_json::json;
use uuid::Uuid;

use crate::models::{ChatRequest, ChatResponse, QuickAdviceRequest, Message};
use crate::state::AppState;
use crate::services::openai;

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

    // Сохраняем в историю
    let mut conversations = state.conversations.lock().unwrap();
    let history = conversations.entry(chat_req.user_id.clone()).or_insert_with(Vec::new);
    
    history.push(Message {
        role: "user".to_string(),
        content: chat_req.message,
        timestamp: chrono::Utc::now().to_rfc3339(),
    });
    
    history.push(Message {
        role: "assistant".to_string(),
        content: ai_response.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    HttpResponse::Ok().json(ChatResponse {
        response: ai_response,
        message_id: Uuid::new_v4().to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
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

pub async fn get_conversation_history(
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> HttpResponse {
    let user_id = path.into_inner();
    
    let conversations = state.conversations.lock().unwrap();
    let history = conversations.get(&user_id).cloned().unwrap_or_default();
    
    HttpResponse::Ok().json(json!({
        "user_id": user_id,
        "messages": history,
        "count": history.len()
    }))
}