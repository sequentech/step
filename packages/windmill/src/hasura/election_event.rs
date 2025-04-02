// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Context;
use anyhow::{anyhow, Result};
use graphql_client::{GraphQLQuery, Response};
use reqwest;
use serde_json::Value;
use std::env;
use tracing::instrument;

use crate::services::to_result::ToResult;
pub use crate::types::hasura_types::*;
use sequent_core::services::connection;

// #[derive(GraphQLQuery)]
// #[graphql(
//     schema_path = "src/graphql/schema.json",
//     query_path = "src/graphql/update_election_event_board.graphql",
//     response_derives = "Debug"
// )]
// pub struct UpdateElectionEventBoard;

// #[derive(GraphQLQuery)]
// #[graphql(
//     schema_path = "src/graphql/schema.json",
//     query_path = "src/graphql/update_election_event_status.graphql",
//     response_derives = "Debug"
// )]
// pub struct UpdateElectionEventStatus;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/update_election_event_public_key.graphql",
    response_derives = "Debug"
)]
pub struct UpdateElectionEventPublicKey;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/get_election_event.graphql",
    response_derives = "Debug, Clone, Deserialize"
)]
pub struct GetElectionEvent;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/insert_election_event.graphql",
    response_derives = "Debug, Clone, Deserialize",
    variables_derives = "Debug, Clone, Deserialize"
)]
pub struct InsertElectionEvent;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/get_batch_election_events.graphql",
    response_derives = "Debug"
)]
pub struct GetBatchElectionEvents;

// #[instrument(skip_all, err)]
// pub async fn update_election_event_board(
//     auth_headers: connection::AuthHeaders,
//     tenant_id: String,
//     election_event_id: String,
//     board: Value,
// ) -> Result<Response<update_election_event_board::ResponseData>> {
//     let variables = update_election_event_board::Variables {
//         tenant_id: tenant_id,
//         election_event_id: election_event_id,
//         board: board,
//     };
//     let hasura_endpoint =
//         env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
//     let request_body = UpdateElectionEventBoard::build_query(variables);

//     let client = reqwest::Client::new();
//     let res = client
//         .post(hasura_endpoint)
//         .header(auth_headers.key, auth_headers.value)
//         .json(&request_body)
//         .send()
//         .await?;
//     let response_body: Response<update_election_event_board::ResponseData> = res.json().await?;
//     response_body.ok()
// }

// #[instrument(skip_all, err)]
// pub async fn update_election_event_status(
//     auth_headers: connection::AuthHeaders,
//     tenant_id: String,
//     election_event_id: String,
//     status: Value,
// ) -> Result<Response<update_election_event_status::ResponseData>> {
//     let variables = update_election_event_status::Variables {
//         tenant_id: tenant_id,
//         election_event_id: election_event_id,
//         status: status,
//     };
//     let hasura_endpoint =
//         env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
//     let request_body = UpdateElectionEventStatus::build_query(variables);

//     let client = reqwest::Client::new();
//     let res = client
//         .post(hasura_endpoint)
//         .header(auth_headers.key, auth_headers.value)
//         .json(&request_body)
//         .send()
//         .await?;
//     let response_body: Response<update_election_event_status::ResponseData> = res.json().await?;
//     response_body.ok()
// }

#[instrument(skip(auth_headers), err)]
pub async fn get_election_event(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
) -> Result<Response<get_election_event::ResponseData>> {
    let variables = get_election_event::Variables {
        tenant_id: tenant_id,
        election_event_id: election_event_id,
    };
    let hasura_endpoint =
        env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = GetElectionEvent::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<get_election_event::ResponseData> = res.json().await?;
    response_body.ok()
}

#[instrument(skip_all, err)]
pub async fn update_election_event_public_key(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
    public_key: String,
) -> Result<Response<update_election_event_public_key::ResponseData>> {
    let variables = update_election_event_public_key::Variables {
        tenant_id: tenant_id,
        election_event_id: election_event_id,
        public_key: public_key,
    };
    let hasura_endpoint =
        env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = UpdateElectionEventPublicKey::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<update_election_event_public_key::ResponseData> =
        res.json().await?;
    response_body.ok()
}

#[instrument(skip_all, err)]
pub async fn insert_election_event(
    auth_headers: connection::AuthHeaders,
    object: insert_election_event::sequent_backend_election_event_insert_input,
) -> Result<Response<insert_election_event::ResponseData>> {
    use insert_election_event::*;
    let variables = Variables { object };
    let hasura_endpoint =
        env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = InsertElectionEvent::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<insert_election_event::ResponseData> = res.json().await?;
    response_body.ok()
}

#[instrument(skip_all, err)]
pub async fn get_batch_election_events(
    auth_headers: connection::AuthHeaders,
    limit: i64,
    offset: i64,
) -> Result<Response<get_batch_election_events::ResponseData>> {
    let variables = get_batch_election_events::Variables {
        limit: limit,
        offset: offset,
    };
    let hasura_endpoint =
        env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = GetBatchElectionEvents::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<get_batch_election_events::ResponseData> = res.json().await?;
    response_body.ok()
}

#[instrument(skip(auth_headers), err)]
pub async fn get_current_bulletin_board_message_id(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
) -> Result<Response<get_election_event::ResponseData>> {
    let variables = get_election_event::Variables {
        tenant_id: tenant_id,
        election_event_id: election_event_id,
    };
    let hasura_endpoint =
        env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = GetElectionEvent::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<get_election_event::ResponseData> = res.json().await?;
    response_body.ok()
}

#[instrument(skip_all, err)]
pub async fn get_election_event_helper(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
) -> Result<get_election_event::GetElectionEventSequentBackendElectionEvent> {
    get_election_event(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
    )
    .await
    .with_context(|| "error fetching election event")?
    .data
    .with_context(|| "error fetching election event")?
    .sequent_backend_election_event
    .get(0)
    .clone()
    .ok_or(anyhow!("can't find election event"))
    .cloned()
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/update_election_event_statistics.graphql",
    response_derives = "Debug"
)]
pub struct UpdateElectionEventStatistics;

#[instrument(skip_all, err)]
pub async fn update_election_event_statistics(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
    statistics: Value,
) -> Result<Response<update_election_event_statistics::ResponseData>> {
    let variables = update_election_event_statistics::Variables {
        tenant_id: tenant_id,
        election_event_id: election_event_id,
        statistics: statistics,
    };
    let hasura_endpoint =
        env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = UpdateElectionEventStatistics::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<update_election_event_statistics::ResponseData> =
        res.json().await?;
    response_body.ok()
}
