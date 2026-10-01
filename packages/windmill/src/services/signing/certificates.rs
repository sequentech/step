// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Staff certificate checks (design §5a, §7). A signer's browser sends the
//! certificate chain and the signatures; the server repeats every check with
//! OpenSSL and answers one [`CertificateCheckResult`] per
//! [`CertificateCheckId`], in the order the signing dialog lists them.
//!
//! [`evaluate`] makes every decision from data already loaded (issuers,
//! revocation lists, checks, registrations, earlier approvals), so it has no
//! database or network access. [`OpensslCertificateVerifier`] loads that
//! data in the caller's transaction, under advisory locks on the
//! certificate's key and holder.

use crate::postgres::certificate_authority::{list_staff_issuers, StaffIssuerRow};
use crate::postgres::signing::{
    find_active_staff_certificates_by_fingerprint, find_active_staff_certificates_by_holder,
    find_active_staff_certificates_by_spki, find_revoked_staff_certificates_by_fingerprint,
    find_revoked_staff_certificates_by_spki, get_signing_checks, insert_staff_certificate,
    list_signing_approvals, NewStaffCertificate, SigningApprovalRow, SigningRequestRow,
    StaffCertificateRow,
};
use crate::postgres::signing_certificates::{list_staff_crls, posts_signed_by_key};
use crate::services::signing::crl::{distribution_points, list_scope, ListScope};
use crate::services::signing::directory::{KeycloakUserDirectory, UserDirectory};
use crate::services::signing::log::{stage, Actor, LogScope, LogStep, SystemOutcome};
use anyhow::{anyhow, bail, Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use deadpool_postgres::Transaction;
use electoral_log::messages::newtypes::SigningStatementKind;
use openssl::asn1::{Asn1Time, Asn1TimeRef};
use openssl::hash::MessageDigest;
use openssl::nid::Nid;
use openssl::pkey::{Id, PKeyRef, Public};
use openssl::rsa::Padding;
use openssl::sign::Verifier;
use openssl::stack::Stack;
use openssl::x509::store::X509StoreBuilder;
use openssl::x509::verify::{X509VerifyFlags, X509VerifyParam};
use openssl::x509::{CrlStatus, X509Crl, X509PurposeId, X509Ref, X509StoreContext, X509};
use sequent_core::signing::{
    CertificateCheckId, CertificateCheckResult, CertificatePostBinding, CertificateRegistration,
    CrlUnavailablePolicy, DocumentKind, RevocationCheck, RevocationStatus, SignatureAlgorithm,
    SigningAction, SigningChecks, SigningScope, StaffCertificateRegistration,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::sync::Arc;
use strum::IntoEnumIterator;
use x509_parser::asn1_rs::Tag;
use x509_parser::prelude::{FromDer, X509Certificate, X509Name};

/// The most certificates a browser chain may hold.
pub const MAX_CHAIN_CERTIFICATES: usize = 10;
/// The smallest RSA key that signs.
pub const MIN_RSA_BITS: u32 = 2048;
/// OpenSSL's security level for the chain: 112-bit keys, no SHA-1 or MD5.
pub const CHAIN_AUTH_LEVEL: i32 = 2;
/// Extended key usages that sign documents, besides anyExtendedKeyUsage,
/// clientAuth and emailProtection: documentSigning (RFC 9336), Adobe
/// authentic documents and Microsoft document signing.
pub const SIGNING_EXTENDED_KEY_USAGES: [&str; 3] = [
    "1.3.6.1.5.5.7.3.36",
    "1.2.840.113583.1.1.5",
    "1.3.6.1.4.1.311.10.3.12",
];

/// The signatures of one approval. The payload signature covers the
/// request's canonical payload bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureInputs {
    pub algorithm: SignatureAlgorithm,
    /// RSASSA-PKCS1-v1_5 or ECDSA (DER), SHA-256, over the canonical payload.
    pub payload_signature: Vec<u8>,
    /// The document signature of an EML action (`DocumentKind::Eml`).
    pub document: Option<DocumentSignatureInput>,
}

/// A raw signature over a document's bytes, made with the same key as the
/// payload signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSignatureInput {
    pub document: Vec<u8>,
    pub signature: Vec<u8>,
}

/// What a certificate is checked for: one signer, one request.
#[derive(Debug, Clone)]
pub struct CertificateVerificationInput<'a> {
    /// The request being signed; its tenant, event, action, Post and
    /// canonical payload are what the checks use.
    pub request: &'a SigningRequestRow,
    /// The signed-in person who signs.
    pub signer: Actor,
    /// The PEMs the browser sent: the signing certificate first, then any
    /// intermediates. A PEM may hold several certificates. Only the event's
    /// staff issuers are trusted; a root the browser sends is not.
    pub chain_pem: Vec<String>,
    /// `None` for the dry run before signing: every check but `signature`.
    pub signatures: Option<SignatureInputs>,
    /// The time the certificate must be valid at.
    pub now: DateTime<Utc>,
}

/// The identities and display data of a signing certificate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CertificateIdentity {
    /// SHA-256 of the certificate DER, lowercase hex.
    pub fingerprint_sha256: String,
    /// SHA-256 of the SubjectPublicKeyInfo DER: the key pair.
    pub spki_sha256: String,
    /// SHA-256 of the subject name DER: the holder.
    pub holder_sha256: String,
    /// Uppercase hex, without separators.
    pub serial: String,
    pub subject: String,
    pub common_name: String,
    pub issuer: String,
    pub not_before: DateTime<Utc>,
    pub not_after: DateTime<Utc>,
    /// The signing certificate.
    pub pem: String,
    /// The certificates the browser sent, one PEM after another.
    pub chain_pem: String,
    /// The signature algorithm the key makes; `None` for an unsupported key
    /// (only RSA and EC P-256 are).
    pub key_algorithm: Option<SignatureAlgorithm>,
}

/// How the certificate stands with the signer's account.
#[derive(Debug, Clone, PartialEq)]
pub enum RegistrationState {
    /// Active for the signer in this event.
    Registered(StaffCertificateRow),
    /// Not registered yet; the checks allow registering it to the signer on
    /// this signature ([`register_first_use`]).
    FirstUse,
    /// Not registered, and it can't be registered on first use.
    NotRegistered,
}

/// The outcome of checking one certificate.
#[derive(Debug, Clone, PartialEq)]
pub struct CertificateVerification {
    /// One result per check, in [`CertificateCheckId`] order. The dry run
    /// has no `signature` result. When the certificate can't be read there
    /// is only a failed `trusted-issuer` result.
    pub checks: Vec<CertificateCheckResult>,
    /// `None` when the signing certificate can't be read.
    pub certificate: Option<CertificateIdentity>,
    /// `Unchecked` when revocation isn't checked, or when no current list
    /// was available and the checks accept that.
    pub revocation_status: RevocationStatus,
    pub registration: RegistrationState,
    /// The account the certificate's key or holder is registered to, when
    /// `registered-to-other` failed.
    pub other_holder: Option<OtherHolder>,
}

/// The account holding a certificate's key or holder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OtherHolder {
    pub user_id: String,
    pub display_name: String,
}

impl Default for CertificateVerification {
    /// Nothing checked: unreadable, not registered, unchecked.
    fn default() -> Self {
        CertificateVerification {
            checks: vec![],
            certificate: None,
            revocation_status: RevocationStatus::Unchecked,
            registration: RegistrationState::NotRegistered,
            other_holder: None,
        }
    }
}

impl CertificateVerification {
    /// Whether every check passed.
    pub fn passed(&self) -> bool {
        self.certificate.is_some() && self.checks.iter().all(|check| check.ok)
    }

    /// The first failed check: the reason a signature is refused. A
    /// certificate bound to another account fails `registered` too, and is
    /// refused as `registered-to-other`, which names that account.
    pub fn refusal(&self) -> Option<&CertificateCheckResult> {
        let first = self.checks.iter().find(|check| !check.ok)?;
        if first.id == CertificateCheckId::Registered {
            if let Some(other) = self
                .checks
                .iter()
                .find(|check| check.id == CertificateCheckId::RegisteredToOther && !check.ok)
            {
                return Some(other);
            }
        }
        Some(first)
    }

    /// The result of `id`, if it was checked.
    pub fn check(&self, id: CertificateCheckId) -> Option<&CertificateCheckResult> {
        self.checks.iter().find(|check| check.id == id)
    }
}

/// Checks a signer's certificate for a request. The OpenSSL implementation
/// is [`OpensslCertificateVerifier`]; tests of the signing core use fakes.
///
/// The implementation reads the event's staff issuers, CRLs, checks and
/// registrations in `hasura_transaction`, and takes the advisory locks on
/// the certificate's key and holder, so a registration made later in the
/// same transaction ([`register_first_use`]) can't race another account's.
#[async_trait]
pub trait CertificateVerifier: Send + Sync {
    async fn verify(
        &self,
        hasura_transaction: &Transaction<'_>,
        input: &CertificateVerificationInput<'_>,
    ) -> Result<CertificateVerification>;
}

// Parsing and identities

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// Parses every certificate of the PEMs, in order. Refuses input without a
/// certificate, and more than [`MAX_CHAIN_CERTIFICATES`].
pub fn parse_chain(chain_pem: &[String]) -> Result<Vec<X509>> {
    let mut certificates = vec![];
    for pem in chain_pem {
        let stack = X509::stack_from_pem(pem.as_bytes()).context("unreadable certificate")?;
        if stack.is_empty() {
            bail!("unreadable certificate");
        }
        certificates.extend(stack);
        if certificates.len() > MAX_CHAIN_CERTIFICATES {
            bail!("more than {MAX_CHAIN_CERTIFICATES} certificates");
        }
    }
    if certificates.is_empty() {
        bail!("no certificate");
    }
    Ok(certificates)
}

/// Parses PEM text (one or more certificates) or the base64 of one DER
/// certificate (a `.cer`/`.der` file).
pub fn parse_pem_or_der(pem: Option<&str>, der_base64: Option<&str>) -> Result<Vec<X509>> {
    match (pem, der_base64) {
        (Some(pem), None) => parse_chain(&[pem.to_owned()]),
        (None, Some(der_base64)) => {
            use base64::Engine;
            let compact: String = der_base64.split_whitespace().collect();
            let der = base64::engine::general_purpose::STANDARD
                .decode(compact)
                .context("the DER certificate is not valid base64")?;
            Ok(vec![
                X509::from_der(&der).context("unreadable DER certificate")?
            ])
        }
        _ => bail!("give either a PEM or a DER certificate"),
    }
}

/// The time an ASN.1 time stands for.
pub fn asn1_time_to_utc(time: &Asn1TimeRef) -> Result<DateTime<Utc>> {
    let epoch = Asn1Time::from_unix(0)?;
    let diff = epoch.diff(time)?;
    let seconds = i64::from(diff.days) * 86_400 + i64::from(diff.secs);
    Utc.timestamp_opt(seconds, 0)
        .single()
        .ok_or_else(|| anyhow!("time out of range"))
}

/// The signature algorithm a public key makes, if it is supported.
pub fn key_algorithm(key: &PKeyRef<Public>) -> Option<SignatureAlgorithm> {
    match key.id() {
        Id::RSA => Some(SignatureAlgorithm::RsaPkcs1Sha256),
        Id::EC => {
            let curve = key.ec_key().ok()?.group().curve_name();
            (curve == Some(Nid::X9_62_PRIME256V1)).then_some(SignatureAlgorithm::EcdsaP256Sha256)
        }
        _ => None,
    }
}

/// A name's attribute value compared case- and spacing-insensitively:
/// trimmed, inner whitespace collapsed, lowercased.
fn canonical_value(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// A distinguished name in a canonical form, like OpenSSL's
/// `x509_name_canon`: per attribute, its OID and its string value (from any
/// string type) canonicalized, in RDN order. Values that are no string keep
/// their bytes, in hex.
pub fn canonical_name(name: &X509Name<'_>) -> Vec<u8> {
    let mut out = String::new();
    for rdn in name.iter() {
        out.push('/');
        for (index, attribute) in rdn.iter().enumerate() {
            if index > 0 {
                out.push('+');
            }
            let value = match attribute.as_str() {
                Ok(text) => Some(text.to_owned()),
                Err(_) if attribute.attr_value().tag() == Tag::BmpString => {
                    let units: Vec<u16> = attribute
                        .as_slice()
                        .chunks(2)
                        .map(|pair| u16::from_be_bytes([pair[0], *pair.get(1).unwrap_or(&0)]))
                        .collect();
                    String::from_utf16(&units).ok()
                }
                Err(_) => None,
            };
            let value = match value {
                Some(text) => canonical_value(&text)
                    .replace('\\', "\\\\")
                    .replace('/', "\\/")
                    .replace('+', "\\+"),
                None => format!("#{}", hex::encode(attribute.as_slice())),
            };
            out.push_str(&attribute.attr_type().to_id_string());
            out.push('=');
            out.push_str(&value);
        }
    }
    out.into_bytes()
}

/// The first common name of a certificate's subject, or the whole subject.
fn common_name(certificate: &X509Certificate<'_>) -> String {
    certificate
        .subject()
        .iter_common_name()
        .next()
        .and_then(|cn| cn.as_str().ok())
        .map(str::to_owned)
        .unwrap_or_else(|| certificate.subject().to_string())
}

impl CertificateIdentity {
    /// The identities of `certificate`; `chain` is what the browser sent.
    pub fn of(certificate: &X509Ref, chain: &[X509]) -> Result<Self> {
        let der = certificate.to_der()?;
        let (_, parsed) = X509Certificate::from_der(&der)
            .map_err(|err| anyhow!("unreadable certificate: {err}"))?;
        let validity = parsed.validity();
        let timestamp = |seconds: i64| {
            Utc.timestamp_opt(seconds, 0)
                .single()
                .ok_or_else(|| anyhow!("time out of range"))
        };
        let mut chain_pem = String::new();
        for member in chain {
            chain_pem.push_str(std::str::from_utf8(&member.to_pem()?)?);
        }
        Ok(CertificateIdentity {
            fingerprint_sha256: sha256_hex(&der),
            spki_sha256: sha256_hex(parsed.public_key().raw),
            holder_sha256: sha256_hex(&canonical_name(parsed.subject())),
            serial: certificate
                .serial_number()
                .to_bn()?
                .to_hex_str()?
                .to_string(),
            subject: parsed.subject().to_string(),
            common_name: common_name(&parsed),
            issuer: parsed.issuer().to_string(),
            not_before: timestamp(validity.not_before.timestamp())?,
            not_after: timestamp(validity.not_after.timestamp())?,
            pem: String::from_utf8(certificate.to_pem()?)?,
            chain_pem,
            key_algorithm: key_algorithm(&*certificate.public_key()?),
        })
    }

    /// The first 16 hex digits of the fingerprint, for descriptions.
    pub fn short_fingerprint(&self) -> &str {
        self.fingerprint_sha256
            .get(..16)
            .unwrap_or(&self.fingerprint_sha256)
    }

    /// The certificate as the log entries describe it.
    pub fn log_details(&self) -> serde_json::Value {
        json!({
            "subject": self.subject,
            "issuer": self.issuer,
            "serial": self.serial,
            "fingerprint": self.fingerprint_sha256,
        })
    }
}

/// Whether `certificate` says it is a CA (basicConstraints CA:TRUE).
pub fn is_ca(certificate: &X509Ref) -> Result<bool> {
    let der = certificate.to_der()?;
    let (_, parsed) =
        X509Certificate::from_der(&der).map_err(|err| anyhow!("unreadable certificate: {err}"))?;
    Ok(parsed
        .basic_constraints()
        .map_err(|err| anyhow!("unreadable basic constraints: {err}"))?
        .is_some_and(|constraints| constraints.value.ca))
}

// Signatures

/// Whether `signature` is a signature of `data` by `certificate`'s key with
/// `algorithm`. An algorithm that doesn't fit the key, or a malformed
/// signature, is not a valid signature.
pub fn verify_signature(
    certificate: &X509Ref,
    algorithm: SignatureAlgorithm,
    data: &[u8],
    signature: &[u8],
) -> Result<bool> {
    let key = certificate.public_key()?;
    if key_algorithm(&key) != Some(algorithm) {
        return Ok(false);
    }
    let mut verifier = Verifier::new(MessageDigest::sha256(), &key)?;
    if algorithm == SignatureAlgorithm::RsaPkcs1Sha256 {
        verifier.set_rsa_padding(Padding::PKCS1)?;
    }
    verifier.update(data)?;
    Ok(verifier.verify(signature).unwrap_or(false))
}

// The chain

/// Builds the path from `leaf` to one of `anchors`, with `intermediates` as
/// untrusted helpers. Time is not checked here (it is `valid-now`), nor is
/// the purpose (a signing certificate may carry clientAuth EKUs only).
/// `Ok(Err(reason))` when there is no trusted path.
pub fn verified_path(
    leaf: &X509,
    intermediates: &[X509],
    anchors: &[X509],
) -> Result<std::result::Result<Vec<X509>, String>> {
    if anchors.is_empty() {
        return Ok(Err("the event has no trusted issuers".to_owned()));
    }
    let mut builder = X509StoreBuilder::new()?;
    for anchor in anchors {
        builder.add_cert(anchor.clone())?;
    }
    let mut param = X509VerifyParam::new()?;
    param.set_flags(X509VerifyFlags::PARTIAL_CHAIN | X509VerifyFlags::NO_CHECK_TIME)?;
    param.set_purpose(X509PurposeId::ANY)?;
    param.set_auth_level(CHAIN_AUTH_LEVEL);
    builder.set_param(&param)?;
    let store = builder.build();
    let mut untrusted = Stack::new()?;
    for intermediate in intermediates {
        untrusted.push(intermediate.clone())?;
    }
    let mut context = X509StoreContext::new()?;
    let path = context.init(&store, leaf, &untrusted, |context| {
        if context.verify_cert()? {
            Ok(Ok(context
                .chain()
                .map(|chain| chain.iter().map(X509Ref::to_owned).collect::<Vec<_>>())
                .unwrap_or_default()))
        } else {
            Ok(Err(context.error().error_string().to_owned()))
        }
    })?;
    // A certificate authority, trusted or not, doesn't sign as a person;
    // nor does a certificate whose constraints can't be read.
    let leaf_is_ca = is_ca(leaf).unwrap_or(true);
    Ok(path.and_then(|path| {
        if path.len() < 2 || leaf_is_ca {
            Err("a certificate authority can't sign as a person".to_owned())
        } else {
            Ok(path)
        }
    }))
}

/// The root CA above `anchor` among the trusted `issuers`: each certificate's
/// issuer in turn, by name and signature, up to a self-signed one or one
/// whose issuer isn't trusted. A verified path stops at the first trusted
/// issuer, which may be an issuing CA under a trusted root.
pub fn trusted_root<'a>(anchor: &'a X509Ref, issuers: &'a [X509]) -> &'a X509Ref {
    let mut current = anchor;
    for _ in 0..issuers.len() {
        if names_equal(current.issuer_name(), current.subject_name()) {
            break;
        }
        let parent = issuers.iter().find(|candidate| {
            names_equal(candidate.subject_name(), current.issuer_name())
                && candidate
                    .public_key()
                    .and_then(|key| current.verify(&key))
                    .unwrap_or(false)
        });
        match parent {
            Some(parent) => current = parent,
            None => break,
        }
    }
    current
}

fn names_equal(a: &openssl::x509::X509NameRef, b: &openssl::x509::X509NameRef) -> bool {
    matches!((a.to_der(), b.to_der()), (Ok(a), Ok(b)) if a == b)
}

/// A revocation list as stored (`staff_crl`).
#[derive(Debug, Clone, PartialEq)]
pub struct StoredCrl {
    /// The distribution point it was downloaded from.
    pub url: String,
    pub der: Option<Vec<u8>>,
    /// When it was last downloaded and accepted.
    pub fetched_at: DateTime<Utc>,
}

/// A stored list, parsed.
struct StoredList {
    crl: X509Crl,
    scope: ListScope,
    url: String,
    fetched_at: DateTime<Utc>,
}

/// The lists that speak for `certificate`: signed by `issuer`, from one of
/// the certificate's distribution points (any of the issuer's when it names
/// none), complete (not delta, partitioned or indirect) for a certificate
/// of its kind.
fn lists_for<'a>(
    lists: &'a [StoredList],
    certificate: &X509Ref,
    issuer: &X509Ref,
) -> Vec<&'a StoredList> {
    let Ok(key) = issuer.public_key() else {
        return vec![];
    };
    let points = distribution_points(certificate);
    let certificate_is_ca = is_ca(certificate).unwrap_or(false);
    lists
        .iter()
        .filter(|list| {
            names_equal(list.crl.issuer_name(), issuer.subject_name())
                && list.crl.verify(&key).unwrap_or(false)
                && (points.is_empty() || points.contains(&list.url))
                && list.scope.covers(certificate_is_ca, &points)
        })
        .collect()
}

fn check(id: CertificateCheckId, ok: bool, detail: impl Into<String>) -> CertificateCheckResult {
    CertificateCheckResult {
        id,
        ok,
        detail: Some(detail.into()),
    }
}

/// A passed check whose detail the dialog doesn't show, or shows as a
/// placeholder of its own text.
fn passed(id: CertificateCheckId, detail: Option<String>) -> CertificateCheckResult {
    CertificateCheckResult {
        id,
        ok: true,
        detail,
    }
}

fn day(time: DateTime<Utc>) -> String {
    time.format("%Y-%m-%d").to_string()
}

/// `not-revoked` over the non-anchor certificates of `path`.
fn revocation_check(
    path: &[X509],
    crls: &[StoredCrl],
    checks: &SigningChecks,
    now: DateTime<Utc>,
) -> Result<(CertificateCheckResult, RevocationStatus)> {
    if checks.revocation_check == RevocationCheck::DontCheck {
        return Ok((
            passed(CertificateCheckId::NotRevoked, None),
            RevocationStatus::Unchecked,
        ));
    }
    let lists: Vec<StoredList> = crls
        .iter()
        .filter_map(|stored| {
            let der = stored.der.as_deref()?;
            Some(StoredList {
                crl: X509Crl::from_der(der).ok()?,
                scope: list_scope(der).ok()?,
                url: stored.url.clone(),
                fetched_at: stored.fetched_at,
            })
        })
        .collect();
    let mut missing = vec![];
    let mut oldest_fetch: Option<DateTime<Utc>> = None;
    for pair in path.windows(2) {
        let (certificate, issuer) = (&pair[0], &pair[1]);
        let usable = lists_for(&lists, certificate, issuer);
        // A revocation stays one in a list that is no longer current.
        if usable
            .iter()
            .any(|list| matches!(list.crl.get_by_cert(certificate), CrlStatus::Revoked(_)))
        {
            let name = CertificateIdentity::of(certificate, &[])
                .map(|identity| identity.common_name)
                .unwrap_or_default();
            return Ok((
                check(
                    CertificateCheckId::NotRevoked,
                    false,
                    format!("{name} is revoked"),
                ),
                RevocationStatus::Checked,
            ));
        }
        match usable
            .iter()
            .filter(|list| list.scope.is_current(now))
            .map(|list| list.fetched_at)
            .max()
        {
            Some(fetched_at) => {
                oldest_fetch =
                    Some(oldest_fetch.map_or(fetched_at, |oldest| oldest.min(fetched_at)))
            }
            None => missing.push(
                CertificateIdentity::of(certificate, &[])
                    .map(|identity| identity.common_name)
                    .unwrap_or_default(),
            ),
        }
    }
    if missing.is_empty() {
        // The detail is when the oldest list used was downloaded.
        return Ok((
            passed(
                CertificateCheckId::NotRevoked,
                oldest_fetch.map(|fetched_at| fetched_at.to_rfc3339()),
            ),
            RevocationStatus::Checked,
        ));
    }
    let missing = missing.join(", ");
    Ok(match checks.crl_unavailable {
        CrlUnavailablePolicy::Refuse => (
            check(
                CertificateCheckId::NotRevoked,
                false,
                format!("no current revocation list for {missing}"),
            ),
            RevocationStatus::Unchecked,
        ),
        CrlUnavailablePolicy::AcceptUnchecked => (
            passed(CertificateCheckId::NotRevoked, None),
            RevocationStatus::Unchecked,
        ),
    })
}

/// `valid-now` over every certificate of the path.
pub(crate) fn validity_check(path: &[X509], now: DateTime<Utc>) -> Result<CertificateCheckResult> {
    for (index, certificate) in path.iter().enumerate() {
        let identity = CertificateIdentity::of(certificate, &[])?;
        let whose = if index == 0 {
            "the certificate".to_owned()
        } else {
            format!("the issuer {}", identity.common_name)
        };
        if now < identity.not_before {
            return Ok(check(
                CertificateCheckId::ValidNow,
                false,
                format!("{whose} is not valid before {}", day(identity.not_before)),
            ));
        }
        if now > identity.not_after {
            return Ok(check(
                CertificateCheckId::ValidNow,
                false,
                format!("{whose} expired on {}", day(identity.not_after)),
            ));
        }
    }
    let leaf = CertificateIdentity::of(&path[0], &[])?;
    Ok(check(
        CertificateCheckId::ValidNow,
        true,
        format!("valid until {}", day(leaf.not_after)),
    ))
}

/// `signing-key-usage`: the key usage extension allows digitalSignature or
/// nonRepudiation; an extended key usage, if any, allows signing documents;
/// the key is one the server verifies (RSA of [`MIN_RSA_BITS`] or more, EC
/// P-256).
pub(crate) fn key_usage_check(
    leaf: &X509Ref,
    identity: &CertificateIdentity,
) -> Result<CertificateCheckResult> {
    let der = leaf.to_der()?;
    let (_, parsed) =
        X509Certificate::from_der(&der).map_err(|err| anyhow!("unreadable certificate: {err}"))?;
    let signs = match parsed.key_usage() {
        Ok(Some(usage)) => usage.value.digital_signature() || usage.value.non_repudiation(),
        _ => false,
    };
    let extended_signs = match parsed.extended_key_usage() {
        Ok(None) => true,
        Ok(Some(usage)) => {
            let usage = usage.value;
            usage.any
                || usage.client_auth
                || usage.email_protection
                || usage
                    .other
                    .iter()
                    .any(|oid| SIGNING_EXTENDED_KEY_USAGES.contains(&oid.to_id_string().as_str()))
        }
        Err(_) => false,
    };
    let key = leaf.public_key()?;
    let weak = key.id() == Id::RSA && key.bits() < MIN_RSA_BITS;
    let refused = |detail: &str| check(CertificateCheckId::SigningKeyUsage, false, detail);
    Ok(if !signs {
        refused("the certificate is not made for signing")
    } else if !extended_signs {
        refused("the certificate's extended key usage doesn't allow signing")
    } else if identity.key_algorithm.is_none() {
        refused("unsupported key: only RSA and EC P-256 keys sign")
    } else if weak {
        refused("the key is too weak: RSA keys need 2048 bits or more")
    } else {
        check(
            CertificateCheckId::SigningKeyUsage,
            true,
            "made for signing",
        )
    })
}

// Registrations

/// Everything [`evaluate`] reads besides the input.
#[derive(Debug, Clone)]
pub struct VerificationContext {
    /// The event's staff issuers: the trust anchors.
    pub issuers: Vec<X509>,
    /// The event's stored revocation lists.
    pub crls: Vec<StoredCrl>,
    pub checks: SigningChecks,
    /// Active registrations with the certificate's fingerprint in the event,
    /// or its key or holder anywhere in the tenant.
    pub registrations: Vec<StaffCertificateRow>,
    /// Revoked registrations with the certificate's fingerprint or key
    /// anywhere in the tenant: a revoked certificate or key never signs.
    pub revoked: Vec<StaffCertificateRow>,
    /// The request's approvals so far.
    pub approvals: Vec<SigningApprovalRow>,
    /// The Posts the certificate's key signed Post-scoped requests for in
    /// the event, through any account.
    pub signed_posts: Vec<uuid::Uuid>,
}

/// The account a registration belongs to: the first account for a link.
pub(crate) fn root_account(row: &StaffCertificateRow) -> &str {
    row.linked_to.as_deref().unwrap_or(&row.user_id)
}

/// Which identity of `identity` a registration shares.
fn shared_identity(
    row: &StaffCertificateRow,
    identity: &CertificateIdentity,
) -> Option<&'static str> {
    if row.fingerprint_sha256 == identity.fingerprint_sha256 {
        Some("certificate")
    } else if row.spki_sha256 == identity.spki_sha256 {
        Some("key")
    } else if row.holder_sha256 == identity.holder_sha256 {
        Some("holder")
    } else {
        None
    }
}

/// Active registrations of the certificate's fingerprint, key or holder to
/// an account other than `user_id` and not linked to it by a Security
/// Officer (§5a rule 2).
pub fn conflicting_registrations<'a>(
    registrations: &'a [StaffCertificateRow],
    identity: &CertificateIdentity,
    user_id: &str,
) -> Vec<&'a StaffCertificateRow> {
    let shares = |row: &&StaffCertificateRow| shared_identity(row, identity).is_some();
    let mut roots: BTreeSet<&str> = BTreeSet::from([user_id]);
    roots.extend(
        registrations
            .iter()
            .filter(shares)
            .filter(|row| row.user_id == user_id)
            .map(root_account),
    );
    registrations
        .iter()
        .filter(shares)
        .filter(|row| row.user_id != user_id && !roots.contains(root_account(row)))
        .collect()
}

/// Active registrations of the certificate's fingerprint, key or holder to
/// any account but `user_id`, linked or not. First use registers only when
/// there is none: linking a second account is a Security Officer's.
pub fn registrations_of_others<'a>(
    registrations: &'a [StaffCertificateRow],
    identity: &CertificateIdentity,
    user_id: &str,
) -> Vec<&'a StaffCertificateRow> {
    registrations
        .iter()
        .filter(|row| shared_identity(row, identity).is_some() && row.user_id != user_id)
        .collect()
}

fn is_post_scoped(scope: SigningScope) -> bool {
    matches!(scope, SigningScope::Post | SigningScope::PostAndCountry)
}

/// The actions whose requests a Post binds a key to.
pub fn post_scoped_actions() -> Vec<String> {
    SigningAction::iter()
        .filter(|action| is_post_scoped(action.scope()))
        .map(|action| action.to_string())
        .collect()
}

/// The Post a registration made for `request` binds the certificate to.
pub fn registration_post(request: &SigningRequestRow) -> Option<uuid::Uuid> {
    if is_post_scoped(request.action.scope()) {
        request.election_id
    } else {
        None
    }
}

/// Every check of `input` against `context`. No database, no network. A
/// certificate the parsers can't read fails `trusted-issuer`.
pub fn evaluate(
    context: &VerificationContext,
    input: &CertificateVerificationInput<'_>,
) -> Result<CertificateVerification> {
    Ok(evaluate_readable(context, input).unwrap_or_else(|err| unreadable(format!("{err:#}"))))
}

fn unreadable(detail: String) -> CertificateVerification {
    CertificateVerification {
        checks: vec![check(CertificateCheckId::TrustedIssuer, false, detail)],
        ..CertificateVerification::default()
    }
}

/// The PEMs of `certificates`, one after another.
fn pem_chain(certificates: &[X509]) -> Result<String> {
    let mut pem = String::new();
    for certificate in certificates {
        pem.push_str(std::str::from_utf8(&certificate.to_pem()?)?);
    }
    Ok(pem)
}

fn evaluate_readable(
    context: &VerificationContext,
    input: &CertificateVerificationInput<'_>,
) -> Result<CertificateVerification> {
    let chain = parse_chain(&input.chain_pem)?;
    let (leaf, intermediates) = (&chain[0], &chain[1..]);
    let mut identity = CertificateIdentity::of(leaf, &[])?;
    let request = input.request;
    let signer = &input.signer.user_id;
    let mut checks = vec![];

    // The chain, its dates and revocation, over the verified path; without
    // one, the leaf alone. Only the verified path is kept as the chain.
    let path = verified_path(leaf, intermediates, &context.issuers)?;
    let revocation_status = match &path {
        Ok(path) => {
            identity.chain_pem = pem_chain(path)?;
            // The detail is the root CA's name.
            let root = CertificateIdentity::of(
                trusted_root(&path[path.len() - 1], &context.issuers),
                &[],
            )?;
            checks.push(passed(
                CertificateCheckId::TrustedIssuer,
                Some(root.common_name),
            ));
            checks.push(validity_check(path, input.now)?);
            checks.push(key_usage_check(leaf, &identity)?);
            let (result, status) =
                revocation_check(path, &context.crls, &context.checks, input.now)?;
            checks.push(result);
            status
        }
        Err(reason) => {
            identity.chain_pem = pem_chain(std::slice::from_ref(leaf))?;
            checks.push(check(
                CertificateCheckId::TrustedIssuer,
                false,
                format!("not issued by a trusted issuer: {reason}"),
            ));
            checks.push(validity_check(std::slice::from_ref(leaf), input.now)?);
            checks.push(key_usage_check(leaf, &identity)?);
            checks.push(check(
                CertificateCheckId::NotRevoked,
                false,
                "not checked without a trusted issuer",
            ));
            RevocationStatus::Unchecked
        }
    };

    // Registration (§5a). A revoked certificate or key never signs again.
    let own = context.registrations.iter().find(|row| {
        row.user_id == *signer
            && row.election_event_id == request.election_event_id
            && row.fingerprint_sha256 == identity.fingerprint_sha256
    });
    let conflicts = conflicting_registrations(&context.registrations, &identity, signer);
    let registration = if !context.revoked.is_empty() {
        checks.push(check(
            CertificateCheckId::Registered,
            false,
            "this certificate or its key was revoked",
        ));
        RegistrationState::NotRegistered
    } else if let Some(row) = own {
        // The detail is when it was registered to the signer.
        checks.push(passed(
            CertificateCheckId::Registered,
            Some(row.registered_at.to_rfc3339()),
        ));
        RegistrationState::Registered(row.clone())
    } else if context.checks.registration == CertificateRegistration::SecurityOfficerOnly {
        checks.push(check(
            CertificateCheckId::Registered,
            false,
            "not registered to you; a Security Officer registers certificates",
        ));
        RegistrationState::NotRegistered
    } else if !registrations_of_others(&context.registrations, &identity, signer).is_empty() {
        if conflicts.is_empty() {
            // Registered to another account the signer is linked to: only a
            // Security Officer links this one too.
            checks.push(check(
                CertificateCheckId::Registered,
                false,
                "registered to another account; a Security Officer must link it",
            ));
        } else {
            // Someone else's: it can't be registered to the signer either,
            // and `registered-to-other` names the account it is bound to.
            checks.push(check(
                CertificateCheckId::Registered,
                false,
                "registered to another account",
            ));
        }
        RegistrationState::NotRegistered
    } else {
        // First use: no date yet.
        checks.push(passed(CertificateCheckId::Registered, None));
        RegistrationState::FirstUse
    };
    // The detail names the person it is registered to: their display name
    // when registered (or the username); the verifier's directory has the
    // current one.
    let other_holder = conflicts.first().map(|row| OtherHolder {
        user_id: row.user_id.clone(),
        display_name: row
            .user_display_name
            .clone()
            .unwrap_or_else(|| row.username.clone()),
    });
    checks.push(match &other_holder {
        Some(holder) => check(
            CertificateCheckId::RegisteredToOther,
            false,
            holder.display_name.clone(),
        ),
        None => passed(CertificateCheckId::RegisteredToOther, None),
    });

    // One person, one slot.
    let signed = context.approvals.iter().find_map(|approval| {
        if approval.user_id == *signer {
            Some("you already signed this request")
        } else if approval.fingerprint_sha256 == identity.fingerprint_sha256 {
            Some("this certificate already signed this request")
        } else if approval.spki_sha256 == identity.spki_sha256 {
            Some("this key already signed this request")
        } else if approval.holder_sha256 == identity.holder_sha256 {
            Some("this certificate's holder already signed this request")
        } else {
            None
        }
    });
    checks.push(match signed {
        Some(reason) => check(CertificateCheckId::AlreadySigned, false, reason),
        None => check(CertificateCheckId::AlreadySigned, true, "not signed yet"),
    });

    // One Post per key: the first Post-scoped signature binds it.
    let post = registration_post(request);
    let bound_elsewhere = match (context.checks.post_binding, post) {
        (CertificatePostBinding::OnePost, Some(post)) => {
            context.signed_posts.iter().any(|signed| *signed != post)
        }
        _ => false,
    };
    checks.push(if bound_elsewhere {
        check(
            CertificateCheckId::PostBinding,
            false,
            "this certificate signs for another Post",
        )
    } else {
        check(CertificateCheckId::PostBinding, true, "no other Post")
    });

    if let Some(signatures) = &input.signatures {
        checks.push(signature_check(leaf, request, signatures)?);
    }

    Ok(CertificateVerification {
        checks,
        certificate: Some(identity),
        revocation_status,
        registration,
        other_holder,
    })
}

/// `signature`: the payload signature, and the document signature of an
/// EML action.
fn signature_check(
    leaf: &X509Ref,
    request: &SigningRequestRow,
    signatures: &SignatureInputs,
) -> Result<CertificateCheckResult> {
    let payload_ok = verify_signature(
        leaf,
        signatures.algorithm,
        request.canonical_payload.as_bytes(),
        &signatures.payload_signature,
    )?;
    if !payload_ok {
        return Ok(check(
            CertificateCheckId::Signature,
            false,
            "the signature doesn't cover this request",
        ));
    }
    let wants_document = request.action.document() == DocumentKind::Eml;
    Ok(match (&signatures.document, wants_document) {
        (None, true) => check(
            CertificateCheckId::Signature,
            false,
            "the document signature is missing",
        ),
        (Some(_), false) => check(
            CertificateCheckId::Signature,
            false,
            "this request has no document to sign",
        ),
        (Some(document), true) => {
            let document_ok = request.document_sha256.as_deref()
                == Some(sha256_hex(&document.document).as_str())
                && verify_signature(
                    leaf,
                    signatures.algorithm,
                    &document.document,
                    &document.signature,
                )?;
            if document_ok {
                check(CertificateCheckId::Signature, true, "signature verified")
            } else {
                check(
                    CertificateCheckId::Signature,
                    false,
                    "the document signature doesn't cover this request's document",
                )
            }
        }
        (None, false) => check(CertificateCheckId::Signature, true, "signature verified"),
    })
}

/// Registers the certificate of a passed verification whose registration is
/// [`RegistrationState::FirstUse`] to the signer, and stages its
/// SigningCertificateRegistered entries. Call it in the transaction the
/// verification ran in, which must hold the event's signing lock
/// ([`crate::postgres::signing::lock_signing_event`]) taken before any
/// other lock. `signer_name` is the signer's display name.
pub async fn register_first_use(
    hasura_transaction: &Transaction<'_>,
    input: &CertificateVerificationInput<'_>,
    verification: &CertificateVerification,
    signer_name: Option<&str>,
) -> Result<StaffCertificateRow> {
    let identity = match (&verification.certificate, &verification.registration) {
        (Some(identity), RegistrationState::FirstUse) if verification.passed() => identity,
        _ => bail!("only a passed first-use verification registers a certificate"),
    };
    let request = input.request;
    if !holds_signing_event_lock(
        hasura_transaction,
        request.tenant_id,
        request.election_event_id,
    )
    .await?
    {
        bail!("registering a certificate needs the event's signing lock");
    }
    // Self-contained: the key and holder are still nobody else's, and
    // neither the certificate nor its key was revoked, under the locks.
    lock_certificate_identities(hasura_transaction, request.tenant_id, identity).await?;
    let context = load_verification_context(hasura_transaction, request, identity).await?;
    if !registrations_of_others(&context.registrations, identity, &input.signer.user_id).is_empty()
    {
        bail!("the certificate's key or holder is registered to another account");
    }
    if !context.revoked.is_empty() {
        bail!("the certificate or its key was revoked");
    }
    let row = insert_staff_certificate(
        hasura_transaction,
        &NewStaffCertificate {
            tenant_id: request.tenant_id,
            election_event_id: request.election_event_id,
            user_id: input.signer.user_id.clone(),
            username: input.signer.username.clone(),
            user_display_name: signer_name.map(str::to_owned),
            election_id: registration_post(request),
            fingerprint_sha256: identity.fingerprint_sha256.clone(),
            spki_sha256: identity.spki_sha256.clone(),
            holder_sha256: identity.holder_sha256.clone(),
            serial: identity.serial.clone(),
            subject: identity.subject.clone(),
            issuer: identity.issuer.clone(),
            not_before: identity.not_before,
            not_after: identity.not_after,
            pem: identity.pem.clone(),
            registration: StaffCertificateRegistration::FirstUse,
            linked_to: None,
            registered_by: input.signer.user_id.clone(),
            registered_by_name: signer_name.map(str::to_owned),
        },
    )
    .await?;
    stage(
        hasura_transaction,
        &LogStep {
            kind: SigningStatementKind::SigningCertificateRegistered,
            user: input.signer.clone(),
            system: SystemOutcome::Info,
            scope: LogScope {
                tenant_id: request.tenant_id,
                election_event_id: request.election_event_id,
                election_id: request.election_id,
                area_id: request.area_id,
            },
            description: format!(
                "Registered certificate {} ({}) to {} on first use",
                identity.common_name,
                identity.short_fingerprint(),
                signer_name.unwrap_or(&input.signer.username)
            ),
            details: json!({
                "election_id": request.election_id,
                "area_id": request.area_id,
                "action": request.action,
                "request_id": request.id,
                "code": request.code,
                "certificate": identity.log_details(),
                "certificate_id": row.id,
                "user_id": row.user_id,
                "username": row.username,
                "registration": row.registration,
                "post_election_id": row.election_id,
            }),
        },
    )
    .await?;
    Ok(row)
}

// The OpenSSL verifier

/// Whether this transaction holds the event's signing lock
/// ([`crate::postgres::signing::lock_signing_event`], a 64-bit advisory
/// lock on `hashtextextended`).
pub async fn holds_signing_event_lock(
    hasura_transaction: &Transaction<'_>,
    tenant_id: uuid::Uuid,
    election_event_id: uuid::Uuid,
) -> Result<bool> {
    Ok(hasura_transaction
        .query_one(
            "SELECT EXISTS (
                 SELECT 1 FROM pg_locks
                 WHERE locktype = 'advisory' AND pid = pg_backend_pid() AND granted
                   AND objsubid = 1
                   AND ((classid::bigint << 32) | objid::bigint)
                       = hashtextextended($1, 0)
             )",
            &[&format!("signing-event:{tenant_id}:{election_event_id}")],
        )
        .await
        .context("Error reading the transaction's locks")?
        .try_get(0)?)
}

/// Takes the advisory locks on the certificate's key, then its holder,
/// tenant-wide, until the transaction ends: registrations of one key or one
/// holder run one after another. Always in this order, and after the
/// event's signing lock, so two transactions never wait for each other.
pub async fn lock_certificate_identities(
    hasura_transaction: &Transaction<'_>,
    tenant_id: uuid::Uuid,
    identity: &CertificateIdentity,
) -> Result<()> {
    lock_identity_keys(
        hasura_transaction,
        tenant_id,
        &identity.spki_sha256,
        &identity.holder_sha256,
    )
    .await
}

/// [`lock_certificate_identities`] by the hashes.
pub async fn lock_identity_keys(
    hasura_transaction: &Transaction<'_>,
    tenant_id: uuid::Uuid,
    spki_sha256: &str,
    holder_sha256: &str,
) -> Result<()> {
    for key in [
        format!("staff-certificate-spki:{tenant_id}:{spki_sha256}"),
        format!("staff-certificate-holder:{tenant_id}:{holder_sha256}"),
    ] {
        hasura_transaction
            .execute(
                "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
                &[&key],
            )
            .await
            .context("Error locking the certificate's identities")?;
    }
    Ok(())
}

/// The event's staff issuers as trust anchors. An issuer that doesn't parse
/// is left out.
pub async fn load_staff_anchors(
    hasura_transaction: &Transaction<'_>,
    tenant_id: uuid::Uuid,
    election_event_id: uuid::Uuid,
) -> Result<Vec<(StaffIssuerRow, X509)>> {
    Ok(
        list_staff_issuers(hasura_transaction, tenant_id, election_event_id)
            .await?
            .into_iter()
            .filter_map(|row| {
                let certificate = X509::from_pem(row.pem.as_bytes()).ok()?;
                Some((row, certificate))
            })
            .collect(),
    )
}

fn dedup_by_id(rows: Vec<StaffCertificateRow>) -> Vec<StaffCertificateRow> {
    let mut seen = BTreeSet::new();
    rows.into_iter().filter(|row| seen.insert(row.id)).collect()
}

/// Loads what [`evaluate`] reads for `identity` signing `request`.
pub async fn load_verification_context(
    hasura_transaction: &Transaction<'_>,
    request: &SigningRequestRow,
    identity: &CertificateIdentity,
) -> Result<VerificationContext> {
    let (tenant_id, election_event_id) = (request.tenant_id, request.election_event_id);
    let issuers = load_staff_anchors(hasura_transaction, tenant_id, election_event_id)
        .await?
        .into_iter()
        .map(|(_, certificate)| certificate)
        .collect();
    let crls = list_staff_crls(hasura_transaction, tenant_id, election_event_id)
        .await?
        .into_iter()
        .map(|row| StoredCrl {
            url: row.url,
            der: row.der,
            fetched_at: row.last_ok_at.unwrap_or(row.fetched_at),
        })
        .collect();
    let checks = get_signing_checks(hasura_transaction, tenant_id, election_event_id)
        .await?
        .map(|row| row.checks)
        .unwrap_or_default();
    let mut registrations = find_active_staff_certificates_by_fingerprint(
        hasura_transaction,
        tenant_id,
        election_event_id,
        &identity.fingerprint_sha256,
    )
    .await?;
    registrations.extend(
        find_active_staff_certificates_by_spki(
            hasura_transaction,
            tenant_id,
            &identity.spki_sha256,
        )
        .await?,
    );
    registrations.extend(
        find_active_staff_certificates_by_holder(
            hasura_transaction,
            tenant_id,
            &identity.holder_sha256,
        )
        .await?,
    );
    let mut revoked = find_revoked_staff_certificates_by_fingerprint(
        hasura_transaction,
        tenant_id,
        &identity.fingerprint_sha256,
    )
    .await?;
    revoked.extend(
        find_revoked_staff_certificates_by_spki(
            hasura_transaction,
            tenant_id,
            &identity.spki_sha256,
        )
        .await?,
    );
    let approvals =
        list_signing_approvals(hasura_transaction, tenant_id, election_event_id, request.id)
            .await?;
    let signed_posts = posts_signed_by_key(
        hasura_transaction,
        tenant_id,
        election_event_id,
        &identity.spki_sha256,
        &post_scoped_actions(),
    )
    .await?;
    Ok(VerificationContext {
        issuers,
        crls,
        checks,
        registrations: dedup_by_id(registrations),
        revoked: dedup_by_id(revoked),
        approvals,
        signed_posts,
    })
}

/// Checks certificates with OpenSSL against the event's staff issuers, and
/// names the person a certificate is registered to from `directory`.
#[derive(Clone)]
pub struct OpensslCertificateVerifier {
    directory: Arc<dyn UserDirectory>,
}

impl Default for OpensslCertificateVerifier {
    /// Names people from the tenant realm's Keycloak users.
    fn default() -> Self {
        Self::with_directory(Arc::new(KeycloakUserDirectory::default()))
    }
}

impl OpensslCertificateVerifier {
    pub fn with_directory(directory: Arc<dyn UserDirectory>) -> Self {
        OpensslCertificateVerifier { directory }
    }
}

#[async_trait]
impl CertificateVerifier for OpensslCertificateVerifier {
    async fn verify(
        &self,
        hasura_transaction: &Transaction<'_>,
        input: &CertificateVerificationInput<'_>,
    ) -> Result<CertificateVerification> {
        let identity = parse_chain(&input.chain_pem)
            .ok()
            .and_then(|chain| CertificateIdentity::of(&chain[0], &chain).ok());
        let Some(identity) = identity else {
            // Unreadable: evaluate reports it without reading anything.
            return evaluate(&VerificationContext::empty(), input);
        };
        lock_certificate_identities(hasura_transaction, input.request.tenant_id, &identity).await?;
        let context =
            load_verification_context(hasura_transaction, input.request, &identity).await?;
        let mut verification = evaluate(&context, input)?;
        // `registered-to-other` names the person: their current display name.
        if let Some(holder) = &mut verification.other_holder {
            let names = self
                .directory
                .display_names(
                    input.request.tenant_id,
                    std::slice::from_ref(&holder.user_id),
                )
                .await?;
            if let Some(name) = names.get(&holder.user_id) {
                holder.display_name = name.clone();
                if let Some(check) = verification
                    .checks
                    .iter_mut()
                    .find(|check| check.id == CertificateCheckId::RegisteredToOther)
                {
                    check.detail = Some(name.clone());
                }
            }
        }
        Ok(verification)
    }
}

impl VerificationContext {
    /// No issuers, lists, registrations or approvals; the default checks.
    pub fn empty() -> Self {
        VerificationContext {
            issuers: vec![],
            crls: vec![],
            checks: SigningChecks::default(),
            registrations: vec![],
            revoked: vec![],
            approvals: vec![],
            signed_posts: vec![],
        }
    }
}
