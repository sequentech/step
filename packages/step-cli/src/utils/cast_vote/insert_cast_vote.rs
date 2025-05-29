use crate::types::config::ConfigData;
use graphql_client::{GraphQLQuery, Response};
use serde_json::Value;
use crate::types::hasura_types::*;
use reqwest::blocking::Client;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/insert_cast_vote.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct InsertCastVote;

pub fn insert_cast_vote(
    config: &ConfigData,
    election_id: &str,
    ballot_id: &str,
    content: &Value,
) -> Result<Value, Box<dyn std::error::Error>> {
    let client = Client::new();
    
    let variables = insert_cast_vote::Variables {
        election_id: election_id.to_string(),
        ballot_id: ballot_id.to_string(),
        content: content.to_string(),
    };
    let request_body = InsertCastVote::build_query(variables);

    let response = client
        .post(&config.endpoint_url)
        .bearer_auth(&config.auth_token)
        .json(&request_body)
        .send()?;

    if response.status().is_success() {
        let response_body: Response<insert_cast_vote::ResponseData> = response.json()?;
        
        if let Some(data) = response_body.data {
            if let Some(cast_vote) = data.insert_cast_vote {
                Ok(serde_json::to_value(cast_vote)?)
            } else {
                Err(Box::from("Failed to insert cast vote"))
            }
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