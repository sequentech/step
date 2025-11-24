// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

#[proc_macro_attribute]
pub fn lambda_runtime(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let name = &input.sig.ident;

    let expanded = quote! {
        use sequent_core::util::init_log::init_log;
        use sequent_core::serialization::deserialize_with_path::deserialize_str;
        use anyhow::{Context, Result};
        use clap::Parser;

        #[derive(Parser)]
        struct CliArgs {
            /// The input to the lambda function in JSON format
            input: String,
        }

        cfg_if::cfg_if! {
            if #[cfg(feature = "aws_lambda")] {
                use aws_lambda_events::{
                    lambda_function_urls::{LambdaFunctionUrlRequest, LambdaFunctionUrlResponse},
                    http::HeaderMap,
                };
                use lambda_runtime::{run, service_fn, tracing, LambdaEvent, Diagnostic, Error};
                use serde_json::{json, Value};

                #[tokio::main]
                async fn main() -> Result<(), Error> {
                    init_log(true);

                    run(service_fn(func)).await?;
                    Ok(())
                }

                async fn func(lambda_event: LambdaEvent<LambdaFunctionUrlRequest>) -> Result<LambdaFunctionUrlResponse, Error> {
                    let input = serde_json::from_str(
                        &lambda_event.payload.body.unwrap(),
                    );
                    let input = input
                        .expect("error reading lambda function arguments");

                    let result = #name(input)
                      .await
                      .map(|result| serde_json::to_string(&result).unwrap())
                        .expect("error calling lambda function");

                    let mut headers = HeaderMap::new();
                    headers.insert("content-type", "text/plain".parse().unwrap());

                    Ok(LambdaFunctionUrlResponse {
                        status_code: 200,
                        headers,
                        body: Some(result.into()),
                        is_base64_encoded: false,
                        cookies: Vec::new(),
                    })
                }
            } else if #[cfg(any(feature = "openwhisk"))] {
                use serde_json;

                fn main() -> Result<()> {
                    init_log(true);

                    // Parse the command-line arguments using Clap
                    let args = CliArgs::parse();
                    let input: Input = deserialize_str(&args.input)
                        .map_err(|e| anyhow::anyhow!("Failed to deserialize input: {e:?}"))?;

                    let output = #name(input);
                    let output_str = serde_json::to_string(&output)
                        .map_err(|e| anyhow::anyhow!("Failed to serialize output: {e:?}"))?;
                    println!("{output_str}");

                    Ok(())
                }
            }
        }
    };

    TokenStream::from(quote! {
        #input
        #expanded
    })
}
