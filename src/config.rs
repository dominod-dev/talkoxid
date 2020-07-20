//! Configuration module.
//!
//! This module contains the logic to resolve
//! the configuration.
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct TomlConfig {
    username: Option<String>,
    password: Option<String>,
    hostname: Option<String>,
    ssl_verify: Option<bool>,
}

/// Chat configuration.
///
/// This type contains all parameters a chat system need
/// to operate.
pub struct ChatConfig {
    /// The User's username.
    pub username: String,
    /// The User's password.
    pub password: String,
    /// The Chat Hostname.
    pub hostname: String,
    /// Wheter we verify ssl certificates or not
    pub ssl_verify: bool,
}

/// Resolve config between runtime provided parameters and configuration file.
pub fn load_config(
    username: Option<&str>,
    password: Option<&str>,
    hostname: Option<&str>,
    ssl_verify_present: bool,
) -> ChatConfig {
    let mut config_path = dirs_next::config_dir().unwrap();
    config_path.push("talkoxid");
    config_path.push("talkoxid.toml");
    let config_file = std::fs::read_to_string(config_path).unwrap_or_else(|_| String::from(""));
    let config: TomlConfig = toml::from_str(&config_file).expect("Corrupted config file");

    let username = username
        .map(|x| x.to_string())
        .or(config.username)
        .expect("Error no username provided");
    let password = password
        .map(|x| x.to_string())
        .or(config.password)
        .expect("Error no password provided");
    let hostname = hostname
        .map(|x| x.to_string())
        .or(config.hostname)
        .expect("Error no hostname provided");
    let ssl_verify = !ssl_verify_present && config.ssl_verify.unwrap_or(true);
    ChatConfig {
        username,
        password,
        hostname,
        ssl_verify,
    }
}
