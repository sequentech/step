// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

#[macro_use]
extern crate rocket;

use rocket::response::Debug;
use graphql_client::{GraphQLQuery, Response};
use reqwest;
use serde::Deserialize;
use rocket::serde::json::Json;

type uuid = String;

#[derive(Deserialize, Debug)]
struct Input {
    string: String
}
#[derive(Deserialize, Debug)]
struct Body {
    input: Input
}

// The paths are relative to the directory where your `Cargo.toml` is located.
// Both json and the GraphQL schema language are supported as sources for the
// schema
#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/query.graphql",
    response_derives = "Debug"
)]
pub struct Query;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/insert.graphql",
    response_derives = "Debug"
)]
pub struct InsertTenant;

#[post("/hello-world", format = "json", data="<body>")]
async fn hello_world(body: Json<Body>) -> Result<&'static str, Debug<reqwest::Error>> {
    println!("{:#?}", body.into_inner());

    let variables = query::Variables {};
    perform_my_query(variables).await?;

    // Answer needs to be valid json
    Ok("\"Hello, world!\"")
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![hello_world])
}

async fn perform_my_query(
    variables: query::Variables,
) -> Result<(), reqwest::Error> {
    let request_body = Query::build_query(variables);

    let client = reqwest::Client::new();
    let mut res = client.post("http://localhost:8080/v1/graphql").json(&request_body).send().await?;
    let response_body: Response<query::ResponseData> = res.json().await?;
    println!("{:#?}", response_body);
    Ok(())
}

async fn insert(
    variables: insert_tenant::Variables,
) -> Result<(), reqwest::Error> {
    let request_body = InsertTenant::build_query(variables);

    let client = reqwest::Client::new();
    let mut res = client.post("http://localhost:8080/v1/graphql").json(&request_body).send().await?;
    let response_body: Response<query::ResponseData> = res.json().await?;
    println!("{:#?}", response_body);
    Ok(())
}