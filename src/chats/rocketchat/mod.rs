pub mod api;
pub mod schema;
use super::super::views::BufferView;
use super::super::{Channel, Chat, ChatEvent, Message};
use api::RocketChatApi;
use cursive::views::SelectView;
use cursive::CbSink;
use cursive::Cursive;
use schema::*;
use sha2::{Digest, Sha256};
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use tungstenite;
use tungstenite::connect;
use url::Url;

pub struct RocketChat {
    api: RocketChatApi,
    cb_sink: CbSink,
    pub current_channel: Option<Channel>,
}

impl RocketChat {
    pub fn new(host: Url, username: String, password: String, cb_sink: CbSink) -> Self {
        let mut api = RocketChatApi::new(host, username, password);
        api.login().unwrap();
        RocketChat {
            api,
            cb_sink,
            current_channel: None,
        }
    }
}

impl Chat for RocketChat {
    fn channels(&self) -> Vec<String> {
        vec![]
    }

    fn users(&self) -> Vec<String> {
        vec![]
    }

    fn init_view(&mut self, channel: Channel) {
        let channels = self.api.channels().unwrap();

        let users = self.api.users().unwrap();
        let channeltmp = match channel.clone() {
            Channel::Group(e) if e == String::from("SWITCH") => {
                self.current_channel.clone().unwrap()
            }
            _ => {
                let return_channel = channel.clone();
                self.current_channel = Some(channel);
                return_channel
            }
        };
        let mut messages = self.api.history(format!("{}", channeltmp), 100).unwrap();
        messages.sort_by(|a, b| a.ts.partial_cmp(&b.ts).unwrap());
        let messages = messages.iter().fold(String::from(""), |x, y| {
            format!("{}[{}]: {}\n", x, y.u.username.clone(), y.msg)
        });
        self.cb_sink
            .send(Box::new(move |siv: &mut Cursive| {
                siv.call_on_name("chat", move |view: &mut BufferView| view.init(messages));
                siv.call_on_name("channel_list", move |view: &mut SelectView<Channel>| {
                    view.clear();
                    view.add_all(
                        channels
                            .iter()
                            .map(|x| (&x.name[..], Channel::Group(x._id.clone())))
                            .collect::<Vec<(&str, Channel)>>(),
                    );
                });
                siv.call_on_name("users_list", move |view: &mut SelectView<Channel>| {
                    view.clear();
                    view.add_all(
                        users
                            .iter()
                            .map(|x| (&x.name[..], Channel::User(x.name.clone())))
                            .collect::<Vec<(&str, Channel)>>(),
                    );
                });
            }))
            .unwrap();
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

    fn display_message(&self, message: Message) {
        if let Some(_) = &self.current_channel {
            self.cb_sink
                .send(Box::new(|siv: &mut Cursive| {
                    siv.call_on_name("chat", move |view: &mut BufferView| {
                        view.add_message(format!("[{}]: {}\n", message.author, message.content))
                    });
                }))
                .unwrap();
        }
    }
    fn add_message(&self, message: Message, channel: Channel) {
        if let Some(current_channel) = &self.current_channel {
            if &channel == current_channel {
                self.display_message(message);
            }
        }
    }
}

pub struct ChatServer {
    pub chat_system: Arc<Mutex<dyn Chat + Send>>,
}

impl ChatServer {
    pub fn start(&self) -> mpsc::Sender<ChatEvent> {
        let chat_system = Arc::clone(&self.chat_system);
        let (tx, rx) = mpsc::channel();
        let tx2 = mpsc::Sender::clone(&tx);
        thread::spawn(move || {
            let mut chat_system = chat_system.lock().unwrap();
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
                        // chat_system.init_view(Channel::Group("SWITCH".to_string()));
                        chat_system.add_message(message, channel);
                    }
                    Err(_) => continue,
                };
            }
        });

        thread::spawn(move || {
            let username = "admin";
            let password = "admin";
            let mut hasher = Sha256::new();
            hasher.update(password);
            let password_digest = format!("{:x}", hasher.finalize());

            let (mut socket, _) = connect(Url::parse("ws://localhost:3000/websocket").unwrap())
                .expect("Can't connect");
            let login = LoginWs {
                msg: "method".into(),
                method: "login".into(),
                id: "42".into(),
                params: vec![LoginParamsWs {
                    user: UsernameWs {
                        username: username.into(),
                    },
                    password: PasswordWs {
                        digest: password_digest,
                        algorithm: "sha-256".into(),
                    },
                }],
            };
            let connect = ConnectWs {
                msg: "connect".into(),
                version: "1".into(),
                support: vec!["1".into()],
            };
            let sub = SubStreamChannelWs {
                msg: "sub".into(),
                id: "1234".into(),
                name: "stream-notify-user".into(),
                params: vec![
                    serde_json::json!("wqJNPhCkTEnGpKtL3/rooms-changed"),
                    serde_json::json!(false),
                ],
            };
            let pong = PongWs { msg: "pong".into() };
            socket
                .write_message(tungstenite::Message::Text(
                    serde_json::to_string(&connect).unwrap(),
                ))
                .unwrap();
            socket
                .write_message(tungstenite::Message::Text(
                    serde_json::to_string(&login).unwrap(),
                ))
                .unwrap();
            socket
                .write_message(tungstenite::Message::Text(
                    serde_json::to_string(&sub).unwrap(),
                ))
                .unwrap();
            loop {
                let msg = socket.read_message().expect("Error reading message");
                if let Ok(ms) = serde_json::from_str::<SocketMessageWs>(&format!("{}", msg)[..]) {
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
                socket
                    .write_message(tungstenite::Message::Text(
                        serde_json::to_string(&pong).unwrap(),
                    ))
                    .unwrap();
            }
        });
        tx
    }
}
