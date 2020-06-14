use super::super::views::BufferView;
use super::super::{Channel, Chat, Message};
use cursive::views::SelectView;
use cursive::CbSink;
use cursive::Cursive;
use reqwest;
use serde::Deserialize;

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
struct AuthorResponse {
    username: String,
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
pub struct RocketChat {
    auth_token: AuthToken,
    cb_sink: CbSink,
    current_channel: Option<Channel>,
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
        let channels = self.channels();
        let users = self.users();
        self.current_channel = Some(channel);
        let client = reqwest::blocking::Client::new();
        let mut messages = client
            .get("http://localhost:3000/api/v1/channels.history?roomId=GENERAL&count=100")
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
                            .map(|x| (&x[..], Channel::Group(x.clone())))
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

    fn send_message(&mut self, content: String) {
        let client = reqwest::blocking::Client::new();
        client
            .post("http://localhost:3000/api/v1/chat.postMessage")
            .body(format!(
                "{{ \"channel\": \"#general\", \"text\": \"{}\" }}",
                content
            ))
            .header("X-Auth-Token", &self.auth_token.auth_token)
            .header("X-User-Id", &self.auth_token.user_id)
            .header("content-type", "application/json")
            .send()
            .unwrap();
    }

    fn display_message(&self, message: Message, channel: &Channel) {
        if let Some(channel) = &self.current_channel {
            self.cb_sink
                .send(Box::new(|siv: &mut Cursive| {
                    siv.call_on_name("chat", move |view: &mut BufferView| {
                        view.add_message(format!("[{}]: {}\n", message.author, message.content))
                    });
                }))
                .unwrap();
        }
    }
    fn add_message(&mut self, message: Message, channel: Channel) {}
}
