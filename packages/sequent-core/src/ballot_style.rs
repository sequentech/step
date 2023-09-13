// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use std::collections::HashMap;
use anyhow::Result;
use crate::hasura_types::*;

pub fn create_ballot_style(
    election_event: ElectionEvent,
    elections: Vec<Election>,
    contests: Vec<Contest>,
    candidates: Vec<Candidate>,
) -> Result<()> {
    Ok(())
}