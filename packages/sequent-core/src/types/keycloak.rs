// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use strum_macros::{Display, EnumString};

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

/// Per-voter vote weight, used when the election event weighted voting policy
/// is `voters-weighted-voting`. A voter without this attribute votes with
/// `DEFAULT_VOTE_WEIGHT`.
pub const VOTE_WEIGHT_ATTR_NAME: &str = "vote-weight";
pub const DEFAULT_VOTE_WEIGHT: u64 = 1;
pub const MIN_VOTE_WEIGHT: u64 = 1;
/// Upper bound for a single voter weight. Bounds the number of ciphertexts a
/// single voter can add to a mix batch.
pub const MAX_VOTE_WEIGHT: u64 = 100_000;
/// Upper bound for the summed weight of one contest area. `MAX_VOTE_WEIGHT`
/// alone bounds a single voter, not the batch, and the batch is materialised as
/// one allocation, where an oversized request aborts the process rather than
/// returning an error.
pub const MAX_TOTAL_VOTE_WEIGHT: u64 = 1_000_000;

/// Board batches reserved per contest area under `VOTERS_WEIGHTED_VOTING`: one
/// per bit of `MAX_VOTE_WEIGHT`, since a voter's weight is applied by placing
/// their ciphertext in the batch for each bit that weight sets. Derived from
/// `MAX_VOTE_WEIGHT` so the two cannot drift apart.
pub const VOTE_WEIGHT_BATCHES: u32 =
    u64::BITS - MAX_VOTE_WEIGHT.leading_zeros();

/// Below this many ballots, a weight batch is small enough that mixing it
/// protects little: its plaintexts are published, so a batch holding one ballot
/// publishes that voter's choice. High bits are the sparse ones, since few
/// voters carry the largest weights. Warned about rather than refused -- see
/// the note on this policy in the election event documentation.
pub const MIN_WEIGHT_BATCH_ANONYMITY: usize = 5;

/// Bit `bit` of `weight` decides whether that voter's ciphertext goes into the
/// batch whose multiplier is `2^bit`. Summing `2^bit` over the set bits
/// reconstructs the weight exactly, which is what makes the tally correct.
pub fn weight_has_bit(weight: u64, bit: u32) -> bool {
    bit < u64::BITS && (weight >> bit) & 1 == 1
}

/// The multiplier velvet must apply to the batch for `bit`.
pub fn weight_bit_multiplier(bit: u32) -> u64 {
    1u64 << bit
}
pub const DATE_OF_BIRTH: &str = "dateOfBirth";
pub const AUTHORIZED_ELECTION_IDS_NAME: &str = "authorized-election-ids";
pub const TENANT_ID_ATTR_NAME: &str = "tenant-id";
pub const PERMISSION_TO_EDIT: &str = "admin";
pub const MOBILE_PHONE_ATTR_NAME: &str = "sequent.read-only.mobile-number";
pub const FIRST_NAME: &str = "firstName";
pub const LAST_NAME: &str = "lastName";
pub const PERMISSION_LABELS: &str = "permission_labels";
pub const REALM_ATTR_VOTER_CERTIFICATE_POLICY: &str =
    "voter-certificate-policy";
pub const REALM_ATTR_CREDENTIAL_INPUT_POLICY: &str = "credential-input-policy";
pub const REALM_ATTR_CREDENTIAL_INPUT_PATTERN: &str =
    "credential-input-pattern";
pub const MAX_CREDENTIAL_PATTERN_GROUPS: usize = 8;
pub const MAX_CREDENTIAL_PATTERN_GROUP_SIZE: usize = 12;
pub const MAX_CREDENTIAL_PATTERN_TOTAL_SIZE: usize = 64;
pub const CERTIFICATES_IDP_ALIAS: &str = "digital-certificates";

#[allow(non_camel_case_types)]
#[derive(
    Default,
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    EnumString,
    JsonSchema,
)]
pub enum CredentialInputPolicy {
    #[default]
    #[strum(serialize = "standard")]
    #[serde(rename = "standard")]
    STANDARD,
    #[strum(serialize = "structured")]
    #[serde(rename = "structured")]
    STRUCTURED,
}

/// Default client ID used by the IVR for system-level interactions.
pub const DEFAULT_IVR_SERVICE_CLIENT_ID: &str = "ivr-service";
/// Client ID used by the IVR for voting.
///  Can't be changed as `authorize_voter_election` depends on its fixed value.
pub const IVR_VOTING_CLIENT_ID: &str = "ivr-voting";

/// Smart Link (external HMAC SSO) realm attributes.
///
/// These configure the Keycloak `election` realm resource that validates
/// externally generated HMAC auth-tokens. The attribute names MUST stay in sync
/// with the constants in the Keycloak extension `HmacSmartLink.java`.
///
/// Smart Link is disabled unless `smart-link-enabled` is `true`. When enabled,
/// the shared secret is the symmetric key the external application uses to sign
/// auth-tokens; it must match the value configured on the external side.
pub const REALM_ATTR_SMARTLINK_ENABLED: &str = "smart-link-enabled";
pub const REALM_ATTR_SMARTLINK_SHARED_SECRET: &str = "smart-link-shared-secret";
/// Seconds an auth-token stays valid after its embedded timestamp (default 90).
pub const REALM_ATTR_SMARTLINK_TIMEOUT_SECS: &str = "smart-link-timeout-secs";
/// Tolerance, in seconds, for the token being slightly ahead of Keycloak's
/// clock (default 5). Future-dated tokens beyond this are rejected.
pub const REALM_ATTR_SMARTLINK_CLOCK_SKEW_SECS: &str =
    "smart-link-clock-skew-secs";
/// OIDC client the voter is logged into (default `voting-portal`).
pub const REALM_ATTR_SMARTLINK_CLIENT_ID: &str = "smart-link-client-id";
/// Comma-separated request/user attributes that must match after HMAC validation.
pub const REALM_ATTR_SMARTLINK_REQUIRED_ATTRIBUTES: &str =
    "smart-link-required-attributes";

/// Maximum accepted length of the Smart Link shared secret.
pub const SMARTLINK_SHARED_SECRET_MAX_LEN: usize = 1000;
/// Maximum accepted length of the comma-separated Smart Link required attributes.
pub const SMARTLINK_REQUIRED_ATTRIBUTES_MAX_LEN: usize = 1000;

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
