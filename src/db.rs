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

    // Seed initial record for Online Education with an explanatory answer
    sqlx::query(
        r#"
        INSERT OR IGNORE INTO analytics_trends (name, percent_change, description, why_popular)
        VALUES (
            'online education',
            NULL,
            'Leading trend capturing growth in remote learning platforms and digital courses.',
            'Online education surged due to wider internet access, flexible self-paced formats, lower costs versus offline options, and the pandemic-driven shift to remote learning which normalized digital-first upskilling.'
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

    Ok(pool)
}
