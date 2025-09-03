// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
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
    DeserializeContestsFailed,
    PokValidationFailed,
    UuidParseFailed,
    UnknownError,
    InvalidEventProcessor,
    ConfirmPolicyShowCastVoteLogsFailed,
    BallotIdMismatch,
    // Add any other needed error codes
}

#[derive(Serialize)]
pub struct ErrorExtensions {
    pub code: String,
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
                },
            }),
        );
    }
}
