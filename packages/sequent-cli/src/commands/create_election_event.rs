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

        let presentation = serde_json::json!({}); // Currently hardcoded to empty
        let encryption_protocol = String::from("RSA256"); // Currently hardcoded to this option only
        let is_archived = false; // Currently hardcoded

        let pb = create_spinner("Creating election event...");
        let event = CreateElectionEventRequest {
            name,
            description,
            presentation,
            encryption_protocol,
            is_archived,
            tenant_id: None
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

    let event_with_tenant_id = CreateElectionEventRequest {
        tenant_id: Some(config.tenant_id.clone()), 
        ..event.clone() // Clone the existing event and override tenant_id
    };
    
    let endpoint_url = format!("{}/insert-election-event", config.endpoint_url);
    let response = client
        .post(endpoint_url)
        .bearer_auth(config.auth_token)
        .json(&event_with_tenant_id)
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
