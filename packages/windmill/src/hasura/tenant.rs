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

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/get_tenant.graphql",
    response_derives = "Debug"
)]
pub struct GetTenant;

#[instrument(skip_all, err)]
pub async fn get_tenant(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
) -> Result<Response<get_tenant::ResponseData>> {
    let variables = get_tenant::Variables {
        tenant_id: tenant_id,
    };
    let hasura_endpoint =
        env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = GetTenant::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<get_tenant::ResponseData> = res.json().await?;
    response_body.ok()
}

// #[derive(GraphQLQuery)]
// #[graphql(
//     schema_path = "src/graphql/schema.json",
//     query_path = "src/graphql/insert_tenant.graphql",
//     response_derives = "Debug"
// )]
// pub struct InsertTenant;

// #[instrument(skip_all, err)]
// pub async fn insert_tenant(
//     auth_headers: connection::AuthHeaders,
//     id: &str,
//     slug: &str,
// ) -> Result<Response<insert_tenant::ResponseData>> {
//     let variables = insert_tenant::Variables {
//         id: id.to_string(),
//         slug: slug.to_string(),
//     };
//     let hasura_endpoint =
//         env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
//     let request_body = InsertTenant::build_query(variables);

//     let client = reqwest::Client::new();
//     let res = client
//         .post(hasura_endpoint)
//         .header(auth_headers.key, auth_headers.value)
//         .json(&request_body)
//         .send()
//         .await?;
//     let response_body: Response<insert_tenant::ResponseData> = res.json().await?;
//     response_body.ok()
// }

// #[derive(GraphQLQuery)]
// #[graphql(
//     schema_path = "src/graphql/schema.json",
//     query_path = "src/graphql/get_tenant_by_slug.graphql",
//     response_derives = "Debug"
// )]
// pub struct GetTenantBySlug;

// #[instrument(skip_all, err)]
// pub async fn get_tenant_by_slug(
//     auth_headers: connection::AuthHeaders,
//     slug: String,
// ) -> Result<Response<get_tenant_by_slug::ResponseData>> {
//     let variables = get_tenant_by_slug::Variables { slug: slug };
//     let hasura_endpoint =
//         env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
//     let request_body = GetTenantBySlug::build_query(variables);

//     let client = reqwest::Client::new();
//     let res = client
//         .post(hasura_endpoint)
//         .header(auth_headers.key, auth_headers.value)
//         .json(&request_body)
//         .send()
//         .await?;
//     let response_body: Response<get_tenant_by_slug::ResponseData> = res.json().await?;
//     response_body.ok()
// }
