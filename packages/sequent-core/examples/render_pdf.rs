use core::result::Result;
use sequent_core::services::pdf::PdfRenderer;
use std::env;
use tokio::runtime::Runtime;

#[tokio::main]
async fn main() -> Result<(), ()> {
    unsafe {
        env::set_var("DOC_RENDERER_BACKEND", "inplace");
    }
    println!(
        "{:?}",
        PdfRenderer::render_pdf("Hello, world!".to_string(), None).await
    );

    Ok(())
}
