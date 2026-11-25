// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Static AWS SES/SNS key pairs that travel beside a signed delivery, never
//! inside it.
//!
//! The signed `tenant_messaging_accounts.json` member carries, per account,
//! only a hiding SHA-256 commitment ([`commitment`]) to the exact key pair
//! and a 32-byte random nonce. The pair and nonce form a separate
//! [`CredentialSidecar`]. Signing, approval and the stored package therefore
//! bind the exact secret bytes without holding them. A receiver accepts a
//! sidecar only when [`verify_sidecar`] matches every committed account of the
//! signed, tenant-bound declarations and nothing else.
//!
//! At rest before delivery, a sidecar is sealed with an installation record
//! key ([`RecordKey`], derived for this purpose only by the deployment) using
//! AES-256-GCM with a random nonce and caller-bound associated data. No new
//! primitive: SHA-256 and AES-256-GCM from `ring`, as `encrypted_delivery`.
//!
//! No type here derives `Debug` over key material, and no error carries a
//! value: errors are fixed strings. Key and secret buffers are cleared when
//! dropped (`zeroize`); this does not reach copies a caller makes.

use super::portable_messaging::{PortableMessagingAccounts, PortableSender};
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use ring::digest::{Context, SHA256};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use uuid::Uuid;
use zeroize::Zeroize;
/// Part of this module's public types; re-exported for their callers.
pub use zeroize::Zeroizing;

pub const SIDECAR_FORMAT: &str = "sequent.messaging-credentials/1";
const COMMITMENT_DOMAIN: &[u8] = b"sequent.messaging-credential-commitment/1\0";
const MAX_SIDECAR_BYTES: usize = 16 * 1024;
const MAX_ACCOUNTS: usize = 32;

/// The one environment line an installation record key file holds.
pub const RECORD_KEY_VARIABLE: &str = "ARCHITECT_CREDENTIALS_KEY";
const RECORD_MAGIC: &[u8; 9] = b"SEQ-MCRED";
const RECORD_FORMAT: u8 = 1;
/// The only record key version. Records naming any other are refused; no
/// key rotation is implemented.
pub const RECORD_KEY_VERSION: u8 = 1;
const RECORD_HEADER_LEN: usize = RECORD_MAGIC.len() + 2 + 12;
const TAG_LEN: usize = 16;

/// Never says which field or value was wrong.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CredentialError {
    InvalidSidecar,
    SidecarMismatch,
    InvalidRecordKey,
    RandomUnavailable,
    SealFailed,
    RecordRejected,
}

impl fmt::Display for CredentialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidSidecar => "The messaging credentials are not a supported AWS key pair document.",
            Self::SidecarMismatch => "The messaging credentials do not match the signed delivery's AWS accounts.",
            Self::InvalidRecordKey => "The installation's credentials key is unavailable or invalid.",
            Self::RandomUnavailable => "Secure randomness is unavailable.",
            Self::SealFailed => "The messaging credentials could not be protected.",
            Self::RecordRejected => "Protected messaging credentials could not be opened for this record.",
        })
    }
}

impl std::error::Error for CredentialError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SidecarFormat {
    #[serde(rename = "sequent.messaging-credentials/1")]
    V1,
}

/// A long-term AWS key pair, in memory only (never serialized). Session
/// (`ASIA…`) keys need a token and are refused: the pair is the whole
/// credential.
#[derive(Clone, PartialEq, Eq)]
pub struct AwsStaticKeyPair {
    pub aws_access_key_id: Zeroizing<String>,
    pub aws_secret_access_key: Zeroizing<String>,
}

impl fmt::Debug for AwsStaticKeyPair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AwsStaticKeyPair(<redacted>)")
    }
}

impl AwsStaticKeyPair {
    pub fn valid(&self) -> bool {
        valid_pair(&self.aws_access_key_id, &self.aws_secret_access_key)
    }
}

fn valid_pair(id: &str, secret: &str) -> bool {
    let id = id.as_bytes();
    let secret = secret.as_bytes();
    id.len() == 20
        && id.starts_with(b"AKIA")
        && id
            .iter()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
        && secret.len() == 40
        && secret
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || *b == b'+' || *b == b'/')
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SidecarEntry {
    /// The declaration's `reference`, without the `package:` prefix.
    pub reference: String,
    /// 32 random bytes, lowercase hex. Only here, never in the manifest.
    pub nonce: String,
    pub aws_access_key_id: String,
    pub aws_secret_access_key: String,
}

impl SidecarEntry {
    pub fn pair(&self) -> AwsStaticKeyPair {
        AwsStaticKeyPair {
            aws_access_key_id: Zeroizing::new(self.aws_access_key_id.clone()),
            aws_secret_access_key: Zeroizing::new(
                self.aws_secret_access_key.clone(),
            ),
        }
    }

    fn valid(&self) -> bool {
        reference(&self.reference)
            && lower_hex(&self.nonce, 32)
            && valid_pair(&self.aws_access_key_id, &self.aws_secret_access_key)
    }
}

impl Drop for SidecarEntry {
    fn drop(&mut self) {
        self.nonce.zeroize();
        self.aws_access_key_id.zeroize();
        self.aws_secret_access_key.zeroize();
    }
}

impl fmt::Debug for SidecarEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SidecarEntry")
            .field("reference", &self.reference)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CredentialSidecar {
    pub format: SidecarFormat,
    /// The destination tenant, as a canonical (lowercase, hyphenated) UUID.
    pub tenant_id: String,
    pub accounts: Vec<SidecarEntry>,
}

impl fmt::Debug for CredentialSidecar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CredentialSidecar")
            .field("tenant_id", &self.tenant_id)
            .field("accounts", &self.accounts.len())
            .finish_non_exhaustive()
    }
}

fn lower_hex(s: &str, bytes: usize) -> bool {
    s.len() == bytes * 2
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn reference(s: &str) -> bool {
    s.len() <= 48
        && s.as_bytes().first().is_some_and(|b| b.is_ascii_lowercase())
        && s.as_bytes()
            .last()
            .is_some_and(|b| b.is_ascii_alphanumeric())
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

impl CredentialSidecar {
    pub fn parse(bytes: &[u8]) -> Result<Self, CredentialError> {
        if bytes.len() > MAX_SIDECAR_BYTES {
            return Err(CredentialError::InvalidSidecar);
        }
        let sidecar: Self = serde_json::from_slice(bytes)
            .map_err(|_| CredentialError::InvalidSidecar)?;
        sidecar.validate()?;
        Ok(sidecar)
    }

    pub fn validate(&self) -> Result<(), CredentialError> {
        let mut seen = BTreeSet::new();
        if canonical_tenant(&self.tenant_id).is_none()
            || !(1..=MAX_ACCOUNTS).contains(&self.accounts.len())
            || !self
                .accounts
                .iter()
                .all(|a| a.valid() && seen.insert(a.reference.as_str()))
        {
            return Err(CredentialError::InvalidSidecar);
        }
        Ok(())
    }

    /// The tenant this sidecar is for.
    pub fn tenant(&self) -> Result<Uuid, CredentialError> {
        canonical_tenant(&self.tenant_id).ok_or(CredentialError::InvalidSidecar)
    }

    pub fn to_bytes(&self) -> Result<Zeroizing<Vec<u8>>, CredentialError> {
        self.validate()?;
        serde_json::to_vec(self)
            .map(Zeroizing::new)
            .map_err(|_| CredentialError::InvalidSidecar)
    }
}

/// Only the canonical spelling: one tenant has exactly one sidecar form.
fn canonical_tenant(s: &str) -> Option<Uuid> {
    Uuid::parse_str(s)
        .ok()
        .filter(|t| !t.is_nil() && t.hyphenated().to_string() == s)
}

/// A fresh 32-byte commitment nonce, lowercase hex.
pub fn new_nonce() -> Result<String, CredentialError> {
    let mut nonce = [0u8; 32];
    SystemRandom::new()
        .fill(&mut nonce)
        .map_err(|_| CredentialError::RandomUnavailable)?;
    Ok(hex::encode(nonce))
}

fn framed(context: &mut Context, part: &[u8]) {
    // Fixed-width length prefixes: no two inputs share an encoding.
    context.update(&(part.len() as u32).to_be_bytes());
    context.update(part);
}

/// SHA-256 over a fixed domain, the destination tenant's 16 UUID bytes, the
/// reference, the random nonce and the exact pair, each length-prefixed. The
/// 256-bit nonce hides the pair even if a secret were guessable; the tenant
/// and reference stop a commitment being replayed for another destination or
/// account.
pub fn commitment(
    tenant: &Uuid,
    entry: &SidecarEntry,
) -> Result<String, CredentialError> {
    if tenant.is_nil() || !entry.valid() {
        return Err(CredentialError::InvalidSidecar);
    }
    let nonce = Zeroizing::new(
        hex::decode(&entry.nonce)
            .map_err(|_| CredentialError::InvalidSidecar)?,
    );
    let mut context = Context::new(&SHA256);
    context.update(COMMITMENT_DOMAIN);
    framed(&mut context, tenant.as_bytes());
    framed(&mut context, entry.reference.as_bytes());
    framed(&mut context, &nonce);
    framed(&mut context, entry.aws_access_key_id.as_bytes());
    framed(&mut context, entry.aws_secret_access_key.as_bytes());
    Ok(hex::encode(context.finish().as_ref()))
}

fn same(a: &str, b: &str) -> bool {
    // Both are public commitments of equal width; constant time anyway.
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0u8, |acc, (x, y)| acc | (x ^ y))
            == 0
}

/// The exact set match: every committed AWS account has exactly one entry
/// whose commitment equals the signed one, and the sidecar names no other
/// account or tenant. Returns the pairs by package reference key.
pub fn verify_sidecar(
    definitions: &PortableMessagingAccounts,
    tenant: &Uuid,
    sidecar: &CredentialSidecar,
) -> Result<BTreeMap<String, AwsStaticKeyPair>, CredentialError> {
    sidecar.validate()?;
    definitions
        .validate()
        .map_err(|_| CredentialError::SidecarMismatch)?;
    if sidecar.tenant()? != *tenant {
        return Err(CredentialError::SidecarMismatch);
    }
    let committed: Vec<_> = definitions
        .accounts
        .iter()
        .filter_map(|a| a.credential_commitment.as_ref().map(|c| (a, c)))
        .collect();
    if committed.len() != sidecar.accounts.len() {
        return Err(CredentialError::SidecarMismatch);
    }
    let mut pairs = BTreeMap::new();
    for (account, signed) in committed {
        if !matches!(
            account.sender,
            PortableSender::AWS_SES { .. } | PortableSender::AWS_SNS { .. }
        ) {
            return Err(CredentialError::SidecarMismatch);
        }
        let entry = sidecar
            .accounts
            .iter()
            .find(|e| e.reference == account.reference)
            .ok_or(CredentialError::SidecarMismatch)?;
        if !same(&commitment(tenant, entry)?, signed) {
            return Err(CredentialError::SidecarMismatch);
        }
        pairs.insert(account.reference_key(), entry.pair());
    }
    Ok(pairs)
}

/// The deployment-derived, purpose-separated installation key. It is never
/// the vault's master secret; the deployment derives it and mounts only it.
pub struct RecordKey {
    version: u8,
    key: LessSafeKey,
}

impl fmt::Debug for RecordKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RecordKey(v{}, <redacted>)", self.version)
    }
}

impl RecordKey {
    /// Exactly one `ARCHITECT_CREDENTIALS_KEY=<64 lowercase hex>` line. An
    /// empty file (the compose default when the deployment provides none)
    /// is "no key": credential intake stays unavailable.
    pub fn from_env_file(
        contents: &str,
    ) -> Result<Option<Self>, CredentialError> {
        let trimmed = contents.trim_end_matches('\n');
        if trimmed.is_empty() {
            return Ok(None);
        }
        let value = trimmed
            .strip_prefix(RECORD_KEY_VARIABLE)
            .and_then(|rest| rest.strip_prefix('='))
            .filter(|v| !v.contains('\n') && lower_hex(v, 32))
            .ok_or(CredentialError::InvalidRecordKey)?;
        let bytes = Zeroizing::new(
            hex::decode(value)
                .map_err(|_| CredentialError::InvalidRecordKey)?,
        );
        let key = UnboundKey::new(&AES_256_GCM, &bytes)
            .map_err(|_| CredentialError::InvalidRecordKey)?;
        Ok(Some(Self {
            version: RECORD_KEY_VERSION,
            key: LessSafeKey::new(key),
        }))
    }

    fn header(&self, nonce: &[u8; 12]) -> [u8; RECORD_HEADER_LEN] {
        let mut header = [0u8; RECORD_HEADER_LEN];
        header[..RECORD_MAGIC.len()].copy_from_slice(RECORD_MAGIC);
        header[RECORD_MAGIC.len()] = RECORD_FORMAT;
        header[RECORD_MAGIC.len() + 1] = self.version;
        header[RECORD_MAGIC.len() + 2..].copy_from_slice(nonce);
        header
    }

    fn aad(header: &[u8], context: &[u8]) -> Vec<u8> {
        let mut aad = Vec::with_capacity(header.len() + 4 + context.len());
        aad.extend_from_slice(header);
        aad.extend_from_slice(&(context.len() as u32).to_be_bytes());
        aad.extend_from_slice(context);
        aad
    }

    /// `context` binds the record to its row (for example configuration,
    /// draft or revision and purpose); opening it under any other fails.
    pub fn seal(
        &self,
        context: &[u8],
        plaintext: &[u8],
    ) -> Result<Vec<u8>, CredentialError> {
        let mut nonce = [0u8; 12];
        SystemRandom::new()
            .fill(&mut nonce)
            .map_err(|_| CredentialError::RandomUnavailable)?;
        let header = self.header(&nonce);
        // Allocated once at its final size and cleared on any early return,
        // so no plaintext copy outlives sealing.
        let mut out = Zeroizing::new(Vec::with_capacity(
            RECORD_HEADER_LEN + plaintext.len() + TAG_LEN,
        ));
        out.extend_from_slice(&header);
        out.extend_from_slice(plaintext);
        let tag = self
            .key
            .seal_in_place_separate_tag(
                Nonce::assume_unique_for_key(nonce),
                Aad::from(Self::aad(&header, context)),
                &mut out[RECORD_HEADER_LEN..],
            )
            .map_err(|_| CredentialError::SealFailed)?;
        out.extend_from_slice(tag.as_ref());
        // Ciphertext only from here.
        Ok(std::mem::take(&mut *out))
    }

    pub fn open(
        &self,
        context: &[u8],
        record: &[u8],
    ) -> Result<Zeroizing<Vec<u8>>, CredentialError> {
        if record.len() <= RECORD_HEADER_LEN + TAG_LEN
            || !record.starts_with(RECORD_MAGIC)
            || record[RECORD_MAGIC.len()] != RECORD_FORMAT
            || record[RECORD_MAGIC.len() + 1] != self.version
        {
            return Err(CredentialError::RecordRejected);
        }
        let (header, sealed) = record.split_at(RECORD_HEADER_LEN);
        let nonce: [u8; 12] = header[RECORD_MAGIC.len() + 2..]
            .try_into()
            .map_err(|_| CredentialError::RecordRejected)?;
        let mut body = Zeroizing::new(sealed.to_vec());
        let len = self
            .key
            .open_in_place(
                Nonce::assume_unique_for_key(nonce),
                Aad::from(Self::aad(header, context)),
                &mut body[..],
            )
            .map_err(|_| CredentialError::RecordRejected)?
            .len();
        body.truncate(len);
        Ok(body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::election_config::portable_messaging::parse as parse_accounts;

    // Synthetic, format-valid, not an AWS credential.
    const KEY_ID: &str = "AKIASYNTHETICEXAMPLE";
    const SECRET: &str = "SyntheticSecretValueForTestsOnly00000000";
    const TENANT: &str = "11111111-1111-4111-8111-111111111111";
    const OTHER_TENANT: &str = "22222222-2222-4222-8222-222222222222";

    fn tenant(s: &str) -> Uuid {
        Uuid::parse_str(s).unwrap()
    }

    fn entry(reference: &str) -> SidecarEntry {
        SidecarEntry {
            reference: reference.into(),
            nonce: "07".repeat(32),
            aws_access_key_id: KEY_ID.into(),
            aws_secret_access_key: SECRET.into(),
        }
    }

    fn sidecar(entries: Vec<SidecarEntry>) -> CredentialSidecar {
        CredentialSidecar {
            format: SidecarFormat::V1,
            tenant_id: TENANT.into(),
            accounts: entries,
        }
    }

    fn definitions(
        commit_email: Option<String>,
        commit_sms: Option<String>,
    ) -> PortableMessagingAccounts {
        let mut email = serde_json::json!({"reference":"email","name":"Demo Email",
            "sender":{"provider":"AWS_SES","from_address":"demo@example.invalid","region":"us-east-1"}});
        let mut sms = serde_json::json!({"reference":"sms","name":"Demo SMS",
            "sender":{"provider":"AWS_SNS","sender_id":"Demo","region":"us-east-1"}});
        if let Some(c) = commit_email {
            email["credential_commitment"] = c.into();
        }
        if let Some(c) = commit_sms {
            sms["credential_commitment"] = c.into();
        }
        let doc = serde_json::json!({"format":"sequent.tenant-messaging-accounts/1","accounts":[email, sms]});
        parse_accounts(&serde_json::to_vec(&doc).unwrap()).unwrap()
    }

    #[test]
    fn pair_shape_is_strict_long_term_and_never_printed() {
        assert!(entry("email").pair().valid());
        for (id, secret) in [
            ("ASIASYNTHETICEXAMPLE", SECRET),
            ("AKIAsyntheticEXAMPLE", SECRET),
            ("AKIASYNTHETIEXAMPLE", SECRET),
            (KEY_ID, &SECRET[..39]),
            (KEY_ID, "SyntheticSecretValueForTestsOnly0000000="),
        ] {
            let pair = AwsStaticKeyPair {
                aws_access_key_id: Zeroizing::new(id.to_string()),
                aws_secret_access_key: Zeroizing::new(secret.to_string()),
            };
            assert!(!pair.valid());
        }
        let s = sidecar(vec![entry("email")]);
        let printed =
            format!("{s:?} {:?} {:?}", s.accounts[0], s.accounts[0].pair());
        assert!(
            !printed.contains(KEY_ID)
                && !printed.contains(SECRET)
                && !printed.contains(&"07".repeat(32))
        );
    }

    #[test]
    fn sidecar_parse_is_strict_and_errors_carry_no_values() {
        let good = sidecar(vec![entry("email")]).to_bytes().unwrap();
        assert!(CredentialSidecar::parse(&good).is_ok());
        let mut doc: serde_json::Value = serde_json::from_slice(&good).unwrap();
        doc["accounts"][0]["aws_session_token"] = "SYNTHETIC".into();
        let err = CredentialSidecar::parse(&serde_json::to_vec(&doc).unwrap())
            .unwrap_err();
        assert!(
            !err.to_string().contains(SECRET)
                && !err.to_string().contains(KEY_ID)
        );
        let mut dup = sidecar(vec![entry("email"), entry("email")]);
        assert!(dup.validate().is_err());
        dup.accounts.pop();
        assert!(dup.validate().is_ok());
        doc = serde_json::from_slice(&good).unwrap();
        doc["format"] = "sequent.messaging-credentials/2".into();
        assert!(CredentialSidecar::parse(&serde_json::to_vec(&doc).unwrap())
            .is_err());
        assert!(CredentialSidecar::parse(&vec![b' '; MAX_SIDECAR_BYTES + 1])
            .is_err());
        // One tenant, one spelling: uppercase, braced, simple or nil are refused.
        for spelled in [
            "AAAAAAAA-1111-4111-8111-111111111111".to_string(),
            format!("{{{TENANT}}}"),
            TENANT.replace('-', ""),
            Uuid::nil().to_string(),
        ] {
            let mut other = sidecar(vec![entry("email")]);
            other.tenant_id = spelled;
            assert_eq!(
                other.validate().unwrap_err(),
                CredentialError::InvalidSidecar
            );
        }
        assert_eq!(
            sidecar(vec![entry("email")]).tenant().unwrap(),
            tenant(TENANT)
        );
    }

    #[test]
    fn commitment_binds_every_input() {
        let t = tenant(TENANT);
        let base = entry("email");
        let c = commitment(&t, &base).unwrap();
        assert_eq!(c.len(), 64);
        assert!(!c.contains(SECRET));
        assert_ne!(c, commitment(&tenant(OTHER_TENANT), &base).unwrap());
        assert!(commitment(&Uuid::nil(), &base).is_err());
        let mut other = base.clone();
        other.reference = "sms".into();
        assert_ne!(c, commitment(&t, &other).unwrap());
        other = base.clone();
        other.nonce = "08".repeat(32);
        assert_ne!(c, commitment(&t, &other).unwrap());
        other = base.clone();
        other.aws_secret_access_key =
            "SyntheticSecretValueForTestsOnly00000001".into();
        assert_ne!(c, commitment(&t, &other).unwrap());
        other = base.clone();
        other.aws_access_key_id = "AKIASYNTHETI2EXAMPLE".into();
        assert_ne!(c, commitment(&t, &other).unwrap());
        // Independent vector (Python hashlib/struct/uuid over the documented
        // framing), so a changed encoding is a deliberate, versioned decision.
        assert_eq!(
            c,
            "542174c09625ce95c4c8874c0376d043bb7437dfd8cd8e386151b28db401f4e3"
        );
        assert_ne!(new_nonce().unwrap(), new_nonce().unwrap());
    }

    #[test]
    fn sidecar_must_match_exactly_the_signed_commitments_and_tenant() {
        let t = tenant(TENANT);
        let email = entry("email");
        let sms = entry("sms");
        let both = definitions(
            Some(commitment(&t, &email).unwrap()),
            Some(commitment(&t, &sms).unwrap()),
        );
        let pairs = verify_sidecar(
            &both,
            &t,
            &sidecar(vec![email.clone(), sms.clone()]),
        )
        .unwrap();
        assert_eq!(
            pairs.keys().cloned().collect::<Vec<_>>(),
            ["package:email", "package:sms"]
        );
        assert_eq!(pairs["package:email"].aws_access_key_id.as_str(), KEY_ID);
        // Missing, extra, foreign tenant, or a different pair: refused.
        assert!(
            verify_sidecar(&both, &t, &sidecar(vec![email.clone()])).is_err()
        );
        let one = definitions(Some(commitment(&t, &email).unwrap()), None);
        assert!(verify_sidecar(
            &one,
            &t,
            &sidecar(vec![email.clone(), sms.clone()])
        )
        .is_err());
        assert!(verify_sidecar(
            &one,
            &tenant(OTHER_TENANT),
            &sidecar(vec![email.clone()])
        )
        .is_err());
        // A sidecar spelled for another tenant, against a commitment for this one.
        let mut foreign = sidecar(vec![email.clone()]);
        foreign.tenant_id = OTHER_TENANT.into();
        assert!(verify_sidecar(&one, &t, &foreign).is_err());
        let mut swapped = email.clone();
        swapped.aws_secret_access_key =
            "SyntheticSecretValueForTestsOnly00000001".into();
        assert!(verify_sidecar(&one, &t, &sidecar(vec![swapped])).is_err());
        let mut renamed = email.clone();
        renamed.reference = "sms".into();
        assert!(verify_sidecar(&one, &t, &sidecar(vec![renamed])).is_err());
        // Nothing committed: nothing may be delivered.
        let none = definitions(None, None);
        assert!(verify_sidecar(&none, &t, &sidecar(vec![sms])).is_err());
        let err = verify_sidecar(&both, &t, &sidecar(vec![email]))
            .unwrap_err()
            .to_string();
        assert!(!err.contains(SECRET) && !err.contains(KEY_ID));
    }

    fn key(byte: &str) -> RecordKey {
        RecordKey::from_env_file(&format!(
            "{RECORD_KEY_VARIABLE}={}\n",
            byte.repeat(32)
        ))
        .unwrap()
        .unwrap()
    }

    #[test]
    fn record_key_file_is_exact_and_never_printed() {
        assert!(RecordKey::from_env_file("").unwrap().is_none());
        for bad in [
            "MASTER_SECRET=".to_string() + &"ab".repeat(32),
            format!("{RECORD_KEY_VARIABLE}={}", "AB".repeat(32)),
            format!("{RECORD_KEY_VARIABLE}={}", "ab".repeat(31)),
            format!("{RECORD_KEY_VARIABLE}={}\nEXTRA=1", "ab".repeat(32)),
        ] {
            assert_eq!(
                RecordKey::from_env_file(&bad).unwrap_err(),
                CredentialError::InvalidRecordKey
            );
        }
        assert!(!format!("{:?}", key("ab")).contains(&"ab".repeat(32)));
    }

    #[test]
    fn records_open_only_with_the_same_key_context_and_bytes() {
        let plaintext = sidecar(vec![entry("email")]).to_bytes().unwrap();
        let k = key("ab");
        let sealed = k.seal(b"configuration/1/draft/3", &plaintext).unwrap();
        assert!(!sealed.windows(SECRET.len()).any(|w| w == SECRET.as_bytes()));
        assert_ne!(
            sealed,
            k.seal(b"configuration/1/draft/3", &plaintext).unwrap()
        );
        assert_eq!(
            k.open(b"configuration/1/draft/3", &sealed).unwrap(),
            plaintext
        );
        assert_eq!(
            k.open(b"configuration/1/draft/4", &sealed).unwrap_err(),
            CredentialError::RecordRejected
        );
        assert!(key("cd").open(b"configuration/1/draft/3", &sealed).is_err());
        for i in [
            0,
            RECORD_MAGIC.len() + 1,
            RECORD_HEADER_LEN,
            sealed.len() - 1,
        ] {
            let mut tampered = sealed.clone();
            tampered[i] ^= 1;
            assert!(k.open(b"configuration/1/draft/3", &tampered).is_err());
        }
        assert!(k
            .open(
                b"configuration/1/draft/3",
                &sealed[..RECORD_HEADER_LEN + TAG_LEN]
            )
            .is_err());
    }
}
