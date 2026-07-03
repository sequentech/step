// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::utils::{DATAFIX_ID_KEY, DATAFIX_PSW_POLICY_KEY, DATAFIX_VOTERVIEW_REQ_KEY};
use crate::postgres::election_event::ElectionEventDatafix;
use crate::services::consolidation::eml_generator::ValidateAnnotations;
use anyhow::{anyhow, Result};
use rand::{distr, Rng};
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::ballot::Annotations;
use sequent_core::serialization::deserialize_with_path::{deserialize_str, deserialize_value};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use tracing::{instrument, warn};

#[derive(Deserialize, Debug)]
pub struct VoterIdBody {
    pub voter_id: String,
}

#[derive(Deserialize, Debug)]
pub struct VoterInformationBody {
    pub voter_id: String,
    pub ward: String,
    pub schoolboard: Option<String>,
    pub poll: Option<String>,
    pub birthdate: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Deserialize, Debug)]
pub struct MarkVotedBody {
    pub voter_id: String,
    pub channel: String,
}

#[derive(Deserialize, Debug)]
pub enum VoterOperationInput {
    VoterInfo(VoterInformationBody),
    VoterId(VoterIdBody),
    MarkVoted(MarkVotedBody),
}

/// Machine-readable error codes returned by the Datafix API. These are a
/// public contract with the Datafix integration: changing an existing value
/// is a breaking change.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Display, EnumString)]
pub enum DatafixErrorCode {
    /// add-voter for a username that already exists in the realm.
    #[serde(rename = "voter-already-exists")]
    #[strum(serialize = "voter-already-exists")]
    VoterAlreadyExists,
    /// No voter matches the given voter_id (username).
    #[serde(rename = "voter-not-found")]
    #[strum(serialize = "voter-not-found")]
    VoterNotFound,
    /// More than one voter matches the given voter_id: data-integrity issue.
    #[serde(rename = "voter-not-unique")]
    #[strum(serialize = "voter-not-unique")]
    VoterNotUnique,
    /// No area matches the ward/schoolboard/poll combination in the request.
    #[serde(rename = "area-not-found")]
    #[strum(serialize = "area-not-found")]
    AreaNotFound,
    /// No election event is annotated with the requester's datafix event id.
    #[serde(rename = "event-not-found")]
    #[strum(serialize = "event-not-found")]
    EventNotFound,
    /// The request body is malformed or contains invalid values.
    #[serde(rename = "invalid-request")]
    #[strum(serialize = "invalid-request")]
    InvalidRequest,
    /// The credentials are missing or invalid.
    #[serde(rename = "unauthorized")]
    #[strum(serialize = "unauthorized")]
    Unauthorized,
    /// The credentials are valid but lack the Datafix permission.
    #[serde(rename = "forbidden")]
    #[strum(serialize = "forbidden")]
    Forbidden,
    /// Unexpected server-side failure; safe to retry later.
    #[serde(rename = "internal-error")]
    #[strum(serialize = "internal-error")]
    InternalError,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DatafixResponse {
    pub code: u16,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<DatafixErrorCode>,
}

/// Error responder for the Datafix API routes: sets the real HTTP status and
/// serializes a `DatafixResponse` body, so clients that only check the HTTP
/// status and clients that parse the body agree on the outcome.
#[derive(Debug)]
pub struct JsonErrorResponse {
    pub status: Status,
    pub body: DatafixResponse,
}

impl JsonErrorResponse {
    pub fn error_code(&self) -> Option<DatafixErrorCode> {
        self.body.error_code
    }
}

impl<'r> rocket::response::Responder<'r, 'static> for JsonErrorResponse {
    fn respond_to(self, request: &'r rocket::Request<'_>) -> rocket::response::Result<'static> {
        let json = Json(self.body).respond_to(request)?;
        Ok(rocket::Response::build_from(json)
            .status(self.status)
            .finalize())
    }
}

impl DatafixResponse {
    /// Success body for the `Ok` arm of the Datafix routes.
    #[instrument]
    pub fn ok() -> Json<DatafixResponse> {
        Json(DatafixResponse {
            code: Status::Ok.code,
            message: Status::Ok.reason().unwrap_or_default().to_string(),
            error_code: None,
        })
    }

    /// Error response carrying both the HTTP status and the stable
    /// machine-readable error code.
    #[instrument]
    pub fn error(status: Status, error_code: DatafixErrorCode) -> JsonErrorResponse {
        JsonErrorResponse {
            status,
            body: DatafixResponse {
                code: status.code,
                message: status.reason().unwrap_or_default().to_string(),
                error_code: Some(error_code),
            },
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct VoterviewRequest {
    pub url: String,
    pub usr: String,
    pub psw: String,
    pub county_mun: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct DatafixAnnotations {
    pub id: String,
    pub password_policy: PasswordPolicy,
    pub voterview_request: VoterviewRequest,
}

#[derive(Default, Display, Serialize, Deserialize, Debug, Clone, Copy, EnumString)]
pub enum BasePolicy {
    #[strum(serialize = "id-password-concatenated")]
    #[serde(rename = "id-password-concatenated")]
    IdPswConcat,
    #[default]
    #[strum(serialize = "password-only")]
    #[serde(rename = "password-only")]
    PswOnly,
}

#[derive(Default, Display, Serialize, Deserialize, Debug, Clone, Copy, EnumString)]
pub enum CharactersPolicy {
    #[strum(serialize = "numeric")]
    #[serde(rename = "numeric")]
    Numeric,
    #[default]
    #[strum(serialize = "alphanumeric")]
    #[serde(rename = "alphanumeric")]
    Alphanumeric,
}

#[derive(Deserialize, Serialize, Debug, Copy, Clone)]
pub struct PasswordPolicy {
    base: BasePolicy,
    size: usize,
    characters: CharactersPolicy,
}

impl PasswordPolicy {
    #[instrument]
    pub fn generate_password(self, voter_id: &str) -> String {
        let pin = match self.characters {
            CharactersPolicy::Numeric => {
                let mut pass = String::new();
                let mut rng = rand::thread_rng();
                for _ in 0..self.size {
                    pass.push_str(rng.gen_range(0..10).to_string().as_str());
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
            BasePolicy::IdPswConcat => format!("{}{}", voter_id, pin),
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

#[derive(Display, Debug, Clone)]
pub enum SoapRequest {
    SetVoted,
    SetNotVoted,
}

#[derive(Display, Debug, Clone)]
pub enum SoapRequestResponse {
    Ok,
    /// Some fields are missing or wrong in the request
    Faultstring(String),
    /// The requested voter has voted already
    #[strum(to_string = "The voter has already voted.")]
    HasVotedErrorMsg,
    /// Other kind of errors like voter not found, CountyMun is required, etc.
    OtherErrorMsg(String),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct SoapRequestData {
    pub county_mun: String,
    pub usr: String,
    pub psw: String,
    pub voter_id: String,
    pub timestamp: String,
}

#[derive(Debug, Display, Clone, Copy)]
pub enum EndpointNames {
    AddVoter,
    UpdateVoter,
    DeleteVoter,
    UnmarkVoted,
    MarkVoted,
    ReplacePin,
}
