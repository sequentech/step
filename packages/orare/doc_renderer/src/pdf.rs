// SPDX-FileCopyrightText: 2024 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::io::{Input, Output};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use sequent_core::services::pdf::PrintToPdfOptions;
use sequent_core::services::s3;
use tokio::runtime::Runtime;
use tracing::{info, instrument};

pub fn render_pdf(input: Input) -> Result<Output, String> {
    let html = if let Some(html) = input.html {
        html
    } else if let Some(html_path) = input.html_path {
        let rt =
            Runtime::new().map_err(|err| format!("error creating Tokio runtime: {:?}", err))?;
        rt.block_on(async { s3::download_s3_file_to_string(&html_path).await })
            .map_err(|err| format!("error downloading HTML to render from s3: {:?}", err))?
    } else {
        return Err(format!("no html or html_path were provided"));
    };
    let bytes = sequent_core::services::pdf::html_to_pdf(html, input.pdf_options)
        .map_err(|e| format!("error generating PDF: {e:?}"))?;

    info!("PDF generation completed");

    Ok(Output {
        pdf: Some(bytes),
        ..Default::default()
    })
}
