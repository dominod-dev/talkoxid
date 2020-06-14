pub mod chats;
pub mod views;
use std::fmt;
use std::hash::{Hash, Hasher};

#[derive(Clone)]
pub struct Message {
    pub author: String,
    pub content: String,
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
    fn channels(&self) -> Vec<String>;
    fn users(&self) -> Vec<String>;
    fn init_view(&mut self, channel: Channel);
    fn send_message(&mut self, content: String);
    fn display_message(&self, message: Message, channel: &Channel);
    fn add_message(&mut self, message: Message, channel: Channel);
}
