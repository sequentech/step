use clap::Args;
use serde::{Deserialize, Serialize};
use crate::utils::{read_config::read_config, read_input::prompt};

#[derive(Args)]
#[command(about = "Create a new election", long_about = None)]
pub struct CreateElection;

#[derive(Serialize, Deserialize, Debug)]
pub struct Election {
    name: String,
    description: String,
    election_event_id: String
}

impl CreateElection{
    pub fn run(&self) {
        let election_event_id = prompt("Enter the election event id: ", true);
        let name = prompt("Enter the name of the election: ", true);
        let description = prompt("Enter the description of the election: ", false);

        let event = Election { name, description, election_event_id };

        match create_election(&event) {
            Ok(_) => println!("Election event created successfully!"),
            Err(err) => eprintln!("Failed to create election event: {}", err),
        }
    }

}

fn create_election(event: &ElectionEvent) -> Result<(), Box<dyn std::error::Error>>{
    let config = read_config()?;
    let client = reqwest::blocking::Client::new();

    let endpoint_url = format!("{}/election", config.endpoint_url);
    let response = client.post(&endpoint_url)
        .header("Authorization", format!("Bearer {}", config.auth_token))
        .json(event)
        .send()?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(Box::from(response.text()?))
    }
}