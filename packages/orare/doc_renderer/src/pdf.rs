use crate::io::{Input, Output};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use headless_chrome::types::PrintToPdfOptions;
use tracing::{info, instrument};

pub async fn render_pdf(input: Input) -> Result<Output, String> {
    let bytes = sequent_core::services::pdf_renderer::PdfRenderer::render_pdf(
        input.html,
        input.pdf_options,
    )
    .await
    .map_err(|e| "error generating PDF: {e:?}")?;

    let pdf_base64 = BASE64.encode(bytes);
    info!("PDF generation completed");

    Ok(Output { pdf_base64 })
}
