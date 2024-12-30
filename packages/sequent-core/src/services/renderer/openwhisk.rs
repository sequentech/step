// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use headless_chrome::types::PrintToPdfOptions;
use tracing::instrument;

#[instrument(skip_all, err)]
pub fn render_html(html: &str, options: Option<PrintToPdfOptions>) -> Result<Vec<u8>> {
    todo!()
}
