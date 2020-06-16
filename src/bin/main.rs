use cursive::theme::{Color, ColorStyle, ColorType};
use cursive::traits::*;
use cursive::view::{ScrollStrategy, SizeConstraint};
use cursive::views::{Layer, LinearLayout, ResizedView, ScrollView, SelectView, TextView};
use cursive::Cursive;
use std::sync::mpsc;

use std::sync::Arc;
use std::sync::Mutex;

use oxychat::chats::{ChatServer, RocketChat};
use oxychat::views::{BufferView, MessageBoxView};
use oxychat::{Channel, ChatEvent};
use url::Url;

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

fn main() {
    let mut siv = cursive::default();
    let cb_sink = siv.cb_sink().clone();
    let chat_system = RocketChat::new(
        Url::parse("http://localhost:3000/").unwrap(),
        "admin".to_string(),
        "admin".to_string(),
        cb_sink.clone(),
    );
    let chat_server = ChatServer {
        chat_system: Arc::new(Mutex::new(chat_system)),
    };
    let tx = chat_server.start();

    siv.add_global_callback('q', |s| s.quit());

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
    let message_input_box = MessageBoxView::new(mpsc::Sender::clone(&tx)).with_name("input");
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
    siv.focus_name("input").unwrap();
    siv.run();
}
