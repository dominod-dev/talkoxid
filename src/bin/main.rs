use async_channel::{unbounded, Receiver, Sender};
use clap::{load_yaml, App};
use std::error::Error;
use talkoxid::chats::RocketChat;
use talkoxid::config::{load_config, ChatConfig};
use talkoxid::ui::CursiveUI;
use talkoxid::{Channel, ChatEvent, UIEvent};
use talkoxid::{Chat, UI};

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
        tx_ui.clone(),
        rx_chat,
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
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    log4rs::init_file("config/log4rs.yaml", Default::default())?;
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

    let ui = tokio::task::spawn_blocking(move || {
        let ui = CursiveUI::new(tx_chat, rx_ui);
        ui.start_loop().unwrap();
    });
    let chat = tokio::task::spawn(chat_loop(rx_chat, tx_ui, config));

    ui.await?;
    chat.await??;
    Ok(())
}
