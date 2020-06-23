use async_channel::{bounded, unbounded};
use cursive::traits::*;
use cursive::view::ScrollStrategy;
use cursive::views::{LinearLayout, TextView};
use cursive::{CbSink, Cursive};

use log::{error, info};
use oxychat::chats::RocketChat;
use oxychat::views::{BufferView, ChannelView, MessageBoxView};
use oxychat::Chat;
use oxychat::{Channel, ChatEvent, CursiveUI};
use std::error::Error;
use std::thread;
use tokio::runtime::Runtime;
use url::Url;

async fn chat_loop(
    tx: async_channel::Sender<ChatEvent>,
    rx: async_channel::Receiver<ChatEvent>,
    close_rx: async_channel::Receiver<()>,
    cb_sink: CbSink,
) {
    let ui = Box::new(CursiveUI::new(cb_sink));
    let chat_system = RocketChat::new(
        Url::parse("http://localhost:3000/").unwrap_or_else(|err| panic!("Bad url :{:?}", err)),
        "collkid".to_string(),
        "collkid".to_string(),
        ui,
        tx,
        rx,
    )
    .await
    .unwrap_or_else(|err| panic!("Can't create chat system: {}", err));
    chat_system
        .init_view(Channel::Group("GENERAL".to_string()))
        .await
        .unwrap_or_else(|err| panic!("Can't init chat system: {}", err));
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
    move |siv: &mut Cursive, item: &Channel| {
        rt.block_on(async { tx.send(ChatEvent::Init(item.clone())).await.unwrap() });
        siv.focus_name("input").unwrap();
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    log4rs::init_file("config/log4rs.yaml", Default::default()).unwrap();

    let (tx, rx) = unbounded();
    let (close_tx, close_rx) = bounded(1);
    let tx_cloned = tx.clone();

    let rt = Runtime::new().unwrap();
    let handle = rt.handle().clone();

    let mut siv = cursive::default();
    let cb_sink = siv.cb_sink().clone();

    let th = thread::spawn(move || {
        handle.block_on(chat_loop(tx_cloned, rx, close_rx, cb_sink));
    });

    let cb_sink = siv.cb_sink().clone();
    siv.add_global_callback('q', |s| s.quit());
    siv.load_theme_file("assets/style.toml").unwrap();
    // siv.load_toml(include_str!("../../assets/style.toml")).unwrap()
    let buffer = BufferView::new(cb_sink.clone())
        .with_name("chat")
        .scrollable()
        .scroll_strategy(ScrollStrategy::StickToBottom)
        .with_name("scroll");
    let message_input_box =
        MessageBoxView::new(None, tx.clone(), rt.handle().clone()).with_name("input");

    let channel_list = ChannelView::new()
        .on_submit(on_channel_changed(tx.clone(), rt.handle().clone()))
        .with_name("channel_list")
        .scrollable();
    let users_list = ChannelView::new()
        .on_submit(on_channel_changed(tx.clone(), rt.handle().clone()))
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
    siv.run();

    rt.handle()
        .clone()
        .block_on(async { close_tx.send(()).await.unwrap() });
    th.join().unwrap();

    Ok(())
}
