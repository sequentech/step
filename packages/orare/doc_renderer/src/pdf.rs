// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use sequent_core::services::pdf::PrintToPdfOptions;
use tracing::info;

pub fn render_pdf(html: String, pdf_options: Option<PrintToPdfOptions>) -> Result<Vec<u8>, String> {
    let bytes = sequent_core::services::pdf::html_to_pdf(html, pdf_options)
        .map_err(|e| format!("error generating PDF: {e:?}"))?;

    info!("PDF generation completed");

    Ok(bytes)
}
