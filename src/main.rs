mod db;
mod models;
mod state;
mod ws;

use std::sync::Arc;
use axum::{extract::State, routing::get, Json, Router};
use state::AppState;
use tower_http::{cors::CorsLayer, services::ServeDir, trace::TraceLayer};
use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

async fn health_check(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let db_status = if state.db_pool.is_some() {
        "connected (PostgreSQL)"
    } else {
        "in-memory fallback"
    };

    Json(serde_json::json!({
        "status": "ok",
        "service": "Rust Global Chat",
        "online_users": state.get_user_count(),
        "database": db_status
    }))
}

#[tokio::main]
async fn main() {
    // Load .env if present
    dotenvy::dotenv().ok();

    // Initialize tracing logger
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "global_chat=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Check for PostgreSQL DATABASE_URL
    let mut db_pool = None;
    let mut initial_messages = Vec::new();

    if let Ok(database_url) = std::env::var("DATABASE_URL") {
        if !database_url.trim().is_empty() {
            info!("Đang kết nối tới PostgreSQL...");
            match db::init_pool(&database_url).await {
                Ok(pool) => {
                    match db::load_recent_messages(&pool, 100).await {
                        Ok(history) => {
                            info!("Đã nạp {} tin nhắn lịch sử từ PostgreSQL.", history.len());
                            initial_messages = history;
                        }
                        Err(e) => {
                            warn!("Không thể đọc lịch sử tin nhắn: {e}");
                        }
                    }
                    db_pool = Some(pool);
                }
                Err(err) => {
                    warn!("Không thể kết nối DATABASE_URL ({err}). Chuyển sang chế độ In-Memory.");
                }
            }
        }
    } else {
        info!("Không có biến môi trường DATABASE_URL. Máy chủ hoạt động ở chế độ In-Memory.");
    }

    // Initialize application state (capacity = 100 messages)
    let state = Arc::new(AppState::new(100, db_pool));
    if !initial_messages.is_empty() {
        state.preload_history(initial_messages).await;
    }

    // Build Axum router
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/ws", get(ws::ws_handler))
        .fallback_service(ServeDir::new("static"))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Read port from environment (Render/Koyeb usually set PORT)
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let bind_addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .unwrap_or_else(|e| panic!("Không thể lắng nghe cổng {bind_addr}: {e}"));

    info!("========================================================");
    info!("🦀 Rust Global Chat Server đang chạy!");
    info!("👉 Truy cập trình duyệt: http://127.0.0.1:{port}");
    info!("👉 WebSocket endpoint:  ws://127.0.0.1:{port}/ws");
    info!("👉 Health check API:     http://127.0.0.1:{port}/health");
    info!("========================================================");

    if let Err(err) = axum::serve(listener, app).await {
        tracing::error!("Lỗi máy chủ Axum: {err}");
    }
}
