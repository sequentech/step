// SPDX-FileCopyrightText: 2024 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// use orare::lambda_runtime;
// use serde::{Deserialize, Serialize};
// use headless_chrome::types::PrintToPdfOptions;
// use std::path::PathBuf;
// use std::fs;
// use chrono;
// use tracing::info;

// #[derive(Serialize, Deserialize)]
// struct Input {
//     name: String,
// }
// #[derive(Serialize, Deserialize)]
// struct Output {
//     message: String,
// }
// #[lambda_runtime]
// fn hello(input: Input) -> Output {
//     Output {
//         message: format!("Hello, {}!", input.name),
//     }
// }

// #[derive(Deserialize)]
// struct Input {
//     html: String,
//     #[serde(default)]
//     pdf_options: Option<PrintToPdfOptions>,
// }

// #[derive(Serialize)]
// struct Output {
//     pdf_base64: String,
// }

// #[cfg(feature = "openwhisk")]
// fn save_development_files(html: &str, pdf_bytes: &[u8]) -> anyhow::Result<()> {
//     let output_dir = PathBuf::from("dev_output");
//     fs::create_dir_all(&output_dir)?;

//     let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");

//     let html_path = output_dir.join(format!("input_{}.html", timestamp));
//     fs::write(&html_path, html)?;
//     info!("Saved input HTML to: {}", html_path.display());

//     let pdf_path = output_dir.join(format!("output_{}.pdf", timestamp));
//     fs::write(&pdf_path, pdf_bytes)?;
//     info!("Saved output PDF to: {}", pdf_path.display());

//     Ok(())
// }

// #[lambda_runtime]
// fn render_pdf(input: Input) -> Result<Output, String> {
//     use sequent_core::services::pdf;
//     use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

//     let pdf_bytes = pdf::html_to_pdf(input.html.clone(), input.pdf_options)
//         .map_err(|e| e.to_string())?;

//     #[cfg(feature = "openwhisk")]
//     {
//         if let Err(e) = save_development_files(&input.html, &pdf_bytes) {
//             eprintln!("Warning: Failed to save development files: {}", e);
//         }
//     }

//     let pdf_base64 = BASE64.encode(pdf_bytes);
//     Ok(Output { pdf_base64 })
// }

// SPDX-FileCopyrightText: 2024 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use headless_chrome::types::PrintToPdfOptions;
use orare::lambda_runtime;
use sequent_core::services::{pdf, pdf_renderer::PdfService};
use serde::{Deserialize, Serialize};
use tracing::info;

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
    #[cfg(feature = "openwhisk-dev")]
    let service = PdfService::with_openwhisk_dev();
    #[cfg(feature = "inplace")]
    let service = PdfService::with_inplace();
    #[cfg(not(any(feature = "openwhisk-dev", feature = "inplace")))]
    let service = PdfService::with_openwhisk(String::from("some/path"));

    let bytes = service
        .render_pdf(input.html, input.pdf_options)
        .map_err(|e| e.to_string())?;

    let pdf_base64 = BASE64.encode(bytes);
    Ok(Output { pdf_base64 })
}
