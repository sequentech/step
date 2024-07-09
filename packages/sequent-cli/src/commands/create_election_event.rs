use crate::utils::{read_config::read_config, read_input::prompt};
use clap::Args;
use serde::{Deserialize, Serialize};

#[derive(Args)]
#[command(about = "Create a new election event", long_about = None)]
pub struct CreateElectionEvent;

#[derive(Serialize, Deserialize, Debug)]
pub struct ElectionEvent {
    name: String,
    description: String,
}

impl CreateElectionEvent {
    pub fn run(&self) {
        let name = prompt("Enter the name of the election event: ", true);
        let description = prompt("Enter the description of the election event: ", false);

        let event = ElectionEvent { name, description };

        match create_election_event(&event) {
            Ok(_) => println!("Election event created successfully!"),
            Err(err) => eprintln!("Failed to create election event: {}", err),
        }
    }
}

fn create_election_event(event: &ElectionEvent) -> Result<(), Box<dyn std::error::Error>> {
    let config = read_config()?;
    let client = reqwest::blocking::Client::new();

    let endpoint_url = format!("{}/insert-election-event", config.endpoint_url);
    let response = client
        .post(&endpoint_url)
        .header("Authorization", format!("Bearer {}", config.auth_token))
        .json(event)
        .send()?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(Box::from(response.text()?))
    }
}
