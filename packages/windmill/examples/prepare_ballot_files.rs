// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{bail, Context, Result};
use windmill::{
    postgres::ballot_publication::{get_ballot_publication_by_id, lock_publication_event},
    services::{
        ballot_styles::publication_files::prepare_publication_files, database::get_hasura_pool,
    },
};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 3 {
        bail!("Usage: prepare_ballot_files TENANT_ID EVENT_ID PUBLICATION_ID");
    }
    let mut client = get_hasura_pool().await.get().await?;
    let tx = client.transaction().await?;
    lock_publication_event(&tx, &args[0], &args[1]).await?;
    let publication = get_ballot_publication_by_id(&tx, &args[0], &args[1], &args[2])
        .await?
        .context("Publication not found")?;
    if publication.deleted_at.is_some() || !publication.is_generated.unwrap_or(false) {
        bail!("Publication must be generated and not deleted");
    }
    prepare_publication_files(&tx, &args[0], &args[1], &args[2]).await?;
    tx.commit().await?;
    println!("Publication files uploaded, verified and committed");
    Ok(())
}
