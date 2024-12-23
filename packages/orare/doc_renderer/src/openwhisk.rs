use bytes::Bytes;
use warp::{Filter, Rejection, Reply};
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

    let bytes = sequent_core::services::pdf::html_to_pdf(input.html.unwrap_or_default(), input.pdf_options)
        .map_err(|e| {
            info!("OpenWhisk: PDF generation failed: {}", e);
            warp::reject::custom(CustomError(e.to_string()))
        })?;

    let pdf_base64 = BASE64.encode(bytes);
    info!("OpenWhisk: PDF generation completed");

    Ok(warp::reply::json(&Output { pdf_base64 }))
}

fn log_body() -> impl Filter<Extract = (), Error = Rejection> + Copy {
    // warp::body::bytes()
    //     .map(|b: Bytes| {
    //         info!("Request body: {:?}", std::str::from_utf8(&b));
    //     })
    //     .untuple_one()

    warp::body::bytes().map(|b: Bytes| {
        let v = b.clone().to_vec();
        let c = &*v;
        info!("Request body: {:?}", std::str::from_utf8(&c));
    }).untuple_one()
}


pub async fn start_server() {
    info!("Starting OpenWhisk server on 0.0.0.0:8080");

    let log = warp::log("ereslibre");

    // Create the render route
    let render = warp::path("render")
        .and(warp::post())
        .and(warp::body::json())

        .and_then(|input: Input| async move {
            handle_render_impl(input).await
        })
        .with(log);

    // Create the init/run routes
    let init = warp::path("init")
        .and(warp::post())
        .map(|| {
            let res = warp::reply::with_status("OK", warp::http::StatusCode::OK);
            println!("XXX_THE_END_OF_A_WHISK_ACTIVATION_XXX");
            eprintln!("XXX_THE_END_OF_A_WHISK_ACTIVATION_XXX");
            res
        })
        .with(log);

    let run = warp::path("run")
        .and(warp::post())
        .and(warp::body::json())
        .and(log_body())
        .and_then(|input: Input| async move {
            let res = handle_render_impl(input).await;
            println!("XXX_THE_END_OF_A_WHISK_ACTIVATION_XXX");
            eprintln!("XXX_THE_END_OF_A_WHISK_ACTIVATION_XXX");
            res
        })
        .with(log);

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
        .run(([0, 0, 0, 0], 8080))
        .await;
}
