use sqlx::{sqlite::{SqlitePoolOptions, SqliteConnectOptions}, SqlitePool};
use std::str::FromStr;

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

    Ok(pool)
}
