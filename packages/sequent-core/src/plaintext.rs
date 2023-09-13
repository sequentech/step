// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::ballot::*;
use crate::ballot_codec::BallotCodec;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub enum InvalidPlaintextErrorType {
    Explicit,
    Implicit,
    EncodingError,
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct InvalidPlaintextError {
    pub error_type: InvalidPlaintextErrorType,
    pub answer_id: Option<i64>,
    pub message: Option<String>,
    pub message_map: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct DecodedVoteQuestion {
    pub is_explicit_invalid: bool,
    pub invalid_errors: Vec<InvalidPlaintextError>,
    pub choices: Vec<DecodedVoteChoice>,
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct DecodedVoteChoice {
    pub id: i64,
    pub selected: i64,
    pub write_in_text: Option<String>,
}

pub fn map_to_decoded_question(
    ballot: &AuditableBallot,
) -> Result<Vec<DecodedVoteQuestion>, String> {
    let mut decoded_questions = vec![];
    if ballot.config.configuration.questions.len() != ballot.choices.len() {
        return Err(format!(
            "Invalid number of choices {} != {}",
            ballot.config.configuration.questions.len(),
            ballot.choices.len()
        ));
    }
    for i in 0..ballot.choices.len() {
        let question = ballot.config.configuration.questions[i].clone();
        let replication_choice: &ReplicationChoice = &ballot.choices[i];

        let decoded_plaintext = question
            .decode_plaintext_question(replication_choice.plaintext.as_str())?;
        decoded_questions.push(decoded_plaintext);
    }
    Ok(decoded_questions)
}
