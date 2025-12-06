use sqlx::{sqlite::{SqlitePoolOptions, SqliteConnectOptions}, SqlitePool};
use std::str::FromStr;
use uuid::Uuid;

async fn seed_analytics_data(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    // Calculate week start (Monday of current week) - same logic as handlers
    let now = chrono::Utc::now();
    let week_start = now.date_naive().week(chrono::Weekday::Mon).first_day();
    let week_start_str = week_start.format("%Y-%m-%d").to_string();
    
    // Calculate month start (1st day of current month) - same logic as handlers
    let today = now.date_naive();
    let formatted = today.format("%Y-%m-%d").to_string();
    let month_start_str = format!("{}-01", &formatted[..7]);
    
    // Check if weekly trends already exist for this week
    let existing_weekly: Option<i64> = sqlx::query_scalar(
        "SELECT COUNT(*) FROM top_weekly_trends WHERE week_start = ?"
    )
    .bind(&week_start_str)
    .fetch_optional(pool)
    .await?
    .and_then(|count: i64| if count > 0 { Some(count) } else { None });
    
    if existing_weekly.is_none() {
        // Insert current top trend (position 1)
        let top_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO top_weekly_trends (id, week_start, position, title, increase, request_percent) VALUES (?, ?, 1, ?, ?, ?)"
        )
        .bind(&top_id)
        .bind(&week_start_str)
        .bind("Gaming laptops")
        .bind(92.0)
        .bind(Some(18.0))
        .execute(pool)
        .await?;
        
        // Insert i18n for top trend
        sqlx::query(
            "INSERT INTO top_weekly_trends_i18n (id, locale, title) VALUES (?, 'en', ?)"
        )
        .bind(&top_id)
        .bind("Gaming laptops")
        .execute(pool)
        .await?;
        sqlx::query(
            "INSERT INTO top_weekly_trends_i18n (id, locale, title) VALUES (?, 'ru', ?)"
        )
        .bind(&top_id)
        .bind("Игровые ноутбуки")
        .execute(pool)
        .await?;
        
        // Insert second place (position 2)
        let second_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO top_weekly_trends (id, week_start, position, title, increase, request_percent) VALUES (?, ?, 2, ?, ?, ?)"
        )
        .bind(&second_id)
        .bind(&week_start_str)
        .bind("Online education")
        .bind(76.0)
        .bind(None::<f64>)
        .execute(pool)
        .await?;
        
        // Insert i18n for second place
        sqlx::query(
            "INSERT INTO top_weekly_trends_i18n (id, locale, title) VALUES (?, 'en', ?)"
        )
        .bind(&second_id)
        .bind("Online education")
        .execute(pool)
        .await?;
        sqlx::query(
            "INSERT INTO top_weekly_trends_i18n (id, locale, title) VALUES (?, 'ru', ?)"
        )
        .bind(&second_id)
        .bind("Онлайн образование")
        .execute(pool)
        .await?;
        
        // Insert geo trends (top 3)
        let geo_countries = vec![
            ("Belgium", "Бельгия", 54.0, 1), 
            ("Netherlands", "Нидерланды", 48.0, 2), 
            ("Germany", "Германия", 42.0, 3)
        ];
        for (country_en, country_ru, increase, rank) in geo_countries {
            let geo_id = Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO geo_trends (id, week_start, country, increase, rank) VALUES (?, ?, ?, ?, ?)"
            )
            .bind(&geo_id)
            .bind(&week_start_str)
            .bind(country_en)
            .bind(increase)
            .bind(rank as i64)
            .execute(pool)
            .await?;
            
            // Insert i18n for geo trend
            sqlx::query(
                "INSERT INTO geo_trends_i18n (id, locale, country) VALUES (?, 'en', ?)"
            )
            .bind(&geo_id)
            .bind(country_en)
            .execute(pool)
            .await?;
            sqlx::query(
                "INSERT INTO geo_trends_i18n (id, locale, country) VALUES (?, 'ru', ?)"
            )
            .bind(&geo_id)
            .bind(country_ru)
            .execute(pool)
            .await?;
        }
    }
    
    // Check if AI analytics already exist
    let existing_ai: Option<i64> = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ai_analytics"
    )
    .fetch_optional(pool)
    .await?
    .and_then(|count: i64| if count > 0 { Some(count) } else { None });
    
    if existing_ai.is_none() {
        let ai_id = Uuid::new_v4().to_string();
        let competitiveness_json = serde_json::to_string(&vec![25.5, 30.2, 35.8, 28.4, 32.1, 40.0, 38.7])
            .unwrap_or_else(|_| "[]".to_string());
        
        let description_en = "Online education trend can be used to increase the brand as a source of benefit";
        let description_ru = "Тренд онлайн-образования можно использовать для повышения бренда как источника выгоды";
        
        sqlx::query(
            "INSERT INTO ai_analytics (id, increase, description, level_of_competitiveness) VALUES (?, ?, ?, ?)"
        )
        .bind(&ai_id)
        .bind(10.0)
        .bind(description_en)
        .bind(&competitiveness_json)
        .execute(pool)
        .await?;
        
        // Insert i18n for AI analytics
        sqlx::query(
            "INSERT INTO ai_analytics_i18n (id, locale, description) VALUES (?, 'en', ?)"
        )
        .bind(&ai_id)
        .bind(description_en)
        .execute(pool)
        .await?;
        sqlx::query(
            "INSERT INTO ai_analytics_i18n (id, locale, description) VALUES (?, 'ru', ?)"
        )
        .bind(&ai_id)
        .bind(description_ru)
        .execute(pool)
        .await?;
    }
    
    // Check if niches already exist for this month
    let existing_niches: Option<i64> = sqlx::query_scalar(
        "SELECT COUNT(*) FROM niches_month WHERE month_start = ?"
    )
    .bind(&month_start_str)
    .fetch_optional(pool)
    .await?
    .and_then(|count: i64| if count > 0 { Some(count) } else { None });
    
    if existing_niches.is_none() {
        let niches = vec![
            ("Beauty", "Красота", 34.0),
            ("Food Delivery", "Доставка еды", -6.0),
            ("Fitness", "Фитнес", 28.5),
            ("Travel", "Путешествия", -12.3),
            ("Technology", "Технологии", 45.2),
        ];
        
        for (title_en, title_ru, change) in niches {
            let niche_id = Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO niches_month (id, month_start, title, change) VALUES (?, ?, ?, ?)"
            )
            .bind(&niche_id)
            .bind(&month_start_str)
            .bind(title_en)
            .bind(change)
            .execute(pool)
            .await?;
            
            // Insert i18n for niche
            sqlx::query(
                "INSERT INTO niches_month_i18n (id, locale, title) VALUES (?, 'en', ?)"
            )
            .bind(&niche_id)
            .bind(title_en)
            .execute(pool)
            .await?;
            sqlx::query(
                "INSERT INTO niches_month_i18n (id, locale, title) VALUES (?, 'ru', ?)"
            )
            .bind(&niche_id)
            .bind(title_ru)
            .execute(pool)
            .await?;
        }
    }
    
    Ok(())
}

pub async fn init_pool(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    let connect_opts = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connect_opts)
        .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS conversations (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            title TEXT,
            created_at TEXT NOT NULL
        );
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS conversation_context (
            conversation_id TEXT PRIMARY KEY,
            user_role TEXT,
            business_stage TEXT,
            goal TEXT,
            urgency TEXT,
            region TEXT,
            business_niche TEXT,
            updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
            FOREIGN KEY(conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
        );
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS messages (
            id TEXT PRIMARY KEY,
            conversation_id TEXT NOT NULL,
            user_id TEXT,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            timestamp TEXT NOT NULL,
            FOREIGN KEY(conversation_id) REFERENCES conversations(id)
        );
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            email TEXT NOT NULL UNIQUE,
            password TEXT NOT NULL,
            business_type TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        "#,
    )
    .execute(&pool)
    .await?;

    let _ = sqlx::query("ALTER TABLE users ADD COLUMN full_name TEXT;")
        .execute(&pool)
        .await;
    let _ = sqlx::query("ALTER TABLE users ADD COLUMN nickname TEXT;")
        .execute(&pool)
        .await;
    let _ = sqlx::query("ALTER TABLE users ADD COLUMN phone TEXT;")
        .execute(&pool)
        .await;
    let _ = sqlx::query("ALTER TABLE users ADD COLUMN country TEXT;")
        .execute(&pool)
        .await;
    let _ = sqlx::query("ALTER TABLE users ADD COLUMN gender TEXT;")
        .execute(&pool)
        .await;
    let _ = sqlx::query("ALTER TABLE users ADD COLUMN profile_picture TEXT;")
        .execute(&pool)
        .await;
    let _ = sqlx::query("ALTER TABLE users ADD COLUMN telegram_username TEXT;")
        .execute(&pool)
        .await;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS sessions (
            token TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            created_at TEXT NOT NULL,
            expires_at TEXT,
            FOREIGN KEY(user_id) REFERENCES users(id)
        );
        "#,
    )
    .execute(&pool)
    .await?;

    // New analytics tables structure
    
    // Top weekly trends: stores current top trend, 2nd place, and geo trends
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS top_weekly_trends (
            id TEXT PRIMARY KEY,
            week_start TEXT NOT NULL,
            position INTEGER NOT NULL CHECK(position IN (1, 2)),
            title TEXT NOT NULL,
            increase REAL NOT NULL,
            request_percent REAL,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
            UNIQUE(week_start, position)
        );
        "#,
    )
    .execute(&pool)
    .await?;

    // Geo trends: top 3 regions per week
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS geo_trends (
            id TEXT PRIMARY KEY,
            week_start TEXT NOT NULL,
            country TEXT NOT NULL,
            increase REAL NOT NULL,
            rank INTEGER NOT NULL CHECK(rank IN (1, 2, 3)),
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
            UNIQUE(week_start, rank)
        );
        "#,
    )
    .execute(&pool)
    .await?;

    // AI analytics
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS ai_analytics (
            id TEXT PRIMARY KEY,
            increase REAL,
            description TEXT,
            level_of_competitiveness TEXT,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
        );
        "#,
    )
    .execute(&pool)
    .await?;

    // Niches of the month
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS niches_month (
            id TEXT PRIMARY KEY,
            month_start TEXT NOT NULL,
            title TEXT NOT NULL,
            change REAL NOT NULL,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
        );
        "#,
    )
    .execute(&pool)
    .await?;

    // i18n tables for new analytics endpoints
    
    // i18n for top_weekly_trends (localized title)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS top_weekly_trends_i18n (
            id TEXT NOT NULL,
            locale TEXT NOT NULL,
            title TEXT,
            PRIMARY KEY (id, locale),
            FOREIGN KEY(id) REFERENCES top_weekly_trends(id) ON DELETE CASCADE
        );
        "#,
    )
    .execute(&pool)
    .await?;

    // i18n for geo_trends (localized country name)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS geo_trends_i18n (
            id TEXT NOT NULL,
            locale TEXT NOT NULL,
            country TEXT,
            PRIMARY KEY (id, locale),
            FOREIGN KEY(id) REFERENCES geo_trends(id) ON DELETE CASCADE
        );
        "#,
    )
    .execute(&pool)
    .await?;

    // i18n for ai_analytics (localized description)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS ai_analytics_i18n (
            id TEXT NOT NULL,
            locale TEXT NOT NULL,
            description TEXT,
            PRIMARY KEY (id, locale),
            FOREIGN KEY(id) REFERENCES ai_analytics(id) ON DELETE CASCADE
        );
        "#,
    )
    .execute(&pool)
    .await?;

    // i18n for niches_month (localized title)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS niches_month_i18n (
            id TEXT NOT NULL,
            locale TEXT NOT NULL,
            title TEXT,
            PRIMARY KEY (id, locale),
            FOREIGN KEY(id) REFERENCES niches_month(id) ON DELETE CASCADE
        );
        "#,
    )
    .execute(&pool)
    .await?;

    // Seed preset data for new analytics endpoints using Rust date calculations
    seed_analytics_data(&pool).await?;

    // Keep old tables for backward compatibility (can be removed later if not needed)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS analytics_trends (
            name TEXT PRIMARY KEY,
            percent_change REAL,
            description TEXT,
            why_popular TEXT,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
        );
        "#,
    )
    .execute(&pool)
    .await?;

    // i18n table for localized text fields of analytics_trends
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS analytics_trends_i18n (
            name TEXT NOT NULL,
            locale TEXT NOT NULL,
            description TEXT,
            why_popular TEXT,
            PRIMARY KEY (name, locale),
            FOREIGN KEY(name) REFERENCES analytics_trends(name)
        );
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        INSERT OR IGNORE INTO analytics_trends (name, percent_change, description, why_popular)
        VALUES (
            'онлайн образование',
            18.5,
            'Лидирующий тренд, отражающий рост дистанционных образовательных платформ и цифровых курсов.',
            'Онлайн‑образование стало популярным благодаря широкой доступности интернета, гибкому формату обучения в удобное время, более низкой стоимости по сравнению с офлайн‑вариантами и пандемии, которая нормализовала дистанционное повышение квалификации.'
        );
        "#,
    )
    .execute(&pool)
    .await?;

    // Seed EN localization row for the same trend
    sqlx::query(
        r#"
        INSERT OR IGNORE INTO analytics_trends_i18n (name, locale, description, why_popular)
        VALUES (
            'онлайн образование',
            'en',
            'Leading trend capturing growth in remote learning platforms and digital courses.',
            'Online education surged due to wider internet access, flexible self-paced formats, lower costs versus offline options, and the pandemic-driven shift to remote learning which normalized digital-first upskilling.'
        );
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS popularity_trends (
            name TEXT PRIMARY KEY,
            direction TEXT NOT NULL CHECK(direction IN ('growing','decreasing')),
            percent_change REAL,
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
        );
        "#,
    )
    .execute(&pool)
    .await?;

    // i18n table for localized notes of popularity_trends
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS popularity_trends_i18n (
            name TEXT NOT NULL,
            locale TEXT NOT NULL,
            notes TEXT,
            PRIMARY KEY (name, locale),
            FOREIGN KEY(name) REFERENCES popularity_trends(name)
        );
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        INSERT OR IGNORE INTO popularity_trends (name, direction, percent_change, notes) VALUES
            ('автосервис',     'growing',    4.2,  'Спрос из‑за старения автопарка и перехода от DIY к сервисам'),
            ('кофейни',        'growing',    3.5,  'Опытное потребление и роль локальных пространств для общения'),
            ('маркетплейсы',   'growing',    6.8,  'Переход к омниканальности, рост продавцов long‑tail и эффект агрегаторов'),
            ('бьюти',          'decreasing', -2.1, 'Нормализация постпандемийного периода и перераспределение бюджета');
        "#,
    )
    .execute(&pool)
    .await?;

    // Seed EN localizations for popularity notes
    sqlx::query(
        r#"
        INSERT OR IGNORE INTO popularity_trends_i18n (name, locale, notes) VALUES
            ('автосервис',   'en', 'Demand from aging car fleets and shifts from DIY to professional service'),
            ('кофейни',      'en', 'Experience-driven consumption and local community spaces'),
            ('маркетплейсы', 'en', 'Shift to omnichannel, long-tail sellers, and aggregation effects'),
            ('бьюти',        'en', 'Post-pandemic normalization and budget reprioritization');
        "#,
    )
    .execute(&pool)
    .await?;

    // Files storage for generated attachments
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS files (
            id TEXT PRIMARY KEY,
            filename TEXT NOT NULL,
            mime TEXT NOT NULL,
            size INTEGER NOT NULL,
            bytes BLOB NOT NULL,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
        );
        "#,
    )
    .execute(&pool)
    .await?;

    // Add optional message_id column to link files with messages (if not present)
    let _ = sqlx::query("ALTER TABLE files ADD COLUMN message_id TEXT;")
        .execute(&pool)
        .await;

    // Support chat tables
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS support_messages (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            message TEXT NOT NULL,
            photo_url TEXT,
            direction TEXT NOT NULL CHECK(direction IN ('user', 'support')),
            telegram_message_id INTEGER,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%S','now'))
        );
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS device_tokens (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            fcm_token TEXT NOT NULL,
            platform TEXT,
            device_id TEXT,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%S','now')),
            UNIQUE(user_id, fcm_token)
        );
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS message_mapping (
            id TEXT PRIMARY KEY,
            telegram_message_id INTEGER NOT NULL,
            user_id TEXT NOT NULL,
            support_message_id TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%S','now')),
            FOREIGN KEY(support_message_id) REFERENCES support_messages(id)
        );
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS greetings_sent (
            user_id TEXT PRIMARY KEY,
            date TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%S','now'))
        );
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS telegram_users (
            id TEXT PRIMARY KEY,
            telegram_user_id INTEGER NOT NULL UNIQUE,
            telegram_username TEXT,
            first_name TEXT,
            last_name TEXT,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
            user_id TEXT,
            FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE SET NULL
        );
        "#,
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}
