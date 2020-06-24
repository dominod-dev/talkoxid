pub mod chats;
pub mod config;
pub mod views;

use async_trait::async_trait;

use chrono::{DateTime, Utc};

use cursive::view::ScrollStrategy;
use cursive::views::{NamedView, ScrollView, SelectView};
use cursive::{CbSink, Cursive};

use log::error;

use std::error::Error;
use std::fmt;
use std::rc::Rc;

use views::{BufferView, ChannelView, MessageBoxView};

#[derive(Clone, Debug)]
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
    fn update_users_in_room(&self, users: Vec<(String, String)>) -> Result<(), Box<dyn Error>>;
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
        let mut chats: Vec<(String, Channel)> = channels
            .iter()
            .cloned()
            .map(|x| match x {
                (repr, Channel::Group(_)) => (format!("#{}", repr), x.1),
                (repr, Channel::Private(_)) => (format!("🔒{}", repr), x.1),
                (repr, Channel::User(_)) => (format!("ጰ{}", repr), x.1),
            })
            .collect();
        chats.sort_by(|a, b| b.1.cmp(&a.1));
        self.cb_sink.send(Box::new(|siv: &mut Cursive| {
            siv.call_on_name("channel_list", move |view: &mut ChannelView| {
                let selected = view
                    .view
                    .selection()
                    .unwrap_or_else(|| Rc::new(Channel::Group("GENERAL".into())));
                let index = chats
                    .iter()
                    .position(|x| &x.1 == selected.as_ref())
                    .unwrap_or_default();
                view.view.clear();
                view.view.add_all(chats);
                view.view.set_selection(index);
            });
        }))?;
        Ok(())
    }

    fn update_users_in_room(&self, users: Vec<(String, String)>) -> Result<(), Box<dyn Error>> {
        self.cb_sink.send(Box::new(|siv: &mut Cursive| {
            siv.call_on_name("users_list", move |view: &mut SelectView<String>| {
                view.clear();
                view.add_all(users);
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
                move |view: &mut NamedView<ScrollView<NamedView<BufferView>>>| {
                    view.get_mut().scroll_to_bottom();
                    view.get_mut()
                        .set_scroll_strategy(ScrollStrategy::StickToBottom);
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
