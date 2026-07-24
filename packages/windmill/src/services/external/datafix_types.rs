// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::utils::{
    DATAFIX_ID_KEY, DATAFIX_LAST_APPLIED_SEQUENCE_KEY, DATAFIX_PSW_POLICY_KEY,
    DATAFIX_VOTERVIEW_REQ_KEY,
};
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
    #[strum(serialize = "voter-voted-online")]
    #[serde(rename = "voter-voted-online")]
    VoterVotedOnline,
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
    #[instrument]
    pub fn status(self) -> Status {
        match self {
            Self::VoterAlreadyExists
            | Self::VoterOperationInProgress
            | Self::VoterStateUnresolved
            | Self::VoterVotedOnline => Status::Conflict,
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
    /// See `DATAFIX_LAST_APPLIED_SEQUENCE_KEY` — `0` if this event has never
    /// had a reconciliation file applied, not an error like the three fields
    /// above (which must already be configured for Datafix to work at all).
    pub last_applied_sequence: i64,
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

        let last_applied_sequence: i64 = annotations
            .get(DATAFIX_LAST_APPLIED_SEQUENCE_KEY)
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);

        Ok(DatafixAnnotations {
            id,
            password_policy,
            voterview_request,
            last_applied_sequence,
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
    #[instrument(skip_all)]
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

// =======================================================================
// Datafix reconciliation types. These live here (not in sequent_core::types)
// since they don't need WASM exposure, and the direct precedent for this
// feature (everything else in this file) already lives here and is reused by
// harvest via its dependency on this crate. `ReconciliationFileMeta`,
// `ReconciliationChangeCategory` and `ReconciliationPatchTarget` live in
// `super::types` instead — they're not Datafix-specific wire shapes.
// =======================================================================

/// The reconciliation file format's own value for an Internet vote in the
/// `Channel` column — always uppercase per the "Accepted Values" spec, and
/// distinct from Keycloak's stored `VOTED_CHANNEL_INTERNET_VALUE` ("Internet"):
/// `VoterSnapshot::voted_channel` is itself uppercased specifically so it can
/// be compared directly against this and against a file row's own `channel`.
pub const FILE_CHANNEL_INTERNET: &str = "INTERNET";

/// One column of the "Patch Files Format" `_old`/`_new` pair contract,
/// carrying that pair directly (`old`, `new`) instead of leaving it to a
/// separate `old_value`/`new_value` on `DiffItem` — a field and its own
/// old/new values are never meaningful apart from each other, so keeping
/// them on `DiffItem` alongside the field was pure duplication. Wire column
/// names match `PATCH_FIELDS` in the admin portal's `types.ts` exactly.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DatafixReconciliationField {
    CountyMun(String, String),
    DoB(String, String),
    Ward(String, String),
    Poll(String, String),
    SchoolSupportCode(String, String),
    Channel(String, String),
    /// Kept as the literal CSV strings ("true"/"false"/"NONE"), not a `bool`
    /// — unlike Sequent's own `Enabled` (a genuine two-state Keycloak flag),
    /// the patch CSV's `Deleted` column can legitimately carry `NONE` (no
    /// prior value, e.g. reporting a Sequent-only voter Datafix has never
    /// seen), which a `bool` can't represent.
    Deleted(String, String),
}

impl DatafixReconciliationField {
    /// Every column name, in the fixed order the patch CSV and the "Patch
    /// Files Format" spec require regardless of which fields changed —
    /// carries no old/new data since it's used to iterate the fixed set of
    /// possible columns, not any voter's actual values.
    pub const NAMES: [&'static str; 7] = [
        "CountyMun",
        "DoB",
        "Ward",
        "Poll",
        "SchoolSupportCode",
        "Channel",
        "Deleted",
    ];

    /// The column name this instance carries a value for.
    pub fn name(&self) -> &'static str {
        match self {
            Self::CountyMun(..) => "CountyMun",
            Self::DoB(..) => "DoB",
            Self::Ward(..) => "Ward",
            Self::Poll(..) => "Poll",
            Self::SchoolSupportCode(..) => "SchoolSupportCode",
            Self::Channel(..) => "Channel",
            Self::Deleted(..) => "Deleted",
        }
    }

    /// The `(old, new)` pair as the literal strings the patch CSV writes.
    pub fn old_new(&self) -> (&str, &str) {
        match self {
            Self::CountyMun(old, new)
            | Self::DoB(old, new)
            | Self::Ward(old, new)
            | Self::Poll(old, new)
            | Self::SchoolSupportCode(old, new)
            | Self::Channel(old, new)
            | Self::Deleted(old, new) => (old.as_str(), new.as_str()),
        }
    }
}

/// One row of an uploaded reconciliation file, after CSV parsing. Field names
/// match the CSV header (`CountyMun,VoterID,DoB,Ward,Poll,SchoolSupportCode,
/// Channel,Deleted`) via `serde(rename)` so `csv::Reader::deserialize` can
/// build this directly — see `reconciliation::csv`.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ParsedDatafixReconciliationRow {
    #[serde(rename = "CountyMun")]
    pub county_mun: String,
    #[serde(rename = "VoterID")]
    pub voter_id: String,
    #[serde(rename = "DoB")]
    pub dob: String,
    #[serde(rename = "Ward")]
    pub ward: String,
    #[serde(rename = "Poll")]
    pub poll: String,
    #[serde(rename = "SchoolSupportCode")]
    pub school_support_code: String,
    #[serde(rename = "Channel")]
    pub channel: String,
    #[serde(rename = "Deleted")]
    pub deleted: String, // "true"/"false" — kept as the wire string, parsed where needed
}

impl ParsedDatafixReconciliationRow {
    /// This row's own value for one of `DatafixReconciliationField::NAMES`,
    /// by column name — used to fill in a field that didn't change for this
    /// voter on the outbound patch CSV with its real current value (this row
    /// is exactly that value, since an unchanged field is one Sequent didn't
    /// disagree with) instead of a placeholder.
    pub fn field_value(&self, name: &str) -> Option<&str> {
        match name {
            "CountyMun" => Some(&self.county_mun),
            "DoB" => Some(&self.dob),
            "Ward" => Some(&self.ward),
            "Poll" => Some(&self.poll),
            "SchoolSupportCode" => Some(&self.school_support_code),
            "Channel" => Some(&self.channel),
            "Deleted" => Some(&self.deleted),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DatafixErrorCode, DatafixResponse};
    use rocket::http::Status;

    #[test]
    fn error_reply_carries_the_documented_status_and_error_code() {
        let response = DatafixResponse::error(DatafixErrorCode::VoterAlreadyExists);
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
            (
                DatafixErrorCode::VoterAlreadyExists,
                "voter-already-exists",
                Status::Conflict,
            ),
            (
                DatafixErrorCode::VoterOperationInProgress,
                "voter-operation-in-progress",
                Status::Conflict,
            ),
            (
                DatafixErrorCode::VoterStateUnresolved,
                "voter-state-unresolved",
                Status::Conflict,
            ),
            (
                DatafixErrorCode::VoterVotedOnline,
                "voter-voted-online",
                Status::Conflict,
            ),
            (
                DatafixErrorCode::VoterNotFound,
                "voter-not-found",
                Status::NotFound,
            ),
            (
                DatafixErrorCode::AreaNotFound,
                "area-not-found",
                Status::UnprocessableEntity,
            ),
            (
                DatafixErrorCode::EventNotFound,
                "event-not-found",
                Status::NotFound,
            ),
            (
                DatafixErrorCode::InvalidRequest,
                "invalid-request",
                Status::BadRequest,
            ),
            (DatafixErrorCode::Forbidden, "forbidden", Status::Forbidden),
            (
                DatafixErrorCode::InternalError,
                "internal-error",
                Status::InternalServerError,
            ),
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
