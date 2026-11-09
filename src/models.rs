use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Chat { content: String },
    SetNickname { nickname: String },
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Chat {
        id: String,
        nickname: String,
        content: String,
        timestamp: String,
    },
    System {
        content: String,
        timestamp: String,
    },
    History {
        messages: Vec<ServerMessage>,
    },
    UserCount {
        count: usize,
    },
    NicknameAssigned {
        nickname: String,
    },
}
