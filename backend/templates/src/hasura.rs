use graphql_client::{GraphQLQuery, Response};
use reqwest;
use rocket::response::Debug;
use rocket::serde::json::Json;
use serde::Deserialize;

type uuid = String;

// The paths are relative to the directory where your `Cargo.toml` is located.
// Both json and the GraphQL schema language are supported as sources for the
// schema
#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/query.graphql",
    response_derives = "Debug"
)]
pub struct GetTenant;

pub async fn perform_my_query(
    variables: get_tenant::Variables,
) -> Result<Response<get_tenant::ResponseData>, reqwest::Error> {
    let request_body = GetTenant::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post("http://hasura:8080/v1/graphql")
        .header("X-Hasura-Admin-Secret", "admin")
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<get_tenant::ResponseData> = res.json().await?;
    println!("{:#?}", response_body);
    Ok(response_body)
}

pub async fn run_query(
    tenant_id: String,
) -> Result<Response<get_tenant::ResponseData>, reqwest::Error> {
    let variables = get_tenant::Variables {
        tenant_id: Some(tenant_id),
    };
    perform_my_query(variables).await
}
