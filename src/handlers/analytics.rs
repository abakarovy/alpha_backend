use actix_web::{web, HttpResponse, HttpRequest};
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::state::AppState;
use crate::i18n;

// --------- DTOs ---------
#[derive(Debug, Deserialize)]
pub struct TopTrendUpsert {
    pub name: String,
    pub percent_change: Option<f64>,
    pub description: Option<String>,
    pub why_popular: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TopTrend {
    pub name: String,
    pub percent_change: Option<f64>,
    pub description: Option<String>,
    pub why_popular: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct PopularityUpsert {
    pub name: String,
    pub direction: String, // 'growing' | 'decreasing'
    pub percent_change: Option<f64>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PopularityTrend {
    pub name: String,
    pub direction: String,
    pub percent_change: Option<f64>,
    pub notes: Option<String>,
    pub created_at: String,
}

pub async fn get_top_trend(req: HttpRequest, state: web::Data<AppState>) -> HttpResponse {
    let pool = &state.pool;
    let loc = i18n::detect_locale(&req);
    let locale = match loc { i18n::Locale::Ru => "ru", _ => "en" };
    let row = sqlx::query(
        "SELECT t.name, t.percent_change,
                COALESCE(i.description, t.description) AS description,
                COALESCE(i.why_popular, t.why_popular) AS why_popular,
                t.created_at
         FROM analytics_trends t
         LEFT JOIN analytics_trends_i18n i
           ON i.name = t.name AND i.locale = ?
         ORDER BY datetime(t.created_at) DESC LIMIT 1"
    )
    .bind(locale)
    .fetch_optional(pool)
    .await;

    match row {
        Ok(Some(r)) => {
            let tt = TopTrend {
                name: r.get::<String, _>("name"),
                percent_change: r.try_get::<Option<f64>, _>("percent_change").unwrap_or(None),
                description: r.try_get::<Option<String>, _>("description").unwrap_or(None),
                why_popular: r.try_get::<Option<String>, _>("why_popular").unwrap_or(None),
                created_at: r.get::<String, _>("created_at"),
            };
            HttpResponse::Ok().json(tt)
        }
        Ok(None) => HttpResponse::Ok().json(serde_json::json!({})),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

pub async fn upsert_top_trend(req: HttpRequest, body: web::Json<TopTrendUpsert>, state: web::Data<AppState>) -> HttpResponse {
    let b = body.into_inner();
    let pool = &state.pool;
    let loc = i18n::detect_locale(&req);
    let locale = match loc { i18n::Locale::Ru => "ru", _ => "en" };

    // Upsert base numeric fields into base table (name, percent_change)
    let base_res = sqlx::query(
        "INSERT INTO analytics_trends (name, percent_change, description, why_popular) VALUES (?, ?, COALESCE(?, description), COALESCE(?, why_popular)) \
         ON CONFLICT(name) DO UPDATE SET \
            percent_change = COALESCE(excluded.percent_change, analytics_trends.percent_change), \
            created_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')"
    )
    .bind(&b.name)
    .bind(b.percent_change)
    .bind(b.description.clone())
    .bind(b.why_popular.clone())
    .execute(pool)
    .await;

    if base_res.is_err() { return HttpResponse::InternalServerError().finish(); }

    // Upsert localized text into i18n table for the detected locale
    let _ = sqlx::query(
        "INSERT INTO analytics_trends_i18n (name, locale, description, why_popular) VALUES (?, ?, ?, ?) \
         ON CONFLICT(name, locale) DO UPDATE SET \
            description = COALESCE(excluded.description, analytics_trends_i18n.description), \
            why_popular = COALESCE(excluded.why_popular, analytics_trends_i18n.why_popular)"
    )
    .bind(&b.name)
    .bind(locale)
    .bind(b.description)
    .bind(b.why_popular)
    .execute(pool)
    .await;

    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}

pub async fn get_popularity_trends(req: HttpRequest, state: web::Data<AppState>) -> HttpResponse {
    let pool = &state.pool;
    let loc = i18n::detect_locale(&req);
    let locale = match loc { i18n::Locale::Ru => "ru", _ => "en" };
    let rows = sqlx::query(
        "SELECT t.name, t.direction, t.percent_change,
                COALESCE(i.notes, t.notes) AS notes,
                t.created_at
         FROM popularity_trends t
         LEFT JOIN popularity_trends_i18n i
           ON i.name = t.name AND i.locale = ?
         ORDER BY t.name"
    )
    .bind(locale)
    .fetch_all(pool)
    .await;

    match rows {
        Ok(rs) => {
            let items: Vec<PopularityTrend> = rs.into_iter().map(|r| PopularityTrend {
                name: r.get::<String, _>("name"),
                direction: r.get::<String, _>("direction"),
                percent_change: r.try_get::<Option<f64>, _>("percent_change").unwrap_or(None),
                notes: r.try_get::<Option<String>, _>("notes").unwrap_or(None),
                created_at: r.get::<String, _>("created_at"),
            }).collect();
            HttpResponse::Ok().json(items)
        }
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

pub async fn upsert_popularity_trend(req: HttpRequest, body: web::Json<PopularityUpsert>, state: web::Data<AppState>) -> HttpResponse {
    let b = body.into_inner();
    if b.direction != "growing" && b.direction != "decreasing" {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "direction must be 'growing' or 'decreasing'"}));
    }

    let pool = &state.pool;
    let loc = i18n::detect_locale(&req);
    let locale = match loc { i18n::Locale::Ru => "ru", _ => "en" };

    // Upsert base fields (direction, percent_change). Keep base notes if provided as fallback.
    let base_res = sqlx::query(
        "INSERT INTO popularity_trends (name, direction, percent_change, notes) VALUES (?, ?, ?, COALESCE(?, notes)) \
         ON CONFLICT(name) DO UPDATE SET \
            direction = excluded.direction, \
            percent_change = COALESCE(excluded.percent_change, popularity_trends.percent_change), \
            created_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')"
    )
    .bind(&b.name)
    .bind(&b.direction)
    .bind(b.percent_change)
    .bind(b.notes.clone())
    .execute(pool)
    .await;

    if base_res.is_err() { return HttpResponse::InternalServerError().finish(); }

    // Upsert localized notes into i18n table
    let _ = sqlx::query(
        "INSERT INTO popularity_trends_i18n (name, locale, notes) VALUES (?, ?, ?) \
         ON CONFLICT(name, locale) DO UPDATE SET \
            notes = COALESCE(excluded.notes, popularity_trends_i18n.notes)"
    )
    .bind(&b.name)
    .bind(locale)
    .bind(b.notes)
    .execute(pool)
    .await;

    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}
