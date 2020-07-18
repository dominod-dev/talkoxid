//! Core module.
//!
//! Core types and traits.
use async_trait::async_trait;

use chrono::{DateTime, Utc};

use std::error::Error;
use std::fmt;

/// Message representation.
///
/// This type represent a message in a chat.
#[derive(Eq, PartialEq, PartialOrd, Clone, Debug)]
pub struct Message {
    /// The message's author.
    pub author: String,
    /// The content of the message.
    pub content: String,
    /// The date and time of when the message was sent.
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

/// Channel representation
///
/// This type represent a channel in a chat.
///
/// A channel is a place where user can send message to.
#[derive(Eq, PartialEq, PartialOrd, Clone, Debug)]
pub enum Channel {
    /// A public group channel.
    Group(String),
    /// A direct message to one or more users.
    User(String),
    /// A private group channel.
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

/// Events sent to the chat system.
///
/// This enum represent all the events that a chat system
/// can receive.
///
/// Those events are sent from the UI via a channel to the chat system.
///
/// # Examples
///
/// ```no_run
/// # use std::error::Error;
/// use talkoxid::core::{ChatEvent, Channel};
/// use async_channel::unbounded;
///
/// # async fn send_hello_world() -> Result<(), Box<dyn Error + Send + Sync>> {
/// let (tx_chat, rx_chat) = unbounded();
/// tx_chat.send(
///     ChatEvent::SendMessage(
///         "Hello world!".to_string(),
///         Channel::Group("GENERAL".to_string())
///     )
/// ).await?;
/// # Ok(())
/// # }
/// ```
///
#[derive(Eq, PartialEq, PartialOrd, Clone, Debug)]
pub enum ChatEvent {
    /// Used when the User send a message.
    SendMessage(String, Channel),
    /// Used when the User select a channel the first time
    /// or when he change the currently selected channel.
    Init(Channel),
}

/// Events sent to the User Interface.
///
/// This enum represent all the events that an UI
/// can receive.
///
/// Those events are sent from the chat system via a channel to the UI.
///
/// # Examples
///
/// ```no_run
/// # use std::error::Error;
/// use talkoxid::core::{UIEvent, Channel};
/// use async_channel::unbounded;
///
/// # async fn send_hello_world() -> Result<(), Box<dyn Error + Send + Sync>> {
/// let (tx_ui, rx_ui) = unbounded();
/// tx_ui.send(
///     UIEvent::SelectChannel(
///         Channel::Group("GENERAL".to_string())
///     )
/// ).await?;
/// # Ok(())
/// # }
/// ```
///
#[derive(Eq, PartialEq, PartialOrd, Clone, Debug)]
pub enum UIEvent {
    /// Used when the messages feed list change.
    UpdateMessages(String),
    /// Used when the channel list change.
    UpdateChannels(Vec<(String, Channel)>),
    /// Used when the users in a room/channel change.
    UpdateUsersInRoom(Vec<(String, String)>),
    /// Used when a message is received and need to be displayed.
    AddMessages(Message),
    /// Used when we select a new channel.
    SelectChannel(Channel),
    /// Used when a fatal error occurred and need to be displayed.
    ShowFatalError(String),
}

/// Chat system trait
///
/// All chat backends should implement this trait.
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
    /// Start the main loop that listen to [ChatEvent](enum.ChatEvent.html)
    async fn start_loop(&self) -> Result<(), Box<dyn Error + Send + Sync>>;
}

/// User Interface trait.
///
/// All UI backends should implement this trait.
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
    /// Start the main loop that listen to [UIEvent](enum.UIEvent.html)
    fn start_loop(&self) -> Result<(), Box<dyn Error + Send + Sync>>;
}
