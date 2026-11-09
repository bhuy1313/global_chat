use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use tracing::info;
use crate::models::ServerMessage;

pub async fn init_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(database_url)
        .await?;

    // Create table if not exists
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

    // Add room column if not present (backward compatibility & migration)
    sqlx::query(
        r#"
        ALTER TABLE messages ADD COLUMN IF NOT EXISTS room VARCHAR(32) NOT NULL DEFAULT 'general'
        "#,
    )
    .execute(&pool)
    .await?;

    // Create indexes
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_messages_created_at ON messages(created_at DESC)
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_messages_room_created_at ON messages(room, created_at DESC)
        "#,
    )
    .execute(&pool)
    .await?;

    info!("✅ Kết nối và cấu hình cơ sở dữ liệu PostgreSQL (hỗ trợ đa phòng) thành công!");
    Ok(pool)
}

pub async fn save_message(
    pool: &PgPool,
    id: &str,
    nickname: &str,
    content: &str,
    timestamp: &str,
    room: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO messages (id, nickname, content, timestamp, room)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(id)
    .bind(nickname)
    .bind(content)
    .bind(timestamp)
    .bind(room)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn load_recent_messages(
    pool: &PgPool,
    room: &str,
    limit: i64,
) -> Result<Vec<ServerMessage>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id, nickname, content, timestamp, room
        FROM (
            SELECT id, nickname, content, timestamp, room, created_at
            FROM messages
            WHERE room = $1
            ORDER BY created_at DESC
            LIMIT $2
        ) sub
        ORDER BY created_at ASC
        "#,
    )
    .bind(room)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let mut messages = Vec::with_capacity(rows.len());
    for row in rows {
        let id: String = row.try_get("id")?;
        let nickname: String = row.try_get("nickname")?;
        let content: String = row.try_get("content")?;
        let timestamp: String = row.try_get("timestamp")?;
        let room_val: String = row.try_get("room")?;

        messages.push(ServerMessage::Chat {
            id,
            nickname,
            content,
            timestamp,
            room: room_val,
        });
    }

    Ok(messages)
}
