use async_channel::{unbounded, Receiver, Sender};
use clap::{load_yaml, App};
use std::error::Error;
use talkoxid::chats::RocketChat;
use talkoxid::config::{load_config, ChatConfig};
use talkoxid::core::{Channel, Chat, ChatEvent, UIEvent, UI};
use talkoxid::notifications::DesktopNotifier;
use talkoxid::ui::CursiveUI;

use log::LevelFilter;
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Root};

use url::Url;

async fn chat_loop(
    rx_chat: Receiver<ChatEvent>,
    tx_ui: Sender<UIEvent>,
    config: ChatConfig,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    match RocketChat::new(
        Url::parse(&config.hostname).unwrap_or_else(|err| panic!("Bad url :{:?}", err)),
        config.username,
        config.password,
        config.ssl_verify,
        tx_ui.clone(),
        rx_chat,
        Box::new(DesktopNotifier {}),
    )
    .await
    {
        Ok(chat_system) => {
            chat_system
                .init_view(Channel::Group("GENERAL".to_string()))
                .await?;
            chat_system.start_loop().await?;
        }
        Err(err) => {
            let err = format!("{}", err);
            tx_ui.send(UIEvent::ShowFatalError(err)).await?;
        }
    }
    Ok(())
}

fn ui_loop(
    tx_chat: Sender<ChatEvent>,
    rx_ui: Receiver<UIEvent>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let ui = CursiveUI::new(tx_chat, rx_ui);
    ui.start_loop()?;
    Ok(())
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let stdout = ConsoleAppender::builder().build();
    let file = FileAppender::builder().build("/tmp/talkoxid.log").unwrap();
    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(Appender::builder().build("file", Box::new(file)))
        .build(Root::builder().appender("file").build(LevelFilter::Info))
        .unwrap();
    log4rs::init_config(config)?;
    log::info!("Starting talkoxid");
    let yaml = load_yaml!("../../config/cli.yaml");
    let matches = App::from_yaml(yaml).get_matches();

    let config = load_config(
        matches.value_of("username"),
        matches.value_of("password"),
        matches.value_of("hostname"),
        matches.is_present("disable_ssl_verify"),
    );

    // Channel used to communicate from ui to chat
    let (tx_chat, rx_chat) = unbounded();
    // Channel used to communicate from chat to ui
    let (tx_ui, rx_ui) = unbounded();

    let ui = tokio::task::spawn_blocking(|| ui_loop(tx_chat, rx_ui));
    let chat = tokio::task::spawn(chat_loop(rx_chat, tx_ui, config));

    ui.await??;
    chat.await??;
    Ok(())
}
