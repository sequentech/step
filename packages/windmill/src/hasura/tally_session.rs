// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::election_event_board::get_election_event_board;
use crate::services::electoral_log::ElectoralLog;
use crate::services::to_result::ToResult;
pub use crate::types::hasura_types::*;
use anyhow::{anyhow, Context, Result};
use b3::messages::newtypes::BatchNumber;
use deadpool_postgres::Transaction;
use graphql_client::{GraphQLQuery, Response};
use reqwest;
use sequent_core::services::connection;
use sequent_core::types::ceremonies::{TallyCeremonyStatus, TallyExecutionStatus};
use serde_json;
use std::env;
use tracing::instrument;

use crate::hasura::election_event::get_election_event_helper;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/get_tally_session_highest_batch.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct GetTallySessionHighestBatch;

#[instrument(skip(auth_headers), err)]
pub async fn get_tally_session_highest_batch(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
) -> Result<BatchNumber> {
    let variables = get_tally_session_highest_batch::Variables {
        tenant_id: tenant_id,
        election_event_id: election_event_id,
    };
    let hasura_endpoint =
        env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = GetTallySessionHighestBatch::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<get_tally_session_highest_batch::ResponseData> = res.json().await?;
    let data: Response<get_tally_session_highest_batch::ResponseData> = response_body.ok()?;
    let tally_session_contest = data
        .data
        .ok_or(anyhow!("Can't find tally session"))?
        .sequent_backend_tally_session_contest;
    if tally_session_contest.len() > 0 {
        Ok((tally_session_contest[0].session_id + 1) as BatchNumber)
    } else {
        Ok(0)
    }
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/insert_tally_session.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct InsertTallySession;

#[instrument(skip(auth_headers), err)]
pub async fn insert_tally_session(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
    election_ids: Vec<String>,
    area_ids: Vec<String>,
    tally_session_id: String,
    keys_ceremony_id: String,
    execution_status: TallyExecutionStatus,
    threshold: i64,
) -> Result<Response<insert_tally_session::ResponseData>> {
    let variables = insert_tally_session::Variables {
        tenant_id: tenant_id,
        election_event_id: election_event_id,
        election_ids: election_ids,
        area_ids: area_ids,
        tally_session_id: tally_session_id,
        keys_ceremony_id: keys_ceremony_id,
        execution_status: Some(execution_status.to_string()),
        threshold,
    };
    let hasura_endpoint =
        env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = InsertTallySession::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<insert_tally_session::ResponseData> = res.json().await?;
    response_body.ok()
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/get_tally_session_by_id.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct GetTallySessionById;

#[instrument(skip(auth_headers), err)]
pub async fn get_tally_session_by_id(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
    tally_session_id: String,
) -> Result<Response<get_tally_session_by_id::ResponseData>> {
    let variables = get_tally_session_by_id::Variables {
        tenant_id: tenant_id,
        election_event_id: election_event_id,
        tally_session_id,
    };
    let hasura_endpoint =
        env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = GetTallySessionById::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<get_tally_session_by_id::ResponseData> = res.json().await?;
    response_body.ok()
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/get_tally_sessions.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct GetTallySessions;

#[instrument(skip(auth_headers), err)]
pub async fn get_tally_sessions(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
) -> Result<Response<get_tally_sessions::ResponseData>> {
    let variables = get_tally_sessions::Variables {
        tenant_id: tenant_id,
        election_event_id: election_event_id,
    };
    let hasura_endpoint =
        env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = GetTallySessions::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<get_tally_sessions::ResponseData> = res.json().await?;
    response_body.ok()
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/set_tally_session_completed.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct SetTallySessionCompleted;

#[instrument(skip(auth_headers), err)]
pub async fn set_tally_session_completed(
    auth_headers: connection::AuthHeaders,
    hasura_transaction: &Transaction<'_>,
    tenant_id: String,
    election_event_id: String,
    tally_session_id: String,
) -> Result<Response<set_tally_session_completed::ResponseData>> {
    let variables = set_tally_session_completed::Variables {
        tenant_id: tenant_id.clone(),
        election_event_id: election_event_id.clone(),
        tally_session_id: tally_session_id.clone(),
        execution_status: Some(TallyExecutionStatus::SUCCESS.to_string()),
    };
    let hasura_endpoint =
        env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = SetTallySessionCompleted::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key.clone(), auth_headers.value.clone())
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<set_tally_session_completed::ResponseData> = res.json().await?;
    let ret = response_body.ok();

    if (ret.is_ok()) {
        // get the election event
        let election_event = get_election_event_helper(
            auth_headers.clone(),
            tenant_id.to_string(),
            election_event_id.to_string(),
        )
        .await?;

        // Save this in the electoral log
        let board_name = get_election_event_board(election_event.bulletin_board_reference.clone())
            .with_context(|| "missing bulletin board")?;

        // Cannot find a user_id with which to post this
        let electoral_log = ElectoralLog::new(
            hasura_transaction,
            &tenant_id,
            Some(&election_event_id),
            board_name.as_str(),
        )
        .await?;
        electoral_log
            .post_tally_close(election_event_id.to_string(), None, None, None)
            .await
            .with_context(|| "error posting to the electoral log")?;
    }

    ret
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/update_tally_session_status.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct UpdateTallySessionStatus;

#[instrument(skip(auth_headers), err)]
pub async fn update_tally_session_status(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
    tally_session_id: String,
    execution_status: TallyExecutionStatus,
) -> Result<Response<update_tally_session_status::ResponseData>> {
    let variables = update_tally_session_status::Variables {
        tenant_id,
        election_event_id,
        tally_session_id,
        execution_status: Some(execution_status.to_string()),
    };
    let hasura_endpoint =
        env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = UpdateTallySessionStatus::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<update_tally_session_status::ResponseData> = res.json().await?;
    response_body.ok()
}
