// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use graphql_client::{GraphQLQuery, Response};
use reqwest;
use rocket::serde::json::Value;
use serde::Deserialize;
use std::env;
use tracing::{event, instrument, Level};

use crate::connection;
pub use crate::hasura::types::*;
use crate::services::to_result::ToResult;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/insert_scheduled_event.graphql",
    response_derives = "Debug"
)]
pub struct InsertScheduledEvent;

#[instrument(skip_all)]
pub async fn insert_scheduled_event(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
    event_processor: String,
    cron_config: Option<String>,
    event_payload: Value,
    created_by: String,
) -> Result<Response<insert_scheduled_event::ResponseData>> {
    let variables = insert_scheduled_event::Variables {
        tenant_id: tenant_id,
        election_event_id: election_event_id,
        event_processor: event_processor,
        cron_config: cron_config,
        event_payload: event_payload,
        created_by: created_by,
    };
    let hasura_endpoint = env::var("HASURA_ENDPOINT")
        .expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = InsertScheduledEvent::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<insert_scheduled_event::ResponseData> =
        res.json().await?;
    if response_body.errors.is_some() {
        let messages = response_body
            .errors
            .clone()
            .unwrap()
            .into_iter()
            .map(|error| error.message.clone())
            .collect::<Vec<String>>()
            .join(" - ");
        event!(Level::ERROR, "response_body: {}", messages);
    }
    response_body.ok()
}
