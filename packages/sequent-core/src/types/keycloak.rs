// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
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
pub const DISABLE_REASON_DELETE_CALL: &str =
    "Disable reason: datafix call to delete-voter endpoint";
pub const DISABLE_REASON_MARKVOTED_CALL: &str =
    "Disable reason: Voter marked as voted via other channel";

/// If there is a call to Datafix mark-voted, we disable the voter and set this
/// value to signal the channel e.g "PHONE", "POST"... whatsoever
///
/// If there is a call to Datafix unmark-voted, we enable the voter and reset
/// this attribute to NONE.
///
/// In addition the voter list, when setting the has_voted flag will check if
/// this attribute is set, then set has_voted true.
pub const VOTED_CHANNEL: &str = "voted-channel";
pub const VOTED_CHANNEL_INTERNET_VALUE: &str = "Internet";
pub const ATTR_RESET_VALUE: &str = "NONE";

pub const AREA_ID_ATTR_NAME: &str = "area-id";
pub const DATE_OF_BIRTH: &str = "dateOfBirth";
pub const AUTHORIZED_ELECTION_IDS_NAME: &str = "authorized-election-ids";
pub const TENANT_ID_ATTR_NAME: &str = "tenant-id";
pub const PERMISSION_TO_EDIT: &str = "admin";
pub const MOBILE_PHONE_ATTR_NAME: &str = "sequent.read-only.mobile-number";
pub const FIRST_NAME: &str = "firstName";
pub const LAST_NAME: &str = "lastName";
pub const PERMISSION_LABELS: &str = "permission_labels";

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct UserArea {
    pub id: Option<String>,
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct VotesInfo {
    pub election_id: String,
    pub num_votes: usize,
    pub last_voted_at: String,
}

#[derive(
    Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone, Default,
)]
pub struct User {
    pub id: Option<String>,
    pub attributes: Option<HashMap<String, Vec<String>>>,
    pub email: Option<String>,
    pub email_verified: Option<bool>,
    pub enabled: Option<bool>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub username: Option<String>,
    pub area: Option<UserArea>,
    pub votes_info: Option<Vec<VotesInfo>>,
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct Permission {
    pub id: Option<String>,
    pub attributes: Option<HashMap<String, Vec<String>>>,
    pub container_id: Option<String>,
    pub description: Option<String>,
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct Role {
    pub id: Option<String>,
    pub name: Option<String>,
    pub permissions: Option<Vec<String>>,
    pub access: Option<HashMap<String, bool>>,
    pub attributes: Option<HashMap<String, Vec<String>>>,
    pub client_roles: Option<HashMap<String, Vec<String>>>,
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct UPAttributePermissions {
    pub edit: Option<Vec<String>>,
    pub view: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct UPAttributeSelector {
    pub scopes: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct UPAttributeRequired {
    pub roles: Option<Vec<String>>,
    pub scopes: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct UserProfileAttribute {
    pub annotations: Option<HashMap<String, Value>>,
    pub display_name: Option<String>,
    pub group: Option<String>,
    pub multivalued: Option<bool>,
    pub name: Option<String>,
    pub required: Option<UPAttributeRequired>,
    pub validations: Option<HashMap<String, HashMap<String, Value>>>,
    pub permissions: Option<UPAttributePermissions>,
    pub selector: Option<UPAttributeSelector>,
}
