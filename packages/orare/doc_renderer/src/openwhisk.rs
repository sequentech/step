use warp::{Filter, Rejection, Reply};
use serde::{Deserialize, Serialize};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use headless_chrome::types::PrintToPdfOptions;
use tracing::{info, instrument};

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

#[instrument(skip(input))]
async fn handle_render(input: Input) -> Result<impl Reply, Rejection> {
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
    info!("Starting OpenWhisk server on 0.0.0.0:8080");
    
    let render = warp::post()
        .and(warp::path("render"))
        .and(warp::body::json())
        .and_then(handle_render);

    warp::serve(render)
        .run(([0, 0, 0, 0], 8080))
        .await;
} 