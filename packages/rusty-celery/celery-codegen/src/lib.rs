#![recursion_limit = "256"]

// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use proc_macro::TokenStream;

mod error;
mod task;

#[proc_macro_attribute]
pub fn task(args: TokenStream, input: TokenStream) -> TokenStream {
    task::impl_macro(args, input)
}
