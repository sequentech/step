use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use bytes::Bytes;
pub use headless_chrome::types::PrintToPdfOptions;
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

    let pdf_renderer = sequent_core::services::pdf_renderer::PdfRenderer{
        transport: sequent_core::services::pdf_renderer::PdfTransport::InPlace,
    };
    let bytes = pdf_renderer.do_render_pdf(
        input.html,
        Some(sequent_core::services::pdf::PrintToPdfOptions::default()),
    )
    .await
    .map_err(|e| {
        info!("OpenWhisk: PDF generation failed: {}", e);
        warp::reject::custom(CustomError(e.to_string()))
    })?;

    let pdf_base64 = BASE64.encode(bytes);
    info!("OpenWhisk: PDF generation completed");

    Ok(warp::reply::json(&Output { pdf_base64 }))
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
