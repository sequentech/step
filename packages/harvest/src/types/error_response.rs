// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use rocket::http::Status;
use rocket::response::status::Custom;
use rocket::serde::{json::Json, Serialize};
use std::convert::AsRef;
use strum_macros::{AsRefStr, Display};
use tracing::instrument;

pub type JsonError = Custom<Json<ErrorResponse>>;

#[derive(Serialize, AsRefStr, Display, Debug)]
pub enum ErrorCode {
    InternalServerError,
    Unauthorized,
    CheckStatusFailed,
    AreaNotFound,
    ElectionEventNotFound,
    ElectoralLogNotFound,
    CheckPreviousVotesFailed,
    CheckRevotesFailed,
    CheckVotesInOtherAreasFailed,
    InsertFailedExceedsAllowedRevotes,
    GetClientCredentialsFailed,
    GetAreaIdFailed,
    GetTransactionFailed,
    DeserializeBallotFailed,
    DeserializeAreaPresentationFailed,
    DeserializeContestsFailed,
    PokValidationFailed,
    UuidParseFailed,
    UnknownError,
    InvalidEventProcessor,
    InvalidPasswordPolicy,
    PasswordPolicyNotConfigured,
    PasswordPolicyMinimumLengthMissing,
    PasswordPolicyCharacterClassMissing,
    PasswordPolicyViolation,
    DocumentPasswordUnavailable,
    VoterInformationLetterUnavailable,
    ConfirmPolicyShowCastVoteLogsFailed,
    BallotIdMismatch,
    // Add any other needed error codes
}

#[derive(Serialize)]
pub struct ErrorExtensions {
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password_policy_rule: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password_policy_required_count: Option<i32>,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub message: String,
    pub extensions: ErrorExtensions,
}

impl ErrorResponse {
    #[instrument]
    pub fn new(status: Status, message: &str, code: ErrorCode) -> JsonError {
        return Custom(
            status,
            Json(ErrorResponse {
                message: message.into(),
                extensions: ErrorExtensions {
                    code: code.as_ref().into(),
                    password_policy_rule: None,
                    password_policy_required_count: None,
                },
            }),
        );
    }

    pub fn password_policy_violation(
        status: Status,
        message: &str,
        rule: &str,
        required_count: i32,
    ) -> JsonError {
        Custom(
            status,
            Json(ErrorResponse {
                message: message.into(),
                extensions: ErrorExtensions {
                    code: ErrorCode::PasswordPolicyViolation.as_ref().into(),
                    password_policy_rule: Some(rule.into()),
                    password_policy_required_count: Some(required_count),
                },
            }),
        )
    }
}
