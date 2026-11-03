// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{bail, Result};
use windmill::services::ballot_styles::ballot_style::update_election_event_ballot_styles;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 3 {
        bail!("Usage: prepare_ballot_files TENANT_ID EVENT_ID PUBLICATION_ID");
    }
    update_election_event_ballot_styles(&args[0], &args[1], &args[2]).await?;
    println!("Publication files are ready");
    Ok(())
}
