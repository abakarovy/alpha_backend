use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::state::AppState;

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

pub async fn get_top_trend(state: web::Data<AppState>) -> HttpResponse {
    let pool = &state.pool;
    let row = sqlx::query(
        "SELECT name, percent_change, description, why_popular, created_at FROM analytics_trends ORDER BY datetime(created_at) DESC LIMIT 1"
    )
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

pub async fn upsert_top_trend(body: web::Json<TopTrendUpsert>, state: web::Data<AppState>) -> HttpResponse {
    let b = body.into_inner();
    let pool = &state.pool;
    let res = sqlx::query(
        "INSERT INTO analytics_trends (name, percent_change, description, why_popular) VALUES (?, ?, ?, ?) \
         ON CONFLICT(name) DO UPDATE SET \
            percent_change = excluded.percent_change, \
            description = excluded.description, \
            why_popular = excluded.why_popular, \
            created_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')"
    )
    .bind(&b.name)
    .bind(b.percent_change)
    .bind(b.description)
    .bind(b.why_popular)
    .execute(pool)
    .await;

    match res {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({"status": "ok"})),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

pub async fn get_popularity_trends(state: web::Data<AppState>) -> HttpResponse {
    let pool = &state.pool;
    let rows = sqlx::query(
        "SELECT name, direction, percent_change, notes, created_at FROM popularity_trends ORDER BY name"
    )
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

pub async fn upsert_popularity_trend(body: web::Json<PopularityUpsert>, state: web::Data<AppState>) -> HttpResponse {
    let b = body.into_inner();
    if b.direction != "growing" && b.direction != "decreasing" {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "direction must be 'growing' or 'decreasing'"}));
    }

    let pool = &state.pool;
    let res = sqlx::query(
        "INSERT INTO popularity_trends (name, direction, percent_change, notes) VALUES (?, ?, ?, ?) \
         ON CONFLICT(name) DO UPDATE SET \
            direction = excluded.direction, \
            percent_change = excluded.percent_change, \
            notes = excluded.notes, \
            created_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')"
    )
    .bind(&b.name)
    .bind(&b.direction)
    .bind(b.percent_change)
    .bind(b.notes)
    .execute(pool)
    .await;

    match res {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({"status": "ok"})),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}
