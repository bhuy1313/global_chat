use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use chrono::Local;
use futures_util::{SinkExt, StreamExt};
use tracing::{info, warn};

use crate::{
    models::{ClientMessage, ServerMessage},
    state::AppState,
};

// Cấu hình Rate Limiting & Heartbeat
const RATE_LIMIT_MAX_MESSAGES: usize = 5;
const RATE_LIMIT_WINDOW_SECS: u64 = 3;
const COOLDOWN_DURATION_SECS: u64 = 5;
const MAX_PAYLOAD_BYTES: usize = 2048; // Tăng lên 2KB để hỗ trợ link ảnh/GIF dài
const HEARTBEAT_INTERVAL_SECS: u64 = 25;
const HEARTBEAT_TIMEOUT_SECS: u64 = 65;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    // 1. Tạo và đăng ký biệt danh Guest an toàn
    let guest_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        % 10000;

    let candidate = format!("Guest_{guest_id:04}");
    let mut nickname = match state.try_claim_nickname(None, &candidate).await {
        Ok(name) => name,
        Err(_) => {
            let alt = format!("Guest_{guest_id:04}_{}", guest_id % 97);
            state.try_claim_nickname(None, &alt).await.unwrap_or(alt)
        }
    };

    let mut current_room = "general".to_string();

    info!("New client connected: {nickname} in #{current_room}");

    let mut rx = state.tx.subscribe();
    let current_users = state.increment_user();

    let (mut sender, mut receiver) = socket.split();

    // 2. Gửi thông tin khởi tạo tới riêng client này
    let init_assigned = ServerMessage::NicknameAssigned {
        nickname: nickname.clone(),
    };
    if let Ok(json) = serde_json::to_string(&init_assigned) {
        let _ = sender.send(Message::Text(json.into())).await;
    }

    let init_room = ServerMessage::RoomJoined {
        room: current_room.clone(),
    };
    if let Ok(json) = serde_json::to_string(&init_room) {
        let _ = sender.send(Message::Text(json.into())).await;
    }

    let init_count = ServerMessage::UserCount {
        count: current_users,
    };
    if let Ok(json) = serde_json::to_string(&init_count) {
        let _ = sender.send(Message::Text(json.into())).await;
    }

    let history_msgs = state.get_room_history(&current_room).await;
    let init_history = ServerMessage::History {
        messages: history_msgs,
        room: current_room.clone(),
    };
    if let Ok(json) = serde_json::to_string(&init_history) {
        let _ = sender.send(Message::Text(json.into())).await;
    }

    // 3. Thông báo cho phòng hiện tại
    let now_str = Local::now().format("%H:%M").to_string();
    let join_event = ServerMessage::System {
        content: format!("{nickname} đã tham gia phòng #{current_room}"),
        timestamp: now_str,
        room: Some(current_room.clone()),
    };
    state.add_to_history(join_event.clone()).await;
    state.broadcast(join_event);
    state.broadcast(ServerMessage::UserCount {
        count: current_users,
    });

    // 4. Rate Limiting & Heartbeat tracking
    let mut msg_timestamps = std::collections::VecDeque::new();
    let mut cooldown_until: Option<Instant> = None;
    let mut last_activity = Instant::now();
    let mut heartbeat_timer = tokio::time::interval(Duration::from_secs(HEARTBEAT_INTERVAL_SECS));

    // 5. Vòng lặp sự kiện bất đồng bộ
    loop {
        tokio::select! {
            // Heartbeat check & Zombie socket cleanup
            _ = heartbeat_timer.tick() => {
                if last_activity.elapsed() > Duration::from_secs(HEARTBEAT_TIMEOUT_SECS) {
                    warn!("Client {nickname} timeout heartbeat, đóng socket.");
                    break;
                }
                if sender.send(Message::Ping(vec![].into())).await.is_err() {
                    break;
                }
            }

            // Nhận tin broadcast từ server
            broadcast_msg = rx.recv() => {
                match broadcast_msg {
                    Ok(msg) => {
                        // Lọc theo phòng: Chỉ gửi nếu tin nhắn thuộc cùng phòng hoặc là tin toàn cục
                        let should_send = match msg.room() {
                            Some(r) => r == current_room,
                            None => true,
                        };

                        if should_send {
                            if let Ok(json) = serde_json::to_string(&msg) {
                                if sender.send(Message::Text(json.into())).await.is_err() {
                                    break;
                                }
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                        warn!("Client {nickname} lagged by {skipped} messages");
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }

            // Nhận tin từ WebSocket client gửi lên
            incoming = receiver.next() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        last_activity = Instant::now();

                        if text.len() > MAX_PAYLOAD_BYTES {
                            let warn_msg = ServerMessage::System {
                                content: "⚠️ Tin nhắn quá dài vượt quá giới hạn cho phép.".to_string(),
                                timestamp: Local::now().format("%H:%M").to_string(),
                                room: Some(current_room.clone()),
                            };
                            if let Ok(json) = serde_json::to_string(&warn_msg) {
                                let _ = sender.send(Message::Text(json.into())).await;
                            }
                            continue;
                        }

                        if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                            match client_msg {
                                ClientMessage::JoinRoom { room } => {
                                    let clean_room: String = room
                                        .trim()
                                        .to_lowercase()
                                        .chars()
                                        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                                        .take(24)
                                        .collect();

                                    let target_room = if clean_room.is_empty() {
                                        "general".to_string()
                                    } else {
                                        clean_room
                                    };

                                    if target_room != current_room {
                                        let old_room = current_room.clone();
                                        current_room = target_room.clone();

                                        let time_str = Local::now().format("%H:%M").to_string();

                                        // Thông báo phòng cũ
                                        let leave_sys = ServerMessage::System {
                                            content: format!("{nickname} đã chuyển sang #{current_room}"),
                                            timestamp: time_str.clone(),
                                            room: Some(old_room),
                                        };
                                        state.broadcast(leave_sys);

                                        // Gửi xác nhận phòng mới và nạp lịch sử phòng mới cho client
                                        let ack_room = ServerMessage::RoomJoined {
                                            room: current_room.clone(),
                                        };
                                        if let Ok(json) = serde_json::to_string(&ack_room) {
                                            let _ = sender.send(Message::Text(json.into())).await;
                                        }

                                        let room_history = state.get_room_history(&current_room).await;
                                        let history_msg = ServerMessage::History {
                                            messages: room_history,
                                            room: current_room.clone(),
                                        };
                                        if let Ok(json) = serde_json::to_string(&history_msg) {
                                            let _ = sender.send(Message::Text(json.into())).await;
                                        }

                                        // Thông báo phòng mới
                                        let enter_sys = ServerMessage::System {
                                            content: format!("{nickname} đã vào phòng #{current_room}"),
                                            timestamp: time_str,
                                            room: Some(current_room.clone()),
                                        };
                                        state.add_to_history(enter_sys.clone()).await;
                                        state.broadcast(enter_sys);
                                    }
                                }

                                ClientMessage::Chat { content } => {
                                    let now = Instant::now();

                                    if let Some(cooldown) = cooldown_until {
                                        if now < cooldown {
                                            let remaining = (cooldown - now).as_secs() + 1;
                                            let warn_msg = ServerMessage::System {
                                                content: format!("⚠️ Bạn đang gửi tin quá nhanh! Vui lòng chờ {remaining} giây."),
                                                timestamp: Local::now().format("%H:%M").to_string(),
                                                room: Some(current_room.clone()),
                                            };
                                            if let Ok(json) = serde_json::to_string(&warn_msg) {
                                                let _ = sender.send(Message::Text(json.into())).await;
                                            }
                                            continue;
                                        } else {
                                            cooldown_until = None;
                                        }
                                    }

                                    while let Some(&t) = msg_timestamps.front() {
                                        if now.duration_since(t) > Duration::from_secs(RATE_LIMIT_WINDOW_SECS) {
                                            msg_timestamps.pop_front();
                                        } else {
                                            break;
                                        }
                                    }

                                    if msg_timestamps.len() >= RATE_LIMIT_MAX_MESSAGES {
                                        cooldown_until = Some(now + Duration::from_secs(COOLDOWN_DURATION_SECS));
                                        let warn_msg = ServerMessage::System {
                                            content: format!("⚠️ Phát hiện gửi tin liên tục! Bạn bị tạm khóa gửi trong {COOLDOWN_DURATION_SECS} giây."),
                                            timestamp: Local::now().format("%H:%M").to_string(),
                                            room: Some(current_room.clone()),
                                        };
                                        if let Ok(json) = serde_json::to_string(&warn_msg) {
                                            let _ = sender.send(Message::Text(json.into())).await;
                                        }
                                        continue;
                                    }

                                    msg_timestamps.push_back(now);

                                    let trimmed = content.trim();
                                    if !trimmed.is_empty() {
                                        let sanitized: String = trimmed.chars().take(500).collect();
                                        let msg_id = format!(
                                            "{}-{}",
                                            chrono::Utc::now().timestamp_millis(),
                                            nickname
                                        );
                                        let time_str = Local::now().format("%H:%M").to_string();

                                        let chat_msg = ServerMessage::Chat {
                                            id: msg_id,
                                            nickname: nickname.clone(),
                                            content: sanitized,
                                            timestamp: time_str,
                                            room: current_room.clone(),
                                        };

                                        state.add_to_history(chat_msg.clone()).await;
                                        state.broadcast(chat_msg);
                                    }
                                }

                                ClientMessage::SetNickname { nickname: new_nick } => {
                                    match state.try_claim_nickname(Some(&nickname), &new_nick).await {
                                        Ok(claimed_name) => {
                                            let old_nick = nickname.clone();
                                            nickname = claimed_name;

                                            let time_str = Local::now().format("%H:%M").to_string();
                                            let sys_msg = ServerMessage::System {
                                                content: format!("{old_nick} đã đổi tên thành {nickname}"),
                                                timestamp: time_str,
                                                room: Some(current_room.clone()),
                                            };

                                            let ack_msg = ServerMessage::NicknameAssigned {
                                                nickname: nickname.clone(),
                                            };
                                            if let Ok(json) = serde_json::to_string(&ack_msg) {
                                                let _ = sender.send(Message::Text(json.into())).await;
                                            }

                                            state.add_to_history(sys_msg.clone()).await;
                                            state.broadcast(sys_msg);
                                        }
                                        Err(err_msg) => {
                                            let warn_msg = ServerMessage::System {
                                                content: format!("⚠️ {err_msg}"),
                                                timestamp: Local::now().format("%H:%M").to_string(),
                                                room: Some(current_room.clone()),
                                            };
                                            if let Ok(json) = serde_json::to_string(&warn_msg) {
                                                let _ = sender.send(Message::Text(json.into())).await;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Pong(_))) => {
                        last_activity = Instant::now();
                    }
                    Some(Ok(Message::Ping(payload))) => {
                        last_activity = Instant::now();
                        if sender.send(Message::Pong(payload)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        info!("Client disconnected: {nickname}");
                        break;
                    }
                    Some(Err(err)) => {
                        warn!("WebSocket error for {nickname}: {err}");
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    // 6. Dọn dẹp biệt danh & cập nhật số lượng người khi rời phòng
    state.release_nickname(&nickname).await;
    let remaining_users = state.decrement_user();
    let leave_time = Local::now().format("%H:%M").to_string();
    let leave_event = ServerMessage::System {
        content: format!("{nickname} đã rời phòng #{current_room}"),
        timestamp: leave_time,
        room: Some(current_room.clone()),
    };
    state.add_to_history(leave_event.clone()).await;
    state.broadcast(leave_event);
    state.broadcast(ServerMessage::UserCount {
        count: remaining_users,
    });
}
