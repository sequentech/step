// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::utils::read_config::read_config;
use clap::Args;
use graphql_client::Response;
use sequent_core::{ballot_receipt::normalize_ballot_id, types::permissions::Permissions};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{error::Error, time::Duration};
use uuid::Uuid;

const MAX_REASON_BYTES: usize = 512;

#[derive(Args)]
#[command(
    about = "Resolve one voter pseudonym; the authority records your identity and reason in the event log"
)]
pub struct ResolveVoterPseudonym {
    /// Election event UUID
    #[arg(long)]
    election_event_id: Uuid,
    /// One eight-character pseudonym, such as 7QKM-2HXD
    #[arg(long, value_parser = parse_pseudonym)]
    pseudonym: String,
    /// Purpose of this resolution; do not include voter or network data
    #[arg(long, value_parser = parse_reason)]
    reason: String,
}

fn parse_pseudonym(value: &str) -> Result<String, String> {
    normalize_ballot_id(value).ok_or_else(|| "Enter one eight-character pseudonym".into())
}

fn parse_reason(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.len() > MAX_REASON_BYTES
        || trimmed.chars().any(char::is_control)
    {
        return Err(format!(
            "Reason must contain 1 to {MAX_REASON_BYTES} UTF-8 bytes and no control characters"
        ));
    }
    Ok(trimmed.to_owned())
}

#[derive(Deserialize)]
struct Resolution {
    voter_id: String,
}

fn resolved_voter(response: Response<Value>) -> Result<Resolution, Box<dyn Error>> {
    if response.errors.is_some_and(|errors| !errors.is_empty()) {
        return Err("Pseudonym resolution was refused or could not complete".into());
    }
    let payload = response
        .data
        .and_then(|mut data| {
            data.get_mut("resolve_voter_pseudonym")
                .and_then(|answer| answer.get_mut("payload"))
                .map(Value::take)
        })
        .ok_or("Pseudonym resolution returned no answer")?;
    let answer: Resolution = serde_json::from_value(payload)
        .map_err(|_| "Pseudonym resolution returned an invalid answer")?;
    if answer.voter_id.trim().is_empty() {
        return Err("Pseudonym resolution returned no voter".into());
    }
    Ok(answer)
}

impl ResolveVoterPseudonym {
    pub fn run(&self) -> Result<(), Box<dyn Error>> {
        let config = read_config()?;
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()?;
        let response: Response<Value> = client
            .post(&config.endpoint_url)
            .bearer_auth(&config.auth_token)
            .header("x-hasura-role", Permissions::VOTER_PSEUDONYM_RESOLVE.to_string())
            .json(&json!({
                "operationName": "ResolveVoterPseudonym",
                "query": "mutation ResolveVoterPseudonym($election_event_id: uuid!, $pseudonym: String!, $reason: String!) { resolve_voter_pseudonym(election_event_id: $election_event_id, pseudonym: $pseudonym, reason: $reason) { payload } }",
                "variables": {"election_event_id": self.election_event_id.to_string(), "pseudonym": self.pseudonym, "reason": self.reason},
            }))
            .send()?
            .error_for_status()?
            .json()?;
        let answer = resolved_voter(response)?;
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({"voter_id": answer.voter_id}))?
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct Arguments {
        #[command(flatten)]
        command: ResolveVoterPseudonym,
    }

    fn response(value: Value) -> Response<Value> {
        serde_json::from_value(value).expect("synthetic GraphQL response")
    }

    #[test]
    fn one_canonical_pseudonym_and_reason_are_required() {
        let args = Arguments::try_parse_from([
            "resolve",
            "--election-event-id",
            "11111111-1111-4111-8111-111111111111",
            "--pseudonym",
            "7qkm2hxd",
            "--reason",
            " Election protest ",
        ])
        .unwrap();
        assert_eq!(args.command.pseudonym, "7QKM-2HXD");
        assert_eq!(args.command.reason, "Election protest");
        for invalid in ["", "7QKM-2HXD,ABCD-EFGH", "7QKM-2HXD ABCD-EFGH"] {
            assert!(parse_pseudonym(invalid).is_err());
        }
        assert!(parse_reason("   ").is_err());
        assert!(parse_reason("first\nsecond").is_err());
        assert!(parse_reason(&"é".repeat(257)).is_err());
        assert!(parse_reason(&"x".repeat(MAX_REASON_BYTES + 1)).is_err());
        assert!(Arguments::try_parse_from([
            "resolve",
            "--election-event-id",
            "11111111-1111-4111-8111-111111111111",
            "--pseudonym",
            "7QKM-2HXD"
        ])
        .is_err());
    }

    #[test]
    fn resolution_reads_only_the_explicit_payload() {
        let answer = resolved_voter(response(json!({"data": {"resolve_voter_pseudonym": {"payload": {"voter_id": "synthetic-voter", "extra": "not printed"}}}}))).unwrap();
        assert_eq!(answer.voter_id, "synthetic-voter");
        for value in [
            json!({"data": null}),
            json!({"data": {"resolve_voter_pseudonym": {"payload": {"voter_id": ""}}}}),
            json!({"data": {"resolve_voter_pseudonym": {"payload": {"other": "value"}}}}),
        ] {
            assert!(resolved_voter(response(value)).is_err());
        }
    }

    #[test]
    fn graphql_errors_refuse_partial_answers() {
        let error = resolved_voter(response(json!({"data": {"resolve_voter_pseudonym": {"payload": {"voter_id": "partial-voter"}}}, "errors": [{"message": "sensitive diagnostic"}]}))).err().unwrap().to_string();
        assert!(!error.contains("partial-voter"));
        assert!(!error.contains("sensitive diagnostic"));
    }
}
