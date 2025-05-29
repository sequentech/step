use crate::types::config::ConfigData;
use graphql_client::{GraphQLQuery, Response};
use serde_json::Value;
use crate::types::hasura_types::*;
use reqwest::blocking::Client;

#[derive(Debug)]
pub struct BallotStyle {
    pub id: String,
    pub election_id: String,
    pub election_event_id: String,
    pub tenant_id: String,
    pub ballot_eml: Value,
    pub ballot_signature: Option<String>,
    // pub created_at: String,
    // pub area_id: String,
    // pub annotations: Option<String>,
    // pub labels: Option<String>,
    // pub last_updated_at: String,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/get_ballot_styles.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct GetBallotStyles;

pub fn get_ballot_styles_fn(config: &ConfigData) -> Result<Vec<BallotStyle>, Box<dyn std::error::Error>> {
    let client = Client::new();
    
    let variables = get_ballot_styles::Variables {};
    let request_body = GetBallotStyles::build_query(variables);

    let response = client
        .post(&config.endpoint_url)
        .bearer_auth(&config.auth_token)
        .json(&request_body)
        .send()?;

    if response.status().is_success() {
        let response_body: Response<get_ballot_styles::ResponseData> = response.json()?;
        
        if let Some(data) = response_body.data {
            let ballot_styles = data.sequent_backend_ballot_style;
            let mut parsed_styles = Vec::new();

            for style in ballot_styles {
                let ballot_eml = style.ballot_eml.clone().unwrap_or_default();
                let ballot_eml: Value = serde_json::from_str(&ballot_eml)?;

                let ballot_style = BallotStyle {
                    id: style.id,
                    election_id: style.election_id,
                    election_event_id: style.election_event_id,
                    tenant_id: style.tenant_id,
                    ballot_eml,
                    ballot_signature: style.ballot_signature,
                    // created_at: style.created_at,
                    // area_id: style.area_id,
                    // annotations: style.annotations,
                    // labels: style.labels,
                    // last_updated_at: style.last_updated_at,
                };

                parsed_styles.push(ballot_style);
            }

            Ok(parsed_styles)
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