// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use graphql_client::{GraphQLQuery, Response};
use reqwest;
use std::env;
use tracing::instrument;
use braid_messages::newtypes::BatchNumber;
use sequent_core::services::connection;

use crate::services::to_result::ToResult;
pub use crate::types::hasura_types::*;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/get_election_event_areas.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct GetElectionEventAreas;

#[instrument(skip_all)]
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
    Ok(data.data.sequent_backend_tally_session_contest[0].session_id as BatchNumber)
}
