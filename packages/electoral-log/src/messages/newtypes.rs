// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use strand::hash::{Hash, HashWrapper};
use strum_macros::Display;

use crate::messages::statement::{StatementEventType, StatementLogType, StatementType};

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

/// An area id, the area half of a ballot box (VOTE-FREEZE).
#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct AreaIdString(pub String);

/// SHA-512 of a ballot box seal manifest (VOTE-FREEZE). Borsh is the raw 64
/// bytes, like every other hash newtype; JSON is lowercase hex, so the
/// readable message in a seal record shows the hash people compare.
#[derive(BorshSerialize, BorshDeserialize, Clone, PartialEq, Eq, Hash, Debug)]
pub struct SealHash(pub HashWrapper);

impl SealHash {
    pub fn new(hash: Hash) -> Self {
        SealHash(HashWrapper::new(hash))
    }

    pub fn to_inner(&self) -> Hash {
        self.0.clone().into_inner()
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.to_inner())
    }
}

impl Serialize for SealHash {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for SealHash {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = <String as Deserialize>::deserialize(deserializer)?;
        let bytes = hex::decode(&text).map_err(serde::de::Error::custom)?;
        let hash: Hash = bytes
            .try_into()
            .map_err(|_| serde::de::Error::custom("a seal hash is 64 bytes"))?;
        Ok(SealHash::new(hash))
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

/// The version of an election event's enrollment approval matrix.
#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct ApprovalMatrixVersion(pub u32);

/// Lowercase hex SHA-256 of an approval matrix's JSON, which binds the
/// entry to the saved version.
#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct ApprovalMatrixDigestString(pub String);

/// A monitoring configuration document's kind, as sequent-core names it:
/// `widget`, `dashboard`, `theme` or `settings`.
#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct MonitoringConfigKindString(pub String);

/// The key a monitoring configuration document is stored under: its id.
#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct MonitoringConfigKeyString(pub String);

/// Lowercase hex SHA-256 of a monitoring configuration document's UTF-8
/// text, which binds the entry to the stored revision.
#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct MonitoringConfigDigestString(pub String);

#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct MonitoringPresetIdString(pub String);

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Deserialize,
    Serialize,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Debug,
    Display,
)]
pub enum MonitoringConfigChangeAction {
    Upsert,
    Delete,
}

/// Who wrote a monitoring configuration change: an administrator in the
/// editor, or a reset to a preset.
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Deserialize,
    Serialize,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Debug,
    Display,
)]
pub enum MonitoringConfigOrigin {
    Editor,
    Preset,
}

/// Which Dashboard tab an election event shows.
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Deserialize,
    Serialize,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Debug,
    Display,
)]
pub enum MonitoringDashboardMode {
    /// The standard dashboard.
    Legacy,
    /// The configured monitoring dashboards.
    Configured,
}

#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct MonitoringPresetRef {
    pub id: MonitoringPresetIdString,
    pub version: u32,
}

/// One stored revision of a monitoring configuration document.
#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct MonitoringConfigRevisionRef {
    pub kind: MonitoringConfigKindString,
    pub key: MonitoringConfigKeyString,
    pub revision: u32,
    pub action: MonitoringConfigChangeAction,
    /// Absent exactly when the revision removes the document.
    pub digest: Option<MonitoringConfigDigestString>,
}

/// What happened to a signed configuration package.
#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub enum ConfigurationPackageAction {
    /// Verified and imported into the election event.
    Imported,
}

/// One ballot design a configuration package approved.
#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct ConfigurationDesignDigest {
    pub area: String,
    pub election: String,
    pub sha256: String,
}

/// A signed configuration package's revision and manifest digest, and the
/// digest of each design it approved.
#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct ConfigurationPackageDetails {
    pub action: ConfigurationPackageAction,
    pub external_id: String,
    pub revision: u64,
    pub manifest_sha256: String,
    pub design_digests: Vec<ConfigurationDesignDigest>,
}

/// The hash manifest a report's generation wrote, and the signed
/// configuration it names.
#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct ReportGeneratedDetails {
    pub report_type: String,
    /// The stored `report-manifest.json`. A tally keeps the manifests of
    /// its folders beside their files instead, and names none.
    pub document_id: Option<String>,
    pub report_manifest_sha256: String,
    pub external_id: String,
    pub revision: u64,
    pub manifest_sha256: String,
}

/// The signed configuration whose ballots a publication published: its
/// revision and manifest digest, and the digest of each design published.
#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct PublishedConfiguration {
    pub external_id: String,
    pub revision: u64,
    pub manifest_sha256: String,
    pub design_digests: Vec<ConfigurationDesignDigest>,
}

/// One change to an election event's monitoring configuration: a save, a
/// reset to a preset, or a switch of the Dashboard tab's mode.
#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct MonitoringConfigChangeDetails {
    pub origin: MonitoringConfigOrigin,
    /// The preset a reset wrote; absent for an editor change.
    pub preset: Option<MonitoringPresetRef>,
    /// The mode after the change.
    pub mode: MonitoringDashboardMode,
    /// The event's configuration generation after the change. Every change
    /// raises it by one, so gaps and reorderings in the log show.
    pub generation: u64,
    /// The revisions written, in the order they were stored.
    pub revisions: Vec<MonitoringConfigRevisionRef>,
}

/// The step of signing a protected action that an entry records. Each kind
/// is also a [`StatementType`] of the same name.
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Deserialize,
    Serialize,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Debug,
    Display,
)]
pub enum SigningStatementKind {
    SigningRequestCreated,
    SigningCertificateOpenFailed,
    SigningRequestSigned,
    SigningSignatureRefused,
    SigningCertificateRegistered,
    SigningHandover,
    SigningRequestCancelled,
    SigningRequestExpired,
    SigningRequestCompleted,
    SigningActionExecuted,
    SigningRuleChanged,
    SigningPermissionChanged,
    SigningIssuerChanged,
    SigningChecksChanged,
    SigningCertificateRevoked,
    SigningRequestsExported,
    /// A scheduled lifecycle window (readiness test, final testing, test
    /// voting) opened or closed for a Post (VOTE-LIFECYCLE). Not a signing
    /// step: it uses the same transactional outbox.
    LifecycleWindowChanged,
    /// Scheduled instants recomputed after a tz database update were
    /// applied (VOTE-LIFECYCLE).
    ScheduleRecomputeApplied,
    /// An admin imported the schedule from a CSV file (VOTE-LIFECYCLE).
    /// Not a signing step, but staged through the same outbox so the entry
    /// commits with the import.
    ScheduleImported,
    /// A write changed what a scheduled opening or closing of voting will do
    /// (VOTE-LIFECYCLE): before and after, each with its explanation.
    ScheduledOutcomeChanged,
    /// A country of a Post (or a Post without countries) was initialized
    /// (VOTE-LIFECYCLE). Staged through the same outbox as the signing
    /// steps, so it is posted once, in order.
    ElectionInitialized,
    /// An election event was locked down, or its lockdown lifted
    /// (VOTE-LIFECYCLE). Staged through the same outbox as the signing
    /// steps, so it is posted once, in order.
    LockdownChanged,
}

impl SigningStatementKind {
    pub fn statement_type(&self) -> StatementType {
        match self {
            Self::SigningRequestCreated => StatementType::SigningRequestCreated,
            Self::SigningCertificateOpenFailed => StatementType::SigningCertificateOpenFailed,
            Self::SigningRequestSigned => StatementType::SigningRequestSigned,
            Self::SigningSignatureRefused => StatementType::SigningSignatureRefused,
            Self::SigningCertificateRegistered => StatementType::SigningCertificateRegistered,
            Self::SigningHandover => StatementType::SigningHandover,
            Self::SigningRequestCancelled => StatementType::SigningRequestCancelled,
            Self::SigningRequestExpired => StatementType::SigningRequestExpired,
            Self::SigningRequestCompleted => StatementType::SigningRequestCompleted,
            Self::SigningActionExecuted => StatementType::SigningActionExecuted,
            Self::SigningRuleChanged => StatementType::SigningRuleChanged,
            Self::SigningPermissionChanged => StatementType::SigningPermissionChanged,
            Self::SigningIssuerChanged => StatementType::SigningIssuerChanged,
            Self::SigningChecksChanged => StatementType::SigningChecksChanged,
            Self::SigningCertificateRevoked => StatementType::SigningCertificateRevoked,
            Self::SigningRequestsExported => StatementType::SigningRequestsExported,
            Self::LifecycleWindowChanged => StatementType::LifecycleWindowChanged,
            Self::ScheduleRecomputeApplied => StatementType::ScheduleRecomputeApplied,
            Self::ScheduleImported => StatementType::ScheduleImported,
            Self::ScheduledOutcomeChanged => StatementType::ScheduledOutcomeChanged,
            Self::ElectionInitialized => StatementType::ElectionInitialized,
            Self::LockdownChanged => StatementType::LockdownChanged,
        }
    }
}

/// One entry of a signing step. Every step writes two entries of the same
/// kind: USER, attributed to the person who took it, and SYSTEM (ERROR for
/// a failure or a refusal), with what the system checked or did. Unlike
/// every other body, the caller sets the head's event type, log type and
/// description.
#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct SigningLogEntry {
    pub kind: SigningStatementKind,
    pub event_type: StatementEventType,
    pub log_type: StatementLogType,
    /// A short English sentence for the Logs tab's Description column, such
    /// as "Started signing request 7F3A-91C2".
    pub description: String,
    /// The step's details as JSON: the Post and country, the action, the
    /// request and its code, and for signatures the certificate and the
    /// signature.
    pub details_json: String,
    /// The step's id (`signing_log_outbox.step_id`, a lowercase hyphenated
    /// UUID), shared by its USER and SYSTEM entries. It links the pair on the
    /// board, and the worker dedupes on (step_id, event_type) before posting,
    /// so a retry never posts an entry twice.
    pub step_id: String,
}
