use crate::io::{Input, Output};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use tracing::{info, instrument};

pub async fn render_pdf(input: Input) -> Result<Output, String> {
    let pdf_renderer = sequent_core::services::pdf_renderer::PdfRenderer {
        transport: sequent_core::services::pdf_renderer::PdfTransport::InPlace,
    };

    let bytes = sequent_core::services::pdf_renderer::PdfRenderer::render_pdf(
        input.html,
        Some(sequent_core::services::pdf::PrintToPdfOptions::default()),
    )
    .await
    .map_err(|e| "error generating PDF: {e:?}")?;

    let pdf_base64 = BASE64.encode(bytes);
    info!("PDF generation completed");

    Ok(Output { pdf_base64 })
}
