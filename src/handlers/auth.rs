use actix_web::{Error, HttpResponse, web};
use bcrypt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;
use sqlx::{self};
use sqlx::Row;

use crate::models::{AuthRequest, User};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct TokenCheck {
    pub token: Option<String>,
}

#[derive(Serialize)]
pub struct TokenStatus {
    pub valid: bool,
    pub message: &'static str,
}
#[derive(Deserialize)]
pub struct EmailCheckReq {
    pub email: String,
}

#[derive(Serialize)]
pub struct EmailCheckRes {
    pub exists: bool,
}

pub async fn email_exists(
    query: web::Query<EmailCheckReq>,
    state: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    let exists: bool = sqlx::query_scalar(
    "SELECT EXISTS(SELECT 1 FROM users WHERE email = ?)",
    )
    .bind(&query.email)
    .fetch_one(&state.pool)
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(EmailCheckRes { exists }))
}

pub async fn check_token(
    query: web::Query<TokenCheck>,
    state: web::Data<AppState>,
) -> HttpResponse {
    let status = match &query.token {
        None => TokenStatus {
            valid: false,
            message: "no-token",
        },
        Some(t) => {
            let now = chrono::Utc::now().to_rfc3339();
            let exists: Option<i64> = sqlx::query_scalar(
                "SELECT CASE WHEN EXISTS(\n                    SELECT 1 FROM sessions\n                    WHERE token = ? AND (expires_at IS NULL OR expires_at > ?)\n                ) THEN 1 ELSE 0 END"
            )
            .bind(t)
            .bind(&now)
            .fetch_optional(&state.pool)
            .await
            .ok()
            .flatten();

            match exists {
                Some(1) => TokenStatus { valid: true, message: "valid" },
                _ => TokenStatus { valid: false, message: "expired-or-invalid" },
            }
        }
    };

    HttpResponse::Ok().json(status)
}

pub async fn register(
    data: web::Json<AuthRequest>,
    state: web::Data<AppState>,
) -> HttpResponse {
    let auth_req = data.into_inner();
    let pool = &state.pool;

    // check existing user
    if let Ok(existing) = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(1) FROM users WHERE email = ?"
    )
    .bind(&auth_req.email)
    .fetch_one(pool)
    .await
    {
        if existing > 0 {
            return HttpResponse::BadRequest().json(json!({
                "error": "User already exists"
            }));
        }
    }
    
    let hashed_password = match bcrypt::hash(&auth_req.password, bcrypt::DEFAULT_COST) {
        Ok(hash) => hash,
        Err(_) => return HttpResponse::InternalServerError().json(json!({
            "error": "Password hashing failed"
        }))
    };
    
    let user = User {
        id: Uuid::new_v4().to_string(),
        email: auth_req.email.clone(),
        password: hashed_password,
        business_type: auth_req.business_type.unwrap_or_else(|| "general".to_string()),
        created_at: chrono::Utc::now().to_rfc3339(),
        full_name: auth_req.full_name.clone(),
        nickname: auth_req.nickname.clone(),
        phone: auth_req.phone.clone(),
        country: auth_req.country.clone(),
        gender: auth_req.gender.clone(),
    };

    if let Err(_) = sqlx::query(
        "INSERT INTO users (id, email, password, business_type, created_at, full_name, nickname, phone, country, gender) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&user.id)
    .bind(&user.email)
    .bind(&user.password)
    .bind(&user.business_type)
    .bind(&user.created_at)
    .bind(&user.full_name)
    .bind(&user.nickname)
    .bind(&user.phone)
    .bind(&user.country)
    .bind(&user.gender)
    .execute(pool)
    .await
    {
        return HttpResponse::InternalServerError().json(json!({"error": "Failed to create user"}));
    }

    // create session token
    let token = Uuid::new_v4().to_string();
    let created_at = chrono::Utc::now().to_rfc3339();
    let expires_at = chrono::Utc::now() + chrono::Duration::days(30);
    let expires_at_str = expires_at.to_rfc3339();

    let _ = sqlx::query(
        "INSERT INTO sessions (token, user_id, created_at, expires_at) VALUES (?, ?, ?, ?)"
    )
    .bind(&token)
    .bind(&user.id)
    .bind(&created_at)
    .bind(&expires_at_str)
    .execute(pool)
    .await;
    
    HttpResponse::Created().json(json!({
        "message": "User registered successfully",
        "user": {
            "id": user.id,
            "email": user.email,
            "business_type": user.business_type
        },
        "token": token
    }))
}

pub async fn login(
    data: web::Json<AuthRequest>,
    state: web::Data<AppState>,
) -> HttpResponse {
    let auth_req = data.into_inner();
    let pool = &state.pool;

    let row = sqlx::query(
        "SELECT id, email, password, business_type, created_at, full_name, nickname, phone, country, gender FROM users WHERE email = ? LIMIT 1"
    )
    .bind(&auth_req.email)
    .fetch_optional(pool)
    .await;

    let row = match row {
        Ok(Some(r)) => r,
        _ => {
            return HttpResponse::Unauthorized().json(json!({
                "error": "Invalid credentials"
            }));
        }
    };

    let user = User {
        id: row.get::<String, _>("id"),
        email: row.get::<String, _>("email"),
        password: row.get::<String, _>("password"),
        business_type: row.get::<String, _>("business_type"),
        created_at: row.get::<String, _>("created_at"),
        full_name: row.try_get::<Option<String>, _>("full_name").unwrap_or(None),
        nickname: row.try_get::<Option<String>, _>("nickname").unwrap_or(None),
        phone: row.try_get::<Option<String>, _>("phone").unwrap_or(None),
        country: row.try_get::<Option<String>, _>("country").unwrap_or(None),
        gender: row.try_get::<Option<String>, _>("gender").unwrap_or(None),
    };
    
    let is_valid = match bcrypt::verify(&auth_req.password, &user.password) {
        Ok(valid) => valid,
        Err(_) => false
    };
    
    if !is_valid {
        return HttpResponse::Unauthorized().json(json!({
            "error": "Invalid credentials"
        }));
    }

    // create session token
    let token = Uuid::new_v4().to_string();
    let created_at = chrono::Utc::now().to_rfc3339();
    let expires_at = chrono::Utc::now() + chrono::Duration::days(30);
    let expires_at_str = expires_at.to_rfc3339();

    let _ = sqlx::query(
        "INSERT INTO sessions (token, user_id, created_at, expires_at) VALUES (?, ?, ?, ?)"
    )
    .bind(&token)
    .bind(&user.id)
    .bind(&created_at)
    .bind(&expires_at_str)
    .execute(pool)
    .await;

    HttpResponse::Ok().json(json!({
        "message": "Login successful",
        "user": {
            "id": user.id,
            "email": user.email,
            "business_type": user.business_type
        },
        "token": token
    }))
}