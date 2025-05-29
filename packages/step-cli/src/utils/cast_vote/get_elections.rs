use crate::types::config::ConfigData;
use graphql_client::{GraphQLQuery, Response};
use serde_json::Value;
use crate::types::hasura_types::*;
use reqwest::blocking::Client;

#[derive(Debug, Clone)]
pub struct Election {
    pub id: String,
    pub election_event_id: String,
    pub tenant_id: String,
    pub name: String,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/get_elections_for_voter.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct GetElections;

pub fn get_elections(config: &ConfigData, election_ids: Vec<String>) -> Result<Vec<Election>, Box<dyn std::error::Error>> {
    let client = Client::new();
    
    let variables = get_elections::Variables {
        election_ids,
    };
    let request_body = GetElections::build_query(variables);

    let response = client
        .post(&config.endpoint_url)
        .bearer_auth(&config.auth_token)
        .json(&request_body)
        .send()?;

    if response.status().is_success() {
        let response_body: Response<get_elections::ResponseData> = response.json()?;
        
        if let Some(data) = response_body.data {
            let elections = data.sequent_backend_election;
            let mut parsed_elections = Vec::new();

            for election in elections {
                let election = Election {
                    id: election.id,
                    election_event_id: election.election_event_id,
                    tenant_id: election.tenant_id,
                    name: election.name,
                };
                parsed_elections.push(election);
            }

            Ok(parsed_elections)
        } else if let Some(errors) = response_body.errors {
            let error_messages: Vec<String> = errors.into_iter().map(|e| e.message).collect();
            Err(Box::from(error_messages.join(", ")))
        } else {
            Err(Box::from("Unknown error occurred"))
        }
    } else {
        let status = response.status();
        let error_message = response.text()?;
        let error = format!("HTTP Status: {}\nError Message: {}", status, error_message);
        Err(Box::from(error))
    }
} 