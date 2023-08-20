// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

#[macro_use]
extern crate rocket;

use rocket::response::Debug;
use reqwest;
use serde::Deserialize;
use rocket::serde::json::Json;
use rocket::serde::json::Value;
use handlebars::Handlebars;

#[derive(Deserialize, Debug)]
struct Body {
    template: String,
    variables: Value, //JSON
    format: String // html|text|pdf
}

#[post("/render-template", format = "json", data="<body>")]
async fn hello_world(body: Json<Body>) -> Result<String, Debug<reqwest::Error>> {
    //println!("{:#?}", body.into_inner());
    let input = body.into_inner();

    let reg = Handlebars::new();
    let render = reg.render_template(input.template.as_str(), &input.variables).unwrap();

    // Answer needs to be valid json
    Ok(render)
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![hello_world])
}