use super::super::views::BufferView;
use super::super::{Channel, Chat, Message};
use cursive::views::SelectView;
use cursive::CbSink;
use cursive::Cursive;
use std::collections::HashMap;

pub struct DummyChat {
    channels: Vec<String>,
    friends: Vec<String>,
    messages: HashMap<Channel, Vec<Message>>,
    cb_sink: CbSink,
    current_channel: Channel,
}

impl DummyChat {
    pub fn new(cb_sink: CbSink) -> Self {
        let channels = vec![
            String::from("general"),
            String::from("another"),
            String::from("YetAnother"),
        ];
        let friends = vec![
            String::from("John"),
            String::from("Bob"),
            String::from("Lola"),
            String::from("general"),
        ];
        let mut messages = HashMap::new();
        messages.insert(
            Channel::Group("general".to_string()),
            vec![
                Message {
                    author: String::from("Bob"),
                    content: String::from("Salut"),
                },
                Message {
                    author: String::from("John"),
                    content: String::from("Hi!"),
                },
                Message {
                    author: String::from("Lola"),
                    content: String::from("Arriba!"),
                },
            ],
        );
        messages.insert(Channel::Group("another".to_string()), vec![]);
        messages.insert(Channel::Group("YetAnother".to_string()), vec![]);
        messages.insert(Channel::User("Bob".to_string()), vec![]);
        messages.insert(Channel::User("Lola".to_string()), vec![]);
        messages.insert(Channel::User("John".to_string()), vec![]);
        let current_channel = Channel::Group("general".to_string());
        DummyChat {
            channels,
            friends,
            messages,
            cb_sink,
            current_channel,
        }
    }
}

impl Chat for DummyChat {
    fn channels(&self) -> &Vec<String> {
        &self.channels
    }

    fn friends(&self) -> &Vec<String> {
        &self.friends
    }

    fn init_view(&mut self, channel: Channel) -> &Vec<Message> {
        let index: usize;
        self.current_channel = channel;
        let messages = match &self.current_channel {
            Channel::Group(g) => {
                index = self.channels.iter().position(|r| r == g).unwrap();
                self.messages.get(&self.current_channel).unwrap()
            }
            Channel::User(f) => {
                index = self.friends.iter().position(|r| r == f).unwrap();
                self.messages.get(&self.current_channel).unwrap()
            }
        };

        let new_content = messages.iter().fold(String::from(""), |x, y| {
            format!("{}[{}]: {}\n", x, y.author, y.content)
        });
        let channels = self.channels.clone();
        let users = self.friends.clone();
        let index = index.clone();
        self.cb_sink
            .send(Box::new(move |siv: &mut Cursive| {
                siv.call_on_name("chat", move |view: &mut BufferView| view.init(new_content));
                siv.call_on_name("channel_list", move |view: &mut SelectView<Channel>| {
                    view.clear();
                    view.add_all(
                        channels
                            .iter()
                            .map(|x| (&x[..], Channel::Group(x.to_string())))
                            .collect::<Vec<(&str, Channel)>>(),
                    );
                    view.set_selection(index)
                });
                siv.call_on_name("users_list", move |view: &mut SelectView<Channel>| {
                    view.clear();
                    view.add_all(
                        users
                            .iter()
                            .map(|x| (&x[..], Channel::User(x.to_string())))
                            .collect::<Vec<(&str, Channel)>>(),
                    );
                    view.set_selection(index)
                });
            }))
            .unwrap();
        return messages;
    }

    fn send_message(&mut self, content: String) {
        let messages = self.messages.get(&self.current_channel).unwrap();
        let mut new_messages = (*messages).clone();
        let new_message = Message {
            author: String::from("me"),
            content,
        };
        let mut new_value = vec![new_message.clone()];
        new_messages.append(&mut new_value);
        self.display_message(new_message, &self.current_channel);
        self.messages
            .insert(self.current_channel.clone(), new_messages);
    }

    fn display_message(&self, message: Message, channel: &Channel) {
        if &self.current_channel == channel {
            self.cb_sink
                .send(Box::new(|siv: &mut Cursive| {
                    siv.call_on_name("chat", move |view: &mut BufferView| {
                        view.add_message(format!("[{}]: {}\n", message.author, message.content))
                    });
                }))
                .unwrap();
        }
    }
    fn add_message(&mut self, message: Message, channel: Channel) {
        let messages = self.messages.get(&channel).unwrap();
        let mut new_messages = (*messages).clone();
        let mut new_value = vec![message.clone()];
        new_messages.append(&mut new_value);
        self.display_message(message, &channel);
        self.messages.insert(channel, new_messages);
    }
}
