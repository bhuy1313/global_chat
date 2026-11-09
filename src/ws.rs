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

// Cấu hình Rate Limiting
const RATE_LIMIT_MAX_MESSAGES: usize = 5;
const RATE_LIMIT_WINDOW_SECS: u64 = 3;
const COOLDOWN_DURATION_SECS: u64 = 5;
const MAX_PAYLOAD_BYTES: usize = 1024;
const HEARTBEAT_INTERVAL_SECS: u64 = 25;
const HEARTBEAT_TIMEOUT_SECS: u64 = 65;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    // 1. Tạo và đăng ký biệt danh Guest an toàn không trùng lặp
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

    info!("New client connected: {nickname}");

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

    let init_count = ServerMessage::UserCount {
        count: current_users,
    };
    if let Ok(json) = serde_json::to_string(&init_count) {
        let _ = sender.send(Message::Text(json.into())).await;
    }

    let history_msgs = state.get_history().await;
    let init_history = ServerMessage::History {
        messages: history_msgs,
    };
    if let Ok(json) = serde_json::to_string(&init_history) {
        let _ = sender.send(Message::Text(json.into())).await;
    }

    // 3. Thông báo cho toàn bộ phòng chat
    let now_str = Local::now().format("%H:%M").to_string();
    let join_event = ServerMessage::System {
        content: format!("{nickname} đã tham gia phòng chat"),
        timestamp: now_str,
    };
    state.add_to_history(join_event.clone()).await;
    state.broadcast(join_event);
    state.broadcast(ServerMessage::UserCount {
        count: current_users,
    });

    // 4. Biến theo dõi Rate Limiting & Heartbeat per-socket
    let mut msg_timestamps = std::collections::VecDeque::new();
    let mut cooldown_until: Option<Instant> = None;
    let mut last_activity = Instant::now();
    let mut heartbeat_timer = tokio::time::interval(Duration::from_secs(HEARTBEAT_INTERVAL_SECS));

    // 5. Vòng lặp sự kiện bất đồng bộ
    loop {
        tokio::select! {
            // Kiểm tra Heartbeat & Dọn dẹp Zombie Connection
            _ = heartbeat_timer.tick() => {
                if last_activity.elapsed() > Duration::from_secs(HEARTBEAT_TIMEOUT_SECS) {
                    warn!("Client {nickname} timeout heartbeat (kết nối ma/mất mạng), đóng socket.");
                    break;
                }
                // Gửi Ping frame định kỳ để duy trì kết nối
                if sender.send(Message::Ping(vec![].into())).await.is_err() {
                    break;
                }
            }

            // Nhận tin nhắn phát sóng từ server để gửi ra client
            broadcast_msg = rx.recv() => {
                match broadcast_msg {
                    Ok(msg) => {
                        if let Ok(json) = serde_json::to_string(&msg) {
                            if sender.send(Message::Text(json.into())).await.is_err() {
                                break;
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

            // Nhận tin nhắn từ client gửi lên
            incoming = receiver.next() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        last_activity = Instant::now();

                        // Kiểm tra kích thước gói tin (Chống tấn công bộ nhớ)
                        if text.len() > MAX_PAYLOAD_BYTES {
                            let warn_msg = ServerMessage::System {
                                content: "⚠️ Tin nhắn quá dài vượt quá giới hạn cho phép.".to_string(),
                                timestamp: Local::now().format("%H:%M").to_string(),
                            };
                            if let Ok(json) = serde_json::to_string(&warn_msg) {
                                let _ = sender.send(Message::Text(json.into())).await;
                            }
                            continue;
                        }

                        if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                            match client_msg {
                                ClientMessage::Chat { content } => {
                                    let now = Instant::now();

                                    // Kiểm tra thời gian phạt Cool-down nếu đang bị khóa
                                    if let Some(cooldown) = cooldown_until {
                                        if now < cooldown {
                                            let remaining = (cooldown - now).as_secs() + 1;
                                            let warn_msg = ServerMessage::System {
                                                content: format!("⚠️ Bạn đang gửi tin quá nhanh! Vui lòng chờ {remaining} giây."),
                                                timestamp: Local::now().format("%H:%M").to_string(),
                                            };
                                            if let Ok(json) = serde_json::to_string(&warn_msg) {
                                                let _ = sender.send(Message::Text(json.into())).await;
                                            }
                                            continue;
                                        } else {
                                            cooldown_until = None;
                                        }
                                    }

                                    // Sliding window: tối đa 5 tin nhắn trong 3 giây
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
                                        };
                                        if let Ok(json) = serde_json::to_string(&warn_msg) {
                                            let _ = sender.send(Message::Text(json.into())).await;
                                        }
                                        continue;
                                    }

                                    msg_timestamps.push_back(now);

                                    // Validate và sanitize nội dung
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
                                        };

                                        state.add_to_history(chat_msg.clone()).await;
                                        state.broadcast(chat_msg);
                                    }
                                }
                                ClientMessage::SetNickname { nickname: new_nick } => {
                                    // Kiểm tra qua bộ lọc bảo vệ của state
                                    match state.try_claim_nickname(Some(&nickname), &new_nick).await {
                                        Ok(claimed_name) => {
                                            let old_nick = nickname.clone();
                                            nickname = claimed_name;

                                            let time_str = Local::now().format("%H:%M").to_string();
                                            let sys_msg = ServerMessage::System {
                                                content: format!("{old_nick} đã đổi tên thành {nickname}"),
                                                timestamp: time_str,
                                            };

                                            // Xác nhận cập nhật tên riêng cho client này
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
                                            // Báo lỗi vi phạm biệt danh chỉ riêng cho người này
                                            let warn_msg = ServerMessage::System {
                                                content: format!("⚠️ {err_msg}"),
                                                timestamp: Local::now().format("%H:%M").to_string(),
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
        content: format!("{nickname} đã rời phòng chat"),
        timestamp: leave_time,
    };
    state.add_to_history(leave_event.clone()).await;
    state.broadcast(leave_event);
    state.broadcast(ServerMessage::UserCount {
        count: remaining_users,
    });
}
