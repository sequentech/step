// SPDX-FileCopyrightText: 2024 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use orare::lambda_runtime;
use serde::{Deserialize, Serialize};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use tracing::info;
use headless_chrome::types::PrintToPdfOptions;
mod openwhisk;

#[derive(Deserialize)]
struct Input {
    html: String,
    #[serde(default)]
    pdf_options: Option<PrintToPdfOptions>,
}

#[derive(Serialize)]
struct Output {
    pdf_base64: String,
}

#[lambda_runtime]
fn render_pdf(input: Input) -> Result<Output, String> {
    info!("Starting PDF generation");

    match std::env::var("PDF_TRANSPORT_NAME").unwrap_or_default().as_str() {
        "orare-openwhisk" => {
            info!("Using OpenWhisk mode");
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(openwhisk::start_server());
            Ok(Output { pdf_base64: "".to_string() }) // Server never returns
        }
        _ => {
            info!("Using Inplace mode");
            let bytes = sequent_core::services::pdf::html_to_pdf(input.html, input.pdf_options)
                .map_err(|e| e.to_string())?;

            let pdf_base64 = BASE64.encode(bytes);
            info!("PDF generation completed");
            Ok(Output { pdf_base64 })
        }
    }
}
