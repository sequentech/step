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
