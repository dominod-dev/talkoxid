pub mod api;
use async_trait::async_trait;
pub mod schema;
use super::super::UI;
use super::super::{Channel, Chat, ChatEvent, Message};
use api::{RocketChatApi, RocketChatWsWriter};
use async_channel::{Receiver, Sender};
use futures_util::StreamExt;
use log::info;
use schema::*;
use tokio_tungstenite::tungstenite;
use url::Url;

pub struct RocketChat {
    api: RocketChatApi,
    ui: Box<dyn UI + Send + Sync>,
    ws: api::RocketChatWsWriter,
    ws_reader: Receiver<tungstenite::Message>,
    ui_tx: Sender<ChatEvent>,
    ponger: Sender<tungstenite::Message>,
    ui_rx: Receiver<ChatEvent>,
}

impl RocketChat {
    pub async fn new(
        host: Url,
        username: String,
        password: String,
        ui: Box<dyn UI + Send + Sync>,
        ui_tx: Sender<ChatEvent>,
        ui_rx: Receiver<ChatEvent>,
    ) -> Self {
        let mut api = RocketChatApi::new(host.clone(), username.clone(), password.clone());
        api.login().await.unwrap();
        let mut ws_host = host.clone();
        ws_host.set_scheme("ws").unwrap();
        ws_host.set_path("/websocket");
        let (socket, _) = tokio_tungstenite::connect_async(ws_host).await.unwrap();
        let (ws_writer, rxws) = async_channel::unbounded();
        let ponger = ws_writer.clone();
        let (txws, ws_reader) = async_channel::unbounded();
        let (write, mut read) = socket.split();
        tokio::spawn(rxws.map(Ok).forward(write));
        tokio::spawn(async move {
            loop {
                let msg = read.next().await.unwrap().unwrap();
                txws.send(msg).await.unwrap();
            }
        });
        let ws = RocketChatWsWriter::new(
            "collkid".to_string(),
            "collkid".to_string(),
            ws_writer,
            &ws_reader,
        )
        .await;
        RocketChat {
            api,
            ui,
            ws,
            ws_reader,
            ui_tx,
            ponger,
            ui_rx,
        }
    }
}
#[async_trait]
impl Chat for RocketChat {
    async fn init_view(&self, channel: Channel) -> Result<(), String> {
        let channels = self.api.rooms().await?;
        let users = self.api.users().await?;
        let channel_to_switch = channel.clone();
        let mut messages = self
            .api
            .history(format!("{}", channel_to_switch), 100)
            .await?;
        messages.sort_by(|a, b| a.ts.partial_cmp(&b.ts).unwrap());
        let messages = messages.iter().fold(String::from(""), |x, y| {
            format!("{}[{}]: {}\n", x, y.u.username.clone(), y.msg)
        });
        self.ui.update_messages(messages);
        self.ui.update_channels(
            channels
                .iter()
                .map(|x| match (&x.name, &x.usernames) {
                    (Some(name), _) => (name.clone(), Channel::Group(x._id.clone())),
                    (None, Some(username)) => {
                        (format!("{:?}", username), Channel::Group(x._id.clone()))
                    }
                    _ => (x._id.clone(), Channel::Group(x._id.clone())),
                })
                .collect::<Vec<(String, Channel)>>(),
            Some(channel),
        );
        self.ui.update_users(
            users
                .iter()
                .map(|x| (x.name.clone(), Channel::User(x.name.clone())))
                .collect::<Vec<(String, Channel)>>(),
        );
        self.ws.subscribe_user().await;
        self.ui.select_channel(channel_to_switch);
        Ok(())
    }

    async fn send_message(&self, content: String, channel: Channel) -> Result<(), String> {
        self.ws.send_message(format!("{}", channel), content).await;
        Ok(())
    }
    async fn wait_for_messages(&self) -> Result<(), String> {
        loop {
            let msg = self.ws_reader.recv().await.unwrap();
            if let Ok(ms) = serde_json::from_str::<SocketMessageWs>(&format!("{}", msg)[..]) {
                &self
                    .ui_tx
                    .send(ChatEvent::RecvMessage(
                        Message {
                            author: ms.fields.args.1.last_message.u.username.clone(),
                            content: ms.fields.args.1.last_message.msg.clone(),
                        },
                        Channel::Group(ms.fields.args.1.last_message.rid.clone()),
                    ))
                    .await
                    .unwrap_or_else(|err| info!("{:?}", err));
            }
            self.ponger.send(r#"{"msg": "pong"}"#.into()).await.unwrap();
        }
    }

    async fn update_ui(&self) -> Result<(), String> {
        loop {
            match self.ui_rx.recv().await {
                Ok(ChatEvent::SendMessage(message, channel)) => {
                    self.send_message(message, channel).await.unwrap();
                }
                Ok(ChatEvent::Init(channel)) => {
                    info!("INIT {}", channel);
                    self.init_view(channel).await.unwrap();
                }
                Ok(ChatEvent::RecvMessage(message, channel)) => {
                    info!("Rcv {} {}", message, channel);
                    self.add_message(message, channel).await;
                }
                Err(_) => continue,
            };
        }
    }

    async fn add_message(&self, message: Message, _channel: Channel) {
        self.ui.add_message(message);
    }
}
