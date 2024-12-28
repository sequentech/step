use bytes::Bytes;
use warp::{reply::Response, Filter, Rejection, Reply};
use serde::{Deserialize, Serialize};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use tracing::{info, instrument};
pub use headless_chrome::types::PrintToPdfOptions;
use std::io::Read;

use crate::io::{Input, Output};

#[derive(Debug)]
struct CustomError(String);
impl warp::reject::Reject for CustomError {}

async fn handle_render_impl(input: Input) -> Result<impl Reply, Rejection> {
    info!("OpenWhisk: Starting PDF generation");

    let bytes = sequent_core::services::pdf::html_to_pdf(input.html.unwrap_or_default(), Some(sequent_core::services::pdf::PrintToPdfOptions::default()))
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
    let init = warp::path("init")
        .and(warp::post())
        .map(|| {
            warp::reply()
        });

    let run = warp::path("run")
        .and(warp::post())
        .and(warp::body::bytes())
        .map(|input: Bytes| {
            let data = input.clone();
            unsafe {
                info!("ereslibre; received: {:?}", String::from_utf8_unchecked(data.to_vec()));
            }
            warp::reply::html(input)
        });

    // Add a health check endpoint
    let health = warp::path("health")
        .and(warp::get())
        .map(|| warp::reply::json(&serde_json::json!({
            "status": "ok"
        })));

    // Combine all routes
    let routes = init
        .or(run)
        .or(health);

    // Start the server
    warp::serve(routes)
        .run(([0, 0, 0, 0], 8080))
        .await;
}
