// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//! Updates ballot style JSON attached to an election event
#![allow(missing_docs)]
#![allow(clippy::missing_docs_in_private_items)]
use celery::error::TaskError;
use tracing::instrument;

use crate::services::ballot_styles::ballot_style;
use crate::types::error::Result;

/// Update Election Event Ballot Styles.
#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn update_election_event_ballot_styles(
    tenant_id: String,
    election_event_id: String,
    ballot_publication_id: String,
) -> Result<()> {
    ballot_style::update_election_event_ballot_styles(
        &tenant_id,
        &election_event_id,
        &ballot_publication_id,
    )
    .await?;

    Ok(())
}
