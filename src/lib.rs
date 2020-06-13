use std::collections::HashMap;

#[derive(Clone)]
pub struct Message {
    pub author: String,
    pub content: String,
}

pub enum Channel {
    Group(String),
    User(String),
}

pub trait Chat {
    fn channels(&self) -> &Vec<String>;
    fn friends(&self) -> &Vec<String>;
    fn last_10_messages(&self, channel: Channel) -> &Vec<Message>;
    fn send_message(&mut self, content: String, channel: Channel);
}

pub struct DummyChat {
    channels: Vec<String>,
    friends: Vec<String>,
    messages: HashMap<String, Vec<Message>>,
}

impl DummyChat {
    pub fn new() -> Self {
        let channels = vec![
            String::from("general"),
            String::from("another"),
            String::from("YetAnother"),
        ];
        let friends = vec![
            String::from("John"),
            String::from("Bob"),
            String::from("Lola"),
        ];
        let mut messages = HashMap::new();
        messages.insert(
            String::from("general"),
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
        DummyChat {
            channels,
            friends,
            messages,
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

    fn last_10_messages(&self, channel: Channel) -> &Vec<Message> {
        let messages = match channel {
            Channel::Group(g) => self.messages.get(&g).unwrap(),
            Channel::User(f) => self.messages.get(&f).unwrap(),
        };
        return messages;
    }

    fn send_message(&mut self, content: String, channel: Channel) {
        let current_channel: String;
        let messages = match channel {
            Channel::Group(g) => {
                current_channel = String::from(&g);
                self.messages.get(&g).unwrap()
            }
            Channel::User(f) => {
                current_channel = String::from(&f);
                self.messages.get(&f).unwrap()
            }
        };
        let mut new_messages = (*messages).clone();
        let mut new_value = vec![Message {
            author: String::from("me"),
            content,
        }];
        new_messages.append(&mut new_value);
        self.messages.insert(current_channel, new_messages);
    }
}
