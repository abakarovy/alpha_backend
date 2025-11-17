use actix_web::{web, HttpResponse};
use serde_json::json;
use uuid::Uuid;

use crate::models::{ChatRequest, ChatResponse, QuickAdviceRequest, MessageRecord, ConversationSummary, FileAttachment, TableSpec};
use crate::state::AppState;
use crate::services::openai;
use sqlx::Row;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use rust_xlsxwriter::Workbook;
use std::io::Cursor;
use serde::Deserialize;

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

    // Derive a human-readable title for the conversation, preferably from the AI response
    let title: Option<String> = chat_req.category.clone().or_else(|| {
        let first_line = ai_response.lines().next().unwrap_or("").trim();
        if first_line.is_empty() {
            None
        } else {
            Some(first_line.chars().take(80).collect())
        }
    });

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

    // Optionally generate a file attachment
    let mut files: Vec<FileAttachment> = Vec::new();
    // Priority: client hint -> AI intent extracted from response
    let (mut fmt_opt, mut table_opt) = (chat_req.output_format.clone(), chat_req.table.clone());
    if fmt_opt.is_none() || table_opt.is_none() {
        if let Some((f, t)) = extract_file_intent(&ai_response) {
            fmt_opt = Some(f);
            table_opt = Some(t);
        }
    }
    if let (Some(fmt), Some(table)) = (fmt_opt.as_deref(), table_opt.as_ref()) {
        match generate_file_and_store(pool, fmt, table).await {
            Ok(att) => files.push(att),
            Err(_) => { /* ignore file errors to not break chat */ }
        }
    }

    HttpResponse::Ok().json(ChatResponse {
        response: ai_response,
        message_id: Uuid::new_v4().to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        conversation_id,
        files: if files.is_empty() { None } else { Some(files) },
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
    .bind(&user_id)
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

#[derive(serde::Deserialize)]
struct FileIntent {
    output_format: String,
    table: TableSpec,
}

fn extract_file_intent(text: &str) -> Option<(String, TableSpec)> {
    let cleaned = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```");

    if let Ok(intent) = serde_json::from_str::<FileIntent>(cleaned) {
        return Some((intent.output_format, intent.table));
    }

    if let (Some(start), Some(end)) = (text.find('{'), text.rfind('}')) {
        if start < end {
            let slice = &text[start..=end];
            if let Ok(intent) = serde_json::from_str::<FileIntent>(slice) {
                return Some((intent.output_format, intent.table));
            }
        }
    }
    None
}

async fn generate_file_and_store(pool: &sqlx::SqlitePool, fmt: &str, table: &TableSpec) -> Result<FileAttachment, Box<dyn std::error::Error>> {
    let (filename, mime, bytes) = match fmt.to_ascii_lowercase().as_str() {
        "xlsx" => {
            let mut wb = Workbook::new();
            let ws = wb.add_worksheet();
            for (c, h) in table.headers.iter().enumerate() {
                ws.write_string(0, c as u16, h)?;
            }
            for (r, row) in table.rows.iter().enumerate() {
                for (c, val) in row.iter().enumerate() {
                    ws.write_string((r as u32) + 1, c as u16, val)?;
                }
            }
            let mut buf: Vec<u8> = Vec::new();
            wb.save_to_writer(&mut Cursor::new(&mut buf))?;
            (
                format!("report-{}.xlsx", chrono::Utc::now().format("%Y%m%d-%H%M%S")),
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".to_string(),
                buf,
            )
        }
        "csv" => {
            let mut s = String::new();
            s.push_str(&table.headers.join(","));
            s.push('\n');
            for row in &table.rows {
                s.push_str(&row.iter().map(|v| v.replace('\n', " ")).collect::<Vec<_>>().join(","));
                s.push('\n');
            }
            (
                format!("report-{}.csv", chrono::Utc::now().format("%Y%m%d-%H%M%S")),
                "text/csv".to_string(),
                s.into_bytes(),
            )
        }
        _ => return Err("unsupported_format".into()),
    };

    let size = bytes.len();
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO files (id, filename, mime, size, bytes) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(&filename)
    .bind(&mime)
    .bind(size as i64)
    .bind(bytes.clone())
    .execute(pool)
    .await?;

    let content_base64 = if size <= 1024 * 1024 {
        Some(B64.encode(&bytes))
    } else {
        None
    };
    let download_url = Some(format!("/api/files/{}", id));

    Ok(FileAttachment {
        id: Some(id),
        filename,
        mime,
        size,
        content_base64,
        download_url,
    })
}