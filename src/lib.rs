pub mod chats;
pub mod config;
pub mod ui;

use async_trait::async_trait;

use chrono::{DateTime, Utc};

use std::error::Error;
use std::fmt;

#[derive(Eq, PartialEq, PartialOrd, Clone, Debug)]
pub struct Message {
    pub author: String,
    pub content: String,
    pub datetime: DateTime<Utc>,
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let today = chrono::offset::Local::today();
        let localtime = self.datetime.with_timezone(&chrono::Local);
        if localtime.date() < today {
            write!(
                f,
                "[{}][{}]: {}",
                localtime.format("%Y-%m-%d %H:%M:%S"),
                self.author,
                self.content
            )
        } else {
            write!(
                f,
                "[{}][{}]: {}",
                localtime.format("%H:%M:%S"),
                self.author,
                self.content
            )
        }
    }
}

#[derive(Debug)]
struct UIError {
    source: String,
}

impl fmt::Display for UIError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "There was an UI Error: {}", self.source)
    }
}

impl Error for UIError {}

#[derive(Eq, PartialEq, PartialOrd, Clone, Debug)]
pub enum Channel {
    Group(String),
    User(String),
    Private(String),
}

impl fmt::Display for Channel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Channel::Group(g) => write!(f, "{}", g),
            Channel::User(u) => write!(f, "{}", u),
            Channel::Private(u) => write!(f, "{}", u),
        }
    }
}

impl Ord for Channel {
    fn cmp(&self, b: &Self) -> std::cmp::Ordering {
        match (self, b) {
            (Channel::Group(_), Channel::Private(_)) => std::cmp::Ordering::Greater,
            (Channel::Group(_), Channel::User(_)) => std::cmp::Ordering::Greater,
            (Channel::Group(c), Channel::Group(d)) => c.cmp(d),
            (Channel::Private(c), Channel::Private(d)) => c.cmp(d),
            (Channel::Private(_), Channel::User(_)) => std::cmp::Ordering::Greater,
            (Channel::Private(_), Channel::Group(_)) => std::cmp::Ordering::Less,
            (Channel::User(_), Channel::Private(_)) => std::cmp::Ordering::Less,
            (Channel::User(c), Channel::User(d)) => c.cmp(d),
            (Channel::User(_), Channel::Group(_)) => std::cmp::Ordering::Less,
        }
    }
}

#[derive(Eq, PartialEq, PartialOrd, Clone, Debug)]
pub enum ChatEvent {
    SendMessage(String, Channel),
    Init(Channel),
}

#[derive(Eq, PartialEq, PartialOrd, Clone, Debug)]
pub enum UIEvent {
    UpdateMessages(String),
    UpdateChannels(Vec<(String, Channel)>),
    UpdateUsersInRoom(Vec<(String, String)>),
    AddMessages(Message),
    SelectChannel(Channel),
    ShowFatalError(String),
}

#[async_trait]
pub trait Chat {
    async fn init_view(&self, channel: Channel) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn send_message(
        &self,
        content: String,
        channel: Channel,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn add_message(
        &self,
        message: Message,
        channel: &Channel,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn start_loop(&self) -> Result<(), Box<dyn Error + Send + Sync>>;
}

pub trait UI {
    fn update_messages(&self, content: String) -> Result<(), Box<dyn Error + Send + Sync>>;
    fn update_channels(
        &self,
        channels: Vec<(String, Channel)>,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    fn update_users_in_room(
        &self,
        users: Vec<(String, String)>,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    fn add_message(&self, message: Message) -> Result<(), Box<dyn Error + Send + Sync>>;
    fn select_channel(&self, channel: Channel) -> Result<(), Box<dyn Error + Send + Sync>>;
    fn show_fatal_error(&self, content: String) -> Result<(), Box<dyn Error + Send + Sync>>;
    fn start_loop(&self) -> Result<(), Box<dyn Error + Send + Sync>>;
}
