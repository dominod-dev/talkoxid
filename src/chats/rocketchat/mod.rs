mod api;
mod schema;

use super::super::core::{Channel, Chat, ChatEvent, Message, UIEvent};
use api::{RocketChatWsWriter, WebSocketWriter};
use async_channel::{unbounded, Receiver, Sender};
use async_trait::async_trait;
use async_tungstenite::tungstenite;
use futures_util::StreamExt;
use log::error;
use schema::*;
use std::error::Error;
use std::sync::Mutex;
use tokio_native_tls::TlsConnector;
use url::Url;

fn resolve_ws_url(
    mut url: Url,
    ssl_verify: bool,
) -> Result<(Url, Option<TlsConnector>), Box<dyn Error + Send + Sync>> {
    let (scheme, tls_config) = match url.scheme() {
        "https" => {
            let mut tls_builder = native_tls::TlsConnector::builder();
            if !ssl_verify {
                tls_builder
                    .danger_accept_invalid_certs(true)
                    .danger_accept_invalid_hostnames(true);
            }
            let tls_config = TlsConnector::from(tls_builder.build()?);
            ("wss", Some(tls_config))
        }
        _ => ("ws", None),
    };
    url.set_scheme(scheme).map_err(|err| format!("{:?}", err))?;
    url.set_path("/websocket");
    Ok((url, tls_config))
}

/// RocketChat chat system.
///
/// This type is a chat system implementation for RocketChat.
pub struct RocketChat<U: WebSocketWriter + Send + Sync> {
    tx_ui: Sender<UIEvent>,
    ws: U,
    rx_ws: Receiver<tungstenite::Message>,
    ponger: Sender<tungstenite::Message>,
    rx_chat: Receiver<ChatEvent>,
    username: String,
    current_channel: Mutex<Option<Channel>>,
}

impl<U> RocketChat<U>
where
    U: WebSocketWriter + Send + Sync,
{
    async fn wait_messages_loop(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        loop {
            let msg = self.rx_ws.recv().await?;
            if let Ok(resp) = serde_json::from_str::<WsResponse>(&format!("{}", msg)[..]) {
                match resp {
                    WsResponse::NewMessage(SocketMessageWs {
                        fields:
                            SocketArgsWs {
                                args: SocketEventResponseWs(_, EventResponseWs { last_message, t }),
                                ..
                            },
                        ..
                    }) => {
                        let channel = match t {
                            x if x == "d" => Channel::User(last_message.rid.clone()),
                            x if x == "p" => Channel::Private(last_message.rid.clone()),
                            _ => Channel::Group(last_message.rid.clone()),
                        };
                        self.add_message(
                            Message {
                                author: last_message.u.username.clone(),
                                content: last_message.msg.clone(),
                                datetime: last_message.ts.date,
                            },
                            &channel,
                        )
                        .await?;
                    }
                    WsResponse::History { id, result, .. } if id == "3" => {
                        let messages =
                            result.messages.iter().rev().fold(String::from(""), |x, y| {
                                format!(
                                    "{}{}\n",
                                    x,
                                    Message {
                                        content: y.msg.clone(),
                                        author: y.u.username.clone(),
                                        datetime: y.ts.date
                                    }
                                )
                            });
                        self.tx_ui.send(UIEvent::UpdateMessages(messages)).await?;
                    }

                    WsResponse::Rooms { id, result, .. } if id == "4" => {
                        let channels = result
                            .update
                            .iter()
                            .map(|x| match x {
                                RoomResponseWs::Direct(DirectChatResponseWs { _id, usernames }) => {
                                    let all_usernames = usernames
                                        .iter()
                                        .cloned()
                                        .filter(|x| x != &self.username || usernames.len() == 1)
                                        .collect::<Vec<String>>()
                                        .join(",");
                                    (all_usernames, Channel::User(_id.clone()))
                                }
                                RoomResponseWs::Chat(ChatResponseWs { _id, name }) => {
                                    (name.clone(), Channel::Group(_id.clone()))
                                }
                                RoomResponseWs::Private(ChatResponseWs { _id, name }) => {
                                    (name.clone(), Channel::Private(_id.clone()))
                                }
                            })
                            .collect::<Vec<(String, Channel)>>();
                        self.tx_ui.send(UIEvent::UpdateChannels(channels)).await?;
                    }
                    WsResponse::JoinedRoom { id, result, .. } if id == "5" => match result {
                        JoinedRoomResponseWs::Direct(result) => {
                            self.init_view(Channel::User(result.rid.clone())).await?;
                        }
                        JoinedRoomResponseWs::Chat(result) => {
                            self.init_view(Channel::Group(result.rid.clone())).await?;
                        }
                        JoinedRoomResponseWs::Private(result) => {
                            self.init_view(Channel::Private(result.rid.clone())).await?;
                        }
                    },
                    WsResponse::UsersInRoom { id, result, .. } if id == "8" => {
                        let users = result
                            .records
                            .iter()
                            .cloned()
                            .map(|x| (x.username, x._id))
                            .collect::<Vec<(String, String)>>();
                        self.tx_ui.send(UIEvent::UpdateUsersInRoom(users)).await?;
                    }
                    WsResponse::Ping { msg } if msg == "ping" => {
                        self.ponger.send(r#"{"msg": "pong"}"#.into()).await?;
                    }
                    _ => {}
                }
            }
        }
    }

    async fn ui_event_loop(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        loop {
            match self.rx_chat.recv().await? {
                ChatEvent::SendMessage(message, channel) => {
                    let split = message.split(' ').collect::<Vec<&str>>();
                    if message.starts_with("/direct") && split.len() > 1 {
                        self.ws.create_direct_chat(split[1].into()).await?;
                    } else {
                        self.send_message(message, channel).await?;
                    }
                }
                ChatEvent::Init(channel) => {
                    self.init_view(channel).await?;
                }
            };
        }
    }
}

impl RocketChat<RocketChatWsWriter> {
    pub async fn new(
        host: Url,
        username: String,
        password: String,
        ssl_verify: bool,
        tx_ui: Sender<UIEvent>,
        rx_chat: Receiver<ChatEvent>,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let (ws_host, tls_config) = resolve_ws_url(host.clone(), ssl_verify)?;
        let (socket, _) =
            async_tungstenite::tokio::connect_async_with_tls_connector(ws_host, tls_config).await?;
        let (tx_ws, rx_forwarder_ws) = unbounded();
        let ponger = tx_ws.clone();
        let (tx_forwarder_ws, rx_ws) = unbounded();
        let (write, mut read) = socket.split();
        tokio::spawn(rx_forwarder_ws.map(Ok).forward(write));
        tokio::spawn(async move {
            loop {
                let msg = read.next().await;
                match msg {
                    Some(Ok(msg)) => {
                        if let Err(err) = tx_forwarder_ws.send(msg).await {
                            error!("Error when sending to ws sender: {}", err);
                            break;
                        }
                    }
                    Some(Err(err)) => {
                        error!("Error when reading websocket: {}", err);
                        break;
                    }
                    None => {
                        error!("No message when reading websocket");
                        break;
                    }
                }
            }
        });
        let ws = RocketChatWsWriter::new(username.clone(), password, tx_ws, &rx_ws).await?;
        Ok(RocketChat {
            tx_ui,
            ws,
            rx_ws,
            ponger,
            rx_chat,
            username,
            current_channel: Mutex::new(None),
        })
    }
}
#[async_trait]
impl<U> Chat for RocketChat<U>
where
    U: WebSocketWriter + Send + Sync,
{
    async fn init_view(&self, channel: Channel) -> Result<(), Box<dyn Error + Send + Sync>> {
        let channel_to_switch = channel.clone();
        self.ws
            .load_history(format!("{}", channel_to_switch), 100)
            .await?;
        self.ws.load_rooms().await?;
        self.ws.subscribe_user().await?;
        self.ws
            .get_users_room(format!("{}", channel_to_switch))
            .await?;
        self.tx_ui
            .send(UIEvent::SelectChannel(channel_to_switch))
            .await?;
        let mut current_channel = self.current_channel.lock().unwrap();
        *current_channel = Some(channel);
        Ok(())
    }

    async fn send_message(
        &self,
        content: String,
        channel: Channel,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.ws
            .send_message(format!("{}", channel), content)
            .await?;
        Ok(())
    }

    async fn start_loop(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let read_loop = self.wait_messages_loop();
        let ui_loop = self.ui_event_loop();
        tokio::select! {
            _ = read_loop => {},
            _ = ui_loop => {},
        }
        Ok(())
    }

    async fn add_message(
        &self,
        message: Message,
        channel: &Channel,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let current_channel = self.current_channel.lock().unwrap().clone();
        if let Some(current) = current_channel.as_ref() {
            if channel == current {
                self.tx_ui.send(UIEvent::AddMessages(message)).await?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::{TimeZone, Utc};
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    struct FakeWsWriter {
        call_map: Arc<Mutex<HashMap<String, Vec<Vec<String>>>>>,
    }
    #[async_trait]
    impl WebSocketWriter for FakeWsWriter {
        async fn init(
            _username: &str,
            _password_digest: &str,
            _websocket: &Sender<tungstenite::Message>,
        ) -> Result<(), Box<dyn Error + Send + Sync>> {
            Ok(())
        }

        async fn login(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
            let mut call_map = self.call_map.lock().unwrap();
            let mut current_vec = call_map.get("login").or(Some(&vec![])).unwrap().clone();
            current_vec.push(vec![]);
            call_map.insert("login".into(), current_vec);
            Ok(())
        }

        async fn connect(
            _writer: &Sender<tungstenite::Message>,
        ) -> Result<(), Box<dyn Error + Send + Sync>> {
            Ok(())
        }

        async fn pong(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
            let mut call_map = self.call_map.lock().unwrap();
            let mut current_vec = call_map.get("pong").or(Some(&vec![])).unwrap().clone();
            current_vec.push(vec![]);
            call_map.insert("pong".into(), current_vec);
            Ok(())
        }
        async fn send_message(
            &self,
            room_id: String,
            content: String,
        ) -> Result<(), Box<dyn Error + Send + Sync>> {
            let mut call_map = self.call_map.lock().unwrap();
            let mut current_vec = call_map
                .get("send_message")
                .or(Some(&vec![]))
                .unwrap()
                .clone();
            current_vec.push(vec![room_id, content]);
            call_map.insert("send_message".into(), current_vec);
            Ok(())
        }
        async fn load_history(
            &self,
            room_id: String,
            count: usize,
        ) -> Result<(), Box<dyn Error + Send + Sync>> {
            let mut call_map = self.call_map.lock().unwrap();
            let mut current_vec = call_map
                .get("load_history")
                .or(Some(&vec![]))
                .unwrap()
                .clone();
            current_vec.push(vec![room_id, format!("{}", count)]);
            call_map.insert("load_history".into(), current_vec);
            Ok(())
        }
        async fn load_rooms(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
            let mut call_map = self.call_map.lock().unwrap();
            let mut current_vec = call_map
                .get("load_rooms")
                .or(Some(&vec![]))
                .unwrap()
                .clone();
            current_vec.push(vec![]);
            call_map.insert("load_rooms".into(), current_vec);
            Ok(())
        }
        async fn create_direct_chat(
            &self,
            username: String,
        ) -> Result<(), Box<dyn Error + Send + Sync>> {
            let mut call_map = self.call_map.lock().unwrap();
            let mut current_vec = call_map
                .get("create_direct_chat")
                .or(Some(&vec![]))
                .unwrap()
                .clone();
            current_vec.push(vec![username]);
            call_map.insert("create_direct_chat".into(), current_vec);
            Ok(())
        }
        async fn subscribe_user(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
            let mut call_map = self.call_map.lock().unwrap();
            let mut current_vec = call_map
                .get("subscribe_user")
                .or(Some(&vec![]))
                .unwrap()
                .clone();
            current_vec.push(vec![]);
            call_map.insert("subscribe_user".into(), current_vec);
            Ok(())
        }
        async fn subscribe_messages(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
            let mut call_map = self.call_map.lock().unwrap();
            let mut current_vec = call_map
                .get("subscribe_messages")
                .or(Some(&vec![]))
                .unwrap()
                .clone();
            current_vec.push(vec![]);
            call_map.insert("subscribe_messages".into(), current_vec);
            Ok(())
        }
        async fn get_users_room(
            &self,
            _room_id: String,
        ) -> Result<(), Box<dyn Error + Send + Sync>> {
            let mut call_map = self.call_map.lock().unwrap();
            let mut current_vec = call_map
                .get("get_users_room")
                .or(Some(&vec![]))
                .unwrap()
                .clone();
            current_vec.push(vec![_room_id]);
            call_map.insert("get_users_room".into(), current_vec);
            Ok(())
        }
    }

    impl RocketChat<FakeWsWriter> {
        pub fn new(
            host: Url,
            username: String,
            tx_ui: Sender<UIEvent>,
            rx_ui: Receiver<ChatEvent>,
            ws: FakeWsWriter,
        ) -> Result<(Self, Sender<tungstenite::Message>), Box<dyn Error + Send + Sync>> {
            let mut ws_host = host.clone();
            ws_host
                .set_scheme("ws")
                .map_err(|err| format!("{:?}", err))?;
            ws_host.set_path("/websocket");
            let (tx_ws, _) = unbounded();
            let ponger = tx_ws.clone();
            let (tx_forwarder_ws, rx_ws) = unbounded();
            Ok((
                RocketChat {
                    tx_ui,
                    ws,
                    rx_ws,
                    ponger,
                    rx_chat: rx_ui,
                    username,
                    current_channel: Mutex::new(Some(Channel::Group("test_channel".to_string()))),
                },
                tx_forwarder_ws,
            ))
        }
    }

    fn create_chat_system() -> (
        FakeWsWriter,
        Receiver<UIEvent>,
        RocketChat<FakeWsWriter>,
        Sender<tungstenite::Message>,
    ) {
        let ws = FakeWsWriter {
            call_map: Arc::new(Mutex::new(HashMap::new())),
        };
        let cloned_ws = ws.clone();
        let (_, rx_ws) = unbounded();
        let (tx_ui, rx_ui) = unbounded();
        let (chat, tx_ws) = RocketChat::<FakeWsWriter>::new(
            Url::parse("http://localhost").unwrap(),
            "usertest".into(),
            tx_ui,
            rx_ws,
            ws,
        )
        .unwrap();
        (cloned_ws, rx_ui, chat, tx_ws)
    }

    #[tokio::test]
    async fn test_send_message() {
        let (ws, _, chat, _) = create_chat_system();
        chat.send_message(
            "test".to_string(),
            Channel::Group("test_channel".to_string()),
        )
        .await
        .unwrap();
        assert_eq!(
            ws.call_map
                .lock()
                .unwrap()
                .get("send_message".into())
                .unwrap()[0],
            vec!["test_channel".to_string(), "test".to_string()]
        );
    }

    #[tokio::test]
    async fn test_add_message() {
        let (_, rx_ui, chat, _) = create_chat_system();
        chat.add_message(
            Message {
                author: "testauthor".into(),
                content: "testcontent".into(),
                datetime: Utc.timestamp_millis(1593435867123),
            },
            &Channel::Group("test_channel".to_string()),
        )
        .await
        .unwrap();
        if let UIEvent::AddMessages(msg) = rx_ui.try_recv().unwrap() {
            assert_eq!(
                msg,
                Message {
                    author: "testauthor".into(),
                    content: "testcontent".into(),
                    datetime: Utc.timestamp_millis(1593435867123),
                }
            );
        } else {
            panic!();
        }
    }

    #[tokio::test]
    async fn test_add_message_not_current_channel() {
        let (_, rx_ui, chat, _) = create_chat_system();
        chat.add_message(
            Message {
                author: "testauthor".into(),
                content: "testcontent".into(),
                datetime: Utc.timestamp_millis(1593435867123),
            },
            &Channel::Group("other_channel".to_string()),
        )
        .await
        .unwrap();
        assert!(rx_ui.try_recv().is_err());
    }

    #[tokio::test]
    async fn test_init() {
        let (ws, _rx_ui, chat, _) = create_chat_system();
        chat.init_view(Channel::Group("test_channel".to_string()))
            .await
            .unwrap();
        let ws_call_map = ws.call_map.lock().unwrap();
        assert_eq!(
            ws_call_map.get("load_history".into()).unwrap()[0],
            vec!["test_channel".to_string(), "100".to_string()]
        );
        assert_eq!(
            ws_call_map.get("load_rooms".into()).unwrap()[0],
            Vec::<String>::new()
        );
        assert_eq!(
            ws_call_map.get("subscribe_user".into()).unwrap()[0],
            Vec::<String>::new()
        );
        assert_eq!(
            ws_call_map.get("get_users_room".into()).unwrap()[0],
            vec!["test_channel".to_string()]
        );
    }

    #[tokio::test]
    async fn test_recv_message() {
        let (_, rx_ui, chat, tx_forwarder_ws) = create_chat_system();
        let message_str =
            std::include_str!("../../../tests/data/test_recv_message.json").to_string();
        let message_loop = chat.wait_messages_loop();
        tx_forwarder_ws
            .send(tungstenite::Message::Text(message_str))
            .await
            .unwrap();
        let msg = rx_ui.recv();
        tokio::select! {
            _ = message_loop => {panic!("Abnormal")},
            Ok(UIEvent::AddMessages(message)) = msg => {
                assert_eq!(
                    message,
                    Message { author: "testauthor".into(), content: "testcontent".into(), datetime: Utc.timestamp_millis(1593435867123) }
                );
            },
        };
    }

    #[tokio::test]
    async fn test_recv_history() {
        let (_, rx_ui, chat, tx_forwarder_ws) = create_chat_system();
        let message_str =
            std::include_str!("../../../tests/data/test_recv_history.json").to_string();
        let expected_str =
            std::include_str!("../../../tests/data/test_recv_history.txt").to_string();
        let message_loop = chat.wait_messages_loop();
        tx_forwarder_ws
            .send(tungstenite::Message::Text(message_str))
            .await
            .unwrap();
        let msg = rx_ui.recv();
        tokio::select! {
            Ok(UIEvent::UpdateMessages(messages)) = msg => {
                assert_eq!(
                    format!("{}", messages).trim(),
                    expected_str.to_string().trim()
                );
            },
            _ = message_loop => {panic!("Abnormal")},
        };
    }

    #[tokio::test]
    async fn test_recv_rooms() {
        let (_, rx_ui, chat, tx_forwarder_ws) = create_chat_system();
        let message_str = std::include_str!("../../../tests/data/test_recv_rooms.json").to_string();
        let expected_str = std::include_str!("../../../tests/data/test_recv_rooms.txt").to_string();
        let message_loop = chat.wait_messages_loop();
        tx_forwarder_ws
            .send(tungstenite::Message::Text(message_str))
            .await
            .unwrap();

        let msg = rx_ui.recv();
        tokio::select! {
            Ok(UIEvent::UpdateChannels(channels)) = msg => {
                assert_eq!(
                    format!("{:?}", channels),
                    expected_str.to_string().trim()
                );
            },
            _ = message_loop => {panic!("Abnormal")},
        };
    }

    #[tokio::test]
    async fn test_recv_users_in_room() {
        let (_, rx_ui, chat, tx_forwarder_ws) = create_chat_system();
        let message_str =
            std::include_str!("../../../tests/data/test_recv_users_in_room.json").to_string();
        let expected_str =
            std::include_str!("../../../tests/data/test_recv_users_in_room.txt").to_string();
        let message_loop = chat.wait_messages_loop();
        tx_forwarder_ws
            .send(tungstenite::Message::Text(message_str))
            .await
            .unwrap();
        let msg = rx_ui.recv();
        tokio::select! {
            Ok(UIEvent::UpdateUsersInRoom(users)) = msg => {
                assert_eq!(
                    format!("{:?}", users),
                    expected_str.to_string().trim()
                );
            },
            _ = message_loop => {panic!("Abnormal")},
        };
    }

    #[tokio::test]
    async fn test_recv_users_in_room_me() {
        let (_, rx_ui, chat, tx_forwarder_ws) = create_chat_system();
        let message_str =
            std::include_str!("../../../tests/data/test_recv_users_in_room_me.json").to_string();
        let message_loop = chat.wait_messages_loop();
        tx_forwarder_ws
            .send(tungstenite::Message::Text(message_str))
            .await
            .unwrap();
        let msg = rx_ui.recv();
        tokio::select! {
            Ok(UIEvent::UpdateUsersInRoom(users)) = msg => {
                assert_eq!(
                    format!("{:?}", users),
                    "[(\"usertest\", \"PqJNPhCjTElGpKtL3\")]".trim()
                );
            },
            _ = message_loop => {panic!("Abnormal")},
        };
    }

    #[tokio::test]
    async fn test_recv_users_in_room_one_not_me() {
        let (_, rx_ui, chat, tx_forwarder_ws) = create_chat_system();
        let message_str =
            std::include_str!("../../../tests/data/test_recv_users_in_room_one_not_me.json")
                .to_string();
        let message_loop = chat.wait_messages_loop();
        tx_forwarder_ws
            .send(tungstenite::Message::Text(message_str))
            .await
            .unwrap();
        let msg = rx_ui.recv();
        tokio::select! {
            Ok(UIEvent::UpdateUsersInRoom(users)) = msg => {
                assert_eq!(
                    format!("{:?}", users),
                    "[(\"someone\", \"eqJNPhCjTEyGpKtL3\")]".trim()
                );
            },
            _ = message_loop => {panic!("Abnormal")},
        };
    }
}
