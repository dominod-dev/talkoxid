use super::schema::*;
use reqwest::blocking::Client;
use sha2::{Digest, Sha256};
use tungstenite::client::AutoStream;
use tungstenite::{connect, WebSocket};
use url::Url;

pub struct RocketChatApi {
    client: Client,
    host: Url,
    username: String,
    password: String,
    auth_token: Option<AuthToken>,
}

impl RocketChatApi {
    pub fn new(host: Url, username: String, password: String) -> Self {
        let client = Client::new();
        RocketChatApi {
            client,
            host,
            username,
            password,
            auth_token: None,
        }
    }

    pub fn login(&mut self) -> Result<String, String> {
        let login_response = self
            .client
            .post(&format!("{}/api/v1/login", &self.host)[..])
            .body(format!(
                "username={}&password={}",
                &self.username, &self.password
            ))
            .header("content-type", "application/x-www-form-urlencoded")
            .send()
            .map_err(|err| format!("{:?}", err))?
            .json::<LoginResponse>()
            .map_err(|err| format!("{:?}", err))?;
        let user_id = login_response.data.user_id.clone();
        self.auth_token = Some(login_response.data);
        Ok(user_id)
    }

    pub fn channels(&self) -> Result<Vec<ChannelResponse>, String> {
        let channels = self
            .client
            .get(&format!("{}/api/v1/channels.list", &self.host)[..])
            .header(
                "X-Auth-Token",
                &self.auth_token.as_ref().unwrap().auth_token,
            )
            .header("X-User-Id", &self.auth_token.as_ref().unwrap().user_id)
            .send()
            .map_err(|err| format!("{:?}", err))?
            .json::<ChannelListResponse>()
            .map_err(|err| format!("{:?}", err))?
            .channels;
        Ok(channels)
    }

    pub fn rooms(&self) -> Result<Vec<RoomResponse>, String> {
        let rooms = self
            .client
            .get(&format!("{}/api/v1/rooms.get", &self.host)[..])
            .header(
                "X-Auth-Token",
                &self.auth_token.as_ref().unwrap().auth_token,
            )
            .header("X-User-Id", &self.auth_token.as_ref().unwrap().user_id)
            .send()
            .map_err(|err| format!("{:?}", err))?
            .json::<RoomsListResponse>()
            .map_err(|err| format!("{:?}", err))?
            .update;
        Ok(rooms)
    }

    pub fn users(&self) -> Result<Vec<UserResponse>, String> {
        let users = self
            .client
            .get(&format!("{}/api/v1/users.list", &self.host)[..])
            .header(
                "X-Auth-Token",
                &self.auth_token.as_ref().unwrap().auth_token,
            )
            .header("X-User-Id", &self.auth_token.as_ref().unwrap().user_id)
            .send()
            .map_err(|err| format!("{:?}", err))?
            .json::<UserListResponse>()
            .map_err(|err| format!("{:?}", err))?
            .users;
        Ok(users)
    }
    pub fn history(&self, room_id: String, count: usize) -> Result<Vec<MessageResponse>, String> {
        let mut messages = self
            .client
            .get(
                &format!(
                    "{}/api/v1/channels.history?roomId={}&count={}",
                    &self.host, room_id, count
                )[..],
            )
            .header(
                "X-Auth-Token",
                &self.auth_token.as_ref().unwrap().auth_token,
            )
            .header("X-User-Id", &self.auth_token.as_ref().unwrap().user_id)
            .send()
            .map_err(|err| format!("{:?}", err))?;
        if messages.status().as_u16() != 200 {
            messages = self
                .client
                .get(
                    &format!(
                        "{}/api/v1/im.history?roomId={}&count={}",
                        &self.host, room_id, count
                    )[..],
                )
                .header(
                    "X-Auth-Token",
                    &self.auth_token.as_ref().unwrap().auth_token,
                )
                .header("X-User-Id", &self.auth_token.as_ref().unwrap().user_id)
                .send()
                .map_err(|err| format!("{:?}", err))?;
        }
        if messages.status().as_u16() != 200 {
            messages = self
                .client
                .get(
                    &format!(
                        "{}/api/v1/groups.history?roomId={}&count={}",
                        &self.host, room_id, count
                    )[..],
                )
                .header(
                    "X-Auth-Token",
                    &self.auth_token.as_ref().unwrap().auth_token,
                )
                .header("X-User-Id", &self.auth_token.as_ref().unwrap().user_id)
                .send()
                .map_err(|err| format!("{:?}", err))?;
        }
        let messages = messages
            .json::<ChannelHistoryResponse>()
            .map_err(|err| format!("{:?}", err))?
            .messages;
        Ok(messages)
    }

    pub fn send_message(&self, room_id: String, content: String) -> Result<(), String> {
        self.client
            .post(&format!("{}/api/v1/chat.postMessage", &self.host)[..])
            .body(format!(
                "{{ \"channel\": \"{}\", \"text\": \"{}\" }}",
                room_id, content
            ))
            .header(
                "X-Auth-Token",
                &self.auth_token.as_ref().unwrap().auth_token,
            )
            .header("X-User-Id", &self.auth_token.as_ref().unwrap().user_id)
            .header("content-type", "application/json")
            .send()
            .map_err(|err| format!("{:?}", err))?;
        Ok(())
    }
}

pub struct RocketChatWs {
    socket: WebSocket<AutoStream>,
    username: String,
    password_digest: String,
    user_id: String,
}

impl RocketChatWs {
    pub fn new(mut host: Url, username: String, password: String, user_id: String) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(password);
        let password_digest = format!("{:x}", hasher.finalize());
        host.set_scheme("ws").unwrap();
        host.set_path("/websocket");
        let (socket, _) = connect(host).expect("Can't connect");
        RocketChatWs {
            socket,
            username,
            password_digest,
            user_id,
        }
    }

    pub fn login(&mut self) {
        let login = LoginWs {
            msg: "method".into(),
            method: "login".into(),
            id: "42".into(),
            params: vec![LoginParamsWs {
                user: UsernameWs {
                    username: self.username.clone(),
                },
                password: PasswordWs {
                    digest: self.password_digest.clone(),
                    algorithm: "sha-256".into(),
                },
            }],
        };
        let connect = ConnectWs {
            msg: "connect".into(),
            version: "1".into(),
            support: vec!["1".into()],
        };

        self.socket
            .write_message(tungstenite::Message::Text(
                serde_json::to_string(&connect).unwrap(),
            ))
            .unwrap();
        self.socket
            .write_message(tungstenite::Message::Text(
                serde_json::to_string(&login).unwrap(),
            ))
            .unwrap();
    }

    pub fn pong(&mut self) {
        let pong = PongWs { msg: "pong".into() };
        self.socket
            .write_message(tungstenite::Message::Text(
                serde_json::to_string(&pong).unwrap(),
            ))
            .unwrap();
    }

    pub fn subscribe_user(&mut self) {
        let sub = SubStreamChannelWs {
            msg: "sub".into(),
            id: "1234".into(),
            name: "stream-notify-user".into(),
            params: vec![
                serde_json::json!(format!("{}/rooms-changed", &self.user_id)),
                serde_json::json!(false),
            ],
        };

        self.socket
            .write_message(tungstenite::Message::Text(
                serde_json::to_string(&sub).unwrap(),
            ))
            .unwrap();
    }

    pub fn read(&mut self) -> Result<String, String> {
        Ok(self
            .socket
            .read_message()
            .map_err(|err| format!("{:?}", err))?
            .to_string())
    }
}
