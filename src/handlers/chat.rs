use actix_web::{web, HttpRequest, HttpResponse};
use serde_json::json;
use uuid::Uuid;

use crate::models::{ChatRequest, ChatResponse, MessageRecord, ConversationSummary, FileAttachment, TableSpec, ConversationContext, ContextFilters, CreateConversationRequest};
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

    let default_business_type = match locale {
        Locale::Ru => "общий бизнес",
        Locale::En => "general business",
    };
    
    let error_message = match locale {
        Locale::Ru => "Извините, произошла ошибка при обработке запроса",
        Locale::En => "Sorry, an error occurred while processing your request",
    };

    let pool = &state.pool;
    
    // Resolve user_id to main user_id for conversation synchronization
    let resolved_user_id = resolve_user_id_for_conversations(pool, &chat_req.user_id).await;
    
    let conversation_id = if let Some(cid) = chat_req.conversation_id.clone() {
        // Validate conversation belongs to resolved user_id (all conversations use resolved_user_id)
        let exists: Option<i64> = sqlx::query_scalar(
            "SELECT CASE WHEN EXISTS(SELECT 1 FROM conversations WHERE id = ? AND user_id = ?) THEN 1 ELSE 0 END"
        )
        .bind(&cid)
        .bind(&resolved_user_id)
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
                .bind(&resolved_user_id)
                .bind::<Option<String>>(None)
                .bind(&now)
                .execute(pool)
                .await;
                
                // Сохранить контекст, если передан
                if let Some(ref ctx) = chat_req.context_filters {
                    let _ = save_conversation_context(pool, &new_id, ctx).await;
                }
                
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
        .bind(&resolved_user_id)
        .bind::<Option<String>>(None)
        .bind(&now)
        .execute(pool)
        .await;
        
        // Сохранить контекст, если передан
        if let Some(ref ctx) = chat_req.context_filters {
            let _ = save_conversation_context(pool, &new_id, ctx).await;
        }
        
        new_id
    };
    
    // Получить контекст для использования в промпте
    let conversation_context = get_conversation_context(pool, &conversation_id).await;
    let user_base_context = get_user_base_context(pool, &resolved_user_id).await;
    let final_context = merge_contexts(user_base_context, conversation_context, chat_req.context_filters.clone());

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
        final_context,
    ).await {
        Ok(response) => response,
        Err(_) => error_message.to_string()
    };

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
                ai_response = raw_ai_response.clone();
            }
        } else {
            ai_response = raw_ai_response.clone();
        }
    }

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
    .bind(&resolved_user_id)
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
    .bind(&resolved_user_id)
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


pub async fn create_conversation(
    _req: HttpRequest,
    data: web::Json<CreateConversationRequest>,
    state: web::Data<AppState>,
) -> HttpResponse {
    let pool = &state.pool;
    
    // Resolve user_id to main user_id for conversation synchronization
    let resolved_user_id = resolve_user_id_for_conversations(pool, &data.user_id).await;
    
    let conversation_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    
    // Создать беседу
    let _ = sqlx::query(
        "INSERT INTO conversations (id, user_id, title, created_at) VALUES (?, ?, ?, ?)"
    )
    .bind(&conversation_id)
    .bind(&resolved_user_id)
    .bind(&data.title)
    .bind(&now)
    .execute(pool)
    .await;
    
    // Сохранить контекст беседы, если передан
    if let Some(ref context) = data.context {
        let _ = save_conversation_context(pool, &conversation_id, context).await;
    }
    
    HttpResponse::Ok().json(json!({
        "conversation_id": conversation_id,
        "created_at": now
    }))
}

pub async fn update_conversation_context(
    _req: HttpRequest,
    path: web::Path<String>,
    data: web::Json<ContextFilters>,
    state: web::Data<AppState>,
) -> HttpResponse {
    let conversation_id = path.into_inner();
    let pool = &state.pool;
    
    // Проверить существование беседы
    let exists: Option<i64> = sqlx::query_scalar(
        "SELECT CASE WHEN EXISTS(SELECT 1 FROM conversations WHERE id = ?) THEN 1 ELSE 0 END"
    )
    .bind(&conversation_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();
    
    match exists {
        Some(1) => {
            let result = save_conversation_context(pool, &conversation_id, &data.into_inner()).await;
            match result {
                Ok(_) => HttpResponse::Ok().json(json!({"status": "ok"})),
                Err(_) => HttpResponse::InternalServerError().json(json!({"error": "Failed to update context"})),
            }
        }
        _ => HttpResponse::NotFound().json(json!({"error": "Conversation not found"})),
    }
}

pub async fn list_conversations(
    _req: HttpRequest,
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> HttpResponse {
    let user_id = path.into_inner();
    let pool = &state.pool;
    
    // Resolve to main user_id - all conversations are stored with main user_id
    let resolved_user_id = resolve_user_id_for_conversations(pool, &user_id).await;
    
    // Show conversations for the resolved user_id
    // Since all conversations are created with resolved_user_id, they will be synced between platforms
    let rows = sqlx::query(
        r#"
        SELECT 
            c.id, c.user_id, c.title, c.created_at,
            ctx.user_role, ctx.business_stage, ctx.goal, ctx.urgency, ctx.region, ctx.business_niche
        FROM conversations c
        LEFT JOIN conversation_context ctx ON c.id = ctx.conversation_id
        WHERE c.user_id = ? 
        ORDER BY datetime(c.created_at) DESC
        "#
    )
    .bind(&resolved_user_id)
    .fetch_all(pool)
    .await;

    match rows {
        Ok(rs) => {
            let list: Vec<ConversationSummary> = rs.into_iter().map(|r| {
                let context = if r.try_get::<Option<String>, _>("user_role").ok().flatten().is_some() {
                    Some(ConversationContext {
                        user_role: r.try_get("user_role").ok().flatten(),
                        business_stage: r.try_get("business_stage").ok().flatten(),
                        goal: r.try_get("goal").ok().flatten(),
                        urgency: r.try_get("urgency").ok().flatten(),
                        region: r.try_get("region").ok().flatten(),
                        business_niche: r.try_get("business_niche").ok().flatten(),
                    })
                } else {
                    None
                };
                
                ConversationSummary {
                    id: r.get("id"),
                    user_id: r.get("user_id"),
                    title: r.try_get("title").ok().flatten(),
                    created_at: r.get("created_at"),
                    context,
                }
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

pub async fn delete_conversation(
    req: HttpRequest,
    path: web::Path<String>,
    state: web::Data<AppState>,
    body: web::Json<ConversationOwner>,
) -> HttpResponse {
    let conversation_id = path.into_inner();
    let pool = &state.pool;

    // Resolve user_id to main user_id
    let resolved_user_id = resolve_user_id_for_conversations(pool, &body.user_id).await;
    
    // Check if conversation belongs to resolved user_id
    let exists: Option<i64> = sqlx::query_scalar(
        "SELECT CASE WHEN EXISTS(SELECT 1 FROM conversations WHERE id = ? AND user_id = ?) THEN 1 ELSE 0 END"
    )
    .bind(&conversation_id)
    .bind(&resolved_user_id)
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
                .bind(&resolved_user_id)
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

    // Resolve user_id to main user_id
    let resolved_user_id = resolve_user_id_for_conversations(pool, &update.user_id).await;
    
    let result = sqlx::query(
        "UPDATE conversations SET title = ? WHERE id = ? AND user_id = ?",
    )
    .bind(update.title.as_deref())
    .bind(&conversation_id)
    .bind(&resolved_user_id)
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
        return None;
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
        "xlsx".to_string()
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

// ========== USER ID RESOLUTION ==========

/// Resolves user_id to the main user_id for conversation synchronization
/// Handles linking between main users and telegram users in both directions:
/// 1. If main user_id is provided - returns it as is
/// 2. If telegram_user_id is provided - finds linked main user_id via:
///    - Direct link through telegram_users.user_id
///    - Link through matching telegram_username between users and telegram_users
/// 3. If telegram_username is provided - finds main user by telegram_username
async fn resolve_user_id_for_conversations(
    pool: &sqlx::SqlitePool,
    user_id: &str,
) -> String {
    // First, check if this is a main user_id (exists in users table)
    let is_main_user: Option<i64> = sqlx::query_scalar(
        "SELECT COUNT(1) FROM users WHERE id = ?"
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();
    
    if let Some(1) = is_main_user {
        // This is a main user_id - return it directly
        // All conversations will be created with this user_id
        return user_id.to_string();
    }
    
    // Check if this is a telegram_user_id (numeric)
    if let Ok(telegram_user_id) = user_id.parse::<i64>() {
        // Try to find linked main user_id through direct link (telegram_users.user_id)
        let linked_user_id: Option<String> = sqlx::query_scalar(
            "SELECT user_id FROM telegram_users WHERE telegram_user_id = ? AND user_id IS NOT NULL"
        )
        .bind(telegram_user_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();
        
        if let Some(main_user_id) = linked_user_id {
            return main_user_id;
        }
        
        // Try to find linked main user through matching telegram_username
        // Get telegram_username from telegram_users
        let telegram_username: Option<String> = sqlx::query_scalar(
            "SELECT telegram_username FROM telegram_users WHERE telegram_user_id = ? AND telegram_username IS NOT NULL"
        )
        .bind(telegram_user_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();
        
        if let Some(username) = telegram_username {
            // Find main user with matching telegram_username
            let main_user_id: Option<String> = sqlx::query_scalar(
                "SELECT id FROM users WHERE telegram_username = ?"
            )
            .bind(&username)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();
            
            if let Some(main_id) = main_user_id {
                return main_id;
            }
        }
    }
    
    // Check if this is a telegram_username string (not numeric)
    // Try to find main user by telegram_username
    let user_by_telegram_username: Option<String> = sqlx::query_scalar(
        "SELECT id FROM users WHERE telegram_username = ?"
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();
    
    if let Some(main_user_id) = user_by_telegram_username {
        return main_user_id;
    }
    
    // If telegram_user_id is provided but not linked to any main user,
    // we can't create conversations (they require main user_id UUID)
    // Return original user_id as fallback (but conversations won't work until linked)
    user_id.to_string()
}

/// Gets all user IDs that should see the same conversations
/// Returns a list including the main user_id and any linked telegram_user_ids
async fn get_synced_user_ids(
    pool: &sqlx::SqlitePool,
    user_id: &str,
) -> Vec<String> {
    let mut user_ids = vec![user_id.to_string()];
    
    // Check if main user has telegram_username
    let telegram_username: Option<String> = sqlx::query_scalar(
        "SELECT telegram_username FROM users WHERE id = ? AND telegram_username IS NOT NULL"
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();
    
    if let Some(username) = telegram_username {
        // Find telegram_user by username
        let telegram_user_id: Option<i64> = sqlx::query_scalar(
            "SELECT telegram_user_id FROM telegram_users WHERE telegram_username = ?"
        )
        .bind(&username)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();
        
        if let Some(tg_user_id) = telegram_user_id {
            user_ids.push(tg_user_id.to_string());
        }
    }
    
    // Also check reverse: if telegram_user is linked to this user
    if let Ok(telegram_user_id) = user_id.parse::<i64>() {
        let linked_user_id: Option<String> = sqlx::query_scalar(
            "SELECT user_id FROM telegram_users WHERE telegram_user_id = ? AND user_id IS NOT NULL"
        )
        .bind(telegram_user_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();
        
        if let Some(main_id) = linked_user_id {
            if !user_ids.contains(&main_id) {
                user_ids.push(main_id);
            }
        }
    }
    
    user_ids
}

// ========== CONTEXT FUNCTIONS ==========

async fn get_conversation_context(
    pool: &sqlx::SqlitePool,
    conversation_id: &str,
) -> Option<ConversationContext> {
    let row = sqlx::query(
        "SELECT user_role, business_stage, goal, urgency, region, business_niche 
         FROM conversation_context WHERE conversation_id = ?"
    )
    .bind(conversation_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()?;
    
    Some(ConversationContext {
        user_role: row.try_get("user_role").ok().flatten(),
        business_stage: row.try_get("business_stage").ok().flatten(),
        goal: row.try_get("goal").ok().flatten(),
        urgency: row.try_get("urgency").ok().flatten(),
        region: row.try_get("region").ok().flatten(),
        business_niche: row.try_get("business_niche").ok().flatten(),
    })
}

async fn get_user_base_context(
    pool: &sqlx::SqlitePool,
    user_id: &str,
) -> ConversationContext {
    let row = sqlx::query(
        "SELECT user_role, business_stage, business_niche, region FROM users WHERE id = ?"
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();
    
    match row {
        Some(r) => ConversationContext {
            user_role: r.try_get("user_role").ok().flatten(),
            business_stage: r.try_get("business_stage").ok().flatten(),
            goal: None,
            urgency: None,
            region: r.try_get("region").ok().flatten(),
            business_niche: r.try_get("business_niche").ok().flatten(),
        },
        None => ConversationContext {
            user_role: None,
            business_stage: None,
            goal: None,
            urgency: None,
            region: None,
            business_niche: None,
        },
    }
}

fn merge_contexts(
    base: ConversationContext,
    conversation: Option<ConversationContext>,
    filters: Option<ContextFilters>,
) -> ConversationContext {
    // Приоритет: filters > conversation > base
    let mut result = base;
    
    // Применить контекст беседы
    if let Some(conv) = conversation {
        if let Some(role) = conv.user_role {
            result.user_role = Some(role);
        }
        if let Some(stage) = conv.business_stage {
            result.business_stage = Some(stage);
        }
        if let Some(goal) = conv.goal {
            result.goal = Some(goal);
        }
        if let Some(urgency) = conv.urgency {
            result.urgency = Some(urgency);
        }
        if let Some(region) = conv.region {
            result.region = Some(region);
        }
        if let Some(niche) = conv.business_niche {
            result.business_niche = Some(niche);
        }
    }
    
    // Применить фильтры из запроса (высший приоритет)
    if let Some(f) = filters {
        if let Some(role) = f.user_role {
            result.user_role = Some(role);
        }
        if let Some(stage) = f.business_stage {
            result.business_stage = Some(stage);
        }
        if let Some(goal) = f.goal {
            result.goal = Some(goal);
        }
        if let Some(urgency) = f.urgency {
            result.urgency = Some(urgency);
        }
        if let Some(region) = f.region {
            result.region = Some(region);
        }
        if let Some(niche) = f.business_niche {
            result.business_niche = Some(niche);
        }
    }
    
    result
}

async fn save_conversation_context(
    pool: &sqlx::SqlitePool,
    conversation_id: &str,
    context: &ContextFilters,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO conversation_context (conversation_id, user_role, business_stage, goal, urgency, region, business_niche)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(conversation_id) DO UPDATE SET
            user_role = COALESCE(excluded.user_role, conversation_context.user_role),
            business_stage = COALESCE(excluded.business_stage, conversation_context.business_stage),
            goal = COALESCE(excluded.goal, conversation_context.goal),
            urgency = COALESCE(excluded.urgency, conversation_context.urgency),
            region = COALESCE(excluded.region, conversation_context.region),
            business_niche = COALESCE(excluded.business_niche, conversation_context.business_niche),
            updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
        "#
    )
    .bind(conversation_id)
    .bind(&context.user_role)
    .bind(&context.business_stage)
    .bind(&context.goal)
    .bind(&context.urgency)
    .bind(&context.region)
    .bind(&context.business_niche)
    .execute(pool)
    .await?;
    
    Ok(())
}