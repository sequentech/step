use crate::io::{Input, Output};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use sequent_core::services::pdf::PrintToPdfOptions;
use tracing::{info, instrument};

pub fn render_pdf(input: Input) -> Result<Output, String> {
    let bytes = sequent_core::services::pdf::html_to_pdf(input.html, input.pdf_options)
        .map_err(|e| "error generating PDF: {e:?}")?;

    let pdf_base64 = BASE64.encode(bytes);
    info!("PDF generation completed");

    Ok(Output { pdf_base64 })
}
