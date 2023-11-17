// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use celery::error::TaskError;
use immu_board::util::get_board_name;
use sequent_core;
use sequent_core::services::connection;
use sequent_core::services::keycloak::{get_client_credentials, KeycloakAdminClient};
use serde_json::Value;
use std::env;
use tracing::{event, Level, instrument};

use crate::hasura::election_event::insert_election_event::sequent_backend_election_event_insert_input as InsertElectionEventInput;
use crate::services::election_event_board::BoardSerializable;
use crate::services::protocol_manager::get_board_client;
use crate::types::error::Result;
use crate::hasura::election_event::{insert_election_event, get_election_event};

#[instrument]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn insert_tenant(object: InsertElectionEventInput, id: String) -> Result<()> {
    let auth_headers = get_client_credentials().await?;
    insert_tenant_db(&auth_headers, &final_object).await?;
}
