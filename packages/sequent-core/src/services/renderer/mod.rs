// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Context, Result};
use headless_chrome::types::PrintToPdfOptions;
use std::env;
use tracing::instrument;

mod aws_lambda;
mod forking;
mod openwhisk;

#[instrument(skip_all, err)]
pub fn render_html(html: &str, options: Option<PrintToPdfOptions>) -> Result<Vec<u8>> {
    match env::var("DOC_RENDERER_BACKEND").as_deref() {
        Ok("aws_lambda") => aws_lambda::render_html(html, options),
        Ok("openwhisk") => openwhisk::render_html(html, options),
        Ok("windmill") => forking::render_html(html, options),
        backend => {
            Err(anyhow!("Unknown renderer backend: {:?}", backend))
        }
    }
}
