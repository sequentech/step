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
use serde_json::Value;
use std::collections::HashSet;
use std::fmt;
use strum_macros::Display;
use uuid::Uuid;

/// Which Posts a caller reaches: the Posts whose requests they start, read,
/// export and cancel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostReach {
    /// A person, by the labels of their token, as Hasura shows them Posts:
    /// an unlabelled Post, and a labelled one whose label they hold. A
    /// person without labels reaches only unlabelled Posts.
    Labels,
    /// The system, starting a request nobody started by hand (the tally's
    /// reports, a scheduled report): every Post. It holds no permission,
    /// so it never signs, reads or cancels.
    AllPosts,
}

/// Who a signing step is taken by: a signed-in person, as their token says,
/// or the system ([`SigningCaller::system`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SigningCaller {
    pub user_id: String,
    pub username: String,
    /// First and last name, as people see it; the username without one.
    pub display_name: String,
    /// The Keycloak roles (permissions) the token carries.
    pub roles: HashSet<String>,
    /// The `permission_labels` of the token.
    pub labels: Vec<String>,
    pub auth_time: Option<DateTime<Utc>>,
    /// The trustee name of a trustee's token.
    pub trustee: Option<String>,
    pub reach: PostReach,
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
            reach: PostReach::Labels,
        }
    }

    /// The system as the requester of a request nobody started by hand,
    /// named `name` in the log: it reaches every Post and holds no
    /// permission.
    pub fn system(name: &str) -> Self {
        SigningCaller {
            user_id: name.into(),
            username: name.into(),
            display_name: name.into(),
            roles: HashSet::new(),
            labels: vec![],
            auth_time: None,
            trustee: None,
            reach: PostReach::AllPosts,
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

    /// Whether the caller may start, read, export or cancel requests of a
    /// Post with `label` ([`PostReach`]): a labelled Post is hidden from a
    /// person without its label, as everywhere on the platform.
    pub fn reaches(&self, label: Option<&str>) -> bool {
        match (self.reach, label) {
            (PostReach::AllPosts, _) | (PostReach::Labels, None) => true,
            (PostReach::Labels, Some(label)) => self.labels.iter().any(|own| own == label),
        }
    }

    /// Whether the caller signs for a Post with `label`, as the signer list
    /// counts them ([`signers::signs_for`]): without labels, only for
    /// unlabeled Posts.
    pub fn signs_for(&self, label: Option<&str>) -> bool {
        signers::signs_for(&self.labels, label)
    }
}

/// What let a person make a change, as its log entry records it in
/// `allowed_by`, like the entries of Users and Roles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Allowance {
    /// A permission they hold.
    Permission(Permissions),
    /// They started the request, which its requester may cancel.
    Requester,
    /// An election event import, which needs no signing permission.
    ElectionEventImport,
}

impl fmt::Display for Allowance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Allowance::Permission(permission) => write!(f, "{permission}"),
            Allowance::Requester => write!(f, "requester"),
            Allowance::ElectionEventImport => write!(f, "election-event-import"),
        }
    }
}

/// The `allowed_by` of a log entry: what allowed the change, as text.
pub fn allowed_by(allowances: &[Allowance]) -> Value {
    Value::from(
        allowances
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
    )
}

/// The `allowed_by` of a change one permission allows.
pub fn allowed_by_permission(permission: Permissions) -> Value {
    allowed_by(&[Allowance::Permission(permission)])
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
    /// The PDF revision a signer prepared no longer extends the document:
    /// another signature came first. The signer prepares again.
    StaleRevision(String),
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
    /// The document's signature fields don't fit the request (one empty
    /// field per signature it needs): generate the document again.
    DocumentFields,
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
            | SigningError::StaleRevision(message)
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
