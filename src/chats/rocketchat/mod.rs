pub mod api;
pub mod schema;
use super::super::UI;
use super::super::{Channel, Chat, ChatEvent, Message};
use api::{RocketChatApi, RocketChatWs};
use schema::*;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use url::Url;

pub struct RocketChat {
    api: RocketChatApi,
    pub ws: Arc<Mutex<RocketChatWs>>,
    pub current_channel: Option<Channel>,
    ui: Box<dyn UI + Send>,
}

impl RocketChat {
    pub fn new(host: Url, username: String, password: String, ui: Box<dyn UI + Send>) -> Self {
        let mut api = RocketChatApi::new(host.clone(), username.clone(), password.clone());
        let user_id = api.login().unwrap();
        let ws = Arc::new(Mutex::new(RocketChatWs::new(
            host, username, password, user_id,
        )));
        ws.lock().unwrap().login();
        RocketChat {
            api,
            ws,
            current_channel: None,
            ui,
        }
    }
}

impl Chat for RocketChat {
    fn init_view(&mut self, channel: Channel) {
        let channels = self.api.channels().unwrap();

        let users = self.api.users().unwrap();
        let channel_to_switch = channel.clone();
        self.current_channel = Some(channel);
        let mut messages = self
            .api
            .history(format!("{}", channel_to_switch), 100)
            .unwrap();
        messages.sort_by(|a, b| a.ts.partial_cmp(&b.ts).unwrap());
        let messages = messages.iter().fold(String::from(""), |x, y| {
            format!("{}[{}]: {}\n", x, y.u.username.clone(), y.msg)
        });
        self.ui.update_messages(messages);
        self.ui.update_channels(
            channels
                .iter()
                .map(|x| (x.name.clone(), Channel::Group(x._id.clone())))
                .collect::<Vec<(String, Channel)>>(),
        );
        self.ui.update_users(
            users
                .iter()
                .map(|x| (x.name.clone(), Channel::User(x.name.clone())))
                .collect::<Vec<(String, Channel)>>(),
        );
    }

    fn send_message(&self, content: String) {
        let channel_to_send = match &self.current_channel {
            Some(channel) => channel.clone(),
            None => Channel::Group("GENERAL".to_string()),
        };
        self.api
            .send_message(format!("{}", channel_to_send), content)
            .unwrap();
    }

    fn add_message(&self, message: Message, channel: Channel) {
        if let Some(current_channel) = &self.current_channel {
            if &channel == current_channel {
                self.ui.add_message(message);
            }
        }
    }
}

pub struct ChatServer {
    pub chat_system: Arc<Mutex<RocketChat>>,
}

impl ChatServer {
    pub fn start(&self) -> mpsc::Sender<ChatEvent> {
        let chat_system = Arc::clone(&self.chat_system);
        let (tx, rx) = mpsc::channel();
        let tx2 = mpsc::Sender::clone(&tx);
        thread::spawn(move || {
            let mut chat_system = chat_system.lock().unwrap();
            let ws = Arc::clone(&chat_system.ws);
            thread::spawn(move || {
                let mut ws = ws.lock().unwrap();
                ws.subscribe_user();
                loop {
                    let msg = ws.read().unwrap();
                    if let Ok(ms) = serde_json::from_str::<SocketMessageWs>(&format!("{}", msg)[..])
                    {
                        match tx2.send(ChatEvent::RecvMessage(
                            Message {
                                author: ms.fields.args.1.last_message.u.username.clone(),
                                content: ms.fields.args.1.last_message.msg.clone(),
                            },
                            Channel::Group(ms.fields.args.1.last_message.rid.clone()),
                        )) {
                            Ok(_) => {}
                            Err(e) => {
                                println!("{}", e);
                            }
                        };
                    };
                    ws.pong();
                }
            });
            chat_system.init_view(Channel::Group("GENERAL".to_string()));
            loop {
                match rx.recv() {
                    Ok(ChatEvent::SendMessage(message)) => {
                        chat_system.send_message(message);
                    }
                    Ok(ChatEvent::Init(channel)) => {
                        chat_system.init_view(channel);
                    }
                    Ok(ChatEvent::RecvMessage(message, channel)) => {
                        chat_system.add_message(message, channel);
                    }
                    Err(_) => continue,
                };
            }
        });
        tx
    }
}
