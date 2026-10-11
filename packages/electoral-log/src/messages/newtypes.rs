// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use strand::hash::{Hash, HashWrapper};
use strum_macros::Display;

#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct EventIdString(pub String);

#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct ElectionsIdsString(pub Option<Vec<String>>);

#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct ElectionIdString(pub Option<String>);
#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct ErrorMessageString(pub String);

#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct KeycloakEventTypeString(pub String);

#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct ContestIdString(pub String);

#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct TrusteeNameString(pub String);

#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct BallotPublicationIdString(pub String);

#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct CastVoteErrorString(pub String);

#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct PseudonymHash(pub HashWrapper);
#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct PublicKeyDerB64(pub String);

#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct TenantIdString(pub String);

#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct AdminUserIdString(pub String);

impl PseudonymHash {
    // Provide methods to work with HashWrapper as needed
    pub fn new(hash: Hash) -> Self {
        PseudonymHash(HashWrapper::new(hash))
    }
}

#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct CastVoteHash(pub HashWrapper);

impl CastVoteHash {
    // Provide methods to work with HashWrapper as needed
    pub fn new(hash: Hash) -> Self {
        CastVoteHash(HashWrapper::new(hash))
    }
}

pub type Timestamp = u64;

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Deserialize,
    Serialize,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Debug,
    Display,
)]
pub enum CertificateAuthEventAction {
    Import,
    Delete,
}

#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct CertificateSubjectDnsString(pub Vec<String>);

#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct VoterIpString(pub String);

#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct VoterCountryString(pub String);

#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct VotingChannelString(pub String);

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Deserialize,
    Serialize,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Debug,
    Display,
)]
pub enum ExtApiRequestDirection {
    Inbound,
    Outbound,
}

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Deserialize,
    Serialize,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Debug,
    Display,
)]
pub enum ExtApiName {
    Datafix,
    Other,
}

/// Subject of an external API request, bound into the signed statement. Both
/// fields are optional because some operations (e.g. adding a voter) act before
/// a Keycloak user id exists or without a resolvable username.
#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct ExternalApiSubject {
    pub user_id: Option<String>,
    pub username: Option<String>,
}

/// Which phase of a third-party voter registry reconciliation run a
/// `StatementBody::ExternalReconciliation` entry records — see
/// `external_reconciliation_message`. Named for the general capability (any
/// external voter registry this system reconciles against), not the specific
/// integration (Datafix) that first needed it.
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Deserialize,
    Serialize,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Debug,
    Display,
)]
pub enum ExternalReconciliationKind {
    /// The external-side diff was computed and (if non-empty) the
    /// downloadable patch was generated and uploaded.
    PatchGenerated,
    /// The Sequent-side diff was applied directly to this system.
    ChangesApplied,
}

/// Decimal `Sequence` of the reconciliation file an `ExternalReconciliation`
/// entry was generated from or applied — kept as a string, like every other
/// *String newtype in this module, rather than a bare integer.
#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct ExternalReconciliationSequenceString(pub String);

/// Decimal Unix-seconds `GeneratedAt` of the reconciliation or patch file.
#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct ExternalReconciliationGeneratedAtString(pub String);

/// SHA-256 of the reconciliation file that produced this run.
#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct ExternalReconciliationInputHashString(pub String);

/// SHA-256 of the generated external-side patch — `None` for a
/// `ChangesApplied` entry (there is no patch file for Sequent's own side, or for a
/// `PatchGenerated` entry whose external-side diff was empty.
#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct ExternalReconciliationOutputHashString(pub Option<String>);

#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct ResolutionIdsString(pub Vec<String>);

#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct PhoneE164String(pub String);

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Deserialize,
    Serialize,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Debug,
    Display,
)]
pub enum PhoneBlacklistAction {
    CreateEntry,
    DeleteEntry,
}

#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct ResultsPublicationIdString(pub String);

#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct ResultsPublicationRouteScopeString(pub String);

#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct ResultsPublicationAccessString(pub String);

#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct ResultsPublicationVisibilityScopeString(pub String);

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Deserialize,
    Serialize,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Debug,
    Display,
)]
pub enum ResultsPublicationAction {
    Publish,
    Revoke,
}

#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct ResultsPublicationDetails {
    pub publication_id: ResultsPublicationIdString,
    pub action: ResultsPublicationAction,
    pub route_scope: ResultsPublicationRouteScopeString,
    pub route_election_id: ElectionIdString,
    pub access: ResultsPublicationAccessString,
    pub visibility_scope: ResultsPublicationVisibilityScopeString,
    pub contest_ids: Vec<ContestIdString>,
}

#[derive(BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, Debug)]
pub enum BallotPublicationStage {
    Generate,
    Publish,
}

#[derive(BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, Debug)]
pub struct BallotPublicationFailure {
    pub publication_id: BallotPublicationIdString,
    pub task_id: String,
    pub stage: BallotPublicationStage,
    pub error: ErrorMessageString,
}

/// Why an electoral-log checkpoint was published. Its name is part of the signed
/// checkpoint and of the stored publication.
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Deserialize,
    Serialize,
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Display,
    strum_macros::EnumString,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum ElectoralLogCheckpointReason {
    VotingClosed,
    TallyCompleted,
    // New reasons go last: signed checkpoint messages store the variant's index.
    VotingOpened,
    /// Published at a fixed interval while voting is open.
    Periodic,
}

/// A published checkpoint of the board's Merkle log, recorded in the log itself, as
/// the first log format named the log: by its key in one database. Readers still
/// decode it; writers record `ElectoralLogCheckpointV2`.
#[derive(BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct ElectoralLogCheckpoint {
    /// Key of the Trellis log in the database that stored it.
    pub log_id: i64,
    /// Entries covered by the root.
    pub tree_size: u64,
    /// Hex-encoded SHA-256 root.
    pub root: String,
    pub reason: ElectoralLogCheckpointReason,
}

/// A published checkpoint of the board's Merkle log, recorded in the log itself.
#[derive(BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct ElectoralLogCheckpointV2 {
    /// Identity of the Trellis log, as a hyphenated UUID.
    pub log_uid: String,
    /// Entries covered by the root.
    pub tree_size: u64,
    /// Hex-encoded SHA-256 root.
    pub root: String,
    pub reason: ElectoralLogCheckpointReason,
}

/// The first record of a log that continues another one, as an imported election
/// event's log continues the log of the event it was exported from. It commits to the
/// last checkpoint of the previous log, which is kept, sealed, in the same database.
#[derive(BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct ElectoralLogContinuation {
    /// Board name of the previous log.
    pub previous_log_name: String,
    /// Identity of the previous log, as a hyphenated UUID.
    pub previous_log_uid: String,
    /// Entries of the previous log.
    pub tree_size: u64,
    /// Hex-encoded SHA-256 root of the previous log.
    pub root: String,
}
