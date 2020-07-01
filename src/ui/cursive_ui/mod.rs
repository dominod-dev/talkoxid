pub mod views;
use super::super::{Channel, Message, UI};

use cursive::view::ScrollStrategy;
use cursive::views::{NamedView, ScrollView, SelectView};
use cursive::{CbSink, Cursive};

use log::error;

use std::error::Error;
use std::fmt;
use std::rc::Rc;

use views::{BufferView, ChannelView, MessageBoxView};

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

fn format_channel(channels: Vec<(String, Channel)>) -> Vec<(String, Channel)> {
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
    chats
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
    fn update_messages(&self, messages: String) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.cb_sink
            .send(Box::new(|siv: &mut Cursive| {
                siv.call_on_name("chat", move |view: &mut BufferView| view.init(messages));
            }))
            .map_err(|err| UIError {
                source: format!("{}", err),
            })?;
        Ok(())
    }

    fn update_channels(
        &self,
        channels: Vec<(String, Channel)>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let chats = format_channel(channels);
        self.cb_sink
            .send(Box::new(|siv: &mut Cursive| {
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
            }))
            .map_err(|err| UIError {
                source: format!("{}", err),
            })?;
        Ok(())
    }

    fn update_users_in_room(
        &self,
        users: Vec<(String, String)>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.cb_sink
            .send(Box::new(|siv: &mut Cursive| {
                siv.call_on_name("users_list", move |view: &mut SelectView<String>| {
                    view.clear();
                    view.add_all(users);
                });
            }))
            .map_err(|err| UIError {
                source: format!("{}", err),
            })?;
        Ok(())
    }

    fn add_message(&self, message: Message) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.cb_sink
            .send(Box::new(|siv: &mut Cursive| {
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
            }))
            .map_err(|err| UIError {
                source: format!("{}", err),
            })?;
        Ok(())
    }

    fn select_channel(&self, channel: Channel) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.cb_sink
            .send(Box::new(|siv: &mut Cursive| {
                siv.call_on_name("input", move |view: &mut MessageBoxView| {
                    view.channel = Some(channel);
                });
            }))
            .map_err(|err| UIError {
                source: format!("{}", err),
            })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_format_channels() {
        let channels = vec![
            ("test_pub".to_string(), Channel::Group("test_pub_id".into())),
            (
                "test_priv".to_string(),
                Channel::Private("test_priv_id".into()),
            ),
            (
                "test_direct".to_string(),
                Channel::User("test_direct_id".into()),
            ),
        ];
        assert_eq!(
            format_channel(channels),
            vec![
                ("#test_pub".into(), Channel::Group("test_pub_id".into())),
                (
                    "🔒test_priv".to_string(),
                    Channel::Private("test_priv_id".into())
                ),
                (
                    "ጰtest_direct".to_string(),
                    Channel::User("test_direct_id".into()),
                ),
            ]
        );
    }
}
