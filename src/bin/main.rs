use async_channel::{bounded, unbounded};
use cursive::traits::*;
use cursive::view::{ScrollStrategy, SizeConstraint};
use cursive::views::{LinearLayout, ResizedView, ScrollView, SelectView, TextView};
use cursive::{CbSink, Cursive};

use log::{error, info};
use oxychat::chats::RocketChat;
use oxychat::views::{BufferView, MessageBoxView};
use oxychat::Chat;
use oxychat::{Channel, ChatEvent, CursiveUI};
use std::thread;
use tokio::runtime::Runtime;
use url::Url;

async fn chat_loop(
    tx: async_channel::Sender<ChatEvent>,
    rx: async_channel::Receiver<ChatEvent>,
    close_rx: async_channel::Receiver<()>,
    cb_sink: CbSink,
) {
    let ui = Box::new(CursiveUI::new(cb_sink.clone()));
    let chat_system = RocketChat::new(
        Url::parse("http://localhost:3000/").unwrap(),
        "collkid".to_string(),
        "collkid".to_string(),
        ui,
        tx,
        rx,
    )
    .await;
    chat_system
        .init_view(Channel::Group("GENERAL".to_string()))
        .await
        .unwrap();
    let read_loop = chat_system.wait_for_messages();
    let ui_event_loop = chat_system.update_ui();
    tokio::select! {
        _ = ui_event_loop => {
            error!("The chat event loop crashed!")
        }
        _ = read_loop => {
            error!("The websocket loop crashed!")
        }

        _ = close_rx.recv() => {
            info!("Disconnecting!")
        }
    };
}

fn on_channel_changed(
    tx: async_channel::Sender<ChatEvent>,
    rt: tokio::runtime::Handle,
) -> impl Fn(&mut Cursive, &Channel) -> () {
    let closure = move |siv: &mut Cursive, item: &Channel| {
        rt.block_on(async { tx.send(ChatEvent::Init(item.clone())).await.unwrap() });
        siv.focus_name("input").unwrap();
    };
    return closure;
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    log4rs::init_file("config/log4rs.yaml", Default::default()).unwrap();
    let mut siv = cursive::default();
    let cb_sink = siv.cb_sink().clone();
    let (tx, rx) = unbounded();
    let (close_tx, close_rx) = bounded(1);
    let tx2 = tx.clone();
    let rt = Runtime::new().unwrap();
    let handle = rt.handle().clone();
    let th = thread::spawn(move || {
        handle.block_on(chat_loop(tx2, rx, close_rx, cb_sink));
    });
    let cb_sink = siv.cb_sink().clone();
    siv.add_global_callback('q', |s| s.quit());
    siv.load_theme_file("assets/style.toml").unwrap();
    // siv.load_toml(include_str!("../../assets/style.toml")).unwrap()
    let buffer = ScrollView::new(
        BufferView::new(cb_sink.clone())
            .with_name("chat")
            .full_screen(),
    )
    .scroll_strategy(ScrollStrategy::StickToBottom)
    .with_name("scroll");
    let chat = ResizedView::new(SizeConstraint::Full, SizeConstraint::Full, buffer);
    let message_input_box =
        MessageBoxView::new(None, async_channel::Sender::clone(&tx), rt.handle().clone())
            .with_name("input");

    let channel_list = SelectView::<Channel>::new()
        .on_submit(on_channel_changed(
            async_channel::Sender::clone(&tx),
            rt.handle().clone(),
        ))
        .with_name("channel_list");
    let users_list = SelectView::<Channel>::new()
        .on_submit(on_channel_changed(
            async_channel::Sender::clone(&tx),
            rt.handle().clone(),
        ))
        .with_name("users_list");
    let channel = ScrollView::new(
        LinearLayout::vertical()
            .child(TextView::new("CHANNELS:"))
            .child(channel_list),
    );
    let users = ScrollView::new(
        LinearLayout::vertical()
            .child(TextView::new("USERS:"))
            .child(users_list),
    );
    let channel_layout =
        ResizedView::new(SizeConstraint::AtLeast(20), SizeConstraint::Full, channel);

    let users_layout = ResizedView::new(SizeConstraint::AtLeast(20), SizeConstraint::Full, users);
    let chat_layout = ResizedView::new(
        SizeConstraint::Full,
        SizeConstraint::Full,
        LinearLayout::vertical()
            .child(cursive::views::Panel::new(chat))
            .child(message_input_box),
    );
    let global_layout = LinearLayout::horizontal()
        .child(channel_layout)
        .child(chat_layout)
        .child(users_layout);

    siv.add_fullscreen_layer(global_layout);
    siv.focus_name("input").unwrap();
    siv.run();
    rt.handle()
        .clone()
        .block_on(async { close_tx.send(()).await.unwrap() });
    th.join().unwrap();
    Ok(())
}
