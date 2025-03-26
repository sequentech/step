// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::to_result::ToResult;
pub use crate::types::hasura_types::*;
use anyhow::Result;
use graphql_client::{GraphQLQuery, Response};
use reqwest;
use sequent_core::services::connection;
use sequent_core::types::ceremonies::TallyCeremonyStatus;
use std::env;
use tracing::instrument;

// #[derive(GraphQLQuery)]
// #[graphql(
//     schema_path = "src/graphql/schema.json",
//     query_path = "src/graphql/get_last_tally_session_execution.graphql",
//     response_derives = "Debug,Clone,Deserialize,Serialize"
// )]
// pub struct GetLastTallySessionExecution;

// #[instrument(skip(auth_headers), err)]
// pub async fn get_last_tally_session_execution(
//     auth_headers: connection::AuthHeaders,
//     tenant_id: String,
//     election_event_id: String,
//     tally_session_id: String,
//     election_ids: Vec<String>,
// ) -> Result<Response<get_last_tally_session_execution::ResponseData>> {
//     let hasura_endpoint =
//         env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));

//     let variables = get_last_tally_session_execution::Variables {
//         tenant_id,
//         election_event_id,
//         tally_session_id,
//         election_ids,
//     };

//     let request_body = GetLastTallySessionExecution::build_query(variables);

//     let client = reqwest::Client::new();

//     let res = client
//         .post(hasura_endpoint)
//         .header(auth_headers.key, auth_headers.value)
//         .json(&request_body)
//         .send()
//         .await?;

//     let response_body: Response<get_last_tally_session_execution::ResponseData> =
//         res.json().await?;

//     response_body.ok()
// }

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/insert_tally_session_execution.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct InsertTallySessionExecution;

#[instrument(skip(auth_headers, status), err)]
pub async fn insert_tally_session_execution(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
    current_message_id: i64,
    tally_session_id: String,
    status: Option<TallyCeremonyStatus>,
    results_event_id: Option<String>,
    session_ids: Option<Vec<i64>>,
) -> Result<Response<insert_tally_session_execution::ResponseData>> {
    let hasura_endpoint =
        env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
    let json_status = match status {
        Some(value) => Some(serde_json::to_value(value)?),
        None => None,
    };

    let variables = insert_tally_session_execution::Variables {
        tenant_id,
        election_event_id,
        current_message_id,
        tally_session_id,
        status: json_status,
        results_event_id,
        session_ids,
    };

    let request_body = InsertTallySessionExecution::build_query(variables);

    let client = reqwest::Client::new();

    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;

    let response_body: Response<insert_tally_session_execution::ResponseData> = res.json().await?;

    response_body.ok()
}
