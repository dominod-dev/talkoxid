# Talkoxid

Talkoxid is a simple TUI chat client written in Rust. It was primarily written to support
Rocket.Chat but its aim is to become modular enough to support other chats
 backends in the future.

## Status

Talkoxid is considered alpha and is not yet very mature.

I'm doing this for fun and I can't assure long term maintenance
of the project.


## Build

Simply build it with cargo :

```bash
cargo build --release --bin talkoxid
```

The resulting binary will be in `./target/release/talkoxid`


## Usage

For now Talkoxid only support Rocket.Chat and the configuration
variables are the following :

 - `username`: Your username in the chat
 - `password`: Your password in the chat
 - `hostname`: Your chat hostname with port. Example: https://mychat.net:1234

You can pass those variables in command line or you can create a config file in toml format
 in `$HOME/.config/talkoxid/talkoxid.toml` and specify the variables here. Example:

 ```toml
username = "admin"
password = "admin"
hostname = "http://localhost:3000"
```

## How does it work ?

For Rocket.Chat, it simply uses the Realtime API via websocket.
