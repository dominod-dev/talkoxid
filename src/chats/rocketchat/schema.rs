use chrono::prelude::*;
use chrono::serde::ts_milliseconds;

use serde::{Deserialize, Serialize};
#[derive(Deserialize, Debug, Clone)]
pub struct AuthorResponseWs {
    pub _id: String,
    pub username: String,
}
#[derive(Serialize, Debug)]
pub struct UsernameWs {
    pub username: String,
}

#[derive(Serialize, Debug)]
pub struct PasswordWs {
    pub digest: String,
    pub algorithm: String,
}

#[derive(Serialize, Debug)]
pub struct LoginParamsWs {
    pub user: UsernameWs,
    pub password: PasswordWs,
}

#[derive(Serialize, Debug)]
pub struct LoginWs {
    pub msg: String,
    pub method: String,
    pub params: Vec<LoginParamsWs>,
    pub id: String,
}

#[derive(Serialize, Debug)]
pub struct ConnectWs {
    pub msg: String,
    pub version: String,
    pub support: Vec<String>,
}

#[derive(Serialize, Debug)]
pub struct PongWs {
    pub msg: String,
}

#[derive(Serialize, Debug)]
pub struct SubStreamChannelWs {
    pub msg: String,
    pub id: String,
    pub name: String,
    pub params: Vec<serde_json::Value>,
}

#[derive(Deserialize, Debug)]
pub struct DateResponseWs {
    #[serde(rename = "$date")]
    #[serde(with = "ts_milliseconds")]
    pub date: DateTime<Utc>,
}

#[derive(Deserialize, Debug)]
pub struct MessageResponseWs {
    pub u: AuthorResponseWs,
    pub rid: String,
    pub msg: String,
    pub ts: DateResponseWs,
}

#[derive(Deserialize, Debug)]
pub struct ChannelHistoryResponseWs {
    pub messages: Vec<MessageResponseWs>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EventResponseWs {
    pub last_message: MessageResponseWs,
    pub t: String,
}
#[derive(Deserialize, Debug)]
pub struct SocketEventResponseWs(pub String, pub EventResponseWs);

#[derive(Deserialize, Debug)]
pub struct SocketArgsWs {
    pub args: SocketEventResponseWs,
}

#[derive(Deserialize, Debug)]
pub struct SocketMessageWs {
    pub id: String,
    pub msg: String,
    pub fields: SocketArgsWs,
}

#[derive(Deserialize, Debug)]
pub struct UserIdResponse {
    pub id: String,
}

#[derive(Deserialize, Debug)]
pub struct DirectChatResponseWs {
    pub _id: String,
    pub usernames: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct ChatResponseWs {
    pub _id: String,
    pub name: String,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "t")]
pub enum RoomResponseWs {
    #[serde(rename = "d")]
    Direct(DirectChatResponseWs),
    #[serde(rename = "c")]
    Chat(ChatResponseWs),
    #[serde(rename = "p")]
    Private(ChatResponseWs),
}

#[derive(Deserialize, Debug)]
pub struct RoomsResponseWs {
    pub update: Vec<RoomResponseWs>,
    pub remove: Vec<serde_json::value::Value>,
}

#[derive(Deserialize, Debug)]
pub struct ResultRoomResponseWs {
    pub rid: String,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "t")]
pub enum JoinedRoomResponseWs {
    #[serde(rename = "d")]
    Direct(ResultRoomResponseWs),
    #[serde(rename = "c")]
    Chat(ResultRoomResponseWs),
    #[serde(rename = "p")]
    Private(ResultRoomResponseWs),
}

#[derive(Deserialize, Debug)]
pub struct UserInRoomResponseWs {
    pub total: usize,
    pub record: Vec<serde_json::value::Value>,
}

#[derive(Deserialize, Debug)]
pub struct UsersInRoomResponseWs {
    pub total: usize,
    pub records: Vec<AuthorResponseWs>,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum WsResponse {
    NewMessage(SocketMessageWs),
    History {
        msg: String,
        id: String,
        result: ChannelHistoryResponseWs,
    },
    Rooms {
        msg: String,
        id: String,
        result: RoomsResponseWs,
    },
    JoinedRoom {
        msg: String,
        id: String,
        result: JoinedRoomResponseWs,
    },
    UsersInRoom {
        msg: String,
        id: String,
        result: UsersInRoomResponseWs,
    },
    Ping {
        msg: String,
    },
}
