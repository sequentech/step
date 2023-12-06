// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};

pub async fn create_tally_ceremony(
    tenant_id: String,
    election_event_id: String,
    election_ids: Vec<String>,
) -> Result<String> {
    Ok("".to_string())
}
