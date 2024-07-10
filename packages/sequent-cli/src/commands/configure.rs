use crate::types::config::ConfigData;
use crate::utils::read_config::get_config_dir;
use crate::utils::read_input::prompt;
use clap::Args;
use std::fs;
use std::path::Path;

#[derive(Args)]
#[command(about = "Create a config file", long_about = None)]
pub struct Config;

impl Config {
    pub fn run(&self) {
        let auth_token = prompt("Enter auth token: ", true);
        let tenant_id = prompt("Enter tenant id: ", true);
        let endpoint_url = prompt("Enter the endpoint URL: ", true);

        let config_data = ConfigData {
            auth_token,
            endpoint_url,
            tenant_id
        };

        let config_dir = get_config_dir();
        let config_file = config_dir.join("configuration.json");

        if !Path::new(&config_dir).exists() {
            fs::create_dir_all(&config_dir).expect("Failed to create config directory");
        }

        let json_data =
            serde_json::to_string_pretty(&config_data).expect("Failed to serialize config data");

        fs::write(&config_file, json_data).expect("Failed to write config file");

        println!("Configuration saved successfully at {:?}", config_file);
    }
}
