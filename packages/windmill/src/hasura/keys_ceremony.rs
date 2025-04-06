// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::to_result::ToResult;
pub use crate::types::hasura_types::*;
use anyhow::Context;
use anyhow::Result;
use graphql_client::{GraphQLQuery, Response};
use reqwest;
use sequent_core::services::connection;
use serde_json::Value;
use std::env;
use tracing::{event, instrument, Level};

// #[derive(GraphQLQuery)]
// #[graphql(
//     schema_path = "src/graphql/schema.json",
//     query_path = "src/graphql/insert_keys_ceremony.graphql",
//     response_derives = "Debug,Clone,Deserialize,Serialize"
// )]
// pub struct InsertKeysCeremony;

// #[instrument(skip(auth_headers), err)]
// pub async fn insert_keys_ceremony(
//     auth_headers: connection::AuthHeaders,
//     id: String,
//     tenant_id: String,
//     election_event_id: String,
//     trustee_ids: Vec<String>,
//     threshold: i64,
//     status: Option<Value>,
//     execution_status: Option<String>,
// ) -> Result<Response<insert_keys_ceremony::ResponseData>> {
//     let variables = insert_keys_ceremony::Variables {
//         id: id,
//         tenant_id: tenant_id,
//         election_event_id: election_event_id,
//         trustee_ids: Some(trustee_ids),
//         status: status,
//         execution_status: execution_status,
//         threshold: threshold,
//     };
//     let hasura_endpoint =
//         env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
//     let request_body = InsertKeysCeremony::build_query(variables);

//     let client = reqwest::Client::new();
//     let res = client
//         .post(hasura_endpoint)
//         .header(auth_headers.key, auth_headers.value)
//         .json(&request_body)
//         .send()
//         .await?;
//     let response_body: Response<insert_keys_ceremony::ResponseData> = res.json().await?;
//     response_body.ok()
// }

// #[derive(GraphQLQuery)]
// #[graphql(
//     schema_path = "src/graphql/schema.json",
//     query_path = "src/graphql/get_keys_ceremonies.graphql",
//     response_derives = "Debug,Clone,Deserialize,Serialize"
// )]
// pub struct GetKeysCeremonies;

// #[instrument(skip(auth_headers), err)]
// pub async fn get_keys_ceremonies(
//     auth_headers: connection::AuthHeaders,
//     tenant_id: String,
//     election_event_id: String,
// ) -> Result<Response<get_keys_ceremonies::ResponseData>> {
//     let variables = get_keys_ceremonies::Variables {
//         tenant_id: tenant_id,
//         election_event_id: election_event_id,
//     };
//     let hasura_endpoint =
//         env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
//     let request_body = GetKeysCeremonies::build_query(variables);

//     let client = reqwest::Client::new();
//     let res = client
//         .post(hasura_endpoint)
//         .header(auth_headers.key, auth_headers.value)
//         .json(&request_body)
//         .send()
//         .await?;
//     let response_body: Response<get_keys_ceremonies::ResponseData> = res.json().await?;
//     response_body.ok()
// }

// #[derive(GraphQLQuery, Debug)]
// #[graphql(
//     schema_path = "src/graphql/schema.json",
//     query_path = "src/graphql/update_keys_ceremony_status.graphql",
//     response_derives = "Debug",
//     variables_derives = "Debug"
// )]
// pub struct UpdateKeysCeremonyStatus;

// #[instrument(skip(auth_headers, status), err)]
// pub async fn update_keys_ceremony_status(
//     auth_headers: connection::AuthHeaders,
//     tenant_id: String,
//     election_event_id: String,
//     keys_ceremony_id: String,
//     status: Value,
//     execution_status: String,
// ) -> Result<Response<update_keys_ceremony_status::ResponseData>> {
//     let variables = update_keys_ceremony_status::Variables {
//         tenant_id: tenant_id,
//         election_event_id: election_event_id,
//         keys_ceremony_id: keys_ceremony_id,
//         status: status,
//         execution_status: execution_status,
//     };
//     let hasura_endpoint =
//         env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
//     let request_body = UpdateKeysCeremonyStatus::build_query(variables);

//     event!(Level::INFO, "Sending graphql query {:?}", request_body);

//     let client = reqwest::Client::new();
//     let res = client
//         .post(hasura_endpoint)
//         .header(auth_headers.key, auth_headers.value)
//         .json(&request_body)
//         .send()
//         .await?;
//     let response_body: Response<update_keys_ceremony_status::ResponseData> = res.json().await?;
//     response_body.ok()
// }

// #[instrument(skip(auth_headers), err)]
// pub async fn get_keys_ceremony_by_id(
//     auth_headers: &connection::AuthHeaders,
//     tenant_id: &str,
//     election_event_id: &str,
//     keys_ceremony_id: &str,
// ) -> Result<get_keys_ceremonies::GetKeysCeremoniesSequentBackendKeysCeremony> {
//     get_keys_ceremonies(
//         auth_headers.clone(),
//         tenant_id.to_string(),
//         election_event_id.to_string(),
//     )
//     .await?
//     .data
//     .with_context(|| "error listing existing keys ceremonies")?
//     .sequent_backend_keys_ceremony
//     .into_iter()
//     .find(|ceremony| ceremony.id == keys_ceremony_id.to_string())
//     .with_context(|| "error listing existing keys ceremonies")
// }
