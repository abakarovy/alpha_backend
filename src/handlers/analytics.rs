use actix_web::{web, HttpResponse, HttpRequest};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::state::AppState;
use crate::i18n;

// ========== TOP WEEKLY TRENDS ==========

#[derive(Debug, Deserialize, Serialize)]
pub struct TopTrendItem {
    pub title: String,
    pub increase: f64, // e.g., 92.0 for +92%
    pub request_percent: Option<f64>, // Only for position 1
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GeoTrendItem {
    pub country: String,
    pub increase: f64,
}

#[derive(Debug, Deserialize)]
pub struct WeeklyTrendsUpsert {
    pub current_top: TopTrendItem,
    pub second_place: TopTrendItem,
    pub geo_trends: Vec<GeoTrendItem>, // Top 3 regions
}

#[derive(Debug, Serialize)]
pub struct WeeklyTrendsResponse {
    pub current_top: TopTrendItem,
    pub second_place: TopTrendItem,
    pub geo_trends: Vec<GeoTrendItem>,
    pub week_start: String,
}

// ========== AI ANALYTICS ==========

#[derive(Debug, Deserialize)]
pub struct AiAnalyticsUpsert {
    pub increase: f64,
    pub description: String,
    pub level_of_competitiveness: Vec<f64>, // Array of at least 5 values for graph
}

#[derive(Debug, Serialize)]
pub struct AiAnalyticsResponse {
    pub increase: f64,
    pub description: String,
    pub level_of_competitiveness: Vec<f64>,
    pub created_at: String,
}

// ========== NICHES OF THE MONTH ==========

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct NicheItem {
    pub title: String,
    pub change: f64, // e.g., 34.0 for +34%, -6.0 for -6%
}

#[derive(Debug, Deserialize)]
pub struct NichesMonthUpsert {
    pub niches: Vec<NicheItem>,
}

#[derive(Debug, Serialize)]
pub struct NichesMonthResponse {
    pub niches: Vec<NicheItem>,
    pub month_start: String,
}

// ========== HANDLERS ==========

pub async fn get_weekly_trends(req: HttpRequest, state: web::Data<AppState>) -> HttpResponse {
    let pool = &state.pool;
    let loc = i18n::detect_locale(&req);
    let locale = match loc { i18n::Locale::Ru => "ru", _ => "en" };
    
    // Get current week start (Monday of current week)
    let now = chrono::Utc::now();
    let week_start = now.date_naive().week(chrono::Weekday::Mon).first_day();
    let week_start_str = week_start.format("%Y-%m-%d").to_string();
    
    // Get top trend (position 1) with localization
    let top_row = sqlx::query(
        "SELECT t.title, t.increase, t.request_percent, t.id,
                COALESCE(i.title, t.title) AS localized_title
         FROM top_weekly_trends t
         LEFT JOIN top_weekly_trends_i18n i
           ON i.id = t.id AND i.locale = ?
         WHERE t.week_start = ? AND t.position = 1 LIMIT 1"
    )
    .bind(locale)
    .bind(&week_start_str)
    .fetch_optional(pool)
    .await;
    
    // Get 2nd place (position 2) with localization
    let second_row = sqlx::query(
        "SELECT t.title, t.increase, t.request_percent, t.id,
                COALESCE(i.title, t.title) AS localized_title
         FROM top_weekly_trends t
         LEFT JOIN top_weekly_trends_i18n i
           ON i.id = t.id AND i.locale = ?
         WHERE t.week_start = ? AND t.position = 2 LIMIT 1"
    )
    .bind(locale)
    .bind(&week_start_str)
    .fetch_optional(pool)
    .await;
    
    // Get geo trends (top 3) with localization
    let geo_rows = sqlx::query(
        "SELECT g.country, g.increase, g.id,
                COALESCE(i.country, g.country) AS localized_country
         FROM geo_trends g
         LEFT JOIN geo_trends_i18n i
           ON i.id = g.id AND i.locale = ?
         WHERE g.week_start = ? ORDER BY g.rank ASC LIMIT 3"
    )
    .bind(locale)
    .bind(&week_start_str)
    .fetch_all(pool)
    .await;
    
    match (top_row, second_row, geo_rows) {
        (Ok(Some(top_r)), Ok(Some(second_r)), Ok(geo_rs)) => {
            let current_top = TopTrendItem {
                title: top_r.get::<String, _>("localized_title"),
                increase: top_r.get("increase"),
                request_percent: top_r.try_get("request_percent").ok().flatten(),
            };
            
            let second_place = TopTrendItem {
                title: second_r.get::<String, _>("localized_title"),
                increase: second_r.get("increase"),
                request_percent: second_r.try_get("request_percent").ok().flatten(),
            };
            
            let geo_trends: Vec<GeoTrendItem> = geo_rs.into_iter().map(|r| GeoTrendItem {
                country: r.get::<String, _>("localized_country"),
                increase: r.get("increase"),
            }).collect();
            
            HttpResponse::Ok().json(WeeklyTrendsResponse {
                current_top,
                second_place,
                geo_trends,
                week_start: week_start_str,
            })
        }
        _ => HttpResponse::Ok().json(serde_json::json!({}))
    }
}

pub async fn upsert_weekly_trends(req: HttpRequest, body: web::Json<WeeklyTrendsUpsert>, state: web::Data<AppState>) -> HttpResponse {
    let data = body.into_inner();
    let pool = &state.pool;
    let loc = i18n::detect_locale(&req);
    let locale = match loc { i18n::Locale::Ru => "ru", _ => "en" };
    
    // Calculate week start
    let now = chrono::Utc::now();
    let week_start = now.date_naive().week(chrono::Weekday::Mon).first_day();
    let week_start_str = week_start.format("%Y-%m-%d").to_string();
    
    // Ensure only top 3 geo trends
    let geo_trends: Vec<GeoTrendItem> = data.geo_trends.into_iter().take(3).collect();
    
    // Delete existing entries for this week (i18n will be deleted via CASCADE)
    let _ = sqlx::query("DELETE FROM top_weekly_trends WHERE week_start = ?")
        .bind(&week_start_str)
        .execute(pool)
        .await;
    
    let _ = sqlx::query("DELETE FROM geo_trends WHERE week_start = ?")
        .bind(&week_start_str)
        .execute(pool)
        .await;
    
    // Insert current top trend
    let top_id = Uuid::new_v4().to_string();
    let _ = sqlx::query(
        "INSERT INTO top_weekly_trends (id, week_start, position, title, increase, request_percent) VALUES (?, ?, 1, ?, ?, ?)"
    )
    .bind(&top_id)
    .bind(&week_start_str)
    .bind(&data.current_top.title)
    .bind(data.current_top.increase)
    .bind(data.current_top.request_percent)
    .execute(pool)
    .await;
    
    // Insert i18n for top trend
    let _ = sqlx::query(
        "INSERT INTO top_weekly_trends_i18n (id, locale, title) VALUES (?, ?, ?) ON CONFLICT(id, locale) DO UPDATE SET title = excluded.title"
    )
    .bind(&top_id)
    .bind(locale)
    .bind(&data.current_top.title)
    .execute(pool)
    .await;
    
    // Insert second place
    let second_id = Uuid::new_v4().to_string();
    let _ = sqlx::query(
        "INSERT INTO top_weekly_trends (id, week_start, position, title, increase, request_percent) VALUES (?, ?, 2, ?, ?, ?)"
    )
    .bind(&second_id)
    .bind(&week_start_str)
    .bind(&data.second_place.title)
    .bind(data.second_place.increase)
    .bind(data.second_place.request_percent)
    .execute(pool)
    .await;
    
    // Insert i18n for second place
    let _ = sqlx::query(
        "INSERT INTO top_weekly_trends_i18n (id, locale, title) VALUES (?, ?, ?) ON CONFLICT(id, locale) DO UPDATE SET title = excluded.title"
    )
    .bind(&second_id)
    .bind(locale)
    .bind(&data.second_place.title)
    .execute(pool)
    .await;
    
    // Insert geo trends
    for (idx, geo) in geo_trends.iter().enumerate() {
        let geo_id = Uuid::new_v4().to_string();
        let rank = (idx + 1) as i64;
        let _ = sqlx::query(
            "INSERT INTO geo_trends (id, week_start, country, increase, rank) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&geo_id)
        .bind(&week_start_str)
        .bind(&geo.country)
        .bind(geo.increase)
        .bind(rank)
        .execute(pool)
        .await;
        
        // Insert i18n for geo trend
        let _ = sqlx::query(
            "INSERT INTO geo_trends_i18n (id, locale, country) VALUES (?, ?, ?) ON CONFLICT(id, locale) DO UPDATE SET country = excluded.country"
        )
        .bind(&geo_id)
        .bind(locale)
        .bind(&geo.country)
        .execute(pool)
        .await;
    }
    
    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}

pub async fn get_ai_analytics(req: HttpRequest, state: web::Data<AppState>) -> HttpResponse {
    let pool = &state.pool;
    let loc = i18n::detect_locale(&req);
    let locale = match loc { i18n::Locale::Ru => "ru", _ => "en" };
    
    let row = sqlx::query(
        "SELECT a.increase, a.description, a.level_of_competitiveness, a.created_at, a.id,
                COALESCE(i.description, a.description) AS localized_description
         FROM ai_analytics a
         LEFT JOIN ai_analytics_i18n i
           ON i.id = a.id AND i.locale = ?
         ORDER BY a.created_at DESC LIMIT 1"
    )
    .bind(locale)
    .fetch_optional(pool)
    .await;
    
    match row {
        Ok(Some(r)) => {
            let competitiveness_json: String = r.get("level_of_competitiveness");
            let competitiveness: Vec<f64> = serde_json::from_str(&competitiveness_json)
                .unwrap_or_else(|_| vec![]);
            
            HttpResponse::Ok().json(AiAnalyticsResponse {
                increase: r.get("increase"),
                description: r.get::<String, _>("localized_description"),
                level_of_competitiveness: competitiveness,
                created_at: r.get("created_at"),
            })
        }
        _ => HttpResponse::Ok().json(serde_json::json!({}))
    }
}

pub async fn upsert_ai_analytics(req: HttpRequest, body: web::Json<AiAnalyticsUpsert>, state: web::Data<AppState>) -> HttpResponse {
    let data = body.into_inner();
    let pool = &state.pool;
    let loc = i18n::detect_locale(&req);
    let locale = match loc { i18n::Locale::Ru => "ru", _ => "en" };
    
    // Ensure at least 5 data points
    if data.level_of_competitiveness.len() < 5 {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "level_of_competitiveness must have at least 5 data points"
        }));
    }
    
    let competitiveness_json = serde_json::to_string(&data.level_of_competitiveness)
        .unwrap_or_else(|_| "[]".to_string());
    
    let id = Uuid::new_v4().to_string();
    
    let result = sqlx::query(
        "INSERT INTO ai_analytics (id, increase, description, level_of_competitiveness) VALUES (?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(data.increase)
    .bind(&data.description)
    .bind(&competitiveness_json)
    .execute(pool)
    .await;
    
    match result {
        Ok(_) => {
            // Insert i18n for AI analytics
            let _ = sqlx::query(
                "INSERT INTO ai_analytics_i18n (id, locale, description) VALUES (?, ?, ?) ON CONFLICT(id, locale) DO UPDATE SET description = excluded.description"
            )
            .bind(&id)
            .bind(locale)
            .bind(&data.description)
            .execute(pool)
            .await;
            
            HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
        }
        Err(_) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Failed to save AI analytics"
        }))
    }
}

pub async fn get_niches_month(req: HttpRequest, state: web::Data<AppState>) -> HttpResponse {
    let pool = &state.pool;
    let loc = i18n::detect_locale(&req);
    let locale = match loc { i18n::Locale::Ru => "ru", _ => "en" };
    
    // Get current month start (first day of current month)
    let now = chrono::Utc::now();
    let today = now.date_naive();
    let formatted = today.format("%Y-%m-%d").to_string();
    let month_start_str = format!("{}-01", &formatted[..7]); // Extract YYYY-MM and append -01
    
    let rows = sqlx::query(
        "SELECT n.title, n.change, n.id,
                COALESCE(i.title, n.title) AS localized_title
         FROM niches_month n
         LEFT JOIN niches_month_i18n i
           ON i.id = n.id AND i.locale = ?
         WHERE n.month_start = ? ORDER BY ABS(n.change) DESC"
    )
    .bind(locale)
    .bind(&month_start_str)
    .fetch_all(pool)
    .await;
    
    match rows {
        Ok(rs) => {
            let niches: Vec<NicheItem> = rs.into_iter().map(|r| NicheItem {
                title: r.get::<String, _>("localized_title"),
                change: r.get("change"),
            }).collect();
            
            HttpResponse::Ok().json(NichesMonthResponse {
                niches,
                month_start: month_start_str,
            })
        }
        _ => HttpResponse::Ok().json(NichesMonthResponse {
            niches: vec![],
            month_start: month_start_str,
        })
    }
}

pub async fn upsert_niches_month(req: HttpRequest, body: web::Json<NichesMonthUpsert>, state: web::Data<AppState>) -> HttpResponse {
    let data = body.into_inner();
    let pool = &state.pool;
    let loc = i18n::detect_locale(&req);
    let locale = match loc { i18n::Locale::Ru => "ru", _ => "en" };
    
    // Get current month start (first day of current month)
    let now = chrono::Utc::now();
    let today = now.date_naive();
    let formatted = today.format("%Y-%m-%d").to_string();
    let month_start_str = format!("{}-01", &formatted[..7]); // Extract YYYY-MM and append -01
    
    // Delete existing entries for this month (i18n will be deleted via CASCADE)
    let _ = sqlx::query("DELETE FROM niches_month WHERE month_start = ?")
        .bind(&month_start_str)
        .execute(pool)
        .await;
    
    // Insert new niches
    for niche in data.niches {
        let id = Uuid::new_v4().to_string();
        let _ = sqlx::query(
            "INSERT INTO niches_month (id, month_start, title, change) VALUES (?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(&month_start_str)
        .bind(&niche.title)
        .bind(niche.change)
        .execute(pool)
        .await;
        
        // Insert i18n for niche
        let _ = sqlx::query(
            "INSERT INTO niches_month_i18n (id, locale, title) VALUES (?, ?, ?) ON CONFLICT(id, locale) DO UPDATE SET title = excluded.title"
        )
        .bind(&id)
        .bind(locale)
        .bind(&niche.title)
        .execute(pool)
        .await;
    }
    
    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}

// Keep old endpoints for backward compatibility (can be removed later)
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
    pub direction: String,
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
