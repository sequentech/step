// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use base64::prelude::*;
use serde::Deserialize;
use tracing::info;
use warp::{Filter, Rejection, Reply};

use crate::io::{Input, Output};

#[derive(Debug, Deserialize)]
pub struct OpenWhiskInput {
    #[serde(rename = "action_name")]
    _action_name: String,
    #[serde(rename = "action_version")]
    _action_version: String,
    #[serde(rename = "activation_id")]
    _activation_id: String,
    #[serde(rename = "deadline")]
    _deadline: String,
    #[serde(rename = "namespace")]
    _namespace: String,
    #[serde(rename = "transaction_id")]
    _transaction_id: String,
    value: Input,
}

struct CustomError(String);

impl std::fmt::Debug for CustomError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_tuple("CustomError").field(&self.0).finish()
    }
}
impl warp::reject::Reject for CustomError {}

async fn handle_render_impl(input: Input) -> Result<impl Reply, Rejection> {
    match input {
        Input::Raw { html, pdf_options } => {
            info!("OpenWhisk: Starting PDF generation");
            let payload = match crate::pdf::render_pdf(html, pdf_options) {
                Ok(pdf) => {
                    Output {
                        pdf_base64: Some(BASE64_STANDARD.encode(pdf))
                    }
                },
                Err(err) => {
                    return Err(warp::reject::custom(CustomError(format!("error rendering PDF due to error: {:?}", err))))
                }
            };
            Ok(warp::reply::json(&payload))
        },
        Input::S3 { .. } => {
            Err(CustomError("You are trying to provide a document through the S3 mechanism to a lambda running locally on OpenWhisk. Please, provide the HTML document directly as an input to the lambda instead".to_string()).into())
        }
    }
}

pub async fn start_server() {
    info!("Starting OpenWhisk server on 0.0.0.0:8080");

    // Create the render route
    // Create the init/run routes
    let init = warp::path("init").and(warp::post()).map(warp::reply);

    let run = warp::path("run")
        .and(warp::post())
        .and(warp::body::json())
        .and_then(|input: OpenWhiskInput| async {
            info!("Input is {:?}", input);
            handle_render_impl(input.value).await
        });

    // Add a health check endpoint
    let health = warp::path("health").and(warp::get()).map(|| {
        warp::reply::json(&serde_json::json!({
            "status": "ok"
        }))
    });

    // Combine all routes
    let routes = init.or(run).or(health);

    // Start the server
    warp::serve(routes).run(([0, 0, 0, 0], 8080)).await;
}
