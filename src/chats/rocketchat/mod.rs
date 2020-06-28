pub mod api;
pub mod schema;

use super::super::UI;
use super::super::{Channel, Chat, ChatEvent, Message};
use api::{RocketChatWsWriter, WebSocketWriter};
use async_channel::{Receiver, Sender};
use async_trait::async_trait;
use futures_util::StreamExt;
use log::error;
use schema::*;
use std::error::Error;
use tokio_tungstenite::tungstenite;
use url::Url;

pub struct RocketChat<T: UI + Sync + Send, U: WebSocketWriter + Send + Sync> {
    ui: T,
    ws: U,
    ws_reader: Receiver<tungstenite::Message>,
    ui_tx: Sender<ChatEvent>,
    ponger: Sender<tungstenite::Message>,
    ui_rx: Receiver<ChatEvent>,
    username: String,
}

impl<T> RocketChat<T, RocketChatWsWriter>
where
    T: UI + Send + Sync,
{
    pub async fn new(
        host: Url,
        username: String,
        password: String,
        ui: T,
        ui_tx: Sender<ChatEvent>,
        ui_rx: Receiver<ChatEvent>,
    ) -> Result<Self, Box<dyn Error>> {
        let mut ws_host = host.clone();
        ws_host
            .set_scheme("ws")
            .map_err(|err| format!("{:?}", err))?;
        ws_host.set_path("/websocket");
        let (socket, _) = tokio_tungstenite::connect_async(ws_host).await?;
        let (ws_writer, rxws) = async_channel::unbounded();
        let ponger = ws_writer.clone();
        let (txws, ws_reader) = async_channel::unbounded();
        let (write, mut read) = socket.split();
        tokio::spawn(rxws.map(Ok).forward(write));
        tokio::spawn(async move {
            loop {
                let msg = read.next().await;
                match msg {
                    Some(Ok(msg)) => {
                        if let Err(err) = txws.send(msg).await {
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
        let ws = RocketChatWsWriter::new(username.clone(), password, ws_writer, &ws_reader).await?;
        Ok(RocketChat {
            ui,
            ws,
            ws_reader,
            ui_tx,
            ponger,
            ui_rx,
            username,
        })
    }
}
#[async_trait]
impl<T, U> Chat for RocketChat<T, U>
where
    T: UI + Send + Sync,
    U: WebSocketWriter + Send + Sync,
{
    async fn init_view(&self, channel: Channel) -> Result<(), Box<dyn Error>> {
        let channel_to_switch = channel.clone();
        self.ws
            .load_history(format!("{}", channel_to_switch), 100)
            .await?;
        self.ws.load_rooms().await?;
        self.ws.subscribe_user().await?;
        self.ws
            .get_users_room(format!("{}", channel_to_switch))
            .await?;
        self.ui.select_channel(channel_to_switch)?;
        Ok(())
    }

    async fn send_message(&self, content: String, channel: Channel) -> Result<(), Box<dyn Error>> {
        self.ws
            .send_message(format!("{}", channel), content)
            .await?;
        Ok(())
    }
    async fn wait_for_messages(&self) -> Result<(), Box<dyn Error>> {
        loop {
            let msg = self.ws_reader.recv().await?;
            if let Ok(resp) = serde_json::from_str::<WsResponse>(&format!("{}", msg)[..]) {
                match resp {
                    WsResponse::NewMessage(ms) => {
                        self.ui_tx
                            .send(ChatEvent::RecvMessage(
                                Message {
                                    author: ms.fields.args.1.last_message.u.username.clone(),
                                    content: ms.fields.args.1.last_message.msg.clone(),
                                    datetime: ms.fields.args.1.last_message.ts.date,
                                },
                                Channel::Group(ms.fields.args.1.last_message.rid.clone()),
                            ))
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
                        self.ui.update_messages(messages)?;
                    }

                    WsResponse::Rooms { id, result, .. } if id == "4" => {
                        let channels = result
                            .update
                            .iter()
                            .map(|x| match x {
                                RoomResponseWs::Direct(DirectChatResponseWs { _id, usernames }) => {
                                    let username = usernames
                                        .iter()
                                        .cloned()
                                        .filter(|x| x != &self.username)
                                        .collect::<Vec<String>>();
                                    if username.len() <= 1 && !usernames.is_empty() {
                                        let username = username.get(0).unwrap_or(&self.username);
                                        (username.into(), Channel::User(_id.clone()))
                                    } else {
                                        let all_usernames = username.join(",");
                                        (all_usernames, Channel::User(_id.clone()))
                                    }
                                }
                                RoomResponseWs::Chat(ChatResponseWs { _id, name }) => {
                                    (name.clone(), Channel::Group(_id.clone()))
                                }
                                RoomResponseWs::Private(ChatResponseWs { _id, name }) => {
                                    (name.clone(), Channel::Private(_id.clone()))
                                }
                            })
                            .collect::<Vec<(String, Channel)>>();
                        self.ui.update_channels(channels)?
                    }
                    WsResponse::JoinedRoom { id, result, .. } if id == "5" => {
                        self.ui_tx
                            .send(ChatEvent::Init(Channel::Group(result.rid)))
                            .await?;
                    }
                    WsResponse::UsersInRoom { id, result, .. } if id == "8" => {
                        let users = result
                            .records
                            .iter()
                            .cloned()
                            .map(|x| (x.username, x._id))
                            .collect::<Vec<(String, String)>>();
                        self.ui.update_users_in_room(users)?;
                    }
                    WsResponse::Ping { msg } if msg == "ping" => {
                        self.ponger.send(r#"{"msg": "pong"}"#.into()).await?;
                    }
                    _ => {}
                }
            }
        }
    }

    async fn update_ui(&self) -> Result<(), Box<dyn Error>> {
        loop {
            match self.ui_rx.recv().await {
                Ok(ChatEvent::SendMessage(message, channel)) => {
                    let split = message.split(' ').collect::<Vec<&str>>();
                    if message.starts_with("/direct") && split.len() > 1 {
                        self.ws.create_direct_chat(split[1].into()).await?;
                    } else {
                        self.send_message(message, channel).await?;
                    }
                }
                Ok(ChatEvent::Init(channel)) => {
                    self.init_view(channel).await?;
                }
                Ok(ChatEvent::RecvMessage(message, channel)) => {
                    self.add_message(message, channel).await?;
                }
                Err(_) => continue,
            };
        }
    }

    async fn add_message(&self, message: Message, _channel: Channel) -> Result<(), Box<dyn Error>> {
        self.ui.add_message(message)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::CursiveUI;
    use super::*;
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    #[derive(Clone)]
    struct FakeWsWriter {
        call_map: Arc<Mutex<HashMap<String, usize>>>,
    }
    #[async_trait]
    impl WebSocketWriter for FakeWsWriter {
        async fn init(
            _username: &str,
            _password_digest: &str,
            _websocket: &Sender<tungstenite::Message>,
        ) -> Result<(), Box<dyn Error>> {
            Ok(())
        }

        async fn login(&self) -> Result<(), Box<dyn Error>> {
            let mut call_map = self.call_map.lock().unwrap();
            let current_count = *call_map.get("login").unwrap();
            call_map.insert("login".into(), current_count + 1);
            Ok(())
        }

        async fn connect(_writer: &Sender<tungstenite::Message>) -> Result<(), Box<dyn Error>> {
            Ok(())
        }

        async fn pong(&self) -> Result<(), Box<dyn Error>> {
            Ok(())
        }
        async fn send_message(
            &self,
            _room_id: String,
            _content: String,
        ) -> Result<(), Box<dyn Error>> {
            let mut call_map = self.call_map.lock().unwrap();
            let current_count = *call_map
                .get("send_message")
                .or(Some(&(0 as usize)))
                .unwrap();
            call_map.insert("send_message".into(), current_count + 1);
            Ok(())
        }
        async fn load_history(
            &self,
            _room_id: String,
            _count: usize,
        ) -> Result<(), Box<dyn Error>> {
            Ok(())
        }
        async fn load_rooms(&self) -> Result<(), Box<dyn Error>> {
            Ok(())
        }
        async fn create_direct_chat(&self, _username: String) -> Result<(), Box<dyn Error>> {
            Ok(())
        }
        async fn subscribe_user(&self) -> Result<(), Box<dyn Error>> {
            Ok(())
        }
        async fn subscribe_messages(&self) -> Result<(), Box<dyn Error>> {
            Ok(())
        }
        async fn get_users_room(&self, _room_id: String) -> Result<(), Box<dyn Error>> {
            Ok(())
        }
    }

    impl<T> RocketChat<T, FakeWsWriter>
    where
        T: UI + Send + Sync,
    {
        pub fn new(
            host: Url,
            username: String,
            ui: T,
            ui_tx: Sender<ChatEvent>,
            ui_rx: Receiver<ChatEvent>,
            ws: FakeWsWriter,
        ) -> Result<
            (
                Self,
                Receiver<tungstenite::Message>,
                Sender<tungstenite::Message>,
            ),
            Box<dyn Error>,
        > {
            let mut ws_host = host.clone();
            ws_host
                .set_scheme("ws")
                .map_err(|err| format!("{:?}", err))?;
            ws_host.set_path("/websocket");
            let (ws_writer, rxws) = async_channel::unbounded();
            let ponger = ws_writer.clone();
            let (txws, ws_reader) = async_channel::unbounded();
            Ok((
                RocketChat {
                    ui,
                    ws,
                    ws_reader,
                    ui_tx,
                    ponger,
                    ui_rx,
                    username,
                },
                rxws,
                txws,
            ))
        }
    }

    fn create_chat_system() -> (
        FakeWsWriter,
        RocketChat<CursiveUI, FakeWsWriter>,
        Receiver<tungstenite::Message>,
        Sender<tungstenite::Message>,
    ) {
        let ws = FakeWsWriter {
            call_map: Arc::new(Mutex::new(HashMap::new())),
        };
        let cloned = ws.clone();
        let (tx, rx) = async_channel::unbounded();
        let (chat, rxws, txws) = RocketChat::<CursiveUI, FakeWsWriter>::new(
            Url::parse("http://localhost").unwrap(),
            "usertest".into(),
            CursiveUI::new(cursive::dummy().cb_sink().clone()),
            tx,
            rx,
            ws,
        )
        .unwrap();
        (cloned, chat, rxws, txws)
    }

    #[tokio::test]
    async fn test_send_message() {
        let (ws, chat, _rxws, _txws) = create_chat_system();
        chat.send_message("test".to_string(), Channel::Group("test".to_string()))
            .await
            .unwrap();
        assert_eq!(
            ws.call_map
                .lock()
                .unwrap()
                .get("send_message".into())
                .unwrap(),
            &1
        );
    }
}
