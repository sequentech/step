// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::{anyhow, Context, Result};
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::ballot::ElectionEventStatus;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::keycloak;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};
use uuid::Uuid;
use windmill::hasura;
use windmill::{
    hasura::election_event::get_election_event::GetElectionEventSequentBackendElectionEvent,
    services::database::get_hasura_pool,
};

use crate::postgres;
use chrono::{DateTime, Utc};
use deadpool_postgres::Client as DbClient;
use sequent_core::ballot::ElectionStatus;
use sequent_core::ballot::VotingStatus;

/*
type Mutation {
  """
  insertCastVote
  """
  InsertCastVote(
    id: uuid
    ballot_id: String!
    election_id: uuid
    election_event_id: uuid
    tenant_id: uuid
    area_id: uuid
    content: String!
  ): InsertCastVoteOutput
}

type InsertCastVoteOutput {
  id: uuid!
  created_at: timestamptz!
  cast_ballot_signature: bytea!
}
*/

/*
mutation insertCastVote {
  InsertCastVote(ballot_id: "a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a21", content: "content", tenant_id: "90505c8a-23a9-4cdf-a26b-4e19f6a097d5", election_event_id: "33f18502-a67c-4853-8333-a58630663559", election_id: "f2f1065e-b784-46d1-b81a-c71bfeb9ad55", area_id:"a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a21") {
  ballot_id,
  cast_ballot_signature,
  created_at,
  id,
  last_updated_at,
  voter_id_string,
  }
}
*/

#[derive(Serialize, Deserialize, Debug)]
pub struct InsertCastVoteInput {
    ballot_id: Uuid,
    election_id: Uuid,
    election_event_id: Uuid,
    tenant_id: Uuid,
    area_id: Uuid,
    content: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InsertCastVoteOutput {
    id: String,
    created_at: DateTime<Utc>,
    cast_ballot_signature: Vec<u8>,
}

// #[instrument(skip(claims))]
#[post("/insert-cast-vote", format = "json", data = "<body>")]
pub async fn insert_cast_vote(
    body: Json<InsertCastVoteInput>, // TODO claims: JwtClaims,
) -> Result<Json<InsertCastVoteOutput>, (Status, String)> {
    // TODO
    // authorize(&claims, true, None, vec![Permissions::XXX])?;

    let result = try_insert_cast_vote(body).await.map_err(|err| {
        (
            Status::InternalServerError,
            format!("Error inserting vote: {:?}", err),
        )
    })?;

    Ok(Json(result))
}

async fn try_insert_cast_vote(
    body: Json<InsertCastVoteInput>,
) -> anyhow::Result<InsertCastVoteOutput> {
    let input = body.into_inner();

    // TODO
    let voter_id = "voter_id";

    check_status(&input).await?;
    check_previous_votes(voter_id, &input).await?;
    let result = insert(&input).await?;

    Ok(result)
}

async fn check_status(input: &InsertCastVoteInput) -> anyhow::Result<()> {
    let auth_headers = keycloak::get_client_credentials().await?;

    let hasura_response = hasura::election_event::get_election_event(
        auth_headers.clone(),
        input.tenant_id.to_string(),
        input.election_event_id.to_string(),
    )
    .await
    .context("Cannot retrieve election event data")?;

    // TODO expect
    let election_event = &hasura_response
        .data
        .expect("expected data".into())
        .sequent_backend_election_event[0];

    if election_event.is_archived {
        return Err(anyhow!("Election event is archived"));
    }

    let status = election_event
        .status
        .clone()
        .ok_or(anyhow!("Could not retrieve election event status"))?;
    let status: ElectionEventStatus = serde_json::from_value(status)
        .context("Failed to deserialize election event status")?;
    if status.voting_status != VotingStatus::OPEN {
        return Err(anyhow!("Election event voting status is not open"));
    }

    let hasura_response = hasura::election::get_election(
        auth_headers.clone(),
        input.tenant_id.to_string(),
        input.election_event_id.to_string(),
        input.election_id.to_string(),
    )
    .await
    .context("Cannot retrieve election data")?;

    // TODO expect
    let election = &hasura_response
        .data
        .expect("expected data".into())
        .sequent_backend_election[0];

    let status = election
        .status
        .clone()
        .ok_or(anyhow!("Could not retrieve election status"))?;
    let status: ElectionStatus = serde_json::from_value(status)
        .context("Failed to deserialize election status")?;
    if status.voting_status != VotingStatus::OPEN {
        return Err(anyhow!("Election voting status is not open"));
    }

    Ok(())
}

async fn check_previous_votes(
    voter_id_string: &str,
    input: &InsertCastVoteInput,
) -> anyhow::Result<()> {
    // TODO
    let max_revotes = 1;

    let mut hasura_db_client: DbClient = get_hasura_pool().await.get().await?;
    let hasura_transaction = hasura_db_client.transaction().await?;

    let result = postgres::cast_vote::get_cast_votes(
        &hasura_transaction,
        &input.tenant_id,
        &input.election_event_id,
        &input.election_id,
        &input.area_id,
        voter_id_string,
    )
    .await?;

    let (same, other): (Vec<Uuid>, Vec<Uuid>) = result
        .into_iter()
        .map(|(_, _, area_id)| area_id)
        .partition(|area_id| area_id == &input.area_id);

    event!(Level::INFO, "get cast votes returns same: {:?}", same);
    if same.len() >= max_revotes {
        return Err(anyhow!(
            "Cannot insert cast vote, maximum votes reached ({}, {})",
            voter_id_string,
            same.len()
        ));
    }
    if other.len() > 0 {
        return Err(anyhow!("Cannot insert cast vote, votes already present in other area(s) ({}, {:?})", voter_id_string, other));
    }

    Ok(())
}

async fn insert(
    input: &InsertCastVoteInput,
) -> anyhow::Result<InsertCastVoteOutput> {
    let mut hasura_db_client: DbClient = get_hasura_pool().await.get().await?;
    let hasura_transaction = hasura_db_client.transaction().await?;

    let signature = vec![];
    let (id, created_at) = postgres::cast_vote::insert_cast_vote(
        &hasura_transaction,
        &input.tenant_id,
        &input.election_event_id,
        &input.election_id,
        &input.area_id,
        "content",
        "voter_id",
        "ballot_id",
        &signature,
    )
    .await?;
    hasura_transaction.commit().await?;

    let ret = InsertCastVoteOutput {
        id: id.to_string(),
        created_at: created_at,
        cast_ballot_signature: signature,
    };

    Ok(ret)
}
