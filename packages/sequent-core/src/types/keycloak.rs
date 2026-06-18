// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Keycloak user-attribute names and API types for voter identity management.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Keycloak user-attribute key storing the disable reason comment.
///
/// A voter can be disabled via a Datafix delete-voter endpoint or a
/// Datafix mark-voted call.
pub const DISABLE_COMMENT: &str = "disable-comment";

/// Disable reason written when a voter is removed via the Datafix delete-voter endpoint.
pub const DISABLE_REASON_DELETE_CALL: &str =
    "Disable reason: datafix call to delete-voter endpoint";

/// Disable reason written when a voter is disabled after being marked voted elsewhere.
pub const DISABLE_REASON_MARKVOTED_CALL: &str =
    "Disable reason: Voter marked as voted via other channel";

/// If there is a call to Datafix mark-voted, we disable the voter and set this
/// value to signal the channel e.g "PHONE", "POST"... whatsoever
///
/// If there is a call to Datafix unmark-voted, we enable the voter and reset
/// this attribute to NONE.
///
/// In addition the voter list, when setting the `has_voted` flag will check if
/// this attribute is set, then set `has_voted` true.
pub const VOTED_CHANNEL: &str = "voted-channel";

/// Value stored in [`VOTED_CHANNEL`] when the voter cast via the online portal.
pub const VOTED_CHANNEL_INTERNET_VALUE: &str = "Internet";

/// Sentinel value indicating a Keycloak attribute is unset.
pub const ATTR_RESET_VALUE: &str = "NONE";

/// Keycloak user-attribute key for the voter's assigned geographic area.
pub const AREA_ID_ATTR_NAME: &str = "area-id";

/// Keycloak user-profile attribute name for date of birth.
pub const DATE_OF_BIRTH: &str = "dateOfBirth";

/// Keycloak user-attribute key listing elections the voter may access.
pub const AUTHORIZED_ELECTION_IDS_NAME: &str = "authorized-election-ids";

/// Keycloak user-attribute key for the owning tenant identifier.
pub const TENANT_ID_ATTR_NAME: &str = "tenant-id";

/// Role name granting permission to edit admin resources.
pub const PERMISSION_TO_EDIT: &str = "admin";

/// Keycloak read-only attribute key for the voter's mobile phone number.
pub const MOBILE_PHONE_ATTR_NAME: &str = "sequent.read-only.mobile-number";

/// Keycloak user-profile attribute name for first name.
pub const FIRST_NAME: &str = "firstName";

/// Keycloak user-profile attribute name for last name.
pub const LAST_NAME: &str = "lastName";

/// Keycloak user-attribute key for permission labels gating election access.
pub const PERMISSION_LABELS: &str = "permission_labels";

/// Realm-level attribute key for the voter certificate authentication policy.
pub const REALM_ATTR_VOTER_CERTIFICATE_POLICY: &str =
    "voter-certificate-policy";

/// Keycloak identity-provider alias for digital-certificate authentication.
pub const CERTIFICATES_IDP_ALIAS: &str = "digital-certificates";

/// Geographic area assigned to a Keycloak voter user.
#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct UserArea {
    /// Area identifier.
    pub id: Option<String>,
    /// Display name.
    pub name: Option<String>,
}

/// Per-election vote history attached to a Keycloak voter user.
#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct VotesInfo {
    /// Election the voter cast in.
    pub election_id: String,
    /// Number of times the voter has cast in this election.
    pub num_votes: usize,
    /// ISO 8601 timestamp of the most recent cast.
    pub last_voted_at: String,
}

/// Keycloak user representation for a voter or administrator.
#[derive(
    Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone, Default,
)]
pub struct User {
    /// Keycloak user identifier.
    pub id: Option<String>,
    /// Custom user attributes (area, tenant, voted channel, etc.).
    pub attributes: Option<HashMap<String, Vec<String>>>,
    /// Email address.
    pub email: Option<String>,
    /// When true, the email address has been verified.
    pub email_verified: Option<bool>,
    /// When false, the user cannot authenticate.
    pub enabled: Option<bool>,
    /// Given name.
    pub first_name: Option<String>,
    /// Family name.
    pub last_name: Option<String>,
    /// Login username.
    pub username: Option<String>,
    /// Assigned geographic area.
    pub area: Option<UserArea>,
    /// Vote history per election.
    pub votes_info: Option<Vec<VotesInfo>>,
}

/// Keycloak authorization permission resource.
#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct Permission {
    /// Permission identifier.
    pub id: Option<String>,
    /// Custom permission attributes.
    pub attributes: Option<HashMap<String, Vec<String>>>,
    /// Resource container (client or realm) identifier.
    pub container_id: Option<String>,
    /// Human-readable description.
    pub description: Option<String>,
    /// Permission name.
    pub name: Option<String>,
}

/// Keycloak realm or client role with attached permissions.
#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct Role {
    /// Role identifier.
    pub id: Option<String>,
    /// Role name.
    pub name: Option<String>,
    /// Permission identifiers granted by this role.
    pub permissions: Option<Vec<String>>,
    /// Resource-level access flags.
    pub access: Option<HashMap<String, bool>>,
    /// Custom role attributes.
    pub attributes: Option<HashMap<String, Vec<String>>>,
    /// Client-scoped role names grouped by client identifier.
    pub client_roles: Option<HashMap<String, Vec<String>>>,
}

/// Edit and view permissions for a Keycloak user-profile attribute.
#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct UPAttributePermissions {
    /// Roles allowed to edit this attribute.
    pub edit: Option<Vec<String>>,
    /// Roles allowed to view this attribute.
    pub view: Option<Vec<String>>,
}

/// Scope selector restricting where a user-profile attribute appears.
#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct UPAttributeSelector {
    /// OAuth scopes where this attribute is included.
    pub scopes: Option<Vec<String>>,
}

/// Conditions under which a user-profile attribute is required.
#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct UPAttributeRequired {
    /// Roles for which this attribute is mandatory.
    pub roles: Option<Vec<String>>,
    /// OAuth scopes for which this attribute is mandatory.
    pub scopes: Option<Vec<String>>,
}

/// Definition of a single attribute in a Keycloak user profile.
#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct UserProfileAttribute {
    /// Admin-defined metadata for this attribute.
    pub annotations: Option<HashMap<String, Value>>,
    /// Localized display label.
    pub display_name: Option<String>,
    /// UI group this attribute belongs to.
    pub group: Option<String>,
    /// When true, the attribute accepts multiple values.
    pub multivalued: Option<bool>,
    /// Attribute key used in Keycloak storage.
    pub name: Option<String>,
    /// When this attribute is required during enrollment.
    pub required: Option<UPAttributeRequired>,
    /// Validation rules (format, length, etc.) keyed by validator name.
    pub validations: Option<HashMap<String, HashMap<String, Value>>>,
    /// Who may view or edit this attribute.
    pub permissions: Option<UPAttributePermissions>,
    /// OAuth scopes where this attribute is exposed.
    pub selector: Option<UPAttributeSelector>,
}
