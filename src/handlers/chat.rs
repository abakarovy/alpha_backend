use actix_web::{web, HttpRequest, HttpResponse};
use serde_json::json;
use uuid::Uuid;

use crate::models::{ChatRequest, ChatResponse, QuickAdviceRequest, MessageRecord, ConversationSummary, FileAttachment, TableSpec};
use crate::state::AppState;
use crate::services::openai;
use crate::i18n::{self, Locale};
use sqlx::Row;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use rust_xlsxwriter::Workbook;
use std::io::Cursor;
use serde::Deserialize;

pub async fn send_message(
    req: HttpRequest,
    data: web::Json<ChatRequest>,
    state: web::Data<AppState>,
) -> HttpResponse {
    let chat_req = data.into_inner();
    
    // Detect locale: priority: request field -> query param -> Accept-Language header -> default EN
    let locale = if let Some(lang) = chat_req.language.as_ref() {
        match lang.to_lowercase().as_str() {
            "ru" | "ru-ru" => Locale::Ru,
            _ => Locale::En,
        }
    } else {
        i18n::detect_locale(&req)
    };
    
    if chat_req.message.is_empty() || chat_req.user_id.is_empty() {
        let error_msg = match locale {
            Locale::Ru => "Требуются сообщение и user_id",
            Locale::En => "Message and user_id are required",
        };
        return HttpResponse::BadRequest().json(json!({
            "error": error_msg
        }));
    }

    // Get locale-aware defaults
    let default_business_type = match locale {
        Locale::Ru => "общий бизнес",
        Locale::En => "general business",
    };
    
    let error_message = match locale {
        Locale::Ru => "Извините, произошла ошибка при обработке запроса",
        Locale::En => "Sorry, an error occurred while processing your request",
    };

    // Ensure conversation exists or create new (needed to get conversation_id for history retrieval)
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
                .bind::<Option<String>>(None)
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
        .bind::<Option<String>>(None)
        .bind(&now)
        .execute(pool)
        .await;
        new_id
    };

    // Retrieve conversation history from this conversation (all previous messages)
    let conversation_history = {
        let history_rows = sqlx::query(
            "SELECT role, content FROM messages WHERE conversation_id = ? ORDER BY datetime(timestamp) ASC"
        )
        .bind(&conversation_id)
        .fetch_all(pool)
        .await
        .ok();
        
        history_rows.map(|rows| {
            rows.into_iter()
                .map(|r| {
                    let role: String = r.get("role");
                    let content: String = r.get("content");
                    (role, content)
                })
                .collect()
        })
    };

    let raw_ai_response = match openai::generate_response(
        &chat_req.message,
        chat_req.category.as_deref().unwrap_or("general"),
        chat_req.business_type.as_deref().unwrap_or(default_business_type),
        &state,
        &chat_req.user_id,
        locale,
        conversation_history,
    ).await {
        Ok(response) => response,
        Err(_) => error_message.to_string()
    };

    // Extract TITLE: ... line from the AI response, if present
    let mut ai_response = String::new();
    let mut title: Option<String> = None;
    {
        let mut lines = raw_ai_response.lines();
        if let Some(first) = lines.next() {
            let trimmed = first.trim();
            if let Some(rest) = trimmed.strip_prefix("TITLE:") {
                let t = rest.trim();
                if !t.is_empty() {
                    title = Some(t.chars().take(80).collect());
                }
                // skip optional blank line after TITLE
                if let Some(second) = lines.next() {
                    let second_trimmed = second.trim();
                    if second_trimmed.is_empty() {
                        ai_response = lines.collect::<Vec<_>>().join("\n");
                    } else {
                        let mut all = Vec::new();
                        all.push(second);
                        all.extend(lines);
                        ai_response = all.join("\n");
                    }
                } else {
                    ai_response.clear();
                }
            } else {
                // No TITLE prefix; keep original content
                ai_response = raw_ai_response.clone();
            }
        } else {
            ai_response = raw_ai_response.clone();
        }
    }

    // Fallback: if no explicit TITLE was provided, derive from first non-empty line
    if title.is_none() {
        let first_line = ai_response
            .lines()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("")
            .trim();
        if !first_line.is_empty() {
            title = Some(first_line.chars().take(80).collect());
        }
    }

    // Update conversation title if we have one and it's a new conversation
    if let Some(ref title_str) = title {
        let _ = sqlx::query(
            "UPDATE conversations SET title = ? WHERE id = ? AND (title IS NULL OR title = '')"
        )
        .bind(title_str)
        .bind(&conversation_id)
        .execute(pool)
        .await;
    }

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

    let mut files: Vec<FileAttachment> = Vec::new();
    let (mut fmt_opt, mut table_opt) = (chat_req.output_format.clone(), chat_req.table.clone());
    
    if fmt_opt.is_none() || table_opt.is_none() {
        if let Some((f, t)) = extract_file_intent(&ai_response) {
            fmt_opt = Some(f);
            table_opt = Some(t);
        }
    }
    
    if table_opt.is_none() {
        if let Some(table) = parse_markdown_table(&ai_response) {
            table_opt = Some(table);
            // Detect format from user message if not already set
            if fmt_opt.is_none() {
                fmt_opt = Some(detect_format_from_message(&chat_req.message));
            }
        }
    }
    
    if let (Some(fmt), Some(table)) = (fmt_opt.as_deref(), table_opt.as_ref()) {
        match generate_file_and_store(pool, fmt, table, Some(&asst_msg_id)).await {
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
    _req: HttpRequest,
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
    _req: HttpRequest,
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
            let messages: Vec<MessageRecord> = rs
                .into_iter()
                .map(|r| MessageRecord {
                id: r.get::<String, _>("id"),
                role: r.get::<String, _>("role"),
                content: r.get::<String, _>("content"),
                timestamp: r.get::<String, _>("timestamp"),
                })
                .collect();

            // For each message, load associated files (if any)
            let mut files_by_message: Vec<serde_json::Value> = Vec::new();
            for msg in &messages {
                let file_rows = sqlx::query(
                    "SELECT id, filename, mime, size, bytes FROM files WHERE message_id = ?"
                )
                .bind(&msg.id)
                .fetch_all(pool)
                .await;

                if let Ok(frs) = file_rows {
                    if frs.is_empty() {
                        continue;
                    }

                    let mut attachments: Vec<FileAttachment> = Vec::new();
                    for fr in frs {
                        let id = fr.get::<String, _>("id");
                        let filename = fr.get::<String, _>("filename");
                        let mime = fr.get::<String, _>("mime");
                        let size = fr.get::<i64, _>("size") as usize;
                        let bytes: Vec<u8> = fr.get("bytes");

                        let content_base64 = if size <= 1024 * 1024 {
                            Some(B64.encode(&bytes))
                        } else {
                            None
                        };
                        let download_url = Some(format!("/api/files/{}", id));

                        attachments.push(FileAttachment {
                            id: Some(id),
                            filename,
                            mime,
                            size,
                            content_base64,
                            download_url,
                        });
                    }

                    if !attachments.is_empty() {
                        files_by_message.push(json!({
                            "message_id": msg.id,
                            "files": attachments,
                        }));
                    }
                }
            }

            HttpResponse::Ok().json(json!({
                "conversation_id": conversation_id,
                "messages": messages,
                "count": messages.len(),
                "attachments": files_by_message,
            }))
        }
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[derive(Deserialize)]
pub struct ConversationOwner {
    pub user_id: String,
}

#[derive(Deserialize)]
pub struct UpdateConversationTitle {
    pub user_id: String,
    pub title: Option<String>,
}

// Delete a conversation and all its messages, only if it belongs to the given user
pub async fn delete_conversation(
    req: HttpRequest,
    path: web::Path<String>,
    state: web::Data<AppState>,
    body: web::Json<ConversationOwner>,
) -> HttpResponse {
    let conversation_id = path.into_inner();
    let user_id = &body.user_id;
    let pool = &state.pool;

    // Ensure the conversation belongs to the user
    let exists: Option<i64> = sqlx::query_scalar(
        "SELECT CASE WHEN EXISTS(SELECT 1 FROM conversations WHERE id = ? AND user_id = ?) THEN 1 ELSE 0 END"
    )
    .bind(&conversation_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    let locale = i18n::detect_locale(&req);
    let error_msg = match locale {
        Locale::Ru => "Разговор не найден или не принадлежит пользователю",
        Locale::En => "conversation-not-found-or-not-owned",
    };

    match exists {
        Some(1) => {
            // Delete messages first due to FK
            let _ = sqlx::query("DELETE FROM messages WHERE conversation_id = ?")
                .bind(&conversation_id)
                .execute(pool)
                .await;

            let _ = sqlx::query("DELETE FROM conversations WHERE id = ? AND user_id = ?")
                .bind(&conversation_id)
                .bind(user_id)
                .execute(pool)
                .await;

            HttpResponse::Ok().json(json!({
                "status": "deleted",
                "conversation_id": conversation_id,
            }))
        }
        _ => HttpResponse::NotFound().json(json!({
            "error": error_msg,
        })),
    }
}

// Update the title of a conversation, only if it belongs to the given user
pub async fn update_conversation_title(
    req: HttpRequest,
    path: web::Path<String>,
    state: web::Data<AppState>,
    body: web::Json<UpdateConversationTitle>,
) -> HttpResponse {
    let conversation_id = path.into_inner();
    let update = body.into_inner();
    let pool = &state.pool;
    let locale = i18n::detect_locale(&req);

    let result = sqlx::query(
        "UPDATE conversations SET title = ? WHERE id = ? AND user_id = ?",
    )
    .bind(update.title.as_deref())
    .bind(&conversation_id)
    .bind(&update.user_id)
    .execute(pool)
    .await;

    let rows_affected = match result {
        Ok(r) => r.rows_affected(),
        Err(_) => {
            let error_msg = match locale {
                Locale::Ru => "Ошибка обновления",
                Locale::En => "update-failed",
            };
            return HttpResponse::InternalServerError().json(json!({
                "error": error_msg,
            }));
        }
    };

    if rows_affected == 0 {
        let error_msg = match locale {
            Locale::Ru => "Разговор не найден или не принадлежит пользователю",
            Locale::En => "conversation-not-found-or-not-owned",
        };
        return HttpResponse::NotFound().json(json!({
            "error": error_msg,
        }));
    }

    HttpResponse::Ok().json(json!({
        "status": "updated",
        "conversation_id": conversation_id,
    }))
}

#[derive(serde::Deserialize)]
struct FileIntent {
    output_format: String,
    table: TableSpec,
}

fn extract_file_intent(text: &str) -> Option<(String, TableSpec)> {
    // First, try to extract JSON from code blocks (```json ... ``` or ``` ... ```)
    let json_block_markers = ["```json", "```"];
    
    for marker in json_block_markers.iter() {
        if let Some(start_idx) = text.find(marker) {
            let after_marker = &text[start_idx + marker.len()..];
            if let Some(end_idx) = after_marker.find("```") {
                let json_content = after_marker[..end_idx].trim();
                if let Ok(intent) = serde_json::from_str::<FileIntent>(json_content) {
        return Some((intent.output_format, intent.table));
                }
            }
        }
    }

    // Try to find JSON object in the text (looking for the last occurrence, likely at the end)
    if let (Some(start), Some(end)) = (text.rfind('{'), text.rfind('}')) {
        if start < end {
            let slice = &text[start..=end];
            if let Ok(intent) = serde_json::from_str::<FileIntent>(slice) {
                return Some((intent.output_format, intent.table));
            }
        }
    }
    
    None
}

fn parse_markdown_table(text: &str) -> Option<TableSpec> {
    let lines: Vec<&str> = text.lines().collect();
    let mut table_lines: Vec<&str> = Vec::new();
    let mut in_table = false;
    
    // Find table boundaries (lines starting with |)
    for line in &lines {
        let trimmed = line.trim();
        if trimmed.starts_with('|') && trimmed.ends_with('|') {
            in_table = true;
            table_lines.push(trimmed);
        } else if in_table && !trimmed.starts_with('|') {
            // End of table
            break;
        }
    }
    
    if table_lines.len() < 2 {
        return None; // Need at least header and separator
    }
    
    // Parse header (first line)
    let header_line = table_lines[0];
    let headers: Vec<String> = header_line
        .split('|')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    
    if headers.is_empty() {
        return None;
    }
    
    // Skip separator line (second line, usually |---|---|)
    let mut rows: Vec<Vec<String>> = Vec::new();
    
    // Parse data rows (starting from third line)
    for line in table_lines.iter().skip(2) {
        let cells: Vec<String> = line
            .split('|')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        
        if cells.len() == headers.len() {
            rows.push(cells);
        } else if cells.len() > 0 {
            // Try to pad or truncate to match header count
            let mut adjusted_cells = cells;
            while adjusted_cells.len() < headers.len() {
                adjusted_cells.push(String::new());
            }
            adjusted_cells.truncate(headers.len());
            rows.push(adjusted_cells);
        }
    }
    
    if rows.is_empty() {
        return None;
    }
    
    Some(TableSpec { headers, rows })
}

fn detect_format_from_message(message: &str) -> String {
    let msg_lower = message.to_lowercase();
    if msg_lower.contains("csv") || msg_lower.contains("comma-separated") || msg_lower.contains(".csv") {
        "csv".to_string()
    } else if msg_lower.contains("excel") || msg_lower.contains("xlsx") || msg_lower.contains(".xlsx") || msg_lower.contains("spreadsheet") {
        "xlsx".to_string()
    } else {
        "xlsx".to_string() // Default to Excel
    }
}

async fn generate_file_and_store(
    pool: &sqlx::SqlitePool,
    fmt: &str,
    table: &TableSpec,
    message_id: Option<&str>,
) -> Result<FileAttachment, Box<dyn std::error::Error>> {
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
        "INSERT INTO files (id, filename, mime, size, bytes, message_id) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(&filename)
    .bind(&mime)
    .bind(size as i64)
    .bind(bytes.clone())
    .bind(message_id)
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