// SPDX-FileCopyrightText: 2025 Rafael Fernández López
// <rafael.fernandez@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Error};
use core::result::Result;
use sequent_core::services::pdf::PdfRenderer;
use std::env;
use tokio::runtime::Runtime;

#[tokio::main]
async fn main() -> Result<(), Error> {
    cfg_if::cfg_if! {
        if #[cfg(feature = "lambda_inplace")] {
            unsafe {
                env::set_var("DOC_RENDERER_BACKEND", "inplace");
            }
        } else if #[cfg(feature = "lambda_openwhisk")] {
            unsafe {
                env::set_var("DOC_RENDERER_BACKEND", "openwhisk");
            }
            match env::var("OPENWHISK_ENDPOINT") {
                Ok(endpoint) => println!("OpenWhisk endpoint '{endpoint}' will be used"),
                Err(_) => println!("Please, set envvar OPENWHISK_ENDPOINT and try again")
            }
        } else if #[cfg(feature = "lambda_aws_lambda")] {
            unsafe {
                env::set_var("DOC_RENDERER_BACKEND", "aws_lambda");
            }
            match env::var("AWS_LAMBDA_ENDPOINT") {
                Ok(endpoint) => println!("AWS Lambda endpoint '{endpoint}' will be used"),
                Err(_) => println!("Please, set envvar AWS_LAMBDA_ENDPOINT and try again")
            }
        } else {
            compile_error!("Either feature lambda_inplace, lambda_openwhisk or lambda_aws_lambda has to be provided");
        }
    }

    match PdfRenderer::render_pdf("Hello, world!".to_string(), None).await {
        Ok(_) => {
            println!("PDF correctly generated. Lambda is working as expected.");
            Ok(())
        }
        Err(err) => Err(anyhow!("Error generating PDF: {err:?}")),
    }
}
