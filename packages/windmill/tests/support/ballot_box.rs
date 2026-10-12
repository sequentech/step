// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Election events' ballot boxes, each in an electoral-log database of its own on
//! the PostgreSQL server that `ELECTORAL_LOG_PG_*` names. Include it with
//! `#[path = "support/ballot_box.rs"] mod ballot_box;`.

use electoral_log::adapters::ballot_box::{AcceptBallot, AcceptOutcome, BallotStatus};
use electoral_log::adapters::postgres::PostgresStore;
use uuid::Uuid;
use windmill::services::ballot_box::BallotFormat;
use windmill::services::protocol_manager::get_event_databases;

/// The board an event's `bulletin_board_reference` names.
pub fn board_reference(board: &str) -> serde_json::Value {
    serde_json::json!({"id": 1, "database_name": board, "is_archived": false})
}

/// The event's database and its ballot box, created on first use.
pub async fn ballot_box(tenant: &Uuid, event: &Uuid) -> PostgresStore {
    let store = get_event_databases()
        .await
        .expect("ELECTORAL_LOG_PG_* must name the test PostgreSQL server")
        .create_event(&tenant.to_string(), &event.to_string())
        .await
        .expect("event database");
    store
        .create_ballot_box(&event.to_string())
        .await
        .expect("ballot box");
    store
}

/// A ballot of the voter in the election and area, before it is accepted.
pub struct Ballot<'a> {
    pub event: &'a Uuid,
    pub election: &'a Uuid,
    pub area: &'a Uuid,
    pub voter_id: &'a str,
    pub ballot_id: &'a str,
    pub content: &'a str,
}

/// Accept a valid ballot, with as many votes as the voter wants.
pub async fn accept(store: &PostgresStore, ballot: Ballot<'_>) -> String {
    let (event, election, area) = (
        ballot.event.to_string(),
        ballot.election.to_string(),
        ballot.area.to_string(),
    );
    let format = BallotFormat::HashableBallot.to_string();
    let outcome = store
        .accept_ballot(&AcceptBallot {
            election_event_id: &event,
            election_id: &election,
            area_id: &area,
            voter_id: ballot.voter_id,
            ballot_id: ballot.ballot_id,
            format: &format,
            content: ballot.content,
            voter_signature: None,
            pseudonym_hash: &[0; 64],
            ballot_hash: &[0; 64],
            voting_channel: "ONLINE",
            status: BallotStatus::Valid,
            voter_ip: None,
            voter_country: None,
            username: None,
            allowed_votes: 0,
        })
        .await
        .expect("accepted ballot");
    match outcome {
        AcceptOutcome::Accepted { id, .. } => id,
        refused => panic!("ballot refused: {refused:?}"),
    }
}
