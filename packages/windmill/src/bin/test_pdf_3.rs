// SPDX-FileCopyrightText: 2024 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use chrono::Local;
use dotenv::dotenv;
use futures::future::join_all;
use sequent_core::util::init_log::init_log;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use structopt::StructOpt;
use windmill::services::providers::pdf_renderer::PdfRenderer;
use tracing::{event, Level};

#[derive(Debug, StructOpt)]
#[structopt(
    name = "test-pdf-3",
    about = "Test batch PDF rendering",
    setting = structopt::clap::AppSettings::ColoredHelp,
)]
struct Opt {
    input: String,
}

async fn generate_numbered_pdf(html_template: String, number: usize) -> Result<()> {
    let html = html_template.replace(
        "Annual Performance Analytics", 
        &format!("Report #{} - Annual Performance Analytics", number)
    );

    let pdf_options = headless_chrome::types::PrintToPdfOptions {
        landscape: Some(false),
        scale: Some(1.0),
        paper_width: Some(8.5),
        paper_height: Some(11.0),
        margin_top: Some(0.5),
        margin_bottom: Some(0.5),
        margin_left: Some(0.75),
        margin_right: Some(0.75),
        print_background: Some(true),
        display_header_footer: Some(false),
        prefer_css_page_size: Some(false),
        page_ranges: None,
        ignore_invalid_page_ranges: None,
        header_template: None,
        footer_template: None,
        transfer_mode: None,
    };

    let renderer = PdfRenderer::new().await?;
    let start_time = std::time::Instant::now();
    let pdf_bytes = renderer.render_pdf(html, Some(pdf_options)).await?;
    let elapsed = start_time.elapsed();

    // Check if it starts with PDF magic number
    if !pdf_bytes.starts_with(b"%PDF") {
        event!(Level::WARN, "Report #{}: Invalid PDF generated", number);
        return Ok(());
    }

    let output_dir = PathBuf::from("/tmp/output/batch_test");
    fs::create_dir_all(&output_dir)?;

    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let output_path = output_dir.join(format!("report_{:04}_{}.pdf", number, timestamp));

    fs::write(&output_path, pdf_bytes)?;
    event!(Level::INFO, "Report #{}: PDF generated in {:?}", number, elapsed);

    Ok(())
}

async fn process_batch(html_template: String, start_num: usize, batch_size: usize) -> Result<()> {
    let batch_start = std::time::Instant::now();
    event!(Level::INFO, "Starting batch {} to {}", start_num, start_num + batch_size - 1);

    let tasks: Vec<_> = (start_num..start_num + batch_size)
        .map(|i| generate_numbered_pdf(html_template.clone(), i))
        .collect();

    let results = join_all(tasks).await;
    let batch_elapsed = batch_start.elapsed();

    // Count successes and failures in this batch
    let (successes, failures): (Vec<_>, Vec<_>) = results.into_iter().partition(Result::is_ok);

    event!(
        Level::INFO,
        "Batch {} to {} completed: {} successful, {} failed, took {:?}",
        start_num,
        start_num + batch_size - 1,
        successes.len(),
        failures.len(),
        batch_elapsed
    );

    // Log any errors
    for error in failures {
        if let Err(e) = error {
            event!(Level::ERROR, "PDF generation failed: {}", e);
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    init_log(true);

    std::env::set_var(
        "PDF_LAMBDA_BINARY_PATH",
        "/workspaces/step/packages/orare/doc_renderer",
    );

    // Base HTML template (same as before)
    let html_template = r#"<html><head><style>*{margin:0;padding:0;box-sizing:border-box}:root{--primary-color:#2c3e50;--secondary-color:#3498db;--accent-color:#e74c3c;--text-color:#333;--background-color:#f9f9f9}body{font-family:'Arial',sans-serif;line-height:1.6;color:var(--text-color);background:var(--background-color);padding:40px}.header{background:linear-gradient(135deg,var(--primary-color) 0%,var(--secondary-color) 100%);color:white;padding:40px;border-radius:15px;margin-bottom:40px;box-shadow:0 5px 15px rgba(0,0,0,0.1)}.header h1{font-size:32px;margin-bottom:15px;text-shadow:2px 2px 4px rgba(0,0,0,0.2)}.header p{font-size:18px;opacity:0.9}.grid-container{display:grid;grid-template-columns:repeat(auto-fit,minmax(300px,1fr));gap:30px;margin-bottom:40px}.card{background:white;padding:25px;border-radius:12px;box-shadow:0 3px 10px rgba(0,0,0,0.05);border:1px solid rgba(0,0,0,0.05)}.card h2{color:var(--primary-color);font-size:24px;margin-bottom:20px;padding-bottom:10px;border-bottom:3px solid var(--secondary-color)}.table-container{overflow-x:auto;margin:30px 0}.table{width:100%;border-collapse:separate;border-spacing:0}.table th,.table td{padding:15px;text-align:left;border:1px solid #eee}.table th{background:var(--primary-color);color:white;font-weight:bold}.table tr:nth-child(even){background:#f8f9fa}.progress-container{margin:20px 0}.progress-bar{height:25px;background:#eee;border-radius:12.5px;overflow:hidden;margin:10px 0}.progress-fill{height:100%;background:linear-gradient(90deg,var(--secondary-color) 0%,var(--primary-color) 100%)}.status{display:inline-block;padding:8px 15px;border-radius:20px;font-size:14px;font-weight:bold}.status-success{background:#d4edda;color:#155724}.status-warning{background:#fff3cd;color:#856404}.footer{text-align:center;padding:30px;margin-top:50px;border-top:2px solid #eee;color:#666}</style></head><body><div class='header'><h1>Annual Performance Analytics</h1><p>Fiscal Year 2023-2024 • Generated on April 15, 2024</p></div><div class='grid-container'><div class='card'><h2>Financial Overview</h2><div class='progress-container'><h3>Revenue Growth</h3><div class='progress-bar'><div class='progress-fill' style='width:85%'></div></div><p>85% increase from previous year</p></div></div><div class='card'><h2>Customer Metrics</h2><div class='progress-container'><h3>Satisfaction Rate</h3><div class='progress-bar'><div class='progress-fill' style='width:92%'></div></div><p>92% positive feedback</p></div></div></div><div class='card'><h2>Project Status Overview</h2><div class='table-container'><table class='table'><thead><tr><th>Project Name</th><th>Status</th><th>Completion</th><th>Budget Utilization</th><th>Team Lead</th></tr></thead><tbody><tr><td>Digital Transformation</td><td><span class='status status-success'>Completed</span></td><td>100%</td><td>95%</td><td>Sarah Johnson</td></tr><tr><td>Cloud Migration</td><td><span class='status status-warning'>In Progress</span></td><td>75%</td><td>80%</td><td>Michael Chen</td></tr><tr><td>Security Enhancement</td><td><span class='status status-success'>Completed</span></td><td>100%</td><td>88%</td><td>Emma Davis</td></tr></tbody></table></div></div><div class='footer'><p>© 2024 Sequent Tech Inc. All rights reserved.</p><p>Generated by Analytics Engine v2.0</p><p class='disclaimer'>Confidential and Proprietary • Internal Use Only</p></div></body></html>"#.to_string();

    let total_pdfs = 10;
    let batch_size = 10;
    let start_time = std::time::Instant::now();

    event!(Level::INFO, "Starting generation of {} PDFs in batches of {}", total_pdfs, batch_size);

    // Process all PDFs in a single batch of 10
    process_batch(html_template, 1, batch_size).await?;

    let total_elapsed = start_time.elapsed();
    event!(
        Level::INFO,
        "All PDFs generated in {:?}, average time per PDF: {:?}",
        total_elapsed,
        total_elapsed / total_pdfs as u32
    );

    Ok(())
} 