use serde_derive::Deserialize;
use std::{fs::read_to_string, vec::Vec};

#[derive(Deserialize, Clone, Debug)]
pub struct NgrokCmd {
    pub description: String,
    pub connection_type: String,
    pub port: u32,
    pub permitted_users: Vec<u64>,
    pub howto: Option<String>,
}

pub type NgrokCmds = Vec<NgrokCmd>;

#[derive(Deserialize, Clone)]
pub struct Config {
    pub bot_key: String,
    pub ngrok_cmds: NgrokCmds,
    pub permitted_chats: Vec<i64>,
}

pub fn load() -> Config {
    let config_path = std::path::Path::new("./config.yaml")
        .canonicalize()
        .unwrap();
    let config_path = config_path.as_path();

    log::info!("Loading config \"{}\"...", config_path.display());

    let config = read_to_string(config_path).unwrap_or_else(|err| {
        panic!(
            "File config.yaml not found by path {}. Error: {}",
            config_path.display(),
            err
        )
    });

    serde_yaml::from_str::<Config>(config.as_str())
        .unwrap_or_else(|err| panic!("Config file is malformed. Error: {}", err))
}
