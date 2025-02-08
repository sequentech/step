// SPDX-FileCopyrightText: 2024 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use base64::prelude::*;
use bytes::Bytes;
use sequent_core::services::pdf::PrintToPdfOptions;
use serde::{Deserialize, Serialize};
use std::io::Read;
use tracing::{info, instrument};
use warp::{reply::Response, Filter, Rejection, Reply};

use crate::io::{Input, Output};

#[derive(Debug, Deserialize)]
pub struct OpenWhiskInput {
    action_name: String,
    action_version: String,
    activation_id: String,
    deadline: String,
    namespace: String,
    transaction_id: String,
    value: Input,
}

#[derive(Debug)]
struct CustomError(String);
impl warp::reject::Reject for CustomError {}

async fn handle_render_impl(input: Input) -> Result<impl Reply, Rejection> {
    info!("OpenWhisk: Starting PDF generation");
    let pdf = crate::pdf::render_pdf(input.clone())
        .map_err(|e| warp::reject::custom(CustomError(e.to_string())))?;
    let payload = if let Some(pdf) = pdf.pdf {
        Output {
            pdf_base64: Some(BASE64_STANDARD.encode(pdf)),
            ..Default::default()
        }
    } else if let Some(pdf_base64) = pdf.pdf_base64 {
        Output {
            pdf_base64: Some(pdf_base64),
            ..Default::default()
        }
    } else {
        return Err(CustomError(format!("missing PDF")).into());
    };
    Ok(warp::reply::json(&payload))
}

pub async fn start_server() {
    info!("Starting OpenWhisk server on 0.0.0.0:8080");

    // Create the render route
    // Create the init/run routes
    let init = warp::path("init").and(warp::post()).map(|| warp::reply());

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
