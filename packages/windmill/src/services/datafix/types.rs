// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//! JSON bodies, annotation payloads, and small enums used by the datafix HTTP API.
use super::utils::{DATAFIX_ID_KEY, DATAFIX_PSW_POLICY_KEY, DATAFIX_VOTERVIEW_REQ_KEY};
use anyhow::{anyhow, Result};
use rand::{distr, Rng};
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::ballot::Annotations;
use sequent_core::serialization::deserialize_with_path::{deserialize_str, deserialize_value};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use tracing::{instrument, warn};

use crate::postgres::election_event::ElectionEventDatafix;
use crate::services::consolidation::eml_generator::ValidateAnnotations;

/// Request body used to create or update a voter through the datafix API.
#[derive(Deserialize, Debug)]
pub struct VoterInformationBody {
    /// Keycloak username for the voter.
    pub voter_id: String,
    /// Ward segment used to build the canonical area name.
    pub ward: String,
    /// Optional school-board segment appended after the ward when present.
    pub schoolboard: Option<String>,
    /// Optional poll segment appended as the final suffix of the composed area name.
    pub poll: Option<String>,
    /// Optional `YYYY-MM-DD` birthdate.
    pub birthdate: Option<String>,
    /// Whether to enable or disable the account.
    pub enabled: Option<bool>,
}

/// Request body to mark a voter as voted via an external channel.
#[derive(Deserialize, Debug)]
pub struct MarkVotedBody {
    /// Keycloak username for the voter.
    pub voter_id: String,
    /// Channel string stored in the `VOTED_CHANNEL` user attribute.
    pub channel: String,
}

/// Standard JSON error response used by datafix endpoints.
#[derive(Serialize, Deserialize, Debug)]
pub struct DatafixResponse {
    /// HTTP status code echoed to API consumers.
    pub code: u16,
    /// Reason phrase for the status.
    pub message: String,
}

/// Convenience alias for JSON-encoded datafix errors.
pub type JsonErrorResponse = Json<DatafixResponse>;

impl DatafixResponse {
    /// Wraps a Rocket [`Status`] inside [`Json`] for uniform error responses.
    #[instrument]
    pub fn new(status: Status) -> JsonErrorResponse {
        Json(DatafixResponse {
            code: status.code,
            message: status.reason().unwrap_or_default().to_string(),
        })
    }
}

/// `VoterView` SOAP connection details embedded in election event annotations.
#[derive(Deserialize, Serialize, Debug)]
pub struct VoterviewRequest {
    /// SOAP endpoint base URL configured per election event.
    pub url: String,
    /// MVV web-service username.
    pub usr: String,
    /// MVV web-service password.
    pub psw: String,
    /// County/municipality code required by the `VoterView` SOAP actions.
    pub county_mun: String,
}

/// Structured datafix annotation payloads attached to an election event.
#[derive(Deserialize, Serialize, Debug)]
pub struct DatafixAnnotations {
    /// Opaque id advertised to datafix API clients.
    pub id: String,
    /// Rules for generating replacement passwords.
    pub password_policy: PasswordPolicy,
    /// Credentials and endpoint data for outbound `VoterView` synchronization.
    pub voterview_request: VoterviewRequest,
}

/// How generated passwords are combined with the voter id.
#[derive(Default, Display, Serialize, Deserialize, Debug, Clone, EnumString)]
pub enum BasePolicy {
    /// Concatenate the voter id with the generated secret (used for legacy PIN formats).
    #[strum(serialize = "id-password-concatenated")]
    #[serde(rename = "id-password-concatenated")]
    IdPswConcat,
    /// Use only the generated secret without prefixing the voter id.
    #[default]
    #[strum(serialize = "password-only")]
    #[serde(rename = "password-only")]
    PswOnly,
}

/// Character set used when generating random passwords.
#[derive(Default, Display, Serialize, Deserialize, Debug, Clone, EnumString)]
pub enum CharactersPolicy {
    /// Digits-only secret.
    #[strum(serialize = "numeric")]
    #[serde(rename = "numeric")]
    Numeric,
    /// Alphanumeric secret.
    #[default]
    #[strum(serialize = "alphanumeric")]
    #[serde(rename = "alphanumeric")]
    Alphanumeric,
}

/// Password generation policy stored in election event annotations.
#[derive(Deserialize, Serialize, Debug)]
pub struct PasswordPolicy {
    /// Whether the voter id is prefixed onto the generated token.
    base: BasePolicy,
    /// Length of the randomly generated portion (digits or alphanumeric run).
    size: usize,
    /// Character set policy for the generated portion.
    characters: CharactersPolicy,
}

impl PasswordPolicy {
    /// Builds a credential string for `voter_id` following `base` and `characters` rules.
    #[instrument]
    pub fn generate_password(self, voter_id: &str) -> String {
        let pin = match self.characters {
            CharactersPolicy::Numeric => {
                let mut pass = String::new();
                let mut rng = rand::thread_rng();
                for _ in 0..self.size {
                    let digit = rng.gen_range(0..10_u32);
                    pass.push(char::from_digit(digit, 10).unwrap_or('0'));
                }
                pass
            }
            CharactersPolicy::Alphanumeric => rand::thread_rng()
                .sample_iter(distr::Alphanumeric)
                .take(self.size)
                .map(char::from)
                .collect(),
        };
        match self.base {
            BasePolicy::IdPswConcat => format!("{voter_id}{pin}"),
            BasePolicy::PswOnly => pin,
        }
    }
}

impl ValidateAnnotations for ElectionEventDatafix {
    type Item = DatafixAnnotations;

    fn get_annotations(&self) -> Result<Self::Item> {
        let annotations_value = self
            .0
            .annotations
            .clone()
            .ok_or_else(|| anyhow!("Missing election event annotations"))?;

        let annotations: Annotations = deserialize_value(annotations_value)?;
        let id = match annotations.get(DATAFIX_ID_KEY) {
            Some(id) => id.clone(),
            None => return Err(anyhow!("{DATAFIX_ID_KEY} not found")),
        };

        let password_policy: PasswordPolicy = match annotations.get(DATAFIX_PSW_POLICY_KEY) {
            Some(value_as_str) => deserialize_str(value_as_str)?,
            None => return Err(anyhow!("{DATAFIX_PSW_POLICY_KEY} not found")),
        };

        let voterview_request: VoterviewRequest = match annotations.get(DATAFIX_VOTERVIEW_REQ_KEY) {
            Some(value_as_str) => deserialize_str(value_as_str)?,
            None => return Err(anyhow!("{DATAFIX_VOTERVIEW_REQ_KEY} not found")),
        };

        Ok(DatafixAnnotations {
            id,
            password_policy,
            voterview_request,
        })
    }
}

/// Supported SOAP request types for the `VoterView` integration.
#[derive(Display, Debug, Clone)]
pub enum SoapRequest {
    /// `SetVoted` SOAP action after an internet ballot is accepted.
    SetVoted,
    /// `SetNotVoted` SOAP action when a vote must be rolled back in `VoterView`.
    SetNotVoted,
}
