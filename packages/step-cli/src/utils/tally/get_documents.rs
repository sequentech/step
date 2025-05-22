use crate::{
    types::hasura_types::*,
    utils::read_config::read_config,
};
use graphql_client::{GraphQLQuery, Response};
use serde_json::Value;
use ::uuid::Uuid;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/get_results_event.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct GetResultsEvent;

pub fn get_documents(
    results_event_id: &str,
) -> Result<Value, Box<dyn std::error::Error>> {
    let config = read_config()?;
    let client = reqwest::blocking::Client::new();

    let variables = get_results_event::Variables {
        results_event_id: Uuid::parse_str(results_event_id)?.to_string(),
    };

    let request_body = GetResultsEvent::build_query(variables);

    let response = client
        .post(&config.endpoint_url)
        .bearer_auth(config.auth_token)
        .json(&request_body)
        .send()?;

    if response.status().is_success() {
        let response_body: Response<get_results_event::ResponseData> = response.json()?;
        if let Some(data) = response_body.data {
            if let Some(event) = data.sequent_backend_results_event.first() {
                match event.documents.clone() {
                    Some(docs) => Ok(docs),
                    None => Err(Box::from("Results event documents were null"))
                }
            } else {
                Err(Box::from("No results event found"))
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