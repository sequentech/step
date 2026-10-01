// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Who takes a signing step, why a step is refused, and what the log calls
//! things.

use super::certificates::OtherHolder;
use super::directory::display_name;
use super::log::{Actor, LogScope};
use super::signers;
use anyhow::anyhow;
use chrono::{DateTime, Utc};
use sequent_core::services::jwt::{decode_permission_labels, JwtClaims};
use sequent_core::signing::{CertificateCheckId, SigningAction, SigningRequestStatus};
use sequent_core::types::permissions::Permissions;
use serde::Serialize;
use std::collections::HashSet;
use std::fmt;
use strum_macros::Display;
use uuid::Uuid;

/// The signed-in person a signing step is taken by, as their token says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SigningCaller {
    pub user_id: String,
    pub username: String,
    /// First and last name, as people see it; the username without one.
    pub display_name: String,
    /// The Keycloak roles (permissions) the token carries.
    pub roles: HashSet<String>,
    /// The `permission_labels` of the token; none means every Post.
    pub labels: Vec<String>,
    pub auth_time: Option<DateTime<Utc>>,
    /// The trustee name of a trustee's token.
    pub trustee: Option<String>,
}

impl SigningCaller {
    pub fn from_claims(claims: &JwtClaims) -> Self {
        let username = claims
            .preferred_username
            .clone()
            .unwrap_or_else(|| claims.hasura_claims.user_id.clone());
        // Given and family name, then `name`, then the username, as the
        // Harvest signing routes name the caller.
        let display_name = Some(display_name(
            claims.given_name.as_deref(),
            claims.family_name.as_deref(),
            "",
        ))
        .filter(|name| !name.is_empty())
        .or_else(|| {
            claims
                .name
                .as_deref()
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(str::to_owned)
        })
        .unwrap_or_else(|| username.clone());
        SigningCaller {
            user_id: claims.hasura_claims.user_id.clone(),
            display_name,
            username,
            roles: claims.hasura_claims.allowed_roles.iter().cloned().collect(),
            labels: decode_permission_labels(claims),
            auth_time: claims
                .auth_time
                .and_then(|seconds| DateTime::<Utc>::from_timestamp(seconds, 0)),
            trustee: claims.trustee.clone(),
        }
    }

    pub fn actor(&self) -> Actor {
        Actor {
            user_id: self.user_id.clone(),
            username: self.username.clone(),
        }
    }

    pub fn has(&self, permission: Permissions) -> bool {
        self.roles.contains(&permission.to_string())
    }

    /// Whether the caller may start, read or cancel requests of a Post with
    /// `label`: an unlabeled Post is everybody's, and a caller without labels
    /// (such as the tally, which starts its reports' requests) acts on every
    /// Post.
    pub fn reaches(&self, label: Option<&str>) -> bool {
        match label {
            None => true,
            Some(label) => self.labels.is_empty() || self.labels.iter().any(|own| own == label),
        }
    }

    /// Whether the caller signs for a Post with `label`, as the signer list
    /// counts them ([`signers::signs_for`]): without labels, only for
    /// unlabeled Posts.
    pub fn signs_for(&self, label: Option<&str>) -> bool {
        signers::signs_for(&self.labels, label)
    }
}

/// Why a signing step was not taken. The Harvest routes answer each with its
/// own status.
#[derive(Debug)]
pub enum SigningError {
    NotFound(String),
    Forbidden(String),
    /// The input can't be taken as it is.
    Invalid {
        reason: InvalidReason,
        message: String,
    },
    /// Another save came first, or two steps met.
    Conflict(String),
    /// The request takes no more steps of this kind: it is `status`.
    Closed {
        status: SigningRequestStatus,
        message: String,
    },
    /// A check refused a signature; the refusal is logged.
    Refused {
        check: CertificateCheckId,
        message: String,
        /// For `registered-to-other`: the account the key or holder is
        /// registered to.
        other_holder: Option<OtherHolder>,
    },
    Internal(anyhow::Error),
}

pub type SigningResult<T> = Result<T, SigningError>;

/// Why an input was refused, so the portal can say it precisely.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Display)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum InvalidReason {
    /// Malformed or out of range.
    Input,
    /// A rule needs at least one signature.
    NoSignatures,
    /// An expiry must be at least one minute; none means no limit.
    ZeroExpiry,
    /// A number beyond what the product supports.
    OutOfRange,
    /// More signatures than any Post has people who can sign.
    OverCapacity,
    /// The election event is locked down.
    LockedDown,
    /// No group of the realm has that id.
    UnknownGroup,
    /// The group holds the sign permission through a composite role, which
    /// only Users and Roles can change.
    Composite,
    /// The action signs a document this server can't take signatures of
    /// yet, or the signature is of another kind of document.
    Document,
}

impl SigningError {
    pub fn invalid(reason: InvalidReason, message: impl Into<String>) -> Self {
        SigningError::Invalid {
            reason,
            message: message.into(),
        }
    }

    /// Malformed input.
    pub fn bad_input(message: impl Into<String>) -> Self {
        SigningError::invalid(InvalidReason::Input, message)
    }
}

impl fmt::Display for SigningError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SigningError::NotFound(message)
            | SigningError::Forbidden(message)
            | SigningError::Invalid { message, .. }
            | SigningError::Conflict(message)
            | SigningError::Closed { message, .. } => write!(f, "{message}"),
            SigningError::Refused { check, message, .. } => write!(f, "{check}: {message}"),
            SigningError::Internal(error) => write!(f, "{error:?}"),
        }
    }
}

impl std::error::Error for SigningError {}

impl From<anyhow::Error> for SigningError {
    fn from(error: anyhow::Error) -> Self {
        SigningError::Internal(error)
    }
}

impl From<tokio_postgres::Error> for SigningError {
    fn from(error: tokio_postgres::Error) -> Self {
        SigningError::Internal(anyhow!(error))
    }
}

/// The English name of an action in log descriptions. The portal shows
/// its own translated label (`signing.actions.<id>`).
pub fn action_title(action: SigningAction) -> &'static str {
    match action {
        SigningAction::InitializeVoting => "Initialize voting",
        SigningAction::OpenVoting => "Open voting",
        SigningAction::CloseVoting => "Close voting",
        SigningAction::GenerateElectionReturns => "Generate election returns",
        SigningAction::GenerateReports => "Generate reports",
        SigningAction::TransmitResults => "Transmit results",
        SigningAction::ApproveVoter => "Approve voter",
        SigningAction::ApproveConfiguration => "Approve configuration",
        SigningAction::ConfirmKeyShare => "Confirm key share",
        SigningAction::ContributeKeyShare => "Contribute key share",
    }
}

/// Where a step is logged: the event, and the request's Post and country.
pub fn log_scope(
    tenant_id: Uuid,
    election_event_id: Uuid,
    election_id: Option<Uuid>,
    area_id: Option<Uuid>,
) -> LogScope {
    LogScope {
        tenant_id,
        election_event_id,
        election_id,
        area_id,
    }
}
