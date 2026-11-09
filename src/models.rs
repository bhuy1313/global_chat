use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Chat { content: String },
    SetNickname { nickname: String },
    JoinRoom { room: String },
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Chat {
        id: String,
        nickname: String,
        content: String,
        timestamp: String,
        room: String,
    },
    System {
        content: String,
        timestamp: String,
        room: Option<String>,
    },
    History {
        messages: Vec<ServerMessage>,
        room: String,
    },
    UserCount {
        count: usize,
    },
    NicknameAssigned {
        nickname: String,
    },
    RoomJoined {
        room: String,
    },
}

impl ServerMessage {
    pub fn room(&self) -> Option<&str> {
        match self {
            ServerMessage::Chat { room, .. } => Some(room.as_str()),
            ServerMessage::System { room, .. } => room.as_deref(),
            ServerMessage::History { room, .. } => Some(room.as_str()),
            ServerMessage::RoomJoined { room } => Some(room.as_str()),
            ServerMessage::UserCount { .. } | ServerMessage::NicknameAssigned { .. } => None,
        }
    }
}
