use super::schema::*;
use async_channel::{Receiver, Sender};
use async_trait::async_trait;
use sha2::{Digest, Sha256};
use std::error::Error;
use tokio_tungstenite::tungstenite;

#[async_trait]
pub trait WebSocketWriter {
    async fn init(
        username: &str,
        password_digest: &str,
        websocket: &Sender<tungstenite::Message>,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;

    async fn login(&self) -> Result<(), Box<dyn Error + Send + Sync>>;

    async fn connect(
        writer: &Sender<tungstenite::Message>,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;

    async fn pong(&self) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn send_message(
        &self,
        room_id: String,
        content: String,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn load_history(
        &self,
        room_id: String,
        count: usize,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn load_rooms(&self) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn create_direct_chat(
        &self,
        username: String,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn subscribe_user(&self) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn subscribe_messages(&self) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn get_users_room(&self, room_id: String) -> Result<(), Box<dyn Error + Send + Sync>>;
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
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
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
}

#[async_trait]
impl WebSocketWriter for RocketChatWsWriter {
    async fn init(
        username: &str,
        password_digest: &str,
        websocket: &Sender<tungstenite::Message>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
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

    async fn login(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
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

    async fn connect(
        writer: &Sender<tungstenite::Message>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
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

    async fn pong(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let pong = PongWs { msg: "pong".into() };
        self.websocket
            .send(tungstenite::Message::Text(serde_json::to_string(&pong)?))
            .await?;
        Ok(())
    }

    async fn send_message(
        &self,
        room_id: String,
        content: String,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
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

    async fn load_history(
        &self,
        room_id: String,
        count: usize,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
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

    async fn load_rooms(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
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

    async fn create_direct_chat(
        &self,
        username: String,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
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

    async fn subscribe_user(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
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

    async fn subscribe_messages(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
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

    async fn get_users_room(&self, room_id: String) -> Result<(), Box<dyn Error + Send + Sync>> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use async_channel::unbounded;
    use serde_json::Value;

    fn compare_json(a: &str, b: &str) {
        assert_eq!(
            serde_json::from_str::<Value>(a).unwrap(),
            serde_json::from_str::<Value>(b).unwrap()
        )
    }

    async fn create_fake_websocket() -> (RocketChatWsWriter, Receiver<tungstenite::Message>) {
        let (tx, rx) = unbounded();
        tx.send(tungstenite::Message::Text("ok".into()))
            .await
            .unwrap();
        tx.send(tungstenite::Message::Text("connect".into()))
            .await
            .unwrap();
        tx.send(tungstenite::Message::Text(r#"{"id": "idtest"}"#.into()))
            .await
            .unwrap();
        let ws = RocketChatWsWriter::new("usertest".into(), "passtest".into(), tx, &rx)
            .await
            .unwrap();
        compare_json(
            &rx.recv().await.unwrap().to_string(),
            r#"{"msg":"connect","version":"1","support":["1"]}"#,
        );

        compare_json(
            &rx.recv().await.unwrap().to_string(),
            r#"
            {
              "msg": "method",
              "method": "login",
              "params": [
                {
                  "user": {
                    "username": "usertest"
                  },
                  "password": {
                    "digest": "b2e6c8f71c847dd0ebc643ca01e2f367d53ff060a8021e7ca1f23f3879e6c0a6",
                    "algorithm": "sha-256"
                  }
                }
              ],
              "id": "1"
            }
            "#,
        );
        (ws, rx)
    }

    #[tokio::test]
    async fn test_init() {
        create_fake_websocket().await;
    }

    #[tokio::test]
    async fn test_login() {
        let (ws, rx) = create_fake_websocket().await;
        ws.login().await.unwrap();
        compare_json(
            &rx.recv().await.unwrap().to_string(),
            r#"
            {
              "msg": "method",
              "method": "login",
              "params": [
                {
                  "user": {
                    "username": "usertest"
                  },
                  "password": {
                    "digest": "b2e6c8f71c847dd0ebc643ca01e2f367d53ff060a8021e7ca1f23f3879e6c0a6",
                    "algorithm": "sha-256"
                  }
                }
              ],
              "id": "1"
            }
            "#,
        );
    }

    #[tokio::test]
    async fn test_pong() {
        let (ws, rx) = create_fake_websocket().await;
        ws.pong().await.unwrap();
        compare_json(
            &rx.recv().await.unwrap().to_string(),
            r#"
            {
              "msg": "pong"
            }
            "#,
        );
    }

    #[tokio::test]
    async fn test_send_message() {
        let (ws, rx) = create_fake_websocket().await;
        ws.send_message("roomtest".into(), "contenttest".into())
            .await
            .unwrap();
        compare_json(
            &rx.recv().await.unwrap().to_string(),
            r#"
            {
                "msg": "method",
                "method": "sendMessage",
                "id": "2",
                "params": [
                    {
                        "rid": "roomtest",
                        "msg": "contenttest"
                    }
                ]
            }
            "#,
        );
    }

    #[tokio::test]
    async fn test_load_history() {
        let (ws, rx) = create_fake_websocket().await;
        ws.load_history("roomtest".into(), 100).await.unwrap();
        compare_json(
            &rx.recv().await.unwrap().to_string(),
            r#"
            {
                "msg": "method",
                "method": "loadHistory",
                "id": "3",
                "params": [ "roomtest", null, 100, null ]
            }
            "#,
        );
    }

    #[tokio::test]
    async fn test_load_rooms() {
        let (ws, rx) = create_fake_websocket().await;
        ws.load_rooms().await.unwrap();
        compare_json(
            &rx.recv().await.unwrap().to_string(),
            r#"
            {
                "msg": "method",
                "method": "rooms/get",
                "id": "4",
                "params": [ { "$date": 0 } ]
            }
            "#,
        );
    }

    #[tokio::test]
    async fn test_create_direct_chat() {
        let (ws, rx) = create_fake_websocket().await;
        ws.create_direct_chat("usertest".into()).await.unwrap();
        compare_json(
            &rx.recv().await.unwrap().to_string(),
            r#"
            {
                "msg": "method",
                "method": "createDirectMessage",
                "id": "5",
                "params": ["usertest"]
            }
            "#,
        );
    }

    #[tokio::test]
    async fn test_subscribe_user() {
        let (ws, rx) = create_fake_websocket().await;
        ws.subscribe_user().await.unwrap();
        compare_json(
            &rx.recv().await.unwrap().to_string(),
            r#"
            {
                "msg": "sub",
                "name": "stream-notify-user",
                "id": "6",
                "params": ["idtest/rooms-changed", false]
            }
            "#,
        );
    }

    #[tokio::test]
    async fn test_subscribe_messages() {
        let (ws, rx) = create_fake_websocket().await;
        ws.subscribe_messages().await.unwrap();
        compare_json(
            &rx.recv().await.unwrap().to_string(),
            r#"
            {
                "msg": "sub",
                "name": "stream-room-messages",
                "id": "7",
                "params": ["__my_messages__", false]
            }
            "#,
        );
    }

    #[tokio::test]
    async fn test_get_user_room() {
        let (ws, rx) = create_fake_websocket().await;
        ws.get_users_room("roomtest".into()).await.unwrap();
        compare_json(
            &rx.recv().await.unwrap().to_string(),
            r#"

            {
                "msg": "method",
                "method": "getUsersOfRoom",
                "params": [
                    "roomtest",
                    true,
                    {
                      "limit": 100,
                      "skip": 0
                    },
                    ""
                ],
                "id": "8"
            }
            "#,
        );
    }
}
