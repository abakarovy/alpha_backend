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

    // Conversations and messages for persistent chat history
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

    // Initialize schema
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

    // Extend users table with optional profile fields (idempotent: ignore errors if columns exist)
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

    // Table for leading trend analytics, including percent change and an explanation
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

    // Seed initial record for Online Education (in Russian) with an explanatory answer
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

    // Table for growing/decreasing popularity across various trends (e.g., autoservice, coffee shops)
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

    // Seed preset popularity trends (in Russian)
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

    Ok(pool)
}
