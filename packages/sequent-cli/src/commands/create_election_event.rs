use crate::{
    types::election_event::{CreateElectionEventRequest, CreateElectionEventResponse}, utils::read_config::read_config,
};
use clap::Args;
use serde_json::Value;

#[derive(Args, Debug)]
#[command(about = "Create a new election event", long_about = None)]
pub struct CreateElectionEvent{
    /// Name of the election event
    #[arg(long)]
    name: String,

    /// Description of the election event
    #[arg(long, default_value = "")]
    description: String,

    /// Presentation details (currently hardcoded to empty)
    #[arg(long, default_value = "{}")]
    presentation: Value,

    /// Encryption protocol (currently hardcoded to RSA256)
    #[arg(long, default_value = "RSA256")]
    encryption_protocol: String,

    /// Whether the event is archived
    #[arg(long, default_value = "false")]
    is_archived: bool,
}

impl CreateElectionEvent {
    pub fn run(&self) {

        let event = CreateElectionEventRequest {
            name: self.name.clone(),
            description: self.description.clone(),
            presentation: self.presentation.clone(),
            encryption_protocol: self.encryption_protocol.clone(),
            is_archived: self.is_archived,
            tenant_id: None // this will be filled in later in the function from the config
        };

        match create_election_event(&event) {
            Ok(id) => {
                println!("Election event created successfully! ID: {}", id);
            }
            Err(err) => {
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
