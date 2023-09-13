// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use std::collections::HashMap;
use anyhow::Result;
use crate::hasura_types;
use crate::ballot;

pub fn create_ballot_style(
    election_event: hasura_types::ElectionEvent,
    elections: Vec<hasura_types::Election>,
    contests: Vec<hasura_types::Contest>,
    candidates: Vec<hasura_types::Candidate>,
) -> Result<Option<ballot::ElectionDTO>> {
    Ok(None)
}