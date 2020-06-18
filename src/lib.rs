pub mod chats;
pub mod views;
use async_trait::async_trait;
use cursive::view::ScrollStrategy;
use cursive::views::{NamedView, ResizedView, ScrollView, SelectView};
use cursive::{CbSink, Cursive};
use std::fmt;
use std::hash::{Hash, Hasher};
use views::{BufferView, MessageBoxView};

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
    SendMessage(String, Channel),
    RecvMessage(Message, Channel),
    Init(Channel),
}

#[async_trait]
pub trait Chat {
    async fn init_view(&self, channel: Channel) -> Result<(), String>;
    async fn send_message(&self, content: String, channel: Channel) -> Result<(), String>;
    async fn add_message(&self, message: Message, channel: Channel);
    async fn wait_for_messages(&self) -> Result<(), String>;
    async fn update_ui(&self) -> Result<(), String>;
}

pub trait UI {
    fn update_messages(&self, content: String);
    fn update_channels(&self, channels: Vec<(String, Channel)>, current_channel: Option<Channel>);
    fn update_users(&self, users: Vec<(String, Channel)>);
    fn add_message(&self, message: Message);
    fn select_channel(&self, channel: Channel);
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

    fn update_channels(&self, channels: Vec<(String, Channel)>, current_channel: Option<Channel>) {
        let mut index = 0;
        if let Some(channel) = current_channel {
            index = channels
                .iter()
                .position(|x| x.1 == channel)
                .unwrap_or_default();
        }
        self.cb_sink
            .send(Box::new(move |siv: &mut Cursive| {
                siv.call_on_name("channel_list", move |view: &mut SelectView<Channel>| {
                    view.clear();
                    view.add_all(channels);
                    view.set_selection(index)
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
                siv.call_on_name(
                    "scroll",
                    move |view: &mut ScrollView<ResizedView<NamedView<BufferView>>>| {
                        view.scroll_to_bottom();
                        view.set_scroll_strategy(ScrollStrategy::StickToBottom);
                    },
                );
            }))
            .unwrap();
    }

    fn select_channel(&self, channel: Channel) {
        self.cb_sink
            .send(Box::new(|siv: &mut Cursive| {
                siv.call_on_name("input", move |view: &mut MessageBoxView| {
                    view.channel = Some(channel);
                });
            }))
            .unwrap();
    }
}
