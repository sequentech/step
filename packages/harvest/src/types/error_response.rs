// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use rocket::http::Status;
use rocket::response::status::Custom;
use rocket::serde::{json::Json, Serialize};
use std::convert::AsRef;
use strum_macros::{AsRefStr, Display};
use tracing::instrument;

/// JSON error payload type returned by routes using [`ErrorResponse`].
pub type JsonError = Custom<Json<ErrorResponse>>;

#[derive(Serialize, AsRefStr, Display, Debug, Copy, Clone)]
/// Error code enum.
pub enum ErrorCode {
    /// Internal server error.
    InternalServerError,
    /// Unauthorized.
    Unauthorized,
    /// Check status failed.
    CheckStatusFailed,
    /// Area not found.
    AreaNotFound,
    /// Election event not found.
    ElectionEventNotFound,
    /// Electoral log not found.
    ElectoralLogNotFound,
    /// Check previous votes failed.
    CheckPreviousVotesFailed,
    /// Check revotes failed.
    CheckRevotesFailed,
    /// Check votes in other areas failed.
    CheckVotesInOtherAreasFailed,
    /// Insert failed because the voter exceeded allowed revotes.
    InsertFailedExceedsAllowedRevotes,
    /// Get client credentials failed.
    GetClientCredentialsFailed,
    /// Get area ID failed.
    GetAreaIdFailed,
    /// Failed to obtain a database transaction.
    GetTransactionFailed,
    /// Deserialize ballot failed.
    DeserializeBallotFailed,
    /// Deserialize area presentation failed.
    DeserializeAreaPresentationFailed,
    /// Deserialize contests failed.
    DeserializeContestsFailed,
    /// Pok validation failed.
    PokValidationFailed,
    /// UUID parse failed.
    UuidParseFailed,
    /// Unknown error.
    UnknownError,
    /// Invalid event processor.
    InvalidEventProcessor,
    /// Confirm policy show cast vote logs failed.
    ConfirmPolicyShowCastVoteLogsFailed,
    /// Ballot ID mismatch.
    BallotIdMismatch,
    // Add any other needed error codes
}

/// GraphQL-style `extensions` object attached to [`ErrorResponse`].
#[derive(Serialize)]
pub struct ErrorExtensions {
    /// Machine-readable error code string.
    pub code: String,
}

/// Structured JSON error body for Rocket `Custom` responses.
#[derive(Serialize)]
pub struct ErrorResponse {
    /// Human-readable error message.
    pub message: String,
    /// Additional error metadata (for example the error code).
    pub extensions: ErrorExtensions,
}

impl ErrorResponse {
    /// Builds a [`JsonError`] with the given HTTP status, message, and [`ErrorCode`].
    #[instrument]
    pub fn new(status: Status, message: &str, code: ErrorCode) -> JsonError {
        Custom(
            status,
            Json(ErrorResponse {
                message: message.into(),
                extensions: ErrorExtensions {
                    code: code.as_ref().into(),
                },
            }),
        )
    }
}
