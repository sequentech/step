// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
// use anyhow::Result;
// use graphql_client::{GraphQLQuery, Response};
// use reqwest;
// use std::env;
// use tracing::{event, instrument, Level};

// use crate::services::to_result::ToResult;
// pub use crate::types::hasura_types::*;
// use sequent_core::services::connection;

// #[derive(GraphQLQuery)]
// #[graphql(
//     schema_path = "src/graphql/schema.json",
//     query_path = "src/graphql/get_election_event_areas.graphql",
//     response_derives = "Debug,Clone,Deserialize,Serialize"
// )]
// pub struct GetElectionEventAreas;

// #[instrument(skip(auth_headers), err)]
// pub async fn get_election_event_areas(
//     auth_headers: connection::AuthHeaders,
//     tenant_id: String,
//     election_event_id: String,
//     election_ids: Vec<String>,
// ) -> Result<Response<get_election_event_areas::ResponseData>> {
//     let variables = get_election_event_areas::Variables {
//         tenant_id: tenant_id,
//         election_event_id: election_event_id,
//         election_ids: election_ids,
//     };
//     let hasura_endpoint =
//         env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
//     let request_body = GetElectionEventAreas::build_query(variables);

//     let client = reqwest::Client::new();
//     let res = client
//         .post(hasura_endpoint)
//         .header(auth_headers.key, auth_headers.value)
//         .json(&request_body)
//         .send()
//         .await?;
//     let response_body: Response<get_election_event_areas::ResponseData> = res.json().await?;
//     response_body.ok()
// }

// #[derive(GraphQLQuery)]
// #[graphql(
//     schema_path = "src/graphql/schema.json",
//     query_path = "src/graphql/get_areas_by_ids.graphql",
//     response_derives = "Debug,Clone,Deserialize,Serialize"
// )]
// pub struct GetAreasByIds;

// #[instrument(skip(auth_headers), err)]
// pub async fn get_areas_by_ids(
//     auth_headers: connection::AuthHeaders,
//     tenant_id: String,
//     election_event_id: String,
//     area_ids: Vec<String>,
// ) -> Result<Response<get_areas_by_ids::ResponseData>> {
//     let variables = get_areas_by_ids::Variables {
//         tenant_id: tenant_id,
//         election_event_id: election_event_id,
//         area_ids: area_ids,
//     };
//     let hasura_endpoint =
//         env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
//     let request_body = GetAreasByIds::build_query(variables);

//     let client = reqwest::Client::new();
//     let res = client
//         .post(hasura_endpoint)
//         .header(auth_headers.key, auth_headers.value)
//         .json(&request_body)
//         .send()
//         .await?;
//     let response_body: Response<get_areas_by_ids::ResponseData> = res.json().await?;
//     response_body.ok()
// }
