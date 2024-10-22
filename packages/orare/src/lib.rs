extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

#[proc_macro_attribute]
pub fn lambda_runtime(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let name = &input.sig.ident;

    let expanded = quote! {
        cfg_if::cfg_if! {
            if #[cfg(feature = "aws_lambda")] {
                use lambda_runtime::{handler_fn, Context, Error};
                use tokio;

                #[tokio::main]
                async fn main() -> Result<(), Error> {
                    let func = handler_fn(#name);
                    lambda_runtime::run(func).await?;
                    Ok(())
                }

            } else if #[cfg(feature = "openwhisk")] {
                use std::env;
                use serde_json;

                fn main() {
                    let args: Vec<String> = env::args().collect();
                    let input: Input = if args.len() > 1 {
                        serde_json::from_str(&args[1]).unwrap()
                    } else {
                        serde_json::from_str("{\"name\":\"world\"}").unwrap()
                    };

                    let output = #name(input);
                    println!("{}", serde_json::to_string(&output).unwrap());
                }
            }
        }
    };

    TokenStream::from(quote! {
        #input
        #expanded
    })
}
