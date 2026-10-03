// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Domain types of signing. Every enum travels as its kebab-case value (serde
//! and strum agree), and the admin portal mirrors them in
//! `src/lib/signing/types.ts`.

use crate::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use std::fmt;
use strum_macros::{Display, EnumIter, EnumString};

/// Derives shared by every signing enum.
macro_rules! signing_enum {
    ($(#[$meta:meta])* pub enum $name:ident { $($(#[$vmeta:meta])* $variant:ident),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
            Display, EnumString, EnumIter,
        )]
        #[serde(rename_all = "kebab-case")]
        #[strum(serialize_all = "kebab-case")]
        pub enum $name { $($(#[$vmeta])* $variant),+ }
    };
}

signing_enum! {
    /// The catalog of protected actions. It is product code, because each
    /// action has its own integration point; whether an action needs
    /// signatures is configuration ([`SigningRule`]). The id is the label key
    /// `signing.actions.<id>` and the suffix of its `sign-<id>` permission.
    pub enum SigningAction {
        InitializeVoting,
        OpenVoting,
        CloseVoting,
        GenerateElectionReturns,
        GenerateReports,
        TransmitResults,
        ApproveVoter,
        ApproveConfiguration,
        #[serde(rename = "key-ceremony")]
        #[strum(serialize = "key-ceremony")]
        ConfirmKeyShare,
        #[serde(rename = "tally-key")]
        #[strum(serialize = "tally-key")]
        ContributeKeyShare,
    }
}

signing_enum! {
    /// What a request of an action is bound to. Post = `election_id`,
    /// country = `area_id`.
    pub enum SigningScope {
        Post,
        PostAndCountry,
        Event,
        Trustee,
    }
}

signing_enum! {
    /// When the action runs. `Deferred`: the server runs it once, when the
    /// last required signature arrives. `Gate`: the signer's own route runs
    /// it afterwards, checking the completed request (trustee steps).
    pub enum ExecutionMode {
        Deferred,
        Gate,
    }
}

signing_enum! {
    /// The document an approval also signs, besides the canonical payload.
    pub enum DocumentKind {
        NoDocument,
        Pdf,
        Eml,
    }
}

signing_enum! {
    /// How the Protected actions sub-tab groups the actions.
    pub enum SigningActionGroup {
        Voting,
        ResultsAndReports,
        Enrollment,
        ConfigurationAndKeys,
    }
}

impl SigningAction {
    /// The permission that lets a person sign this action.
    pub fn sign_permission(&self) -> Permissions {
        match self {
            SigningAction::InitializeVoting => {
                Permissions::SIGN_INITIALIZE_VOTING
            }
            SigningAction::OpenVoting => Permissions::SIGN_OPEN_VOTING,
            SigningAction::CloseVoting => Permissions::SIGN_CLOSE_VOTING,
            SigningAction::GenerateElectionReturns => {
                Permissions::SIGN_GENERATE_ELECTION_RETURNS
            }
            SigningAction::GenerateReports => {
                Permissions::SIGN_GENERATE_REPORTS
            }
            SigningAction::TransmitResults => {
                Permissions::SIGN_TRANSMIT_RESULTS
            }
            SigningAction::ApproveVoter => Permissions::SIGN_APPROVE_VOTER,
            SigningAction::ApproveConfiguration => {
                Permissions::SIGN_APPROVE_CONFIGURATION
            }
            SigningAction::ConfirmKeyShare => Permissions::SIGN_KEY_CEREMONY,
            SigningAction::ContributeKeyShare => Permissions::SIGN_TALLY_KEY,
        }
    }

    pub fn scope(&self) -> SigningScope {
        match self {
            SigningAction::InitializeVoting
            | SigningAction::OpenVoting
            | SigningAction::CloseVoting
            | SigningAction::GenerateReports
            | SigningAction::ApproveVoter => SigningScope::Post,
            SigningAction::GenerateElectionReturns
            | SigningAction::TransmitResults => SigningScope::PostAndCountry,
            SigningAction::ApproveConfiguration => SigningScope::Event,
            SigningAction::ConfirmKeyShare
            | SigningAction::ContributeKeyShare => SigningScope::Trustee,
        }
    }

    pub fn mode(&self) -> ExecutionMode {
        if self.is_trustee() {
            ExecutionMode::Gate
        } else {
            ExecutionMode::Deferred
        }
    }

    pub fn document(&self) -> DocumentKind {
        match self {
            SigningAction::GenerateElectionReturns
            | SigningAction::GenerateReports => DocumentKind::Pdf,
            SigningAction::TransmitResults => DocumentKind::Eml,
            _ => DocumentKind::NoDocument,
        }
    }

    /// Trustee steps: each trustee signs their own step, once.
    pub fn is_trustee(&self) -> bool {
        self.scope() == SigningScope::Trustee
    }

    pub fn group(&self) -> SigningActionGroup {
        match self {
            SigningAction::InitializeVoting
            | SigningAction::OpenVoting
            | SigningAction::CloseVoting => SigningActionGroup::Voting,
            SigningAction::GenerateElectionReturns
            | SigningAction::GenerateReports
            | SigningAction::TransmitResults => {
                SigningActionGroup::ResultsAndReports
            }
            SigningAction::ApproveVoter => SigningActionGroup::Enrollment,
            SigningAction::ApproveConfiguration
            | SigningAction::ConfirmKeyShare
            | SigningAction::ContributeKeyShare => {
                SigningActionGroup::ConfigurationAndKeys
            }
        }
    }
}

signing_enum! {
    /// "Needs signatures".
    pub enum SigningRequirement {
        NotRequired,
        Required,
    }
}

signing_enum! {
    /// "The person who starts it can also sign".
    pub enum RequesterSigning {
        Allowed,
        NotAllowed,
    }
}

signing_enum! {
    pub enum SigningRequestStatus {
        Waiting,
        Completed,
        Executed,
        Cancelled,
        Expired,
        Failed,
    }
}

signing_enum! {
    pub enum CancelReason {
        ByRequester,
        ByOperator,
        RuleChanged,
        PayloadChanged,
        Superseded,
        /// A Security Officer revoked the key of one of its signatures.
        CertificateRevoked,
    }
}

signing_enum! {
    /// Whether signatures are checked against downloaded revocation lists.
    pub enum RevocationCheck {
        Check,
        DontCheck,
    }
}

signing_enum! {
    /// What to do when a revocation list can't be downloaded.
    pub enum CrlUnavailablePolicy {
        Refuse,
        AcceptUnchecked,
    }
}

signing_enum! {
    /// Who may bind a certificate to a person.
    pub enum CertificateRegistration {
        OnFirstUse,
        SecurityOfficerOnly,
    }
}

signing_enum! {
    /// Whether a certificate signs for one Post only (the 2025 rule).
    pub enum CertificatePostBinding {
        OnePost,
        AnyPost,
    }
}

signing_enum! {
    pub enum StaffCertificateStatus {
        Active,
        Revoked,
    }
}

signing_enum! {
    /// How a staff certificate was registered (`staff_certificate.registration`).
    pub enum StaffCertificateRegistration {
        FirstUse,
        SecurityOfficer,
    }
}

signing_enum! {
    /// What a certificate authority is trusted for. Voter sign-in is the
    /// purpose of every authority that existed before staff signatures.
    #[derive(Default)]
    pub enum CertificateAuthorityPurpose {
        #[default]
        VoterSignIn,
        StaffSignatures,
    }
}

signing_enum! {
    pub enum SignatureAlgorithm {
        #[serde(rename = "rsa-pkcs1-sha256")]
        #[strum(serialize = "rsa-pkcs1-sha256")]
        RsaPkcs1Sha256,
        #[serde(rename = "ecdsa-p256-sha256")]
        #[strum(serialize = "ecdsa-p256-sha256")]
        EcdsaP256Sha256,
    }
}

signing_enum! {
    /// Whether an approval's certificate was checked against a current
    /// revocation list (`signing_approval.revocation_status`).
    pub enum RevocationStatus {
        Checked,
        Unchecked,
    }
}

signing_enum! {
    /// A PAdES revision of a signed document
    /// (`signing_document_revision.state`).
    pub enum DocumentRevisionState {
        Base,
        Prepared,
        Signed,
    }
}

signing_enum! {
    /// The last download of a revocation list (`staff_crl.status`).
    pub enum CrlStatus {
        Ok,
        Unavailable,
    }
}

signing_enum! {
    /// Why a certificate file didn't open in the browser. Logged without
    /// the file or its password.
    pub enum CertificateOpenFailure {
        WrongPassword,
        Unreadable,
        NoKey,
    }
}

signing_enum! {
    /// The server's certificate checks, in the order the dialog lists them.
    pub enum CertificateCheckId {
        TrustedIssuer,
        ValidNow,
        SigningKeyUsage,
        NotRevoked,
        Registered,
        RegisteredToOther,
        AlreadySigned,
        PostBinding,
        Signature,
    }
}

/// The result of one certificate check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CertificateCheckResult {
    pub id: CertificateCheckId,
    pub ok: bool,
    pub detail: Option<String>,
}

/// How an election event signs one action. A missing row means
/// [`SigningRule::default_for`], which keeps today's behaviour.
///
/// The fields are stored as configured. Read the count and the requester
/// policy through [`SigningRule::required`] and
/// [`SigningRule::requester_signing_effective`], which apply the trustee
/// override.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SigningRule {
    pub action: SigningAction,
    pub requirement: SigningRequirement,
    /// How many different people must sign, as configured.
    pub signatures: u16,
    pub requester_signing: RequesterSigning,
    /// `None` (an explicit null): requests never expire. Absent in JSON:
    /// [`DEFAULT_EXPIRES_MINUTES`].
    #[serde(default = "default_expires_minutes")]
    pub expires_minutes: Option<u32>,
    #[serde(default)]
    pub revision: i64,
}

fn default_expires_minutes() -> Option<u32> {
    Some(DEFAULT_EXPIRES_MINUTES)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigningRuleError {
    /// "At least 1."
    NoSignatures,
    /// An expiry must be at least one minute; `None` means no limit.
    ZeroExpiry,
}

impl fmt::Display for SigningRuleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SigningRuleError::NoSignatures => {
                write!(f, "a rule needs at least one signature")
            }
            SigningRuleError::ZeroExpiry => {
                write!(f, "a request expiry must be at least one minute")
            }
        }
    }
}

impl std::error::Error for SigningRuleError {}

/// The expiry of a rule nobody has configured.
pub const DEFAULT_EXPIRES_MINUTES: u32 = 60;

/// The most signatures a rule may need, wherever a rule is checked: the
/// Signatures tab, the server and an imported bundle.
pub const MAX_SIGNATURES: u16 = 100;

/// The longest expiry a rule may set: a year. `None` is no limit.
pub const MAX_EXPIRES_MINUTES: u32 = 525_600;

impl SigningRule {
    pub fn default_for(action: SigningAction) -> Self {
        SigningRule {
            action,
            requirement: SigningRequirement::NotRequired,
            signatures: 1,
            requester_signing: RequesterSigning::NotAllowed,
            expires_minutes: Some(DEFAULT_EXPIRES_MINUTES),
            revision: 0,
        }
    }

    pub fn is_required(&self) -> bool {
        self.requirement == SigningRequirement::Required
    }

    /// How many different people must sign. A trustee signs their own
    /// step once, so a trustee action needs one signature.
    pub fn required(&self) -> u16 {
        if self.action.is_trustee() {
            1
        } else {
            self.signatures
        }
    }

    /// Whether the person who started a request may sign it. A trustee's
    /// step is signed by the trustee who started it.
    pub fn requester_signing_effective(&self) -> RequesterSigning {
        if self.action.is_trustee() {
            RequesterSigning::Allowed
        } else {
            self.requester_signing
        }
    }

    /// Checks what doesn't depend on the signers; capacity per Post is
    /// checked by the server against the Keycloak signers.
    pub fn validate(&self) -> Result<(), SigningRuleError> {
        // The stored count, not required(): a trustee rule still stores
        // one, and the database refuses fewer for every rule.
        if self.signatures < 1 {
            return Err(SigningRuleError::NoSignatures);
        }
        if self.expires_minutes == Some(0) {
            return Err(SigningRuleError::ZeroExpiry);
        }
        Ok(())
    }
}

/// The event's certificate checks (the Certificates sub-tab).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SigningChecks {
    pub revocation_check: RevocationCheck,
    pub crl_unavailable: CrlUnavailablePolicy,
    pub registration: CertificateRegistration,
    pub post_binding: CertificatePostBinding,
    pub revision: i64,
}

impl Default for SigningChecks {
    /// The 2025 behaviour.
    fn default() -> Self {
        SigningChecks {
            revocation_check: RevocationCheck::Check,
            crl_unavailable: CrlUnavailablePolicy::Refuse,
            registration: CertificateRegistration::OnFirstUse,
            post_binding: CertificatePostBinding::OnePost,
            revision: 0,
        }
    }
}

// Subjects: the action-specific part of the canonical payload.

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InitializeVotingSubject {
    pub publication_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenVotingSubject {
    pub channel: String,
}

/// Sorts and deduplicates a list field, so equal sets canonicalize equally.
fn sorted_set(mut items: Vec<String>) -> Vec<String> {
    items.sort();
    items.dedup();
    items
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "CloseVotingSubjectWire")]
pub struct CloseVotingSubject {
    channels: Vec<String>,
}

#[derive(Deserialize)]
struct CloseVotingSubjectWire {
    channels: Vec<String>,
}

impl From<CloseVotingSubjectWire> for CloseVotingSubject {
    fn from(wire: CloseVotingSubjectWire) -> Self {
        CloseVotingSubject::new(wire.channels)
    }
}

impl CloseVotingSubject {
    pub fn new(channels: Vec<String>) -> Self {
        CloseVotingSubject {
            channels: sorted_set(channels),
        }
    }

    /// Sorted, without duplicates.
    pub fn channels(&self) -> &[String] {
        &self.channels
    }
}

/// Election returns and other reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentSubject {
    pub report_type: String,
    pub document_sha256: String,
    pub template_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "TransmitResultsSubjectWire")]
pub struct TransmitResultsSubject {
    pub tally_session_id: String,
    pub package_sha256: String,
    pub eml_sha256: String,
    destinations: Vec<String>,
}

#[derive(Deserialize)]
struct TransmitResultsSubjectWire {
    tally_session_id: String,
    package_sha256: String,
    eml_sha256: String,
    destinations: Vec<String>,
}

impl From<TransmitResultsSubjectWire> for TransmitResultsSubject {
    fn from(wire: TransmitResultsSubjectWire) -> Self {
        TransmitResultsSubject::new(
            wire.tally_session_id,
            wire.package_sha256,
            wire.eml_sha256,
            wire.destinations,
        )
    }
}

impl TransmitResultsSubject {
    pub fn new(
        tally_session_id: String,
        package_sha256: String,
        eml_sha256: String,
        destinations: Vec<String>,
    ) -> Self {
        TransmitResultsSubject {
            tally_session_id,
            package_sha256,
            eml_sha256,
            destinations: sorted_set(destinations),
        }
    }

    /// Server ids, sorted, without duplicates.
    pub fn destinations(&self) -> &[String] {
        &self.destinations
    }
}

signing_enum! {
    pub enum ApplicationDecision {
        Approve,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApproveVoterSubject {
    pub application_id: String,
    pub applicant_registry_id: String,
    pub decision: ApplicationDecision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApproveConfigurationSubject {
    pub ballot_publication_id: String,
    /// SHA-256 of the canonical publication diff.
    pub digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyCeremonySubject {
    pub keys_ceremony_id: String,
    pub trustee_id: String,
    pub key_share_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TallyKeySubject {
    pub tally_session_id: String,
    pub trustee_id: String,
    pub key_share_sha256: String,
}

#[cfg(test)]
#[path = "types_tests.rs"]
mod types_tests;
