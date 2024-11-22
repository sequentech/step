// SPDX-FileCopyrightText: 2024 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use orare::lambda_runtime;
use serde::{Deserialize, Serialize};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use headless_chrome::types::PrintToPdfOptions;

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
    let bytes = sequent_core::services::pdf::html_to_text(input.html)
        .map_err(|e| e.to_string())?;

    #[cfg(feature = "inplace")]
    let bytes = sequent_core::services::pdf::html_to_pdf(input.html, input.pdf_options)
        .map_err(|e| e.to_string())?;

    #[cfg(not(any(feature = "openwhisk-dev", feature = "inplace")))]
    let bytes = sequent_core::services::pdf::html_to_pdf(input.html, input.pdf_options)
        .map_err(|e| e.to_string())?;

    let pdf_base64 = BASE64.encode(bytes);
    Ok(Output { pdf_base64 })
}
