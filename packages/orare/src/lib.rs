// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
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
                use lambda_runtime::{handler_fn, Context, Error};
                use tokio;

                #[tokio::main]
                async fn main() -> Result<(), Error> {
                    init_log(true);
                    let func = handler_fn(#name);
                    lambda_runtime::run(func).await.map_err(|e| anyhow::anyhow!("Failed to run the lambda function #name: {e:?}"))?;
                    Ok(())
                }

            } else if #[cfg(any(feature = "openwhisk", feature = "openwhisk-dev"))] {
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