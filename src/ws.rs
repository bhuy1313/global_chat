use std::sync::Arc;
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

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    // Generate an initial guest nickname
    let guest_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        % 10000;
    let mut nickname = format!("Guest_{guest_id:04}");

    info!("New client connected: {nickname}");

    let mut rx = state.tx.subscribe();
    let current_users = state.increment_user();

    let (mut sender, mut receiver) = socket.split();

    // 1. Send initial info directly to this client
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

    // 2. Broadcast join notification & user count to all users
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

    // 3. Main event loop: select on incoming WS messages or broadcast channel
    loop {
        tokio::select! {
            // Outbound: Received a broadcast message from state
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

            // Inbound: Received a message from this client's WebSocket
            incoming = receiver.next() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                            match client_msg {
                                ClientMessage::Chat { content } => {
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
                                    let clean_nick: String = new_nick.trim().chars().take(24).collect();
                                    if clean_nick.len() >= 2 && clean_nick != nickname {
                                        let old_nick = nickname.clone();
                                        nickname = clean_nick.clone();

                                        let time_str = Local::now().format("%H:%M").to_string();
                                        let sys_msg = ServerMessage::System {
                                            content: format!("{old_nick} đã đổi tên thành {nickname}"),
                                            timestamp: time_str,
                                        };

                                        // Acknowledge back to user
                                        let ack_msg = ServerMessage::NicknameAssigned {
                                            nickname: nickname.clone(),
                                        };
                                        if let Ok(json) = serde_json::to_string(&ack_msg) {
                                            let _ = sender.send(Message::Text(json.into())).await;
                                        }

                                        state.add_to_history(sys_msg.clone()).await;
                                        state.broadcast(sys_msg);
                                    }
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Ping(payload))) => {
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

    // 4. Handle client departure
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
