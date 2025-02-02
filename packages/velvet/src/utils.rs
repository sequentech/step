// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::pipes::error::{Error, Result};
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;

use crate::pipes::pipe_inputs::InputElectionConfig;
use sequent_core::plaintext::DecodedVoteChoice;

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

// unmarked choices
// contest_id -> (candidate_id -> dcv)
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

// Generic function to fetch configuration values with fallbacks from environment
pub fn get_config_value<T>(config_value: Option<T>, env_var: &str, fallback: T) -> T
where
    T: std::str::FromStr + std::fmt::Debug,
{
    config_value.unwrap_or_else(|| {
        std::env::var(env_var)
            .ok()
            .and_then(|val| val.parse::<T>().ok())
            .unwrap_or_else(|| {
                eprintln!(
                    "Invalid or missing {} value. Falling back to default: {:?}",
                    env_var, fallback
                );
                fallback
            })
    })
}
