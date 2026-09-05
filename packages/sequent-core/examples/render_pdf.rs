// SPDX-FileCopyrightText: 2025 Rafael Fernández López
// <rafael.fernandez@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Error};
use core::result::Result;
use sequent_core::services::pdf::PdfRenderer;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Error> {
    cfg_if::cfg_if! {
        if #[cfg(feature = "lambda_inplace")] {
            env::set_var("DOC_RENDERER_BACKEND", "inplace");
        } else if #[cfg(feature = "lambda_openwhisk")] {
            env::set_var("DOC_RENDERER_BACKEND", "openwhisk");
            match env::var("OPENWHISK_DOC_RENDERER_ENDPOINT") {
                Ok(endpoint) => println!("OpenWhisk doc renderer endpoint '{endpoint}' will be used"),
                Err(_) => {
                    match env::var("OPENWHISK_API_HOST") {
                        Ok(api_host) => println!("OpenWhisk doc renderer endpoint will be defaulted from '{api_host}'"),
                        Err(_) => println!("Please, set envvar OPENWHISK_DOC_RENDERER_ENDPOINT or OPENWHISK_API_HOST and try again")
                    }
                },
            }
        } else if #[cfg(feature = "lambda_aws_lambda")] {
            env::set_var("DOC_RENDERER_BACKEND", "aws_lambda");
            match env::var("AWS_LAMBDA_DOC_RENDERER_ENDPOINT") {
                Ok(endpoint) => println!("AWS Lambda doc renderer endpoint '{endpoint}' will be used"),
                Err(_) => println!("Please, set envvar AWS_LAMBDA_DOC_RENDERER_ENDPOINT and try again")
            }
        }
    }

    // With no example-specific backend feature, use DOC_RENDERER_BACKEND and the
    // renderer's normal local Chromium default.

    match PdfRenderer::render_pdf("Hello, world!".to_string(), None).await {
        Ok(data) => {
            println!("PDF correctly generated. Lambda is working as expected. Writing PDF to out.pdf");
            std::fs::write("out.pdf", data).expect("failed to write PDF file");
            Ok(())
        }
        Err(err) => Err(anyhow!("Error generating PDF: {err:?}")),
    }
}
