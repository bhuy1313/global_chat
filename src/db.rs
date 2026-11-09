use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use tracing::info;
use crate::models::ServerMessage;

pub async fn init_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(database_url)
        .await?;

    // Create table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS messages (
            id VARCHAR(64) PRIMARY KEY,
            nickname VARCHAR(64) NOT NULL,
            content TEXT NOT NULL,
            timestamp VARCHAR(32) NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(&pool)
    .await?;

    // Create index
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_messages_created_at ON messages(created_at DESC)
        "#,
    )
    .execute(&pool)
    .await?;

    info!("✅ Kết nối và cấu hình cơ sở dữ liệu PostgreSQL thành công!");
    Ok(pool)
}

pub async fn save_message(
    pool: &PgPool,
    id: &str,
    nickname: &str,
    content: &str,
    timestamp: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO messages (id, nickname, content, timestamp)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(id)
    .bind(nickname)
    .bind(content)
    .bind(timestamp)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn load_recent_messages(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<ServerMessage>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id, nickname, content, timestamp
        FROM (
            SELECT id, nickname, content, timestamp, created_at
            FROM messages
            ORDER BY created_at DESC
            LIMIT $1
        ) sub
        ORDER BY created_at ASC
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let mut messages = Vec::with_capacity(rows.len());
    for row in rows {
        let id: String = row.try_get("id")?;
        let nickname: String = row.try_get("nickname")?;
        let content: String = row.try_get("content")?;
        let timestamp: String = row.try_get("timestamp")?;

        messages.push(ServerMessage::Chat {
            id,
            nickname,
            content,
            timestamp,
        });
    }

    Ok(messages)
}
