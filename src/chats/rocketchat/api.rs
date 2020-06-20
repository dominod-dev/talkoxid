use super::schema::*;
use async_channel::{Receiver, Sender};
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::error::Error;
use tokio_tungstenite::tungstenite;
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

    pub async fn login(&mut self) -> Result<String, String> {
        let login_response = self
            .client
            .post(&format!("{}/api/v1/login", &self.host)[..])
            .body(format!(
                "username={}&password={}",
                &self.username, &self.password
            ))
            .header("content-type", "application/x-www-form-urlencoded")
            .send()
            .await
            .map_err(|err| format!("{:?}", err))?
            .json::<LoginResponse>()
            .await
            .map_err(|err| format!("{:?}", err))?;
        let user_id = login_response.data.user_id.clone();
        self.auth_token = Some(login_response.data);
        Ok(user_id)
    }

    pub async fn channels(&self) -> Result<Vec<ChannelResponse>, String> {
        let channels = self
            .client
            .get(&format!("{}/api/v1/channels.list", &self.host)[..])
            .header(
                "X-Auth-Token",
                &self.auth_token.as_ref().unwrap().auth_token,
            )
            .header("X-User-Id", &self.auth_token.as_ref().unwrap().user_id)
            .send()
            .await
            .map_err(|err| format!("{:?}", err))?
            .json::<ChannelListResponse>()
            .await
            .map_err(|err| format!("{:?}", err))?
            .channels;
        Ok(channels)
    }

    pub async fn rooms(&self) -> Result<Vec<RoomResponse>, String> {
        let rooms = self
            .client
            .get(&format!("{}/api/v1/rooms.get", &self.host)[..])
            .header(
                "X-Auth-Token",
                &self.auth_token.as_ref().unwrap().auth_token,
            )
            .header("X-User-Id", &self.auth_token.as_ref().unwrap().user_id)
            .send()
            .await
            .map_err(|err| format!("{:?}", err))?
            .json::<RoomsListResponse>()
            .await
            .map_err(|err| format!("{:?}", err))?
            .update;
        Ok(rooms)
    }

    pub async fn users(&self) -> Result<Vec<UserResponse>, String> {
        let users = self
            .client
            .get(&format!("{}/api/v1/users.list", &self.host)[..])
            .header(
                "X-Auth-Token",
                &self.auth_token.as_ref().unwrap().auth_token,
            )
            .header("X-User-Id", &self.auth_token.as_ref().unwrap().user_id)
            .send()
            .await
            .map_err(|err| format!("{:?}", err))?
            .json::<UserListResponse>()
            .await
            .map_err(|err| format!("{:?}", err))?
            .users;
        Ok(users)
    }
    pub async fn history(
        &self,
        room_id: String,
        count: usize,
    ) -> Result<Vec<MessageResponse>, String> {
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
            .await
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
                .await
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
                .await
                .map_err(|err| format!("{:?}", err))?;
        }
        let messages = messages
            .json::<ChannelHistoryResponse>()
            .await
            .map_err(|err| format!("{:?}", err))?
            .messages;
        Ok(messages)
    }

    pub async fn send_message(&self, room_id: String, content: String) -> Result<(), String> {
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
            .await
            .map_err(|err| format!("{:?}", err))?;
        Ok(())
    }
}

pub struct RocketChatWsWriter {
    username: String,
    password_digest: String,
    user_id: String,
    websocket: Sender<tokio_tungstenite::tungstenite::Message>,
}

impl RocketChatWsWriter {
    pub async fn new(
        username: String,
        password: String,
        websocket: Sender<tungstenite::Message>,
        reader: &Receiver<tungstenite::Message>,
    ) -> Result<Self, Box<dyn Error>> {
        let mut hasher = Sha256::new();
        hasher.update(password);
        let password_digest = format!("{:x}", hasher.finalize());
        reader.recv().await?;
        RocketChatWsWriter::connect(&websocket).await?;
        reader.recv().await?;
        RocketChatWsWriter::init(&username, &password_digest, &websocket).await?;
        let msg = reader.recv().await?;
        let user_id = serde_json::from_str::<UserIdResponse>(&msg.to_string()[..])?.id;
        Ok(RocketChatWsWriter {
            username,
            password_digest,
            user_id,
            websocket,
        })
    }

    pub async fn init(
        username: &str,
        password_digest: &str,
        websocket: &Sender<tungstenite::Message>,
    ) -> Result<(), Box<dyn Error>> {
        let login = LoginWs {
            msg: "method".into(),
            method: "login".into(),
            id: "42".into(),
            params: vec![LoginParamsWs {
                user: UsernameWs {
                    username: username.into(),
                },
                password: PasswordWs {
                    digest: password_digest.into(),
                    algorithm: "sha-256".into(),
                },
            }],
        };
        websocket
            .send(tungstenite::Message::Text(serde_json::to_string(&login)?))
            .await?;
        Ok(())
    }

    pub async fn login(&self) -> Result<(), Box<dyn Error>> {
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
        self.websocket
            .send(tungstenite::Message::Text(serde_json::to_string(&login)?))
            .await?;
        Ok(())
    }

    pub async fn connect(writer: &Sender<tungstenite::Message>) -> Result<(), Box<dyn Error>> {
        let connect = ConnectWs {
            msg: "connect".into(),
            version: "1".into(),
            support: vec!["1".into()],
        };
        writer
            .send(tungstenite::Message::Text(serde_json::to_string(&connect)?))
            .await?;
        Ok(())
    }

    pub async fn pong(&self) -> Result<(), Box<dyn Error>> {
        let pong = PongWs { msg: "pong".into() };
        self.websocket
            .send(tungstenite::Message::Text(serde_json::to_string(&pong)?))
            .await?;
        Ok(())
    }

    pub async fn subscribe_user(&self) -> Result<(), Box<dyn Error>> {
        let sub = SubStreamChannelWs {
            msg: "sub".into(),
            id: "1234".into(),
            name: "stream-notify-user".into(),
            params: vec![
                serde_json::json!(format!("{}/rooms-changed", &self.user_id)),
                serde_json::json!(false),
            ],
        };
        self.websocket
            .send(tungstenite::Message::Text(serde_json::to_string(&sub)?))
            .await?;
        Ok(())
    }

    pub async fn send_message(
        &self,
        room_id: String,
        content: String,
    ) -> Result<(), Box<dyn Error>> {
        let msg = format!(
            r#"
            {{
                "msg": "method",
                "method": "sendMessage",
                "id": "42",
                "params": [
                    {{
                        "rid": "{}",
                        "msg": "{}"
                    }}
                ]
            }}
        "#,
            room_id, content
        );
        self.websocket.send(tungstenite::Message::Text(msg)).await?;
        Ok(())
    }

    pub async fn load_history(&self, room_id: String, count: usize) -> Result<(), Box<dyn Error>> {
        let msg = format!(
            r#"
            {{
                "msg": "method",
                "method": "loadHistory",
                "id": "42",
                "params": [ "{}", null, {}, null ]
            }}
        "#,
            room_id, count
        );
        self.websocket.send(tungstenite::Message::Text(msg)).await?;
        Ok(())
    }

    pub async fn load_rooms(&self) -> Result<(), Box<dyn Error>> {
        let msg = r#"
            {
                "msg": "method",
                "method": "rooms/get",
                "id": "42",
                "params": [ { "$date": 0 } ]
            }
        "#;
        self.websocket
            .send(tungstenite::Message::Text(msg.into()))
            .await?;
        Ok(())
    }
}
