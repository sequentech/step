// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What the routes of protected actions share: who starts the action, the
//! `signing_request` they answer while it waits for signatures, and their
//! errors: a signing refusal in the signing contract's JSON body
//! (`{message, extensions: {code, reason?}}`), every other error as the
//! route has always answered it.

use crate::routes::signing::signing_failure;
use crate::services::signing_http::{
    SigningError as SigningFailure, SigningErrorCode,
};
use crate::types::error_response::JsonError;
use rocket::http::Status;
use rocket::request::Request;
use rocket::response::{self, Responder};
use sequent_core::services::jwt::JwtClaims;
use windmill::services::signing::guard::{GuardOutcome, SigningRequestSummary};
use windmill::services::signing::{SigningCaller, SigningError};

/// The person starting a protected action.
pub fn caller(claims: &JwtClaims) -> SigningCaller {
    SigningCaller::from_claims(claims)
}

/// The request a route answers while the action waits, if it does.
pub fn waiting(outcome: GuardOutcome) -> Option<SigningRequestSummary> {
    match outcome {
        GuardOutcome::Proceed => None,
        GuardOutcome::SigningRequired(summary) => Some(summary),
    }
}

/// A route's error: its own (`E`), or a signing refusal.
#[derive(Debug)]
pub enum Guarded<E> {
    Route(E),
    Signing(SigningFailure),
}

impl From<(Status, String)> for Guarded<(Status, String)> {
    fn from(error: (Status, String)) -> Self {
        Guarded::Route(error)
    }
}

impl From<JsonError> for Guarded<JsonError> {
    fn from(error: JsonError) -> Self {
        Guarded::Route(error)
    }
}

impl<E> From<SigningFailure> for Guarded<E> {
    fn from(error: SigningFailure) -> Self {
        Guarded::Signing(error)
    }
}

impl<E> From<SigningError> for Guarded<E> {
    fn from(error: SigningError) -> Self {
        Guarded::Signing(signing_failure(error))
    }
}

impl<E> Guarded<E> {
    /// Maps the route's own error.
    pub fn map_route<F>(self, map: impl FnOnce(E) -> F) -> Guarded<F> {
        match self {
            Guarded::Route(error) => Guarded::Route(map(error)),
            Guarded::Signing(error) => Guarded::Signing(error),
        }
    }
}

impl<'r, E: Responder<'r, 'static>> Responder<'r, 'static> for Guarded<E> {
    fn respond_to(self, request: &'r Request<'_>) -> response::Result<'static> {
        match self {
            Guarded::Route(error) => error.respond_to(request),
            Guarded::Signing(error) => error.respond_to(request),
        }
    }
}

/// 409 `signing-required`: the whole event's change would bypass a Post
/// action that needs signatures.
pub fn signing_required(action: &str) -> SigningFailure {
    SigningFailure::new(
        Status::Conflict,
        SigningErrorCode::SigningRequired,
        format!("{action} needs signatures: start it for each Post."),
    )
}
