// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
// use anyhow::Result;
// use graphql_client::{GraphQLQuery, Response};
// use reqwest;
// use std::env;
// use tracing::instrument;

// use crate::services::to_result::ToResult;
// pub use crate::types::hasura_types::*;
// use sequent_core::services::connection;

// #[derive(GraphQLQuery)]
// #[graphql(
//     schema_path = "src/graphql/schema.json",
//     query_path = "src/graphql/get_area_contests.graphql",
//     response_derives = "Debug,Clone,Deserialize,Serialize"
// )]
// pub struct GetAreaContests;

// #[instrument(skip_all, err)]
// pub async fn get_area_contests(
//     auth_headers: connection::AuthHeaders,
//     tenant_id: String,
//     election_event_id: String,
//     area_id: String,
// ) -> Result<Response<get_area_contests::ResponseData>> {
//     let variables = get_area_contests::Variables {
//         tenant_id: tenant_id,
//         election_event_id: election_event_id,
//         area_id: area_id,
//     };
//     let hasura_endpoint =
//         env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
//     let request_body = GetAreaContests::build_query(variables);

//     let client = reqwest::Client::new();
//     let res = client
//         .post(hasura_endpoint)
//         .header(auth_headers.key, auth_headers.value)
//         .json(&request_body)
//         .send()
//         .await?;
//     let response_body: Response<get_area_contests::ResponseData> = res.json().await?;
//     response_body.ok()
// }
