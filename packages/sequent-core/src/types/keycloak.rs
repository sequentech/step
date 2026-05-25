// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// A voter can be disabled:
///
/// Via Datafix delete-voter end point.
///
/// A call to Datafix mark-voted.
pub const DISABLE_COMMENT: &str = "disable-comment";
/// Reason string for disabling a voter via Datafix delete-voter endpoint.
pub const DISABLE_REASON_DELETE_CALL: &str =
    "Disable reason: datafix call to delete-voter endpoint";
/// Reason string for disabling a voter via Datafix mark-voted call.
pub const DISABLE_REASON_MARKVOTED_CALL: &str =
    "Disable reason: Voter marked as voted via other channel";

/// If there is a call to Datafix mark-voted, we disable the voter and set this
/// value to signal the channel e.g. `PHONE`, `POST`, etc.
///
/// If there is a call to Datafix unmark-voted, we enable the voter and reset
/// this attribute to `NONE`.
///
/// In addition, the voter list, when setting the `has_voted` flag, will check if
/// this attribute is set, then set `has_voted` to true.
pub const VOTED_CHANNEL: &str = "voted-channel";
/// Value for internet voting channel.
pub const VOTED_CHANNEL_INTERNET_VALUE: &str = "Internet";
/// Value used to reset an attribute.
pub const ATTR_RESET_VALUE: &str = "NONE";

/// Attribute name for area ID.
pub const AREA_ID_ATTR_NAME: &str = "area-id";
/// Attribute name for date of birth.
pub const DATE_OF_BIRTH: &str = "dateOfBirth";
/// Attribute name for authorized election IDs.
pub const AUTHORIZED_ELECTION_IDS_NAME: &str = "authorized-election-ids";
/// Attribute name for tenant ID.
pub const TENANT_ID_ATTR_NAME: &str = "tenant-id";
/// Permission name for editing.
pub const PERMISSION_TO_EDIT: &str = "admin";
/// Attribute name for mobile phone number.
pub const MOBILE_PHONE_ATTR_NAME: &str = "sequent.read-only.mobile-number";
/// Attribute name for first name.
pub const FIRST_NAME: &str = "firstName";
/// Attribute name for last name.
pub const LAST_NAME: &str = "lastName";
/// Attribute name for permission labels.
pub const PERMISSION_LABELS: &str = "permission_labels";
/// Realm attribute key storing the voter certificate policy.
pub const REALM_ATTR_VOTER_CERTIFICATE_POLICY: &str =
    "voter-certificate-policy";
/// Identity-provider alias for the digital-certificates `IdP` configured on tenant realms.
pub const CERTIFICATES_IDP_ALIAS: &str = "digital-certificates";

/// Represents an area assigned to a user.
#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct UserArea {
    /// Area identifier.
    pub id: Option<String>,
    /// Area name.
    pub name: Option<String>,
}

/// Information about votes cast by a user in an election.
#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct VotesInfo {
    /// Election identifier.
    pub election_id: String,
    /// Number of votes cast.
    pub num_votes: usize,
    /// Timestamp of last vote cast.
    pub last_voted_at: String,
}

/// Represents a user in Keycloak with profile and voting information.
#[derive(
    Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone, Default,
)]
pub struct User {
    /// User identifier.
    pub id: Option<String>,
    /// User attributes.
    pub attributes: Option<HashMap<String, Vec<String>>>,
    /// User email address.
    pub email: Option<String>,
    /// Whether the email is verified.
    pub email_verified: Option<bool>,
    /// Whether the user is enabled.
    pub enabled: Option<bool>,
    /// User's first name.
    pub first_name: Option<String>,
    /// User's last name.
    pub last_name: Option<String>,
    /// Username.
    pub username: Option<String>,
    /// Area assigned to the user.
    pub area: Option<UserArea>,
    /// Voting information for the user.
    pub votes_info: Option<Vec<VotesInfo>>,
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
/// Represents a permission in Keycloak.
pub struct Permission {
    /// Permission identifier.
    pub id: Option<String>,
    /// Permission attributes.
    pub attributes: Option<HashMap<String, Vec<String>>>,
    /// Container identifier for the permission.
    pub container_id: Option<String>,
    /// Description of the permission.
    pub description: Option<String>,
    /// Name of the permission.
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
/// Represents a role in Keycloak.
pub struct Role {
    /// Role identifier.
    pub id: Option<String>,
    /// Role name.
    pub name: Option<String>,
    /// Permissions associated with the role.
    pub permissions: Option<Vec<String>>,
    /// Access map for the role.
    pub access: Option<HashMap<String, bool>>,
    /// Role attributes.
    pub attributes: Option<HashMap<String, Vec<String>>>,
    /// Client roles associated with the role.
    pub client_roles: Option<HashMap<String, Vec<String>>>,
}

/// Permissions for editing and viewing user profile attributes.
#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct UPAttributePermissions {
    /// Roles allowed to edit the attribute.
    pub edit: Option<Vec<String>>,
    /// Roles allowed to view the attribute.
    pub view: Option<Vec<String>>,
}

/// Selector for user profile attribute scopes.
#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct UPAttributeSelector {
    /// Scopes for the attribute selector.
    pub scopes: Option<Vec<String>>,
}

/// Required roles and scopes for a user profile attribute.
#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct UPAttributeRequired {
    /// Roles required for the attribute.
    pub roles: Option<Vec<String>>,
    /// Scopes required for the attribute.
    pub scopes: Option<Vec<String>>,
}

/// Represents a user profile attribute in Keycloak.
#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct UserProfileAttribute {
    /// Annotations for the attribute.
    pub annotations: Option<HashMap<String, Value>>,
    /// Display name for the attribute.
    pub display_name: Option<String>,
    /// Group to which the attribute belongs.
    pub group: Option<String>,
    /// Whether the attribute is multivalued.
    pub multivalued: Option<bool>,
    /// Name of the attribute.
    pub name: Option<String>,
    /// Required roles and scopes for the attribute.
    pub required: Option<UPAttributeRequired>,
    /// Validations for the attribute.
    pub validations: Option<HashMap<String, HashMap<String, Value>>>,
    /// Permissions for the attribute.
    pub permissions: Option<UPAttributePermissions>,
    /// Selector for the attribute.
    pub selector: Option<UPAttributeSelector>,
}
