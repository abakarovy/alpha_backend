use actix_web::{Error, HttpRequest, HttpResponse, web};
use bcrypt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;
use sqlx::{self};
use sqlx::Row;

use crate::models::{AuthRequest, User};
use crate::state::AppState;
use crate::i18n::{self, Locale};

#[derive(Deserialize)]
pub struct TokenCheck {
    pub token: Option<String>,
}

#[derive(Serialize)]
pub struct TokenStatus {
    pub valid: bool,
    pub message: &'static str,
}

#[derive(Serialize)]
pub struct UserProfile {
    pub id: String,
    pub email: String,
    pub business_type: String,
    pub created_at: String,
    pub full_name: Option<String>,
    pub nickname: Option<String>,
    pub phone: Option<String>,
    pub country: Option<String>,
    pub gender: Option<String>,
    pub profile_picture: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateUserData {
    pub business_type: Option<String>,
    pub full_name: Option<String>,
    pub nickname: Option<String>,
    pub phone: Option<String>,
    pub country: Option<String>,
    pub gender: Option<String>,
    pub profile_picture: Option<String>,
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
    _req: HttpRequest,
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
    _req: HttpRequest,
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
                "SELECT CASE WHEN EXISTS(\n                    SELECT 1 FROM sessions s\n                    JOIN users u ON s.user_id = u.id\n                    WHERE s.token = ? AND (s.expires_at IS NULL OR s.expires_at > ?)\n                ) THEN 1 ELSE 0 END"
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

pub async fn get_profile(
    req: HttpRequest,
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> HttpResponse {
    let user_id = path.into_inner();

    let row = sqlx::query(
        "SELECT id, email, business_type, created_at, full_name, nickname, phone, country, gender, profile_picture
         FROM users
         WHERE id = ?
         LIMIT 1",
    )
    .bind(&user_id)
    .fetch_optional(&state.pool)
    .await;

    let locale = i18n::detect_locale(&req);
    let row = match row {
        Ok(Some(r)) => r,
        _ => {
            let error_msg = match locale {
                Locale::Ru => "Пользователь не найден",
                Locale::En => "user-not-found",
            };
            return HttpResponse::NotFound().json(json!({
                "error": error_msg,
            }));
        }
    };

    let profile = UserProfile {
        id: row.get::<String, _>("id"),
        email: row.get::<String, _>("email"),
        business_type: row.get::<String, _>("business_type"),
        created_at: row.get::<String, _>("created_at"),
        full_name: row.try_get::<Option<String>, _>("full_name").unwrap_or(None),
        nickname: row.try_get::<Option<String>, _>("nickname").unwrap_or(None),
        phone: row.try_get::<Option<String>, _>("phone").unwrap_or(None),
        country: row.try_get::<Option<String>, _>("country").unwrap_or(None),
        gender: row.try_get::<Option<String>, _>("gender").unwrap_or(None),
        profile_picture: row.try_get::<Option<String>, _>("profile_picture").unwrap_or(None),
    };

    HttpResponse::Ok().json(profile)
}

pub async fn update_profile(
    req: HttpRequest,
    query: web::Query<TokenCheck>,
    state: web::Data<AppState>,
    data: web::Json<UpdateUserData>,
) -> HttpResponse {
    let locale = i18n::detect_locale(&req);
    let token = match &query.token {
        Some(t) if !t.is_empty() => t,
        _ => {
            let error_msg = match locale {
                Locale::Ru => "Токен не предоставлен",
                Locale::En => "no-token",
            };
            return HttpResponse::Unauthorized().json(json!({
                "error": error_msg,
            }));
        }
    };

    let now = chrono::Utc::now().to_rfc3339();

    let update = data.into_inner();
    
    let profile_picture_was_provided = update.profile_picture.is_some();
    let profile_picture_value: Option<&str> = update.profile_picture.as_ref()
        .and_then(|s| if s.is_empty() { None } else { Some(s.as_str()) });

    let result = sqlx::query(
        "UPDATE users SET
            business_type = COALESCE(?, business_type),
            full_name = COALESCE(?, full_name),
            nickname = COALESCE(?, nickname),
            phone = COALESCE(?, phone),
            country = COALESCE(?, country),
            gender = COALESCE(?, gender),
            profile_picture = CASE 
                WHEN ? = 0 THEN profile_picture
                ELSE ?
            END
         WHERE id = (
            SELECT user_id FROM sessions
            WHERE token = ? AND (expires_at IS NULL OR expires_at > ?)
         )",
    )
    .bind(update.business_type.as_deref())
    .bind(update.full_name.as_deref())
    .bind(update.nickname.as_deref())
    .bind(update.phone.as_deref())
    .bind(update.country.as_deref())
    .bind(update.gender.as_deref())
    .bind(if profile_picture_was_provided { 1 } else { 0 })
    .bind(profile_picture_value)
    .bind(token)
    .bind(&now)
    .execute(&state.pool)
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
            Locale::Ru => "Недействительный или истекший токен",
            Locale::En => "invalid-or-expired-token",
        };
        return HttpResponse::Unauthorized().json(json!({
            "error": error_msg,
        }));
    }

    let row = sqlx::query(
        "SELECT u.id, u.email, u.business_type, u.created_at, u.full_name, u.nickname, u.phone, u.country, u.gender, u.profile_picture
         FROM sessions s
         JOIN users u ON s.user_id = u.id
         WHERE s.token = ? AND (s.expires_at IS NULL OR s.expires_at > ?)
         LIMIT 1",
    )
    .bind(token)
    .bind(&now)
    .fetch_optional(&state.pool)
    .await;

    let row = match row {
        Ok(Some(r)) => r,
        _ => {
            let error_msg = match locale {
                Locale::Ru => "Ошибка перезагрузки профиля",
                Locale::En => "reload-failed",
            };
            return HttpResponse::InternalServerError().json(json!({
                "error": error_msg,
            }));
        }
    };

    let profile = UserProfile {
        id: row.get::<String, _>("id"),
        email: row.get::<String, _>("email"),
        business_type: row.get::<String, _>("business_type"),
        created_at: row.get::<String, _>("created_at"),
        full_name: row.try_get::<Option<String>, _>("full_name").unwrap_or(None),
        nickname: row.try_get::<Option<String>, _>("nickname").unwrap_or(None),
        phone: row.try_get::<Option<String>, _>("phone").unwrap_or(None),
        country: row.try_get::<Option<String>, _>("country").unwrap_or(None),
        gender: row.try_get::<Option<String>, _>("gender").unwrap_or(None),
        profile_picture: row.try_get::<Option<String>, _>("profile_picture").unwrap_or(None),
    };

    HttpResponse::Ok().json(profile)
}

pub async fn register(
    req: HttpRequest,
    data: web::Json<AuthRequest>,
    state: web::Data<AppState>,
) -> HttpResponse {
    let auth_req = data.into_inner();
    let pool = &state.pool;
    let locale = i18n::detect_locale(&req);

    // check existing user
    if let Ok(existing) = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(1) FROM users WHERE email = ?"
    )
    .bind(&auth_req.email)
    .fetch_one(pool)
    .await
    {
        if existing > 0 {
            let error_msg = match locale {
                Locale::Ru => "Пользователь уже существует",
                Locale::En => "User already exists",
            };
            return HttpResponse::BadRequest().json(json!({
                "error": error_msg
            }));
        }
    }
    
    let hashed_password = match bcrypt::hash(&auth_req.password, bcrypt::DEFAULT_COST) {
        Ok(hash) => hash,
        Err(_) => {
            let error_msg = match locale {
                Locale::Ru => "Ошибка хеширования пароля",
                Locale::En => "Password hashing failed",
            };
            return HttpResponse::InternalServerError().json(json!({
                "error": error_msg
            }));
        }
    };
    
    // Normalize empty profile_picture strings to None (NULL in DB)
    let profile_picture_value = auth_req.profile_picture.as_ref()
        .and_then(|s| if s.is_empty() { None } else { Some(s.as_str()) });

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
        profile_picture: profile_picture_value.map(|s| s.to_string()),
    };

    if let Err(_) = sqlx::query(
        "INSERT INTO users (id, email, password, business_type, created_at, full_name, nickname, phone, country, gender, profile_picture) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
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
    .bind(&user.profile_picture)
    .execute(pool)
    .await
    {
        let error_msg = match locale {
            Locale::Ru => "Не удалось создать пользователя",
            Locale::En => "Failed to create user",
        };
        return HttpResponse::InternalServerError().json(json!({"error": error_msg}));
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
    
    let success_msg = match locale {
        Locale::Ru => "Пользователь успешно зарегистрирован",
        Locale::En => "User registered successfully",
    };
    HttpResponse::Created().json(json!({
        "message": success_msg,
        "user": {
            "id": user.id,
            "email": user.email,
            "business_type": user.business_type
        },
        "token": token
    }))
}

pub async fn login(
    req: HttpRequest,
    data: web::Json<AuthRequest>,
    state: web::Data<AppState>,
) -> HttpResponse {
    let auth_req = data.into_inner();
    let pool = &state.pool;
    let locale = i18n::detect_locale(&req);

    let row = sqlx::query(
        "SELECT id, email, password, business_type, created_at, full_name, nickname, phone, country, gender, profile_picture FROM users WHERE email = ? LIMIT 1"
    )
    .bind(&auth_req.email)
    .fetch_optional(pool)
    .await;

    let row = match row {
        Ok(Some(r)) => r,
        _ => {
            let error_msg = match locale {
                Locale::Ru => "Неверные учетные данные",
                Locale::En => "Invalid credentials",
            };
            return HttpResponse::Unauthorized().json(json!({
                "error": error_msg
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
        profile_picture: row.try_get::<Option<String>, _>("profile_picture").unwrap_or(None),
    };
    
    let is_valid = match bcrypt::verify(&auth_req.password, &user.password) {
        Ok(valid) => valid,
        Err(_) => false
    };
    
    if !is_valid {
        let error_msg = match locale {
            Locale::Ru => "Неверные учетные данные",
            Locale::En => "Invalid credentials",
        };
        return HttpResponse::Unauthorized().json(json!({
            "error": error_msg
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

    let success_msg = match locale {
        Locale::Ru => "Вход выполнен успешно",
        Locale::En => "Login successful",
    };
    HttpResponse::Ok().json(json!({
        "message": success_msg,
        "user": {
            "id": user.id,
            "email": user.email,
            "business_type": user.business_type
        },
        "token": token
    }))
}