// SPDX-FileCopyrightText: 2025 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use once_cell::sync::OnceCell;
use std::sync::Arc;
use tokio::sync::{Semaphore, SemaphorePermit};
use tracing::instrument;

// Static OnceCell to hold the semaphore
pub static SEMAPHORE: OnceCell<Arc<Semaphore>> = OnceCell::new();

#[instrument(err)]
pub fn init_semaphore(count: usize) -> Result<()> {
    // Create the semaphore and wrap it in Arc
    let semaphore = Arc::new(Semaphore::new(count));

    // Set the semaphore in the OnceCell
    SEMAPHORE
        .set(semaphore)
        .map_err(|e| anyhow!("Error setting semaphore: {:?}", e))?;

    Ok(())
}

#[instrument(err)]
pub async fn acquire_semaphore() -> Result<SemaphorePermit<'static>> {
    // Get the semaphore and acquire a permit
    SEMAPHORE
        .get()
        .with_context(|| "Error fetching semaphore")?
        .acquire()
        .await
        .with_context(|| "Error acquiring semaphore")
}
