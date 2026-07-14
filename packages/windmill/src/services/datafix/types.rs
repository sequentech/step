// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::utils::{DATAFIX_ID_KEY, DATAFIX_PSW_POLICY_KEY, DATAFIX_VOTERVIEW_REQ_KEY};
use anyhow::{anyhow, Result};
use rand::{distr, Rng};
use rocket::http::Status;
use rocket::response::status::Custom;
use rocket::serde::json::Json;
use sequent_core::ballot::Annotations;
use sequent_core::serialization::deserialize_with_path::{deserialize_str, deserialize_value};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use tracing::{instrument, warn};

use crate::postgres::election_event::ElectionEventDatafix;
use crate::services::consolidation::eml_generator::ValidateAnnotations;
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

/// Stable, machine-readable `error_code` values of the Datafix API error
/// contract: new codes may be added, but existing ones never change meaning.
#[derive(Display, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatafixErrorCode {
    #[strum(serialize = "voter-already-exists")]
    #[serde(rename = "voter-already-exists")]
    VoterAlreadyExists,
    #[strum(serialize = "voter-operation-in-progress")]
    #[serde(rename = "voter-operation-in-progress")]
    VoterOperationInProgress,
    #[strum(serialize = "voter-state-unresolved")]
    #[serde(rename = "voter-state-unresolved")]
    VoterStateUnresolved,
    #[strum(serialize = "voter-not-found")]
    #[serde(rename = "voter-not-found")]
    VoterNotFound,
    #[strum(serialize = "area-not-found")]
    #[serde(rename = "area-not-found")]
    AreaNotFound,
    #[strum(serialize = "event-not-found")]
    #[serde(rename = "event-not-found")]
    EventNotFound,
    #[strum(serialize = "invalid-request")]
    #[serde(rename = "invalid-request")]
    InvalidRequest,
    #[strum(serialize = "forbidden")]
    #[serde(rename = "forbidden")]
    Forbidden,
    #[strum(serialize = "internal-error")]
    #[serde(rename = "internal-error")]
    InternalError,
}

impl DatafixErrorCode {
    /// The HTTP status the Datafix API contract pairs with this error code.
    pub fn status(self) -> Status {
        match self {
            Self::VoterAlreadyExists
            | Self::VoterOperationInProgress
            | Self::VoterStateUnresolved => Status::Conflict,
            Self::VoterNotFound | Self::EventNotFound => Status::NotFound,
            Self::AreaNotFound => Status::UnprocessableEntity,
            Self::InvalidRequest => Status::BadRequest,
            Self::Forbidden => Status::Forbidden,
            Self::InternalError => Status::InternalServerError,
        }
    }
}

/// JSON body of every Datafix API reply, carrying the HTTP status code and its
/// reason phrase so a client that only reads the body still sees the outcome.
/// Errors may additionally carry a [`DatafixErrorCode`].
#[derive(Serialize, Deserialize, Debug)]
pub struct DatafixResponse {
    pub code: u16,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<DatafixErrorCode>,
}

/// Error half of the Datafix route `Result`: the same JSON body as the
/// success half, wrapped in [`Custom`] so the reply's HTTP status matches the
/// `code` carried in the body.
pub type JsonErrorResponse = Custom<Json<DatafixResponse>>;

impl DatafixResponse {
    /// Success body of a Datafix API reply.
    #[instrument]
    pub fn ok() -> Json<DatafixResponse> {
        Json(DatafixResponse {
            code: Status::Ok.code,
            message: Status::Ok.reason().unwrap_or_default().to_string(),
            error_code: None,
        })
    }

    /// Error reply carrying one of the stable machine-readable
    /// [`DatafixErrorCode`] values, answered with the HTTP status the
    /// contract pairs with it.
    #[instrument]
    pub fn error(error_code: DatafixErrorCode) -> JsonErrorResponse {
        let status = error_code.status();
        Custom(
            status,
            Json(DatafixResponse {
                code: status.code,
                message: status.reason().unwrap_or_default().to_string(),
                error_code: Some(error_code),
            }),
        )
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

#[derive(Default, Display, Serialize, Deserialize, Debug, Clone, EnumString)]
pub enum BasePolicy {
    #[strum(serialize = "id-password-concatenated")]
    #[serde(rename = "id-password-concatenated")]
    IdPswConcat,
    #[default]
    #[strum(serialize = "password-only")]
    #[serde(rename = "password-only")]
    PswOnly,
}

#[derive(Default, Display, Serialize, Deserialize, Debug, Clone, EnumString)]
pub enum CharactersPolicy {
    #[strum(serialize = "numeric")]
    #[serde(rename = "numeric")]
    Numeric,
    #[default]
    #[strum(serialize = "alphanumeric")]
    #[serde(rename = "alphanumeric")]
    Alphanumeric,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PasswordPolicy {
    base: BasePolicy,
    size: usize,
    characters: CharactersPolicy,
}

impl PasswordPolicy {
    #[instrument]
    pub fn generate_password(&self, voter_id: &str) -> String {
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

#[derive(Display, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoapRequest {
    SetVoted,
    SetNotVoted,
}

/// Classified outcome of a VoterView SOAP call. `AlreadyVoted`/`AlreadyNotVoted`
/// are the idempotent "already in that state" replies the caller treats as
/// success; `Fault` carries a transport/SOAP-fault detail and `Rejected` an
/// application `Success=false` message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SoapRequestResponse {
    Ok,
    AlreadyVoted,
    AlreadyNotVoted,
    Fault(String),
    Rejected(String),
}

impl SoapRequestResponse {
    /// Stable, low-cardinality tag for the electoral log—never the raw VoterView
    /// message, which may contain sensitive data.
    pub fn classification(&self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::AlreadyVoted => "already-voted",
            Self::AlreadyNotVoted => "already-not-voted",
            Self::Fault(_) => "soap-fault",
            Self::Rejected(_) => "rejected",
        }
    }
}

/// A classified [`SoapRequestResponse`] paired with the SHA-256 of the template
/// that produced the request, so the audit trail records which template was sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoapRequestResult {
    pub response: SoapRequestResponse,
    pub template_sha256: String,
}

/// Borrowed view of the values interpolated into a VoterView SOAP template,
/// serialized to the Handlebars variables and re-checked against the rendered
/// XML so a template cannot silently alter them.
#[derive(Serialize)]
pub struct SoapRequestData<'a> {
    pub county_mun: &'a str,
    pub usr: &'a str,
    pub psw: &'a str,
    pub voter_id: &'a str,
    pub timestamp: &'a str,
}

#[cfg(test)]
mod tests {
    use super::{DatafixErrorCode, DatafixResponse};
    use rocket::http::Status;

    #[test]
    fn error_reply_carries_the_documented_status_and_error_code() {
        let response =
            DatafixResponse::error(DatafixErrorCode::VoterAlreadyExists);
        assert_eq!(response.0, Status::Conflict);
        assert_eq!(
            serde_json::to_value(&*response.1).unwrap(),
            serde_json::json!({
                "code": 409,
                "message": "Conflict",
                "error_code": "voter-already-exists"
            })
        );
    }

    #[test]
    fn error_codes_keep_their_documented_wire_names_and_statuses() {
        let contract = [
            (DatafixErrorCode::VoterAlreadyExists, "voter-already-exists", Status::Conflict),
            (DatafixErrorCode::VoterOperationInProgress, "voter-operation-in-progress", Status::Conflict),
            (DatafixErrorCode::VoterStateUnresolved, "voter-state-unresolved", Status::Conflict),
            (DatafixErrorCode::VoterNotFound, "voter-not-found", Status::NotFound),
            (DatafixErrorCode::AreaNotFound, "area-not-found", Status::UnprocessableEntity),
            (DatafixErrorCode::EventNotFound, "event-not-found", Status::NotFound),
            (DatafixErrorCode::InvalidRequest, "invalid-request", Status::BadRequest),
            (DatafixErrorCode::Forbidden, "forbidden", Status::Forbidden),
            (DatafixErrorCode::InternalError, "internal-error", Status::InternalServerError),
        ];
        for (code, wire_name, status) in contract {
            assert_eq!(
                serde_json::to_value(code).unwrap(),
                serde_json::json!(wire_name)
            );
            assert_eq!(code.status(), status);
        }
    }

    #[test]
    fn success_body_omits_the_error_code() {
        let body = DatafixResponse::ok();
        assert_eq!(
            serde_json::to_value(&*body).unwrap(),
            serde_json::json!({"code": 200, "message": "OK"})
        );
    }
}
