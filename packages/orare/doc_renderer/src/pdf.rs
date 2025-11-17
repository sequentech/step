// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::io::{Input, Output};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use sequent_core::util::aws::get_region;
use sequent_core::{services::pdf::PrintToPdfOptions, util::convert_vec::IntoVec};
use tokio::runtime::Runtime;
use tracing::{info, instrument};

pub fn render_pdf(html: String, pdf_options: Option<PrintToPdfOptions>) -> Result<Vec<u8>, String> {
    let bytes = sequent_core::services::pdf::html_to_pdf(html, pdf_options)
        .map_err(|e| format!("error generating PDF: {e:?}"))?;

    info!("PDF generation completed");

    Ok(bytes)
}
