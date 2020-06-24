use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct TomlConfig {
    username: Option<String>,
    password: Option<String>,
    hostname: Option<String>,
}

pub struct ChatConfig {
    pub username: String,
    pub password: String,
    pub hostname: String,
}

pub fn load_config(
    username: Option<&str>,
    password: Option<&str>,
    hostname: Option<&str>,
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
    ChatConfig {
        username,
        password,
        hostname,
    }
}
