use actix_web::{HttpResponse, web};
use sqlx::Row;
use crate::state::AppState;

pub async fn download_file(path: web::Path<String>, state: web::Data<AppState>) -> HttpResponse {
    let id = path.into_inner();
    let pool = &state.pool;

    let row = sqlx::query("SELECT filename, mime, bytes FROM files WHERE id = ?")
        .bind(&id)
        .fetch_optional(pool)
        .await;

    match row {
        Ok(Some(r)) => {
            let filename = r.get::<String, _>("filename");
            let mime = r.get::<String, _>("mime");
            let bytes = r.get::<Vec<u8>, _>("bytes");
            HttpResponse::Ok()
                .append_header(("Content-Type", mime))
                .append_header(("Content-Disposition", format!("attachment; filename=\"{}\"", filename)))
                .body(bytes)
        }
        Ok(None) => HttpResponse::NotFound().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}
