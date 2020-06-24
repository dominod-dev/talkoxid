use super::schema::*;
use async_channel::{Receiver, Sender};
use sha2::{Digest, Sha256};
use std::error::Error;
use tokio_tungstenite::tungstenite;

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
            id: "1".into(),
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
            id: "1".into(),
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
                "id": "2",
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
                "id": "3",
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
                "id": "4",
                "params": [ { "$date": 0 } ]
            }
        "#;
        self.websocket
            .send(tungstenite::Message::Text(msg.into()))
            .await?;
        Ok(())
    }

    pub async fn create_direct_chat(&self, username: String) -> Result<(), Box<dyn Error>> {
        let msg = format!(
            r#"
            {{
                "msg": "method",
                "method": "createDirectMessage",
                "id": "5",
                "params": ["{}"]
            }}
        "#,
            username
        );
        self.websocket.send(tungstenite::Message::Text(msg)).await?;
        Ok(())
    }

    pub async fn subscribe_user(&self) -> Result<(), Box<dyn Error>> {
        let sub = SubStreamChannelWs {
            msg: "sub".into(),
            id: "6".into(),
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

    pub async fn subscribe_messages(&self) -> Result<(), Box<dyn Error>> {
        let sub = SubStreamChannelWs {
            msg: "sub".into(),
            id: "7".into(),
            name: "stream-room-messages".into(),
            params: vec![
                serde_json::json!("__my_messages__".to_string()),
                serde_json::json!(false),
            ],
        };
        self.websocket
            .send(tungstenite::Message::Text(serde_json::to_string(&sub)?))
            .await?;
        Ok(())
    }

    pub async fn get_users_room(&self, room_id: String) -> Result<(), Box<dyn Error>> {
        let msg = format!(
            r#"
        {{
            "msg": "method",
            "method": "getUsersOfRoom",
            "params": [
                "{}",
                true,
                {{
                  "limit": 100,
                  "skip": 0
                }},
                ""
            ],
            "id": "8"
        }}
        "#,
            room_id
        );
        self.websocket.send(tungstenite::Message::Text(msg)).await?;
        Ok(())
    }
}
