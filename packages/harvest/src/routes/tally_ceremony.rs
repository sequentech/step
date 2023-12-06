// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::anyhow;
use anyhow::{Context, Result};
use rocket::http::Status;
use rocket::response::Debug;
use rocket::serde::json::Json;
use sequent_core::ballot::ElectionEventStatus;
use sequent_core::services::connection;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::keycloak;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{event, instrument, Level};
use uuid::Uuid;
use windmill::hasura::election_event::get_election_event;
use windmill::hasura::keys_ceremony::{
    get_keys_ceremony, insert_keys_ceremony,
};
use windmill::hasura::trustee::get_trustees_by_name;
use windmill::services::celery_app::get_celery_app;
use windmill::services::election_event_board::get_election_event_board;
use windmill::services::private_keys::get_trustee_encrypted_private_key;
use windmill::tasks::create_keys::{create_keys, CreateKeysBody};
use windmill::types::keys_ceremony::{
    CeremonyStatus, ExecutionStatus, Trustee, TrusteeStatus,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateTallyCeremonyInput {
    election_event_id: String,
    threshold: usize,
    trustee_names: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateTallyCeremonyOutput {
    tally_session_id: String,
}

// The main function to start a key ceremony
#[instrument(skip(claims))]
#[post("/create-tally-ceremony", format = "json", data = "<body>")]
pub async fn create_tally_ceremony(
    body: Json<CreateTallyCeremonyInput>,
    claims: JwtClaims,
) -> Result<Json<CreateTallyCeremonyOutput>, (Status, String)> {
    Err((Status::InternalServerError, "".to_string()))
}
