// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

#[macro_use]
extern crate rocket;

use graphql_client::{GraphQLQuery, Response};
use reqwest;
use std::error::Error;

type uuid = String;

// The paths are relative to the directory where your `Cargo.toml` is located.
// Both json and the GraphQL schema language are supported as sources for the
// schema
#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/schema.json",
    query_path = "src/query.graphql",
    response_derives = "Debug"
)]
pub struct Query;

#[post("/hello-world")]
fn hello_world() -> &'static str {
    // Answer needs to be valid json
    "\"Hello, world!\""
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![hello_world])
}

async fn perform_my_query(
    variables: query::Variables,
) -> Result<(), Box<dyn Error>> {
    // this is the important line
    let request_body = Query::build_query(variables);

    let client = reqwest::Client::new();
    let mut res = client.post("/graphql").json(&request_body).send().await?;
    let response_body: Response<query::ResponseData> = res.json().await?;
    println!("{:#?}", response_body);
    Ok(())
}
