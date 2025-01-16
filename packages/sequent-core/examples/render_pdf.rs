use core::result::Result;
use sequent_core::services::pdf::PdfRenderer;
use std::env;
use tokio::runtime::Runtime;

#[tokio::main]
async fn main() -> Result<(), ()> {
    cfg_if::cfg_if! {
        if #[cfg(feature = "lambda_inplace")] {
            unsafe {
                env::set_var("DOC_RENDERER_BACKEND", "inplace");
            }
        } else if #[cfg(feature = "lambda_openwhisk")] {
            unsafe {
                env::set_var("DOC_RENDERER_BACKEND", "openwhisk");
                env::set_var("OPENWHISK_ENDPOINT", "http://127.0.0.2:3233/api/v1/namespaces/_/actions/pdf-tools/doc_renderer?blocking=true&result=true");
            }
        } else if #[cfg(feature = "lambda_aws_lambda")] {
            unsafe {
                env::set_var("DOC_RENDERER_BACKEND", "aws_lambda");
                env::set_var("AWS_LAMBDA_ENDPOINT", "https://rq5jtxuv4rxo5viu5jmxmpxuqq0oisgh.lambda-url.us-east-1.on.aws/");
            }
        } else {
            compile_error!("Either feature inplace, openwhisk or aws_lambda has to be provided");
        }
    }

    println!(
        "{:?}",
        PdfRenderer::render_pdf("Hello, world!".to_string(), None).await
    );

    Ok(())
}
