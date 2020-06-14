use cursive::theme::{Color, ColorStyle, ColorType};
use cursive::traits::*;
use cursive::view::{ScrollStrategy, SizeConstraint};
use cursive::views::{Layer, LinearLayout, ResizedView, ScrollView, SelectView, TextView};
use cursive::Cursive;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use oxychat::chats::RocketChat;
use oxychat::views::{BufferView, MessageBoxView};
use oxychat::{Channel, Chat, ChatEvent, Message};

fn update_channel<'r>(tx: mpsc::Sender<ChatEvent>) -> impl Fn(&mut Cursive, &Channel) -> () {
    let closure = move |siv: &mut Cursive, item: &Channel| {
        siv.call_on_name("channel_name", |view: &mut TextView| match item {
            Channel::Group(_) => view.set_content(format!("#{}", item)),
            Channel::User(_) => view.set_content(format!("{}", item)),
        });
        tx.send(ChatEvent::Init(item.clone())).unwrap();
        siv.focus_name("input").unwrap();
    };
    return closure;
}

fn async_chat_update(mut chat_system: Box<dyn Chat>, rx: mpsc::Receiver<ChatEvent>) {
    chat_system.init_view(Channel::Group("general".to_string()));
    loop {
        match rx.recv() {
            Ok(ChatEvent::SendMessage(message)) => {
                chat_system.send_message(message);
            }
            Ok(ChatEvent::Init(channel)) => {
                chat_system.init_view(channel);
            }
            Ok(ChatEvent::RecvMessage(message, channel)) => {
                chat_system.init_view(Channel::Group("general".to_string()));
                chat_system.add_message(message, channel);
            }
            Err(_) => continue,
        };
    }
}

fn main() {
    let mut siv = cursive::default();
    let cb_sink = siv.cb_sink().clone();
    let (tx, rx) = mpsc::channel();

    siv.add_global_callback('q', |s| s.quit());
    let tx1 = mpsc::Sender::clone(&tx);

    let white = ColorType::Color(Color::Rgb(255, 255, 255));
    let black = ColorType::Color(Color::Rgb(0, 0, 0));
    let white_on_black = ColorStyle::new(white, black);

    // You can load a theme from a file at runtime for fast development.
    siv.load_theme_file("assets/style.toml").unwrap();

    // Or you can directly load it from a string for easy deployment.
    // siv.load_toml(include_str!("../../assets/style.toml"))
    //     .unwrap();
    let mut buffer = ScrollView::new(
        BufferView::new(cb_sink.clone())
            .with_name("chat")
            .full_screen(),
    );
    buffer.set_scroll_strategy(ScrollStrategy::StickToBottom);
    let chat = Layer::with_color(
        ResizedView::new(SizeConstraint::Full, SizeConstraint::Full, buffer),
        white_on_black,
    );
    let message_input_box = MessageBoxView::new(tx1).with_name("input");
    let message_input = ResizedView::new(
        SizeConstraint::Full,
        SizeConstraint::AtLeast(10),
        message_input_box,
    );
    let chat_layout = ResizedView::new(
        SizeConstraint::Full,
        SizeConstraint::Full,
        LinearLayout::vertical()
            .child(TextView::new("#general").with_name("channel_name"))
            .child(chat)
            .child(TextView::new("Message:"))
            .child(message_input),
    );
    let channel_list = SelectView::<Channel>::new()
        .on_submit(update_channel(mpsc::Sender::clone(&tx)))
        .with_name("channel_list");
    let users_list = SelectView::<Channel>::new()
        .on_submit(update_channel(mpsc::Sender::clone(&tx)))
        .with_name("users_list");
    let channel_users = ScrollView::new(
        LinearLayout::vertical()
            .child(TextView::new("CHANNELS:"))
            .child(channel_list)
            .child(TextView::new("USERS:"))
            .child(users_list),
    );
    let channel_layout = ResizedView::new(
        SizeConstraint::AtLeast(20),
        SizeConstraint::Full,
        channel_users,
    );
    let global_layout = LinearLayout::horizontal()
        .child(channel_layout)
        .child(chat_layout);

    siv.add_fullscreen_layer(global_layout);
    let chat_system = Box::new(RocketChat::new(
        "admin".to_string(),
        "admin".to_string(),
        cb_sink.clone(),
    ));
    thread::spawn(|| async_chat_update(chat_system, rx));
    thread::spawn(move || loop {
        match mpsc::Sender::clone(&tx).send(ChatEvent::RecvMessage(
            Message {
                author: "bot".to_string(),
                content: "Hi".to_string(),
            },
            Channel::Group("general".to_string()),
        )) {
            Ok(_) => {}
            Err(e) => {
                println!("{}", e);
            }
        };
        thread::sleep(Duration::from_millis(500));
    });
    siv.focus_name("input").unwrap();
    siv.run();
}
