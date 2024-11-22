// SPDX-FileCopyrightText: 2024 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use chrono::Local;
use dotenv::dotenv;
use sequent_core::util::init_log::init_log;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use structopt::StructOpt;
use windmill::services::providers::pdf_renderer::PdfRenderer;
use tracing::{event, Level};

#[derive(Debug, StructOpt)]
#[structopt(
    name = "test-pdf-4",
    about = "Test the PDF renderer with OpenWhisk",
    setting = structopt::clap::AppSettings::ColoredHelp,
)]
struct Opt {
    input: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    init_log(true);

    // Set OpenWhisk transport
    std::env::set_var("PDF_TRANSPORT_NAME", "orare-openwhisk");
    std::env::set_var(
        "PDF_LAMBDA_BINARY_PATH",
        "/workspaces/step/packages/orare/doc_renderer",
    );

    // Log environment for debugging
    event!(Level::INFO, "Current environment variables:");
    for (key, value) in std::env::vars() {
        if key.contains("PDF") {
            event!(Level::INFO, "{}: {}", key, value);
        }
    }

    // Simple test input first
    let test_input = r#"{
        "html": "<html><body><h1>OpenWhisk Test</h1><p>Testing PDF generation via OpenWhisk</p></body></html>",
        "pdf_options": {
            "landscape": false,
            "scale": 1.0,
            "paper_width": 8.5,
            "paper_height": 11.0,
            "margin_top": 0.5,
            "margin_bottom": 0.5,
            "margin_left": 0.75,
            "margin_right": 0.75,
            "print_background": true,
            "display_header_footer": false,
            "prefer_css_page_size": false
        }
    }"#;

    let input: Value = serde_json::from_str(test_input)?;

    let html = input["html"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing html in input"))?
        .to_string();
    let pdf_options = input["pdf_options"].clone();
    let pdf_options = if pdf_options.is_null() {
        None
    } else {
        Some(serde_json::from_value(pdf_options)?)
    };

    event!(Level::INFO, "Starting OpenWhisk PDF generation test");
    let start_time = std::time::Instant::now();

    let renderer = PdfRenderer::new().await?;
    let pdf_bytes = renderer.render_pdf(html, pdf_options).await?;

    let elapsed = start_time.elapsed();
    event!(Level::INFO, "PDF generation completed in {:?}", elapsed);

    // Verify PDF content
    let is_valid_pdf = pdf_bytes.starts_with(b"%PDF");
    if is_valid_pdf {
        event!(Level::INFO, "Valid PDF generated (magic number verified)");
    } else {
        event!(Level::WARN, "Generated file might not be a valid PDF!");
    }

    // Save the output
    let output_dir = PathBuf::from("/tmp/output/openwhisk_test");
    fs::create_dir_all(&output_dir)?;

    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let output_path = output_dir.join(format!("openwhisk_test_{}.pdf", timestamp));

    fs::write(&output_path, &pdf_bytes)?;
    event!(Level::INFO, "PDF written to {}", output_path.display());

    // Print statistics
    println!("OpenWhisk PDF Generation Test Results:");
    println!("--------------------------------------");
    println!("Generation Time: {:?}", elapsed);
    println!("Output Size: {} bytes", pdf_bytes.len());
    println!("Output Location: {}", output_path.display());
    println!("PDF Magic Number Present: {}", is_valid_pdf);

    Ok(())
} 