// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{
    types::hasura_types::*,
    utils::{
        elections::get::GetElections,
        read_config::read_config,
        signing::{exit_pending, Signed},
    },
};
use clap::Args;
use colored::Colorize;
use graphql_client::{GraphQLQuery, Response};
#[derive(Args)]
#[command(about = "Start Tally Ceremony", long_about = None)]
pub struct StartTallyCeremony {
    /// Election event id - the election event to start the key ceremony for
    #[arg(long)]
    election_event_id: String,

    /// Election ids- optional specific elections to start the tally for - if not provided - all elections will be tallied
    #[arg(long)]
    election_ids: Option<Vec<String>>,

    /// Tally type - the type of tally to perform (ELECTORAL_RESULTS or INITIALIZATION_REPORT)
    #[arg(long)]
    tally_type: String,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/create_tally_ceremony.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct CreateTallyCeremony;

impl StartTallyCeremony {
    pub fn run(&self) {
        match start_ceremony_signed(
            &self.election_event_id,
            self.election_ids.clone(),
            &self.tally_type,
        ) {
            Ok(Signed::Pending(pending)) => exit_pending("The tally", &pending),
            Ok(Signed::Done(id)) => {
                println!(
                    "{} {}",
                    "Success! Successfully started Tally ceremony. ID:".green(),
                    id.cyan()
                );
            }
            Err(err) => {
                eprintln!("Error! Failed to start Tally ceremony: {}", err)
            }
        }
    }
}

/// The session started, or the signing request that initializing voting
/// needs first (no session exists until it is signed).
fn tally_outcome(
    output: create_tally_ceremony::CreateTallyCeremonyCreateTallyCeremony,
) -> Result<Signed<String>, Box<dyn std::error::Error>> {
    if let Some(pending) = crate::pending_signatures!(output.signing_request) {
        return Ok(Signed::Pending(pending));
    }
    output
        .tally_session_id
        .map(Signed::Done)
        .ok_or_else(|| Box::from("Tally ceremony returned no session ID"))
}

/// Starts the tally, failing when it waits for signatures.
pub fn start_ceremony(
    election_event_id: &str,
    election_ids: Option<Vec<String>>,
    tally_type: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    match start_ceremony_signed(election_event_id, election_ids, tally_type)? {
        Signed::Done(id) => Ok(id),
        Signed::Pending(pending) => Err(format!("the tally is {pending}").into()),
    }
}

pub fn start_ceremony_signed(
    election_event_id: &str,
    election_ids: Option<Vec<String>>,
    tally_type: &str,
) -> Result<Signed<String>, Box<dyn std::error::Error>> {
    let config = read_config()?;
    let client = reqwest::blocking::Client::new();

    let elections = match election_ids {
        Some(el) => el,
        None => GetElections::get_by_election_event(&election_event_id)?,
    };

    let variables = create_tally_ceremony::Variables {
        election_event_id: election_event_id.to_string(),
        election_ids: elections,
        configuration: None,
        tally_type: Some(tally_type.to_string()),
    };

    let request_body = CreateTallyCeremony::build_query(variables);

    let response = client
        .post(&config.endpoint_url)
        .bearer_auth(config.auth_token)
        .json(&request_body)
        .send()?;

    if response.status().is_success() {
        let response_body: Response<create_tally_ceremony::ResponseData> = response.json()?;
        if let Some(data) = response_body.data {
            if let Some(e) = data.create_tally_ceremony {
                tally_outcome(e)
            } else {
                Err(Box::from("failed updating status"))
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn outcome(value: serde_json::Value) -> Result<Signed<String>, String> {
        tally_outcome(serde_json::from_value(value).unwrap()).map_err(|error| error.to_string())
    }

    #[test]
    fn a_started_tally_answers_its_session() {
        assert_eq!(
            outcome(json!({"tally_session_id": "t1", "signing_request": null})),
            Ok(Signed::Done("t1".into()))
        );
    }

    #[test]
    fn a_tally_waiting_for_signatures_has_no_session_yet() {
        let answer = outcome(json!({
            "tally_session_id": null,
            "signing_request": {"id": "r1", "code": "AB-12", "required": 3,
                                "expires_at": "2026-11-22T00:00:00Z"}
        }));
        match answer {
            Ok(Signed::Pending(pending)) => {
                assert_eq!((pending.id.as_str(), pending.required), ("r1", 3));
            }
            other => panic!("expected a pending tally, got {other:?}"),
        }
    }

    #[test]
    fn a_tally_without_session_or_request_is_an_error() {
        assert!(outcome(json!({"tally_session_id": null, "signing_request": null})).is_err());
    }
}
