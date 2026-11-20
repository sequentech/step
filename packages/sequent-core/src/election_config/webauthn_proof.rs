// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Browser FIDO2 (WebAuthn) approvals and final signatures, checked offline.
//!
//! A configuration revision is approved by officials, each with a hardware
//! security key in a browser, and then sealed by a separate final signer.
//! Each signature is a WebAuthn assertion whose challenge is derived from
//! exactly what it is for (see [`Binding`]); a [`Bundle`] carries them, and
//! [`verify_bundle`] checks it against a [`Roster`] of the officials' keys
//! that the verifier loads from its own provisioning, never from the bundle.
//!
//! What is checked, for every assertion: the binding (action, relying party,
//! tenant, signer, configuration, revision, content), the challenge it gives,
//! the client data (type, challenge, an origin the roster allows, no cross
//! origin), the authenticator data (the relying party's hash; user presence
//! and user verification; no backup eligibility or state), and the signature
//! over both with the roster's key (ES256, verified by `ring`). Then, for the
//! bundle: enough approvals by distinct people with distinct approver keys,
//! and a final signature by a final signer who approved nothing, over the
//! statement naming exactly those approvals.
//!
//! Nothing here trusts what a bundle says about who may sign: the roster
//! does. Generic: no installation's settings are built in.
//!
//! A package is admitted only through [`PinnedRoster`] (a roster and
//! threshold checked, pinned and bound to one tenant, with private fields)
//! and [`verify_bundle_for_package`], whose final signature covers the
//! package's whole manifest ([`package_seal_sha256`]); see
//! `super::webauthn_package`. [`verify_bundle`] and its statement remain for
//! the service's own checks while it records signatures.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use ring::digest::{digest, SHA256};
use ring::signature::{UnparsedPublicKey, ECDSA_P256_SHA256_ASN1};
use serde::{Deserialize, Serialize};

/// A bundle of this format, and only this one, is read.
pub const BUNDLE_FORMAT: &str = "sequent.webauthn-bundle.v1";
/// A roster of this format, and only this one, is read.
pub const ROSTER_FORMAT: &str = "sequent.webauthn-roster.v1";

/// Domain tags, shared with the service that issues the challenges.
pub const BINDING_DOMAIN: &[u8] = b"sequent.architect.webauthn.binding.v1\0";
pub const CHALLENGE_DOMAIN: &[u8] =
    b"sequent.architect.webauthn.challenge.v1\0";
pub const PROOF_DOMAIN: &[u8] = b"sequent.architect.webauthn.proof.v1\0";
pub const STATEMENT_DOMAIN: &[u8] =
    b"sequent.architect.webauthn.statement.v1\0";

/// Authenticator data flags (WebAuthn §6.1).
const USER_PRESENT: u8 = 0x01;
const USER_VERIFIED: u8 = 0x04;
const BACKUP_ELIGIBLE: u8 = 0x08;
const BACKUP_STATE: u8 = 0x10;
const ATTESTED_DATA: u8 = 0x40;
const EXTENSION_DATA: u8 = 0x80;
/// rpIdHash, flags and the signature counter.
const AUTHENTICATOR_DATA_LENGTH: usize = 37;

/// Why a bundle, a proof or a roster is refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProofError {
    Format(&'static str),
    Roster(String),
    Encoding(&'static str),
    Binding(&'static str),
    UnknownKey(String),
    WrongKey(String),
    ClientData(String),
    AuthenticatorData(&'static str),
    Signature,
    TooFewApprovals {
        needed: usize,
        given: usize,
    },
    Duplicate(String),
    SignerApproved,
    /// The pinned policy itself is not one a verifier may use.
    Policy(String),
}

impl std::fmt::Display for ProofError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProofError::Format(format) => write!(f, "not a {format} document"),
            ProofError::Roster(why) => write!(f, "the roster is not usable: {why}"),
            ProofError::Encoding(what) => write!(f, "{what} is not base64url"),
            ProofError::Binding(what) => write!(f, "the binding is not for this {what}"),
            ProofError::UnknownKey(key) => write!(f, "key {key} is not on the roster"),
            ProofError::WrongKey(key) => {
                write!(f, "key {key} is not this person's, this credential or for this role")
            }
            ProofError::ClientData(why) => write!(
                f,
                "the client data is not a WebAuthn assertion for this challenge and origin: {why}"
            ),
            ProofError::AuthenticatorData(why) => {
                write!(f, "the authenticator data is not acceptable: {why}")
            }
            ProofError::Signature => write!(f, "the signature does not verify"),
            ProofError::TooFewApprovals { needed, given } => {
                write!(f, "{needed} approvals by distinct approvers are needed; {given} given")
            }
            ProofError::Duplicate(who) => write!(f, "{who} approved twice"),
            ProofError::SignerApproved => write!(f, "the final signer approved this revision too"),
            ProofError::Policy(why) => write!(f, "the pinned signing policy is not usable: {why}"),
        }
    }
}

impl std::error::Error for ProofError {}

/// What an official's key is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Role {
    Approver,
    FinalSigner,
}

/// What a signature is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Action {
    ApproveRevision,
    SignPackage,
}

impl Action {
    fn role(self) -> Role {
        match self {
            Action::ApproveRevision => Role::Approver,
            Action::SignPackage => Role::FinalSigner,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Action::ApproveRevision => "approve-revision",
            Action::SignPackage => "sign-package",
        }
    }
}

/// One official's key, as provisioned.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RosterKey {
    /// The service's id for the key (a UUID): what a binding names.
    pub key: String,
    /// The WebAuthn credential id, base64url.
    pub credential_id: String,
    /// The realm's immutable user id.
    pub subject: String,
    pub role: Role,
    /// Only `es256`.
    pub algorithm: String,
    /// The P-256 public key, uncompressed SEC1 (65 bytes), base64url.
    pub public_key: String,
    /// The key's registration record digest, as its custodian approved it.
    pub record_sha256: String,
}

/// The officials' keys of one tenant at one relying party: the verifier's
/// own, provisioned apart from any bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Roster {
    pub format: String,
    pub rp_id: String,
    /// The exact origins the ceremonies may come from.
    pub origins: Vec<String>,
    pub tenant: String,
    pub keys: Vec<RosterKey>,
}

impl Roster {
    /// Reads and checks a roster: its format, one key per person, one person
    /// per key, ES256 keys of the right shape.
    pub fn read(text: &str) -> Result<Self, ProofError> {
        let roster: Roster = serde_json::from_str(text)
            .map_err(|error| ProofError::Roster(error.to_string()))?;
        if roster.format != ROSTER_FORMAT {
            return Err(ProofError::Format(ROSTER_FORMAT));
        }
        if roster.rp_id.is_empty()
            || roster.tenant.is_empty()
            || roster.origins.is_empty()
        {
            return Err(ProofError::Roster(
                "no relying party, tenant or origin".to_string(),
            ));
        }
        let mut seen_keys = std::collections::BTreeSet::new();
        let mut seen_subjects = std::collections::BTreeSet::new();
        let mut seen_credentials = std::collections::BTreeSet::new();
        let mut seen_public_keys = std::collections::BTreeSet::new();
        for key in &roster.keys {
            if key.algorithm != "es256" {
                return Err(ProofError::Roster(format!(
                    "key {} is not es256",
                    key.key
                )));
            }
            let public = decode(&key.public_key, "a roster public key")?;
            if public.len() != 65 || public[0] != 0x04 {
                return Err(ProofError::Roster(format!(
                    "key {} is not an uncompressed P-256 point",
                    key.key
                )));
            }
            // Compared as bytes: one physical key under two people, two ids
            // or two spellings of its credential is one key, never two.
            let credential =
                decode(&key.credential_id, "a roster credential id")?;
            if !seen_keys.insert(key.key.clone())
                || !seen_subjects.insert(key.subject.clone())
                || !seen_credentials.insert(credential)
                || !seen_public_keys.insert(public)
            {
                return Err(ProofError::Roster(format!(
                    "key {} or its person is listed twice",
                    key.key
                )));
            }
        }
        Ok(roster)
    }

    /// The roster's digest, for the verifier to pin.
    pub fn sha256(text: &str) -> String {
        hex::encode(digest(&SHA256, text.as_bytes()))
    }

    /// Reads a roster only if it is, byte for byte, the one the verifier was
    /// provisioned with: `pinned` is its SHA-256 as the verifier's own
    /// configuration records it, never taken from a package or a bundle.
    pub fn read_pinned(text: &str, pinned: &str) -> Result<Self, ProofError> {
        if Self::sha256(text) != pinned.to_ascii_lowercase() {
            return Err(ProofError::Roster(
                "not the roster this verifier was provisioned with".to_string(),
            ));
        }
        Self::read(text)
    }
}

/// Everything a signature is bound to. Its canonical form is the service's
/// challenge binding, byte for byte.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub rp_id: String,
    pub tenant: String,
    pub subject: String,
    pub action: Action,
    pub configuration: String,
    pub revision: i64,
    /// The revision's content digest (an approval), or the package
    /// statement's digest (the final signature).
    pub content_sha256: String,
    /// The roster key that signs.
    pub signer: String,
}

#[derive(Serialize)]
struct Canonical<'a> {
    v: u8,
    rp_id: &'a str,
    tenant: &'a str,
    subject: &'a str,
    action: &'a str,
    role: Option<&'a str>,
    target_credential: Option<&'a str>,
    target_record_sha256: Option<&'a str>,
    configuration: Option<&'a str>,
    revision: Option<i64>,
    content_sha256: Option<&'a str>,
    signer: Option<&'a str>,
}

fn tagged(domain: &[u8], parts: &[&[u8]]) -> [u8; 32] {
    let mut context = ring::digest::Context::new(&SHA256);
    context.update(domain);
    for part in parts {
        context.update(part);
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(context.finish().as_ref());
    out
}

impl Binding {
    pub fn canonical(&self) -> String {
        serde_json::to_string(&Canonical {
            v: 1,
            rp_id: &self.rp_id,
            tenant: &self.tenant,
            subject: &self.subject,
            action: self.action.name(),
            role: None,
            target_credential: None,
            target_record_sha256: None,
            configuration: Some(&self.configuration),
            revision: Some(self.revision),
            content_sha256: Some(&self.content_sha256),
            signer: Some(&self.signer),
        })
        .expect("a canonical binding serializes")
    }

    /// The challenge the browser signed for this binding and `nonce`.
    pub fn challenge(&self, nonce: &[u8; 32]) -> [u8; 32] {
        let binding = tagged(BINDING_DOMAIN, &[self.canonical().as_bytes()]);
        tagged(CHALLENGE_DOMAIN, &[&binding, nonce])
    }
}

/// One WebAuthn assertion, with what it is bound to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Proof {
    pub binding: Binding,
    /// base64url, 32 bytes.
    pub nonce: String,
    /// base64url, as the authenticator gave them.
    pub credential_id: String,
    pub authenticator_data: String,
    pub client_data_json: String,
    /// ASN.1 DER ECDSA signature, base64url.
    pub signature: String,
}

impl Proof {
    /// The proof's digest: what the final signature's statement lists.
    pub fn sha256(&self) -> String {
        let canonical =
            serde_json::to_string(self).expect("a proof serializes");
        hex::encode(tagged(PROOF_DOMAIN, &[canonical.as_bytes()]))
    }
}

/// What the final signer seals: one revision's content and exactly these
/// approvals, by their proofs' digests in order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Statement<'a> {
    pub v: u8,
    pub configuration: &'a str,
    /// The configuration as a package's manifest names it.
    pub external_id: &'a str,
    pub revision: i64,
    pub content_sha256: &'a str,
    pub approvals: Vec<String>,
}

impl Statement<'_> {
    pub fn sha256(&self) -> String {
        let canonical =
            serde_json::to_string(self).expect("a statement serializes");
        hex::encode(tagged(STATEMENT_DOMAIN, &[canonical.as_bytes()]))
    }
}

/// A revision's approvals and final signature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bundle {
    pub format: String,
    pub configuration: String,
    /// The configuration's external id: what a package's manifest names.
    pub external_id: String,
    pub revision: i64,
    /// The revision's content digest: a package's manifest `content_sha256`.
    pub content_sha256: String,
    pub approvals: Vec<Proof>,
    pub final_signature: Proof,
}

impl Bundle {
    /// The statement the final signer seals for these approvals.
    pub fn statement(&self) -> Statement<'_> {
        Statement {
            v: 1,
            configuration: &self.configuration,
            external_id: &self.external_id,
            revision: self.revision,
            content_sha256: &self.content_sha256,
            approvals: self.approvals.iter().map(Proof::sha256).collect(),
        }
    }
}

/// Canonical, unpadded base64url only: `=` and non-canonical trailing bits
/// are refused, so one value has one spelling.
fn decode(text: &str, what: &'static str) -> Result<Vec<u8>, ProofError> {
    URL_SAFE_NO_PAD
        .decode(text)
        .map_err(|_| ProofError::Encoding(what))
}

/// The client data fields an assertion must have, and nothing that widens it.
#[derive(Deserialize)]
struct ClientData {
    #[serde(rename = "type")]
    kind: String,
    challenge: String,
    origin: String,
    #[serde(default, rename = "crossOrigin")]
    cross_origin: Option<bool>,
    #[serde(default, rename = "topOrigin")]
    top_origin: Option<String>,
}

/// What one verified assertion says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verified {
    pub subject: String,
    pub key: String,
    pub role: Role,
    pub sign_count: u32,
}

/// Checks one proof for `action` on `configuration`/`revision` over
/// `content_sha256`, against `roster`.
pub fn verify_proof(
    proof: &Proof,
    roster: &Roster,
    action: Action,
    configuration: &str,
    revision: i64,
    content_sha256: &str,
) -> Result<Verified, ProofError> {
    let binding = &proof.binding;
    if binding.action != action {
        return Err(ProofError::Binding("action"));
    }
    if binding.rp_id != roster.rp_id {
        return Err(ProofError::Binding("relying party"));
    }
    if binding.tenant != roster.tenant {
        return Err(ProofError::Binding("tenant"));
    }
    if revision < 1
        || binding.configuration != configuration
        || binding.revision != revision
    {
        return Err(ProofError::Binding("revision"));
    }
    if binding.content_sha256 != content_sha256 {
        return Err(ProofError::Binding("content"));
    }
    let key = roster
        .keys
        .iter()
        .find(|key| key.key == binding.signer)
        .ok_or_else(|| ProofError::UnknownKey(binding.signer.clone()))?;
    if key.subject != binding.subject
        || key.role != action.role()
        || key.credential_id != proof.credential_id
    {
        return Err(ProofError::WrongKey(binding.signer.clone()));
    }

    let nonce: [u8; 32] = decode(&proof.nonce, "the nonce")?
        .try_into()
        .map_err(|_| ProofError::Encoding("the nonce"))?;
    let challenge = binding.challenge(&nonce);

    let client_data_bytes = decode(&proof.client_data_json, "the client data")?;
    let client_data: ClientData = serde_json::from_slice(&client_data_bytes)
        .map_err(|error| ProofError::ClientData(error.to_string()))?;
    if client_data.kind != "webauthn.get" {
        return Err(ProofError::ClientData("not webauthn.get".to_string()));
    }
    if decode(&client_data.challenge, "the client data challenge")?.as_slice()
        != challenge.as_slice()
    {
        return Err(ProofError::ClientData("another challenge".to_string()));
    }
    if !roster
        .origins
        .iter()
        .any(|origin| *origin == client_data.origin)
    {
        return Err(ProofError::ClientData(format!(
            "origin {}",
            client_data.origin
        )));
    }
    if client_data.cross_origin == Some(true)
        || client_data.top_origin.is_some()
    {
        return Err(ProofError::ClientData("cross-origin".to_string()));
    }

    let authenticator_data =
        decode(&proof.authenticator_data, "the authenticator data")?;
    if authenticator_data.len() < AUTHENTICATOR_DATA_LENGTH {
        return Err(ProofError::AuthenticatorData("too short"));
    }
    if authenticator_data[..32]
        != *digest(&SHA256, roster.rp_id.as_bytes()).as_ref()
    {
        return Err(ProofError::AuthenticatorData("another relying party"));
    }
    let flags = authenticator_data[32];
    if flags & USER_PRESENT == 0 || flags & USER_VERIFIED == 0 {
        return Err(ProofError::AuthenticatorData(
            "no user presence and verification",
        ));
    }
    if flags & (BACKUP_ELIGIBLE | BACKUP_STATE) != 0 {
        return Err(ProofError::AuthenticatorData(
            "a key that can be backed up",
        ));
    }
    if flags & ATTESTED_DATA != 0 {
        return Err(ProofError::AuthenticatorData(
            "attested credential data in an assertion",
        ));
    }
    if flags & EXTENSION_DATA == 0
        && authenticator_data.len() != AUTHENTICATOR_DATA_LENGTH
    {
        return Err(ProofError::AuthenticatorData("trailing bytes"));
    }
    let sign_count = u32::from_be_bytes([
        authenticator_data[33],
        authenticator_data[34],
        authenticator_data[35],
        authenticator_data[36],
    ]);

    let mut message = authenticator_data.clone();
    message.extend_from_slice(digest(&SHA256, &client_data_bytes).as_ref());
    let public_key = decode(&key.public_key, "the roster public key")?;
    let signature = decode(&proof.signature, "the signature")?;
    UnparsedPublicKey::new(&ECDSA_P256_SHA256_ASN1, &public_key)
        .verify(&message, &signature)
        .map_err(|_| ProofError::Signature)?;

    Ok(Verified {
        subject: key.subject.clone(),
        key: key.key.clone(),
        role: key.role,
        sign_count,
    })
}

/// What a verified bundle says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedBundle {
    pub approvers: Vec<String>,
    pub final_signer: String,
}

/// Checks a bundle: `required` approvals by distinct approvers with distinct
/// keys, and a final signature by a separate final signer over exactly
/// those approvals, every one against `roster`.
pub fn verify_bundle(
    bundle: &Bundle,
    roster: &Roster,
    required: usize,
) -> Result<VerifiedBundle, ProofError> {
    verify_sealed(bundle, roster, required, &bundle.statement().sha256())
}

/// The roster's entry for a verified key: its credential and public key, as
/// bytes, so one physical key never counts twice whatever it is called.
fn key_material(
    roster: &Roster,
    key: &str,
) -> Result<(Vec<u8>, Vec<u8>), ProofError> {
    let entry = roster
        .keys
        .iter()
        .find(|entry| entry.key == key)
        .ok_or_else(|| ProofError::UnknownKey(key.to_string()))?;
    Ok((
        decode(&entry.credential_id, "a roster credential id")?,
        decode(&entry.public_key, "a roster public key")?,
    ))
}

/// The approvals, then the final signature over `seal`, which the caller
/// derives from what the final signer must have sealed.
fn verify_sealed(
    bundle: &Bundle,
    roster: &Roster,
    required: usize,
    seal: &str,
) -> Result<VerifiedBundle, ProofError> {
    if bundle.format != BUNDLE_FORMAT {
        return Err(ProofError::Format(BUNDLE_FORMAT));
    }
    let mut approvers = Vec::new();
    let mut keys = Vec::new();
    let mut material = Vec::new();
    for proof in &bundle.approvals {
        let verified = verify_proof(
            proof,
            roster,
            Action::ApproveRevision,
            &bundle.configuration,
            bundle.revision,
            &bundle.content_sha256,
        )?;
        let (credential, public) = key_material(roster, &verified.key)?;
        if approvers.contains(&verified.subject)
            || keys.contains(&verified.key)
            || material
                .iter()
                .any(|(c, p)| *c == credential || *p == public)
        {
            return Err(ProofError::Duplicate(verified.subject));
        }
        approvers.push(verified.subject);
        keys.push(verified.key);
        material.push((credential, public));
    }
    if approvers.len() < required.max(1) {
        return Err(ProofError::TooFewApprovals {
            needed: required.max(1),
            given: approvers.len(),
        });
    }
    let signer = verify_proof(
        &bundle.final_signature,
        roster,
        Action::SignPackage,
        &bundle.configuration,
        bundle.revision,
        seal,
    )?;
    let (credential, public) = key_material(roster, &signer.key)?;
    if approvers.contains(&signer.subject)
        || material
            .iter()
            .any(|(c, p)| *c == credential || *p == public)
    {
        return Err(ProofError::SignerApproved);
    }
    Ok(VerifiedBundle {
        approvers,
        final_signer: signer.subject,
    })
}

/// The most approvals a pinned policy may require.
pub const MAX_REQUIRED_APPROVALS: usize = 16;

/// A roster and approval threshold a verifier provisioned for one tenant,
/// checked and pinned. The only way a package is admitted on browser
/// signatures: its fields are private, so nothing gets one without
/// [`PinnedRoster::load`], and nothing in a package or its imported settings
/// supplies one.
#[derive(Debug, Clone)]
pub struct PinnedRoster {
    roster: Roster,
    required: usize,
    sha256: String,
}

fn is_lower_hex_sha256(text: &str) -> bool {
    text.len() == 64
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// `https://host[:port]`, lowercase, nothing after it; plain http only for
/// the loopback names a local test uses.
fn exact_origin(origin: &str) -> bool {
    let rest = match origin.split_once("://") {
        Some(("https", rest)) => rest,
        Some(("http", rest))
            if rest == "localhost"
                || rest.starts_with("localhost:")
                || rest == "127.0.0.1"
                || rest.starts_with("127.0.0.1:") =>
        {
            rest
        }
        _ => return false,
    };
    let (host, port) = match rest.rsplit_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (rest, None),
    };
    !host.is_empty()
        && host.bytes().all(|b| {
            b.is_ascii_lowercase()
                || b.is_ascii_digit()
                || b == b'.'
                || b == b'-'
        })
        && match port {
            None => true,
            Some(port) => {
                !port.is_empty()
                    && port.len() <= 5
                    && port.bytes().all(|b| b.is_ascii_digit())
            }
        }
}

impl PinnedRoster {
    /// Reads `text` only if it is the roster `pinned_sha256` names, for
    /// exactly `tenant`, needing `required` approvals: all three from the
    /// verifier's own provisioning. Besides [`Roster::read`] (one key per
    /// person, no key, credential or public key twice), it refuses a roster
    /// for another tenant, a relying party or origin that isn't exact, a key
    /// whose registration digest isn't a SHA-256, a threshold outside
    /// 1..=[`MAX_REQUIRED_APPROVALS`] or above the approvers it lists, and a
    /// roster with no final signer.
    pub fn load(
        text: &str,
        pinned_sha256: &str,
        tenant: &str,
        required: usize,
    ) -> Result<Self, ProofError> {
        if !is_lower_hex_sha256(pinned_sha256) {
            return Err(ProofError::Policy(
                "the pin is not a lowercase SHA-256".to_string(),
            ));
        }
        if tenant.is_empty() {
            return Err(ProofError::Policy("no tenant".to_string()));
        }
        let roster = Roster::read_pinned(text, pinned_sha256)?;
        if roster.tenant != tenant {
            return Err(ProofError::Policy(format!(
                "the roster is for tenant {}, not this one",
                roster.tenant
            )));
        }
        if roster.rp_id.is_empty()
            || !roster.rp_id.bytes().all(|b| {
                b.is_ascii_lowercase()
                    || b.is_ascii_digit()
                    || b == b'.'
                    || b == b'-'
            })
        {
            return Err(ProofError::Policy(
                "the relying party is not a lowercase host name".to_string(),
            ));
        }
        let mut origins = std::collections::BTreeSet::new();
        for origin in &roster.origins {
            if !exact_origin(origin) || !origins.insert(origin.as_str()) {
                return Err(ProofError::Policy(format!(
                    "origin {origin} is not one exact origin"
                )));
            }
        }
        for key in &roster.keys {
            if key.key.is_empty()
                || key.subject.is_empty()
                || !is_lower_hex_sha256(&key.record_sha256)
            {
                return Err(ProofError::Policy(format!(
                    "key {} is not fully identified",
                    key.key
                )));
            }
        }
        let approvers = roster
            .keys
            .iter()
            .filter(|key| key.role == Role::Approver)
            .count();
        let signers = roster
            .keys
            .iter()
            .filter(|key| key.role == Role::FinalSigner)
            .count();
        if required == 0
            || required > MAX_REQUIRED_APPROVALS
            || required > approvers
        {
            return Err(ProofError::Policy(format!(
                "{required} approvals can't be required of {approvers} approvers"
            )));
        }
        if signers == 0 {
            return Err(ProofError::Policy("no final signer".to_string()));
        }
        Ok(PinnedRoster {
            roster,
            required,
            sha256: pinned_sha256.to_string(),
        })
    }

    pub fn tenant(&self) -> &str {
        &self.roster.tenant
    }

    pub fn rp_id(&self) -> &str {
        &self.roster.rp_id
    }

    pub fn required(&self) -> usize {
        self.required
    }

    /// The pin it was loaded with.
    pub fn sha256(&self) -> &str {
        &self.sha256
    }
}

/// What a browser-signed package's final signer seals: the package's whole
/// manifest (by its exact bytes' digest), what that manifest says it
/// delivers and follows, the tenant and relying party, the envelope format
/// and exactly these approvals.
pub const PACKAGE_SEAL_DOMAIN: &[u8] =
    b"sequent.architect.webauthn.package-seal.v1\0";

/// A manifest as the verifier read it, recomputed from its own bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SealedManifest<'a> {
    /// The envelope format the package declared.
    pub envelope: &'a str,
    pub external_id: &'a str,
    pub revision: u64,
    /// The previous signed revision and its manifest digest, if any.
    pub previous: Option<(u64, &'a str)>,
    pub content_sha256: &'a str,
    /// SHA-256 of `manifest.json`'s exact bytes.
    pub manifest_sha256: &'a str,
}

#[derive(Serialize)]
struct PackageSeal<'a> {
    v: u8,
    envelope: &'a str,
    tenant: &'a str,
    rp_id: &'a str,
    configuration: &'a str,
    external_id: &'a str,
    revision: u64,
    previous_revision: Option<u64>,
    previous_manifest_sha256: Option<&'a str>,
    content_sha256: &'a str,
    manifest_sha256: &'a str,
    approvals: Vec<String>,
}

/// The digest a browser-signed package's final signature binds: what a
/// producer has the final signer sign, and what [`verify_bundle_for_package`]
/// recomputes.
pub fn package_seal_sha256(
    bundle: &Bundle,
    tenant: &str,
    rp_id: &str,
    manifest: SealedManifest<'_>,
) -> String {
    package_seal_for_approvals(
        &bundle.configuration,
        tenant,
        rp_id,
        manifest,
        bundle.approvals.iter().map(Proof::sha256).collect(),
    )
}

/// Producer-side equivalent when the final assertion does not exist yet.
/// The exact frozen approval digests and full manifest are challenged.
pub fn package_seal_for_approvals(
    configuration: &str,
    tenant: &str,
    rp_id: &str,
    manifest: SealedManifest<'_>,
    approvals: Vec<String>,
) -> String {
    let seal = PackageSeal {
        v: 1,
        envelope: manifest.envelope,
        tenant,
        rp_id,
        configuration,
        external_id: manifest.external_id,
        revision: manifest.revision,
        previous_revision: manifest.previous.map(|(revision, _)| revision),
        previous_manifest_sha256: manifest.previous.map(|(_, sha)| sha),
        content_sha256: manifest.content_sha256,
        manifest_sha256: manifest.manifest_sha256,
        approvals,
    };
    let canonical =
        serde_json::to_string(&seal).expect("a package seal serializes");
    hex::encode(tagged(PACKAGE_SEAL_DOMAIN, &[canonical.as_bytes()]))
}

/// Checks a bundle for exactly the manifest a package carries, against a
/// [`PinnedRoster`] and its own threshold: the bundle's configuration,
/// revision and content are the manifest's, every approval verifies, and the
/// final signature is over [`package_seal_sha256`] of this tenant, this
/// relying party and this manifest.
pub fn verify_bundle_for_package(
    bundle: &Bundle,
    pinned: &PinnedRoster,
    manifest: SealedManifest<'_>,
) -> Result<VerifiedBundle, ProofError> {
    if bundle.external_id != manifest.external_id {
        return Err(ProofError::Binding("package's configuration"));
    }
    if u64::try_from(bundle.revision).ok() != Some(manifest.revision) {
        return Err(ProofError::Binding("package's revision"));
    }
    if bundle.content_sha256 != manifest.content_sha256 {
        return Err(ProofError::Binding("package's content"));
    }
    let seal =
        package_seal_sha256(bundle, pinned.tenant(), pinned.rp_id(), manifest);
    verify_sealed(bundle, &pinned.roster, pinned.required, &seal)
}

/// What a package's manifest says of the revision it delivers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManifestRevision<'a> {
    pub external_id: &'a str,
    pub revision: u64,
    pub content_sha256: &'a str,
}

/// Checks a bundle for exactly the revision a package's manifest delivers:
/// the same configuration (by its external id), revision and content
/// digest, all sealed by the final signer, then [`verify_bundle`] against
/// `roster` (read with [`Roster::read_pinned`] from the verifier's own
/// provisioning, never from the package).
pub fn verify_bundle_for_manifest(
    bundle: &Bundle,
    roster: &Roster,
    required: usize,
    manifest: ManifestRevision<'_>,
) -> Result<VerifiedBundle, ProofError> {
    if bundle.external_id != manifest.external_id {
        return Err(ProofError::Binding("package's configuration"));
    }
    if u64::try_from(bundle.revision).ok() != Some(manifest.revision) {
        return Err(ProofError::Binding("package's revision"));
    }
    if bundle.content_sha256 != manifest.content_sha256 {
        return Err(ProofError::Binding("package's content"));
    }
    verify_bundle(bundle, roster, required)
}

#[cfg(test)]
#[path = "webauthn_proof_tests.rs"]
mod webauthn_proof_tests;
