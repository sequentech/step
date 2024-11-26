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

    // Complex HTML template with advanced styling and SVG charts
    let html = r#"<!DOCTYPE html>
<html>
<head>
    <style>
        :root {
            --primary: #2c3e50;
            --secondary: #3498db;
            --accent: #e74c3c;
            --success: #2ecc71;
            --warning: #f1c40f;
            --text: #2c3e50;
            --light: #ecf0f1;
        }
        
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
            font-family: 'Arial', sans-serif;
        }
        
        body {
            background: #f9f9f9;
            color: var(--text);
            line-height: 1.6;
            padding: 2rem;
        }
        
        .header {
            background: linear-gradient(135deg, var(--primary) 0%, var(--secondary) 100%);
            color: white;
            padding: 2rem;
            border-radius: 10px;
            margin-bottom: 2rem;
            box-shadow: 0 4px 6px rgba(0,0,0,0.1);
        }
        
        .header h1 {
            font-size: 2.5rem;
            margin-bottom: 1rem;
        }
        
        .grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
            gap: 2rem;
            margin-bottom: 2rem;
        }
        
        .card {
            background: white;
            padding: 1.5rem;
            border-radius: 10px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.05);
        }
        
        .chart {
            width: 100%;
            height: 200px;
            margin: 1rem 0;
        }
        
        table {
            width: 100%;
            border-collapse: collapse;
            margin: 1rem 0;
        }
        
        th, td {
            padding: 1rem;
            text-align: left;
            border-bottom: 1px solid var(--light);
        }
        
        th {
            background: var(--primary);
            color: white;
        }
        
        tr:nth-child(even) {
            background: var(--light);
        }
        
        .status {
            padding: 0.5rem 1rem;
            border-radius: 20px;
            font-size: 0.9rem;
            font-weight: bold;
        }
        
        .status-success {
            background: var(--success);
            color: white;
        }
        
        .status-warning {
            background: var(--warning);
            color: var(--text);
        }
        
        .footer {
            text-align: center;
            padding: 2rem;
            margin-top: 2rem;
            border-top: 2px solid var(--light);
            color: var(--text);
        }

        .metric {
            display: flex;
            align-items: center;
            margin: 1rem 0;
        }

        .metric-value {
            font-size: 2rem;
            font-weight: bold;
            margin-right: 1rem;
        }

        .metric-label {
            color: var(--text);
            opacity: 0.8;
        }

        .progress-bar {
            width: 100%;
            height: 10px;
            background: var(--light);
            border-radius: 5px;
            overflow: hidden;
            margin: 0.5rem 0;
        }

        .progress-fill {
            height: 100%;
            background: linear-gradient(90deg, var(--secondary) 0%, var(--primary) 100%);
            transition: width 0.3s ease;
        }
    </style>
</head>
<body>
    <div class="header">
        <h1>OpenWhisk PDF Generation Test</h1>
        <p>Generated on ${timestamp} • Complex Layout Demo</p>
    </div>

    <div class="grid">
        <div class="card">
            <h2>Performance Metrics</h2>
            <div class="metric">
                <div class="metric-value">98.5%</div>
                <div class="metric-label">
                    <div>System Uptime</div>
                    <div class="progress-bar">
                        <div class="progress-fill" style="width: 98.5%"></div>
                    </div>
                </div>
            </div>
            <div class="metric">
                <div class="metric-value">45ms</div>
                <div class="metric-label">
                    <div>Average Response Time</div>
                    <div class="progress-bar">
                        <div class="progress-fill" style="width: 75%"></div>
                    </div>
                </div>
            </div>
        </div>

        <div class="card">
            <h2>System Status</h2>
            <table>
                <thead>
                    <tr>
                        <th>Service</th>
                        <th>Status</th>
                        <th>Load</th>
                    </tr>
                </thead>
                <tbody>
                    <tr>
                        <td>API Gateway</td>
                        <td><span class="status status-success">Operational</span></td>
                        <td>23%</td>
                    </tr>
                    <tr>
                        <td>Database</td>
                        <td><span class="status status-warning">High Load</span></td>
                        <td>78%</td>
                    </tr>
                    <tr>
                        <td>Cache</td>
                        <td><span class="status status-success">Operational</span></td>
                        <td>45%</td>
                    </tr>
                </tbody>
            </table>
        </div>
    </div>

    <div class="card">
        <h2>Test Configuration</h2>
        <table>
            <tr>
                <th>Parameter</th>
                <th>Value</th>
                <th>Description</th>
            </tr>
            <tr>
                <td>Transport</td>
                <td>OpenWhisk</td>
                <td>PDF generation transport method</td>
            </tr>
            <tr>
                <td>Endpoint</td>
                <td>http://orare:8082/render</td>
                <td>Service endpoint for PDF generation</td>
            </tr>
            <tr>
                <td>Paper Size</td>
                <td>Letter (8.5" x 11")</td>
                <td>Output document dimensions</td>
            </tr>
        </table>
    </div>

    <div class="footer">
        <p>© 2024 Sequent Tech Inc • PDF Generation Test Report</p>
        <p>Generated using OpenWhisk Transport</p>
    </div>
</body>
</html>"#.replace("${timestamp}", &Local::now().format("%Y-%m-%d %H:%M:%S").to_string());

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

    event!(Level::INFO, "Starting OpenWhisk PDF generation test");
    let start_time = std::time::Instant::now();

    let renderer = PdfRenderer::new().await?;
    let pdf_bytes = renderer.render_pdf(html, Some(pdf_options)).await?;

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
    let output_path = output_dir.join(format!("complex_test_{}.pdf", timestamp));

    fs::write(&output_path, &pdf_bytes)?;
    event!(Level::INFO, "PDF written to {}", output_path.display());

    // Print statistics
    println!("\nOpenWhisk PDF Generation Test Results:");
    println!("--------------------------------------");
    println!("Generation Time: {:?}", elapsed);
    println!("Output Size: {} bytes", pdf_bytes.len());
    println!("Output Location: {}", output_path.display());
    println!("PDF Magic Number Present: {}", is_valid_pdf);

    Ok(())
} 