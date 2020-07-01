pub mod views;
use super::super::{Channel, ChatEvent, Message, UIEvent, UI};
use cursive::traits::*;
use cursive::view::ScrollStrategy;
use cursive::views::{LinearLayout, SelectView, TextView};
use cursive::views::{NamedView, ScrollView};
use cursive::{CbSink, Cursive};

use log::error;

use std::cell::RefCell;
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

fn on_channel_changed(
    tx: async_channel::Sender<ChatEvent>,
    rt: tokio::runtime::Handle,
) -> impl Fn(&mut Cursive, &Channel) -> () {
    move |siv: &mut Cursive, item: &Channel| {
        rt.block_on(async { tx.send(ChatEvent::Init(item.clone())).await.unwrap() });
        siv.focus_name("input").unwrap();
    }
}

pub struct CursiveUI {
    cb_sink: CbSink,
    siv: RefCell<Cursive>,
    rx: async_channel::Receiver<UIEvent>,
}

impl CursiveUI {
    pub fn new(
        tx: async_channel::Sender<ChatEvent>,
        rx: async_channel::Receiver<UIEvent>,
        rt: tokio::runtime::Handle,
    ) -> Self {
        let mut siv = cursive::default();

        let cb_sink = siv.cb_sink().clone();
        siv.add_global_callback('q', |s| s.quit());
        siv.load_toml(include_str!("../../../assets/style.toml"))
            .unwrap();
        let buffer = BufferView::new(cb_sink.clone())
            .with_name("chat")
            .scrollable()
            .scroll_strategy(ScrollStrategy::StickToBottom)
            .with_name("scroll");
        let message_input_box =
            MessageBoxView::new(None, tx.clone(), rt.clone()).with_name("input");

        let channel_list = ChannelView::new()
            .on_submit(on_channel_changed(tx, rt))
            .with_name("channel_list")
            .scrollable();
        let users_list = SelectView::<String>::new()
            .with_name("users_list")
            .scrollable();
        let channels = LinearLayout::vertical()
            .child(TextView::new("CHANNELS:"))
            .child(channel_list)
            .min_width(20);
        let users = LinearLayout::vertical()
            .child(TextView::new("USERS:"))
            .child(users_list)
            .min_width(20);
        let chat_layout = LinearLayout::vertical()
            .child(cursive::views::Panel::new(buffer).full_height())
            .child(message_input_box)
            .full_width();
        let global_layout = LinearLayout::horizontal()
            .child(channels)
            .child(chat_layout)
            .child(users);

        siv.add_fullscreen_layer(global_layout);
        siv.focus_name("input").unwrap();
        CursiveUI {
            cb_sink,
            siv: RefCell::new(siv),
            rx,
        }
    }
}

impl UI for CursiveUI {
    fn start_loop(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut siv = self.siv.borrow_mut();
        while siv.is_running() {
            siv.step();
            match self.rx.try_recv() {
                Ok(UIEvent::AddMessages(msg)) => self.add_message(msg)?,
                Ok(UIEvent::UpdateChannels(channels)) => self.update_channels(channels)?,
                Ok(UIEvent::UpdateMessages(messages)) => self.update_messages(messages)?,
                Ok(UIEvent::UpdateUsersInRoom(users)) => self.update_users_in_room(users)?,
                Ok(UIEvent::SelectChannel(channel)) => self.select_channel(channel)?,
                Ok(UIEvent::ShowFatalError(content)) => self.show_fatal_error(content)?,
                _ => continue,
            };
        }
        Ok(())
    }
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

    fn show_fatal_error(&self, content: String) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.cb_sink
            .send(Box::new(move |siv: &mut Cursive| {
                siv.add_layer(
                    cursive::views::Dialog::new()
                        .title("Error")
                        .content(TextView::new(content))
                        .button("Quit", |s| s.quit()),
                );
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
