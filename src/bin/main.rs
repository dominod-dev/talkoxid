use async_channel::{bounded, unbounded, Receiver, Sender};
use clap::{load_yaml, App};
use log::{error, info};
use std::error::Error;
use std::thread;
use talkoxid::chats::RocketChat;
use talkoxid::config::{load_config, ChatConfig};
use talkoxid::ui::cursive_ui::CursiveUI;
use talkoxid::{Channel, ChatEvent, UIEvent};
use talkoxid::{Chat, UI};

use tokio::runtime::Runtime;

use url::Url;

async fn chat_loop(
    rx_chat: Receiver<ChatEvent>,
    rx_close: Receiver<()>,
    tx_ui: Sender<UIEvent>,
    config: ChatConfig,
) {
    match RocketChat::new(
        Url::parse(&config.hostname).unwrap_or_else(|err| panic!("Bad url :{:?}", err)),
        config.username,
        config.password,
        tx_ui.clone(),
        rx_chat,
    )
    .await
    {
        Ok(chat_system) => {
            chat_system
                .init_view(Channel::Group("GENERAL".to_string()))
                .await
                .unwrap_or_else(|err| panic!("Can't init chat system: {}", err));
            tokio::select! {
                _ = chat_system.start_loop() => {
                    error!("The websocket loop crashed!")
                }

                _ = rx_close.recv() => {
                    info!("Disconnecting!")
                }
            };
        }
        Err(err) => {
            let err = format!("{}", err);
            tx_ui.send(UIEvent::ShowFatalError(err)).await.unwrap();
            rx_close.recv().await.unwrap();
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    log4rs::init_file("config/log4rs.yaml", Default::default()).unwrap();
    let yaml = load_yaml!("../../config/cli.yaml");
    let matches = App::from_yaml(yaml).get_matches();

    let config = load_config(
        matches.value_of("username"),
        matches.value_of("password"),
        matches.value_of("hostname"),
    );

    // Channel used to communicate from ui to chat
    let (tx_chat, rx_chat) = unbounded();
    // Channel used to communicate from chat to ui
    let (tx_ui, rx_ui) = unbounded();
    // Channel used to terminate chat
    let (tx_close, rx_close) = bounded(1);

    let rt = Runtime::new().unwrap();
    let handle = rt.handle().clone();
    let ui = CursiveUI::new(tx_chat, rx_ui);
    let th = thread::spawn(move || {
        handle.block_on(chat_loop(rx_chat, rx_close, tx_ui, config));
    });

    ui.start_loop().unwrap();

    rt.handle()
        .block_on(async { tx_close.send(()).await.unwrap() });
    th.join().unwrap();

    Ok(())
}
