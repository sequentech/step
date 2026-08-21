// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use rocket::http::Status;
use rocket::response::status::Custom;
use rocket::serde::{json::Json, Serialize};
use sequent_core::services::keycloak::UserProfileValidationError;
use serde_json::Value;
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
    UserProfileValidation,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_profile_field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_profile_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_profile_params: Option<Vec<Value>>,
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
                    user_profile_field: None,
                    user_profile_error: None,
                    user_profile_params: None,
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
                    user_profile_field: None,
                    user_profile_error: None,
                    user_profile_params: None,
                },
            }),
        )
    }

    /// A user profile constraint Keycloak refused a write against. The offending
    /// field and the constraint's arguments travel in the extensions so the
    /// admin portal can name the field and state the limit in its own language,
    /// while the message stays readable on its own for any other consumer.
    pub fn user_profile_validation(
        status: Status,
        message: &str,
        validation: &UserProfileValidationError,
    ) -> JsonError {
        Custom(
            status,
            Json(ErrorResponse {
                message: message.into(),
                extensions: ErrorExtensions {
                    code: ErrorCode::UserProfileValidation.as_ref().into(),
                    password_policy_rule: None,
                    password_policy_required_count: None,
                    user_profile_field: validation.field.clone(),
                    user_profile_error: validation.error_message.clone(),
                    user_profile_params: Some(validation.params.clone()),
                },
            }),
        )
    }
}
