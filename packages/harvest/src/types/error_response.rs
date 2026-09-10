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
    PrivateKeyDownloadUnavailable,
    ConfirmPolicyShowCastVoteLogsFailed,
    BallotIdMismatch,
    BallotPublicationValidation,
    // Add any other needed error codes
}

#[derive(Serialize, Default)]
pub struct ErrorExtensions {
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password_policy_rule: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password_policy_required_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_profile_errors: Option<Vec<UserProfileErrorExtension>>,
    /// How many attributes Keycloak refused in total, which can exceed the
    /// number reported above.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_profile_errors_total: Option<usize>,
}

/// One refused attribute, as the admin portal reads it.
#[derive(Serialize)]
pub struct UserProfileErrorExtension {
    pub field: Option<String>,
    pub error: Option<String>,
    pub params: Vec<Value>,
}

impl From<&UserProfileValidationError> for UserProfileErrorExtension {
    fn from(validation: &UserProfileValidationError) -> Self {
        Self {
            field: validation.field.clone(),
            error: validation.error_message.clone(),
            params: validation.params.clone().unwrap_or_default(),
        }
    }
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
                    ..Default::default()
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
                    ..Default::default()
                },
            }),
        )
    }

    /// The user profile constraints Keycloak refused a write against. Each
    /// offending field and the constraint's arguments travel in the extensions
    /// so the admin portal can name the fields and state their limits in its
    /// own language, while the message stays readable on its own for any other
    /// consumer. `total` counts everything Keycloak refused, which can exceed
    /// what is reported.
    pub fn user_profile_validation(
        status: Status,
        message: &str,
        validations: &[UserProfileValidationError],
        total: usize,
    ) -> JsonError {
        Custom(
            status,
            Json(ErrorResponse {
                message: message.into(),
                extensions: ErrorExtensions {
                    code: ErrorCode::UserProfileValidation.as_ref().into(),
                    user_profile_errors: Some(
                        validations
                            .iter()
                            .map(UserProfileErrorExtension::from)
                            .collect(),
                    ),
                    user_profile_errors_total: Some(total),
                    ..Default::default()
                },
            }),
        )
    }
}
