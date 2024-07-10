use crate::{
    types::election_event::{CreateElectionEventRequest, CreateElectionEventResponse},
    utils::{loaders::create_spinner, read_config::read_config, read_input::prompt},
};
use clap::Args;

#[derive(Args)]
#[command(about = "Create a new election event", long_about = None)]
pub struct CreateElectionEvent;

impl CreateElectionEvent {
    pub fn run(&self) {
        let name = prompt("Enter the name of the election event: ", true);
        let description = prompt("Enter the description of the election event: ", false);
        let tenant_id = prompt("Enter the tenant ID: ", true);

        let presentation = serde_json::json!({});
        let encryption_protocol = String::from("RSA256");
        let is_archived = false;

        let pb = create_spinner("Creating election event...");
        let event = CreateElectionEventRequest {
            name,
            description,
            presentation,
            tenant_id,
            encryption_protocol,
            is_archived,
        };

        match create_election_event(&event) {
            Ok(id) => {
                pb.finish_with_message("Election event created successfully!");
                println!("Election event created successfully! ID: {}", id);
            }
            Err(err) => {
                pb.finish_with_message("Failed to create election event!");
                eprintln!("Failed to create election event: {}", err)
            }
        }
    }
}

fn create_election_event(
    event: &CreateElectionEventRequest,
) -> Result<String, Box<dyn std::error::Error>> {
    let config = read_config()?;
    let client = reqwest::blocking::Client::new();

    let endpoint_url = format!("{}/insert-election-event", config.endpoint_url);
    let response = client
        .post(endpoint_url)
        .bearer_auth(config.auth_token)
        .json(&event)
        .send()?;

    if response.status().is_success() {
        let response_data: CreateElectionEventResponse = response.json()?;
        Ok(response_data.id)
    } else {
        let status = response.status();
        let error_message = response.text()?;
        let error = format!("HTTP Status: {}\nError Message: {}", status, error_message);
        Err(Box::from(error))
    }
}
