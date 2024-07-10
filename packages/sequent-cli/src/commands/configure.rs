use crate::types::config::ConfigData;
use crate::utils::read_config::get_config_dir;
use clap::Args;
use std::fs;
use std::path::Path;

#[derive(Args, Debug)]
#[command(about = "Create a config file", long_about = None)]
pub struct Config {
    /// Authorization token
    #[arg(long)]
    auth_token: String,

    /// Tenant ID
    #[arg(long)]
    tenant_id: String,

    /// Endpoint URL
    #[arg(long)]
    endpoint_url: String,
}

impl Config {
    pub fn run(&self) {
        let config_data = ConfigData {
            auth_token: self.auth_token.clone(),
            endpoint_url: self.endpoint_url.clone(),
            tenant_id: self.tenant_id.clone(),
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
