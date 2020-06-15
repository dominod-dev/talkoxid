use super::super::views::BufferView;
use super::super::{Channel, Chat, ChatEvent, Message};
use cursive::views::SelectView;
use cursive::CbSink;
use cursive::Cursive;
use reqwest;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use tungstenite;
use tungstenite::connect;
use url::Url;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct AuthToken {
    auth_token: String,
    user_id: String,
}

#[derive(Deserialize, Debug)]
struct LoginResponse {
    data: AuthToken,
}

#[derive(Deserialize, Debug)]
struct ChannelResponse {
    name: String,
    _id: String,
}

#[derive(Deserialize, Debug)]
struct ChannelListResponse {
    channels: Vec<ChannelResponse>,
}

#[derive(Deserialize, Debug)]
struct UserResponse {
    name: String,
}

#[derive(Deserialize, Debug)]
struct UserListResponse {
    users: Vec<UserResponse>,
}

#[derive(Deserialize, Debug)]
pub struct AuthorResponse {
    pub username: String,
}

#[derive(Deserialize, Debug)]
struct MessageResponse {
    u: AuthorResponse,
    msg: String,
    ts: String,
}

#[derive(Deserialize, Debug)]
struct ChannelHistoryResponse {
    messages: Vec<MessageResponse>,
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

pub struct RocketChat {
    auth_token: AuthToken,
    cb_sink: CbSink,
    pub current_channel: Option<Channel>,
}

impl RocketChat {
    pub fn new(username: String, password: String, cb_sink: CbSink) -> Self {
        let client = reqwest::blocking::Client::new();
        let res = client
            .post("http://localhost:3000/api/v1/login")
            .body(format!("username={}&password={}", username, password))
            .header("content-type", "application/x-www-form-urlencoded")
            .send()
            .unwrap()
            .json::<LoginResponse>()
            .unwrap();

        let auth_token = res.data;
        let auth_token_cloned = auth_token.clone();
        cb_sink
            .send(Box::new(|siv: &mut Cursive| {
                siv.call_on_name("chat", move |view: &mut BufferView| {
                    view.add_message(format!("{}", auth_token_cloned.auth_token))
                });
            }))
            .unwrap();
        RocketChat {
            auth_token,
            cb_sink,
            current_channel: None,
        }
    }
}

impl Chat for RocketChat {
    fn channels(&self) -> Vec<String> {
        let client = reqwest::blocking::Client::new();
        client
            .get("http://localhost:3000/api/v1/channels.list")
            .header("X-Auth-Token", &self.auth_token.auth_token)
            .header("X-User-Id", &self.auth_token.user_id)
            .send()
            .unwrap()
            .json::<ChannelListResponse>()
            .unwrap()
            .channels
            .iter()
            .map(|x| x.name.clone())
            .collect()
    }

    fn users(&self) -> Vec<String> {
        let client = reqwest::blocking::Client::new();
        client
            .get("http://localhost:3000/api/v1/users.list")
            .header("X-Auth-Token", &self.auth_token.auth_token)
            .header("X-User-Id", &self.auth_token.user_id)
            .send()
            .unwrap()
            .json::<UserListResponse>()
            .unwrap()
            .users
            .iter()
            .map(|x| x.name.clone())
            .collect()
    }

    fn init_view(&mut self, channel: Channel) {
        let client = reqwest::blocking::Client::new();
        let channels = client
            .get("http://localhost:3000/api/v1/channels.list")
            .header("X-Auth-Token", &self.auth_token.auth_token)
            .header("X-User-Id", &self.auth_token.user_id)
            .send()
            .unwrap()
            .json::<ChannelListResponse>()
            .unwrap()
            .channels;

        let users = self.users();
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
        let mut messages = client
            .get(
                &format!(
                    "http://localhost:3000/api/v1/channels.history?roomId={}&count=100",
                    channeltmp
                )[..],
            )
            .header("X-Auth-Token", &self.auth_token.auth_token)
            .header("X-User-Id", &self.auth_token.user_id)
            .send()
            .unwrap()
            .json::<ChannelHistoryResponse>()
            .unwrap()
            .messages;
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
                            .map(|x| (&x[..], Channel::User(x.clone())))
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
        let client = reqwest::blocking::Client::new();
        client
            .post("http://localhost:3000/api/v1/chat.postMessage")
            .body(format!(
                "{{ \"channel\": \"{}\", \"text\": \"{}\" }}",
                channel_to_send, content
            ))
            .header("X-Auth-Token", &self.auth_token.auth_token)
            .header("X-User-Id", &self.auth_token.user_id)
            .header("content-type", "application/json")
            .send()
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
