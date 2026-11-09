use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use sqlx::PgPool;
use tokio::sync::{broadcast, RwLock};
use tracing::warn;

use crate::models::ServerMessage;

pub struct AppState {
    pub tx: broadcast::Sender<ServerMessage>,
    pub history: Arc<RwLock<VecDeque<ServerMessage>>>,
    pub user_count: Arc<AtomicUsize>,
    pub capacity: usize,
    pub db_pool: Option<PgPool>,
}

impl AppState {
    pub fn new(capacity: usize, db_pool: Option<PgPool>) -> Self {
        let (tx, _rx) = broadcast::channel(256);
        Self {
            tx,
            history: Arc::new(RwLock::new(VecDeque::with_capacity(capacity))),
            user_count: Arc::new(AtomicUsize::new(0)),
            capacity,
            db_pool,
        }
    }

    pub async fn preload_history(&self, messages: Vec<ServerMessage>) {
        let mut hist = self.history.write().await;
        hist.clear();
        for msg in messages {
            if hist.len() >= self.capacity {
                hist.pop_front();
            }
            hist.push_back(msg);
        }
    }

    pub async fn add_to_history(&self, message: ServerMessage) {
        // Asynchronously persist chat messages to PostgreSQL if DB pool is present
        if let ServerMessage::Chat {
            ref id,
            ref nickname,
            ref content,
            ref timestamp,
        } = message
        {
            if let Some(ref pool) = self.db_pool {
                let pool = pool.clone();
                let id = id.clone();
                let nickname = nickname.clone();
                let content = content.clone();
                let timestamp = timestamp.clone();

                tokio::spawn(async move {
                    if let Err(e) =
                        crate::db::save_message(&pool, &id, &nickname, &content, &timestamp).await
                    {
                        warn!("Không thể ghi tin nhắn vào PostgreSQL: {e}");
                    }
                });
            }
        }

        // Always store in-memory for instant WebSocket synchronization
        let mut hist = self.history.write().await;
        if hist.len() >= self.capacity {
            hist.pop_front();
        }
        hist.push_back(message);
    }

    pub async fn get_history(&self) -> Vec<ServerMessage> {
        let hist = self.history.read().await;
        hist.iter().cloned().collect()
    }

    pub fn broadcast(&self, message: ServerMessage) {
        let _ = self.tx.send(message);
    }

    pub fn increment_user(&self) -> usize {
        self.user_count.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn decrement_user(&self) -> usize {
        let mut current = self.user_count.load(Ordering::SeqCst);
        loop {
            if current == 0 {
                return 0;
            }
            match self.user_count.compare_exchange_weak(
                current,
                current - 1,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => return current - 1,
                Err(actual) => current = actual,
            }
        }
    }

    pub fn get_user_count(&self) -> usize {
        self.user_count.load(Ordering::SeqCst)
    }
}
