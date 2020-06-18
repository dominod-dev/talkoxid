use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AuthToken {
    pub auth_token: String,
    pub user_id: String,
}

#[derive(Deserialize, Debug)]
pub struct LoginResponse {
    pub data: AuthToken,
}

#[derive(Deserialize, Debug)]
pub struct ChannelResponse {
    pub name: String,
    pub _id: String,
}

#[derive(Deserialize, Debug)]
pub struct ChannelListResponse {
    pub channels: Vec<ChannelResponse>,
}

#[derive(Deserialize, Debug)]
pub struct RoomResponse {
    pub _id: String,
    pub name: Option<String>,
    pub usernames: Option<Vec<String>>,
}

#[derive(Deserialize, Debug)]
pub struct RoomsListResponse {
    pub update: Vec<RoomResponse>,
}

#[derive(Deserialize, Debug)]
pub struct UserResponse {
    pub name: String,
}

#[derive(Deserialize, Debug)]
pub struct UserListResponse {
    pub users: Vec<UserResponse>,
}

#[derive(Deserialize, Debug)]
pub struct AuthorResponse {
    pub username: String,
}

#[derive(Deserialize, Debug)]
pub struct MessageResponse {
    pub u: AuthorResponse,
    pub msg: String,
    pub ts: String,
}

#[derive(Deserialize, Debug)]
pub struct ChannelHistoryResponse {
    pub messages: Vec<MessageResponse>,
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
pub struct MessageResponseWs {
    pub u: AuthorResponse,
    pub rid: String,
    pub msg: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EventResponseWs {
    pub last_message: MessageResponseWs,
}
#[derive(Deserialize, Debug)]
pub struct SocketEventResponseWs(pub String, pub EventResponseWs);

#[derive(Deserialize, Debug)]
pub struct SocketArgsWs {
    pub args: SocketEventResponseWs,
}

#[derive(Deserialize, Debug)]
pub struct SocketMessageWs {
    pub msg: String,
    pub fields: SocketArgsWs,
}

#[derive(Deserialize, Debug)]
pub struct UserIdResponse {
    pub id: String,
}
