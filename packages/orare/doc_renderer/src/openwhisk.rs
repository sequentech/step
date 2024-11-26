use warp::{Filter, Rejection, Reply};
use serde::{Deserialize, Serialize};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use tracing::{info, instrument};
use headless_chrome::types::PrintToPdfOptions;

#[derive(Deserialize)]
pub struct Input {
    html: String,
    #[serde(default)]
    pdf_options: Option<PrintToPdfOptions>,
}

#[derive(Serialize)]
pub struct Output {
    pdf_base64: String,
}

#[derive(Debug)]
struct CustomError(String);
impl warp::reject::Reject for CustomError {}

async fn handle_render_impl(input: Input) -> Result<impl Reply, Rejection> {
    info!("OpenWhisk: Starting PDF generation");
    
    let bytes = sequent_core::services::pdf::html_to_pdf(input.html, input.pdf_options)
        .map_err(|e| {
            info!("OpenWhisk: PDF generation failed: {}", e);
            warp::reject::custom(CustomError(e.to_string()))
        })?;

    let pdf_base64 = BASE64.encode(bytes);
    info!("OpenWhisk: PDF generation completed");

    Ok(warp::reply::json(&Output { pdf_base64 }))
}

pub async fn start_server() {
    info!("Starting OpenWhisk server on 0.0.0.0:8082");
    
    // Create the render route
    let render = warp::path("render")
        .and(warp::post())
        .and(warp::body::json())
        .and_then(|input: Input| async move {
            handle_render_impl(input).await
        });

    // Create the init/run routes
    let init = warp::path("init")
        .and(warp::post())
        .map(|| warp::reply::json(&serde_json::json!({
            "status": "ok",
            "message": "Initialized"
        })));

    let run = warp::path("run")
        .and(warp::post())
        .and(warp::body::json())
        .and_then(|input: Input| async move {
            handle_render_impl(input).await
        });

    // Add a health check endpoint
    let health = warp::path("health")
        .and(warp::get())
        .map(|| warp::reply::json(&serde_json::json!({
            "status": "ok"
        })));

    // Combine all routes
    let routes = render
        .or(init)
        .or(run)
        .or(health)
        .with(warp::cors().allow_any_origin());

    // Start the server
    warp::serve(routes)
        .run(([0, 0, 0, 0], 8082))
        .await;
} 