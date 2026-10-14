// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::pipes::error::{Error, Result};
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use serde::Deserialize;
use std::fs::File;
use std::io::Read;

#[cfg(feature = "miru")]
use crate::pipes::pipe_inputs::InputElectionConfig;
#[cfg(feature = "miru")]
use sequent_core::plaintext::DecodedVoteChoice;
#[cfg(feature = "miru")]
use std::collections::HashMap;

pub trait HasId {
    fn id(&self) -> &str;
}

pub fn parse_file<T: for<'a> Deserialize<'a>>(mut file: File) -> Result<T> {
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    deserialize_str(&contents).map_err(|err| {
        Error::UnexpectedError(format!("Parse error: {:?} . Contents {contents}", err))
    })
}

// Unmarked choices for the Miru ballot-image conversion.
// contest_id -> (candidate_id -> dcv)
#[cfg(feature = "miru")]
pub(crate) fn get_contest_dvc_map(
    election_input: &InputElectionConfig,
) -> HashMap<String, HashMap<String, DecodedVoteChoice>> {
    let mut ret = HashMap::new();

    for contest in &election_input.contest_list {
        let mut map = HashMap::new();
        for candidate in &contest.contest.candidates {
            let choice = DecodedVoteChoice {
                id: candidate.id.clone(),
                selected: -1,
                write_in_text: None,
            };
            map.insert(candidate.id.clone(), choice);
        }

        ret.insert(contest.id.to_string(), map);
    }

    ret
}
