use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use bytes::Bytes;
use sequent_core::services::pdf::PrintToPdfOptions;
use sequent_core::services::s3;
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
    // FIXME(ereslibre): share this code with the AWS Lambda backend
    let bucket = input.bucket;
    if let Some(bucket) = bucket {
        let bucket_path = input.bucket_path.ok_or_else(|| {
            warp::reject::custom(CustomError(format!("missing path in bucket for PDF")))
        })?;
        let raw_pdf = BASE64.decode(pdf.clone().pdf_base64).map_err(|e| {
            warp::reject::custom(CustomError(format!(
                "error deserializing PDF in base64 encoding: {e:?}"
            )))
        })?;
        s3::upload_file_to_s3(
            sha256::digest(raw_pdf),
            true,
            bucket,
            "application/pdf".to_string(),
            bucket_path,
            None,
        )
        .await
        .map_err(|e| {
            warp::reject::custom(CustomError(format!(
                "error uploading PDF file to S3: {e:?}"
            )))
        })?;
    }
    Ok(warp::reply::json(&pdf))
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
