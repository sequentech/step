use crate::connection;
use graphql_client::{GraphQLQuery, Response};
use reqwest;
use rocket::response::Debug;
use rocket::serde::json::Json;
use serde::Deserialize;
use std::env;

type uuid = String;

// The paths are relative to the directory where your `Cargo.toml` is located.
// Both json and the GraphQL schema language are supported as sources for the
// schema
#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/get_tenant.graphql",
    response_derives = "Debug"
)]
pub struct GetTenant;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/insert_document.graphql",
    response_derives = "Debug"
)]
pub struct InsertDocument;

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

pub async fn perform_insert_document(
    auth_headers: connection::AuthHeaders,
    variables: insert_document::Variables,
) -> Result<Response<insert_document::ResponseData>, reqwest::Error> {
    let hasura_endpoint = env::var("HASURA_ENDPOINT")
        .expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = InsertDocument::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<insert_document::ResponseData> =
        res.json().await?;
    Ok(response_body)
}

pub async fn insert_document(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
    name: String,
    media_type: String,
    size: i64,
) -> Result<Response<insert_document::ResponseData>, reqwest::Error> {
    let variables = insert_document::Variables {
        tenant_id: tenant_id,
        election_event_id: election_event_id,
        name: name,
        media_type: media_type,
        size: size,
    };
    perform_insert_document(auth_headers, variables).await
}
