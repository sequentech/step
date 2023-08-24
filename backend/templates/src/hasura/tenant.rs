// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::connection;
use graphql_client::{GraphQLQuery, Response};
use reqwest;
use rocket::response::Debug;
use rocket::serde::json::Json;
use serde::Deserialize;
use std::env;

type uuid = String;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/get_tenant.graphql",
    response_derives = "Debug"
)]
pub struct GetTenant;


pub async fn perform_get_tenant(
    auth_headers: connection::AuthHeaders,
    variables: get_tenant::Variables,
) -> Result<Response<get_tenant::ResponseData>, reqwest::Error> {
    let hasura_endpoint = env::var("HASURA_ENDPOINT")
        .expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = GetTenant::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<get_tenant::ResponseData> = res.json().await?;
    Ok(response_body)
}

pub async fn get_tenant(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
) -> Result<Response<get_tenant::ResponseData>, reqwest::Error> {
    let variables = get_tenant::Variables {
        tenant_id: Some(tenant_id),
    };
    perform_get_tenant(auth_headers, variables).await
}
