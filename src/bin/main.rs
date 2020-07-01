use async_channel::{bounded, unbounded};
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
    rx: async_channel::Receiver<ChatEvent>,
    close_rx: async_channel::Receiver<()>,
    tx_ui: async_channel::Sender<UIEvent>,
    config: ChatConfig,
) {
    match RocketChat::new(
        Url::parse(&config.hostname).unwrap_or_else(|err| panic!("Bad url :{:?}", err)),
        config.username,
        config.password,
        tx_ui.clone(),
        rx,
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

                _ = close_rx.recv() => {
                    info!("Disconnecting!")
                }
            };
        }
        Err(err) => {
            let err = format!("{}", err);
            tx_ui.send(UIEvent::ShowFatalError(err)).await.unwrap();
            close_rx.recv().await.unwrap();
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

    let (tx, rx) = unbounded();
    let (tx_ui, rx_ui) = unbounded();
    let (close_tx, close_rx) = bounded(1);

    let rt = Runtime::new().unwrap();
    let handle = rt.handle().clone();
    let ui = CursiveUI::new(tx, rx_ui);
    let th = thread::spawn(move || {
        handle.block_on(chat_loop(rx, close_rx, tx_ui, config));
    });

    ui.start_loop().unwrap();

    rt.handle()
        .block_on(async { close_tx.send(()).await.unwrap() });
    th.join().unwrap();

    Ok(())
}
