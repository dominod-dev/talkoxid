pub mod chats;
pub mod views;
use async_trait::async_trait;
use cursive::view::ScrollStrategy;
use cursive::views::{NamedView, ResizedView, ScrollView, SelectView};
use cursive::{CbSink, Cursive};
use log::error;
use std::error::Error;
use std::fmt;
use views::{BufferView, MessageBoxView};
use std::rc::Rc;
use chrono::{DateTime, Utc};

#[derive(Clone)]
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
            write!(f, "[{}][{}]: {}", localtime.format("%Y-%m-%d %H:%M:%S"), self.author, self.content)
        } else {
            write!(f, "[{}][{}]: {}", localtime.format("%H:%M:%S"), self.author, self.content)
        }
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

pub enum ChatEvent {
    SendMessage(String, Channel),
    RecvMessage(Message, Channel),
    Init(Channel),
}

#[async_trait]
pub trait Chat {
    async fn init_view(&self, channel: Channel) -> Result<(), Box<dyn Error>>;
    async fn send_message(&self, content: String, channel: Channel) -> Result<(), Box<dyn Error>>;
    async fn add_message(&self, message: Message, channel: Channel) -> Result<(), Box<dyn Error>>;
    async fn wait_for_messages(&self) -> Result<(), Box<dyn Error>>;
    async fn update_ui(&self) -> Result<(), Box<dyn Error>>;
}

pub trait UI {
    fn update_messages(&self, content: String) -> Result<(), Box<dyn Error>>;
    fn update_channels(&self, channels: Vec<(String, Channel)>) -> Result<(), Box<dyn Error>>;
    fn add_message(&self, message: Message) -> Result<(), Box<dyn Error>>;
    fn select_channel(&self, channel: Channel) -> Result<(), Box<dyn Error>>;
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
    fn update_messages(&self, messages: String) -> Result<(), Box<dyn Error>> {
        self.cb_sink.send(Box::new(|siv: &mut Cursive| {
            siv.call_on_name("chat", move |view: &mut BufferView| view.init(messages));
        }))?;
        Ok(())
    }

    fn update_channels(&self, channels: Vec<(String, Channel)>) -> Result<(), Box<dyn Error>> {
        let chats: Vec<(String, Channel)> = channels
            .iter()
            .cloned()
            .filter(move |x| {
                if let (_, Channel::Group(_)) = x {
                    true
                } else {
                    false
                }
            })
            .collect();
        let users: Vec<(String, Channel)> = channels
            .iter()
            .cloned()
            .filter(move |x| {
                if let (_, Channel::User(_)) = x {
                    true
                } else {
                    false
                }
            })
            .collect();
        self.cb_sink.send(Box::new(|siv: &mut Cursive| {
            siv.call_on_name("channel_list", move |view: &mut SelectView<Channel>| {
                let selected = view
                    .selection()
                    .unwrap_or_else(|| Rc::new(Channel::Group("GENERAL".into())));
                let index = chats
                    .iter()
                    .position(|x| &x.1 == selected.as_ref())
                    .unwrap_or_default();
                view.clear();
                view.add_all(chats);
                if let Channel::Group(_) = *selected.as_ref() {
                    view.set_selection(index);
                }
            });
            siv.call_on_name("users_list", move |view: &mut SelectView<Channel>| {
                let selected = view
                    .selection()
                    .unwrap_or_else(|| Rc::new(Channel::Group("GENERAL".into())));
                let index = users
                    .iter()
                    .position(|x| &x.1 == selected.as_ref())
                    .unwrap_or_default();
                view.clear();
                view.add_all(users);
                if let Channel::User(_) = *selected.as_ref() {
                    view.set_selection(index);
                }
            });
        }))?;
        Ok(())
    }

    fn add_message(&self, message: Message) -> Result<(), Box<dyn Error>> {
        self.cb_sink.send(Box::new(|siv: &mut Cursive| {
            siv.call_on_name("chat", move |view: &mut BufferView| {
                view.add_message(format!("{}\n", message))
                    .unwrap_or_else(|err| error!("Can't add message: {}", err))
            });
            siv.call_on_name(
                "scroll",
                move |view: &mut ScrollView<ResizedView<NamedView<BufferView>>>| {
                    view.scroll_to_bottom();
                    view.set_scroll_strategy(ScrollStrategy::StickToBottom);
                },
            );
        }))?;
        Ok(())
    }

    fn select_channel(&self, channel: Channel) -> Result<(), Box<dyn Error>> {
        self.cb_sink.send(Box::new(|siv: &mut Cursive| {
            siv.call_on_name("input", move |view: &mut MessageBoxView| {
                view.channel = Some(channel);
            });
        }))?;
        Ok(())
    }
}
