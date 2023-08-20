// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

#[macro_use]
extern crate rocket;

use rocket::response::Debug;
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

#[post("/hello-world", format = "json", data="<body>")]
async fn hello_world(body: Json<Body>) -> Result<&'static str, Debug<reqwest::Error>> {
    println!("{:#?}", body.into_inner());

    // Answer needs to be valid json
    Ok("\"Hello, world!\"")
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![hello_world])
}