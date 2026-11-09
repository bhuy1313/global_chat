use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use sqlx::PgPool;
use tokio::sync::{broadcast, RwLock};
use tracing::warn;

use crate::models::ServerMessage;

const RESERVED_NAMES: &[&str] = &[
    "admin",
    "administrator",
    "system",
    "server",
    "mod",
    "moderator",
    "root",
    "bot",
    "support",
    "everyone",
    "here",
    "all",
];

pub struct AppState {
    pub tx: broadcast::Sender<ServerMessage>,
    pub rooms_history: Arc<RwLock<HashMap<String, VecDeque<ServerMessage>>>>,
    pub user_count: Arc<AtomicUsize>,
    pub capacity: usize,
    pub db_pool: Option<PgPool>,
    pub active_nicknames: Arc<RwLock<HashSet<String>>>,
}

impl AppState {
    pub fn new(capacity: usize, db_pool: Option<PgPool>) -> Self {
        let (tx, _rx) = broadcast::channel(256);
        Self {
            tx,
            rooms_history: Arc::new(RwLock::new(HashMap::new())),
            user_count: Arc::new(AtomicUsize::new(0)),
            capacity,
            db_pool,
            active_nicknames: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Lấy lịch sử tin nhắn của một phòng cụ thể (kết hợp cache RAM và Postgres)
    pub async fn get_room_history(&self, room: &str) -> Vec<ServerMessage> {
        {
            let rooms = self.rooms_history.read().await;
            if let Some(deque) = rooms.get(room) {
                if !deque.is_empty() {
                    return deque.iter().cloned().collect();
                }
            }
        }

        // Nếu trong RAM chưa có, tải từ PostgreSQL
        if let Some(ref pool) = self.db_pool {
            if let Ok(msgs) = crate::db::load_recent_messages(pool, room, self.capacity as i64).await {
                let mut rooms = self.rooms_history.write().await;
                let deque = rooms
                    .entry(room.to_string())
                    .or_insert_with(|| VecDeque::with_capacity(self.capacity));
                deque.clear();
                for m in &msgs {
                    deque.push_back(m.clone());
                }
                return msgs;
            }
        }

        Vec::new()
    }

    pub async fn add_to_history(&self, message: ServerMessage) {
        let room_name = message.room().unwrap_or("general").to_string();

        if let ServerMessage::Chat {
            ref id,
            ref nickname,
            ref content,
            ref timestamp,
            ref room,
        } = message
        {
            if let Some(ref pool) = self.db_pool {
                let pool = pool.clone();
                let id = id.clone();
                let nickname = nickname.clone();
                let content = content.clone();
                let timestamp = timestamp.clone();
                let room = room.clone();

                tokio::spawn(async move {
                    if let Err(e) =
                        crate::db::save_message(&pool, &id, &nickname, &content, &timestamp, &room).await
                    {
                        warn!("Không thể ghi tin nhắn vào PostgreSQL: {e}");
                    }
                });
            }
        }

        let mut rooms = self.rooms_history.write().await;
        let deque = rooms
            .entry(room_name)
            .or_insert_with(|| VecDeque::with_capacity(self.capacity));
        if deque.len() >= self.capacity {
            deque.pop_front();
        }
        deque.push_back(message);
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

    /// Đăng ký và bảo vệ biệt danh
    pub async fn try_claim_nickname(
        &self,
        old_name: Option<&str>,
        proposed: &str,
    ) -> Result<String, &'static str> {
        let clean: String = proposed.trim().chars().take(24).collect();
        if clean.len() < 2 {
            return Err("Biệt danh quá ngắn (tối thiểu 2 ký tự).");
        }
        if clean.chars().any(|c| c.is_control()) {
            return Err("Biệt danh chứa ký tự điều khiển không hợp lệ.");
        }

        let lower = clean.to_lowercase();
        for &reserved in RESERVED_NAMES {
            if lower == reserved
                || lower.starts_with(&format!("{reserved}_"))
                || lower.starts_with(&format!("{reserved}-"))
                || lower.starts_with(&format!("{reserved} "))
            {
                return Err("Biệt danh này đã được bảo lưu bởi hệ thống, vui lòng chọn tên khác.");
            }
        }

        let mut set = self.active_nicknames.write().await;
        if let Some(old) = old_name {
            if old == clean {
                return Ok(clean);
            }
        }

        if set.contains(&clean) {
            return Err("Biệt danh này hiện đang có người sử dụng trong phòng.");
        }

        if let Some(old) = old_name {
            set.remove(old);
        }
        set.insert(clean.clone());
        Ok(clean)
    }

    pub async fn release_nickname(&self, name: &str) {
        let mut set = self.active_nicknames.write().await;
        set.remove(name);
    }
}
