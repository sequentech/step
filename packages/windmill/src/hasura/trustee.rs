// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use graphql_client::{GraphQLQuery, Response};
use reqwest;
use std::env;
use tracing::instrument;

use crate::services::to_result::ToResult;
pub use crate::types::hasura_types::*;
use sequent_core::services::connection;

// #[derive(GraphQLQuery)]
// #[graphql(
//     schema_path = "src/graphql/schema.json",
//     query_path = "src/graphql/get_trustees_by_id.graphql",
//     response_derives = "Debug,Clone,Deserialize,Serialize"
// )]
// pub struct GetTrusteesById;

// #[instrument(skip(auth_headers), err)]
// pub async fn get_trustees_by_id(
//     auth_headers: connection::AuthHeaders,
//     tenant_id: String,
//     trustee_ids: Vec<String>,
// ) -> Result<Response<get_trustees_by_id::ResponseData>> {
//     let variables = get_trustees_by_id::Variables {
//         tenant_id: tenant_id,
//         trustee_ids: trustee_ids,
//     };
//     let hasura_endpoint =
//         env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
//     let request_body = GetTrusteesById::build_query(variables);

//     let client = reqwest::Client::new();
//     let res = client
//         .post(hasura_endpoint)
//         .header(auth_headers.key, auth_headers.value)
//         .json(&request_body)
//         .send()
//         .await?;
//     let response_body: Response<get_trustees_by_id::ResponseData> = res.json().await?;
//     response_body.ok()
// }

// #[derive(GraphQLQuery)]
// #[graphql(
//     schema_path = "src/graphql/schema.json",
//     query_path = "src/graphql/get_trustees_by_name.graphql",
//     response_derives = "Debug,Clone,Deserialize,Serialize"
// )]
// pub struct GetTrusteesByName;

// #[instrument(skip(auth_headers), err)]
// pub async fn get_trustees_by_name(
//     auth_headers: &connection::AuthHeaders,
//     tenant_id: &str,
//     trustee_names: &Vec<String>,
// ) -> Result<Response<get_trustees_by_name::ResponseData>> {
//     let variables = get_trustees_by_name::Variables {
//         tenant_id: tenant_id.to_string(),
//         trustee_names: trustee_names.clone(),
//     };
//     let hasura_endpoint =
//         env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
//     let request_body = GetTrusteesByName::build_query(variables);

//     let client = reqwest::Client::new();
//     let res = client
//         .post(hasura_endpoint)
//         .header(auth_headers.key.clone(), auth_headers.value.clone())
//         .json(&request_body)
//         .send()
//         .await?;
//     let response_body: Response<get_trustees_by_name::ResponseData> = res.json().await?;
//     response_body.ok()
// }
