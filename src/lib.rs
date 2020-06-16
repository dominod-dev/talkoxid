pub mod chats;
pub mod views;
use cursive::views::SelectView;
use cursive::{CbSink, Cursive};
use std::fmt;
use std::hash::{Hash, Hasher};
use views::BufferView;

#[derive(Clone)]
pub struct Message {
    pub author: String,
    pub content: String,
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}]: {}", self.author, self.content)
    }
}

#[derive(Eq, PartialEq, Clone)]
pub enum Channel {
    Group(String),
    User(String),
}

impl fmt::Display for Channel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Channel::Group(g) => write!(f, "{}", g),
            Channel::User(u) => write!(f, "{}", u),
        }
    }
}

impl Hash for Channel {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Channel::Group(g) => {
                "group".hash(state);
                g.hash(state);
            }
            Channel::User(u) => {
                "user".hash(state);
                u.hash(state);
            }
        }
    }
}

pub enum ChatEvent {
    SendMessage(String),
    RecvMessage(Message, Channel),
    Init(Channel),
}

pub trait Chat {
    fn init_view(&mut self, channel: Channel);
    fn send_message(&self, content: String);
    fn add_message(&self, message: Message, channel: Channel);
}

pub trait UI {
    fn update_messages(&self, content: String);
    fn update_channels(&self, channels: Vec<(String, Channel)>);
    fn update_users(&self, users: Vec<(String, Channel)>);
    fn add_message(&self, message: Message);
}

pub struct CursiveUI {
    cb_sink: CbSink,
}

impl CursiveUI {
    pub fn new(cb_sink: CbSink) -> Self {
        CursiveUI { cb_sink }
    }
}

impl UI for CursiveUI {
    fn update_messages(&self, messages: String) {
        self.cb_sink
            .send(Box::new(|siv: &mut Cursive| {
                siv.call_on_name("chat", move |view: &mut BufferView| view.init(messages));
            }))
            .unwrap();
    }

    fn update_channels(&self, channels: Vec<(String, Channel)>) {
        self.cb_sink
            .send(Box::new(|siv: &mut Cursive| {
                siv.call_on_name("channel_list", move |view: &mut SelectView<Channel>| {
                    view.clear();
                    view.add_all(channels);
                });
            }))
            .unwrap();
    }

    fn update_users(&self, users: Vec<(String, Channel)>) {
        self.cb_sink
            .send(Box::new(|siv: &mut Cursive| {
                siv.call_on_name("users_list", move |view: &mut SelectView<Channel>| {
                    view.clear();
                    view.add_all(users);
                });
            }))
            .unwrap();
    }

    fn add_message(&self, message: Message) {
        self.cb_sink
            .send(Box::new(|siv: &mut Cursive| {
                siv.call_on_name("chat", move |view: &mut BufferView| {
                    view.add_message(format!("{}\n", message))
                });
            }))
            .unwrap();
    }
}
