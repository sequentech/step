// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The receiver's check of an ER transmission package (SW-F-0226): that the
//! data received is what the OVCS sent. Offline and without Java: it opens
//! the package with the destination's private key, compares the EML's
//! SHA-256 with the one the package declares (and the one the page shows),
//! checks the OVCS signature with the OVCS public key, and each board
//! member's signature, with its certificate and chain when given.
//!
//! The package is the `all_servers.zip` the OVCS generates, with one folder
//! per destination holding `er_<station>.zip`, or one destination's
//! `er_<station>.zip` on its own. Each of those holds `er_<station>.exz`, the
//! sealed and compressed EML, and `er_<station>.json`, the [`ACMJson`].

use super::eml_types::{ACMJson, ACMTrustee};
use super::sealing::{ecies_decrypt, openssl_enc_decrypt, SealingError};
use crate::services::signing::certificates::{key_algorithm, verified_path};
use anyhow::{anyhow, bail, Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use openssl::hash::MessageDigest;
use openssl::pkey::{HasPublic, PKey, PKeyRef, Private, Public};
use openssl::rsa::Padding;
use openssl::sign::Verifier;
use openssl::x509::X509;
use roxmltree::{Document, Node};
use sequent_core::signing::SignatureAlgorithm;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt;
use std::io::{Cursor, Read};
use x509_parser::prelude::{FromDer, X509Certificate};
use xz2::read::XzDecoder;
use zip::ZipArchive;

/// The largest member of a package that is read, and the largest EML.
pub const MAX_PACKAGE_MEMBER_BYTES: u64 = 64 * 1024 * 1024;
/// The prefix of an ER file's names (`al_` is the logs').
pub const ER_FILE_PREFIX: &str = "er_";
const ZIP_EXTENSION: &str = ".zip";
const JSON_EXTENSION: &str = ".json";
const EXZ_EXTENSION: &str = ".exz";
const SHA256_HEX_LEN: usize = 64;

const EML_ROOT: &str = "EML";
const EML_TRANSACTION_ID: &str = "TransactionId";
const EML_ELECTION: &str = "Election";
const EML_ELECTION_IDENTIFIER: &str = "ElectionIdentifier";
const EML_ELECTION_NAME: &str = "ElectionName";
const EML_CONTEST: &str = "Contest";
const EML_CONTEST_IDENTIFIER: &str = "ContestIdentifier";
const EML_CONTEST_NAME: &str = "ContestName";
const EML_COUNT_METRIC: &str = "CountMetric";
const EML_SELECTION: &str = "Selection";
const EML_CANDIDATE: &str = "Candidate";
const EML_CANDIDATE_IDENTIFIER: &str = "CandidateIdentifier";
const EML_CANDIDATE_NAME: &str = "CandidateName";
const EML_VALID_VOTES: &str = "ValidVotes";
const EML_ID_NUMBER: &str = "IdNumber";
const EML_METRIC_ID: &str = "Id";
const EML_METRIC_TYPE: &str = "Type";

/// What the board members' signatures are checked against.
#[derive(Debug, Clone)]
pub enum MemberTrust {
    /// The key each member's line in the package carries.
    PackageKeys,
    /// That key must also be one of these certificates'.
    Certificates(Vec<X509>),
    /// That key must be one of `certificates`', and the certificate must
    /// chain to `ca` (the event's root and intermediate CAs). With no
    /// matching certificate, a member is refused.
    ChainedCertificates {
        ca: Vec<X509>,
        certificates: Vec<X509>,
    },
}

/// What the receiver brings to the check.
#[derive(Debug, Clone)]
pub struct VerifyOptions {
    destination_key: PKey<Private>,
    destination_key_sha256: String,
    destination: Option<String>,
    ovcs_public_key: PKey<Public>,
    member_trust: MemberTrust,
    required_signatures: usize,
    expected_sha256: Option<String>,
}

impl VerifyOptions {
    /// The destination's private key and the OVCS public key, as PEM.
    pub fn new(destination_key_pem: &[u8], ovcs_public_key_pem: &[u8]) -> Result<Self> {
        let destination_key = PKey::private_key_from_pem(destination_key_pem)
            .context("the destination key is not a PEM private key")?;
        if destination_key.ec_key().is_err() {
            bail!("the destination key is not an elliptic curve key");
        }
        let ovcs_public_key = PKey::public_key_from_pem(ovcs_public_key_pem)
            .context("the OVCS key is not a PEM public key")?;
        let destination_key_sha256 = public_key_sha256(&destination_key)?;
        Ok(VerifyOptions {
            destination_key,
            destination_key_sha256,
            destination: None,
            ovcs_public_key,
            member_trust: MemberTrust::PackageKeys,
            required_signatures: 0,
            expected_sha256: None,
        })
    }

    /// The destination (the package's folder) to open; by default, the
    /// first whose package opens with the key.
    pub fn with_destination(mut self, destination: &str) -> Self {
        self.destination = Some(destination.to_string());
        self
    }

    /// The CA certificates (PEM, root and intermediates) and the board
    /// members' certificates (PEM) to check the members' keys against.
    pub fn with_member_certificates(
        mut self,
        ca_pem: Option<&[u8]>,
        certificates_pem: Option<&[u8]>,
    ) -> Result<Self> {
        let certificates = certificates_pem
            .map(|pem| read_certificates(pem, "the board members' certificates"))
            .transpose()?;
        self.member_trust = match (ca_pem, certificates) {
            (None, None) => MemberTrust::PackageKeys,
            (None, Some(certificates)) => MemberTrust::Certificates(certificates),
            (Some(ca_pem), certificates) => MemberTrust::ChainedCertificates {
                ca: read_certificates(ca_pem, "the CA certificates")?,
                certificates: certificates.unwrap_or_default(),
            },
        };
        Ok(self)
    }

    /// How many board members (distinct keys) must have signed.
    pub fn with_required_signatures(mut self, required: usize) -> Self {
        self.required_signatures = required;
        self
    }

    /// The SHA-256 the receiver was given for the EML, in hex; case and
    /// spaces are ignored.
    pub fn with_expected_sha256(mut self, expected: &str) -> Result<Self> {
        self.expected_sha256 = Some(normalize_sha256(expected)?);
        Ok(self)
    }

    /// The uppercase hex SHA-256 of the destination's public key (its DER
    /// SubjectPublicKeyInfo).
    pub fn destination_key_sha256(&self) -> &str {
        &self.destination_key_sha256
    }
}

fn read_certificates(pem: &[u8], what: &str) -> Result<Vec<X509>> {
    let certificates =
        X509::stack_from_pem(pem).with_context(|| format!("{what} are not PEM certificates"))?;
    if certificates.is_empty() {
        bail!("{what} hold no certificate");
    }
    Ok(certificates)
}

/// A SHA-256 in hex as people copy it: in groups, in either case.
pub fn normalize_sha256(text: &str) -> Result<String> {
    let compact: String = text
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>()
        .to_ascii_uppercase();
    if compact.len() != SHA256_HEX_LEN || !compact.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(anyhow!("{text:?} is not a SHA-256 in hex"));
    }
    Ok(compact)
}

/// The uppercase hex SHA-256 of a key's DER SubjectPublicKeyInfo.
pub fn public_key_sha256<T: HasPublic>(key: &PKeyRef<T>) -> Result<String> {
    Ok(hex::encode_upper(Sha256::digest(
        key.public_key_to_der()
            .context("Error encoding the public key")?,
    )))
}

/// A SHA-256 in uppercase hex, in groups of 8 as the page shows it.
pub fn grouped_sha256(sha256: &str) -> String {
    sha256
        .as_bytes()
        .chunks(8)
        .map(|chunk| String::from_utf8_lossy(chunk).into_owned())
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureStatus {
    Valid,
    Invalid,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CertificateStatus {
    /// No certificates were given, or the member did not sign.
    NotChecked,
    /// The member's key is this certificate's.
    Matched { subject: String },
    /// ... and the certificate chains to the CA.
    Chained { subject: String },
    /// No certificate given has the member's key.
    NotFound,
    /// The certificate with the member's key does not chain to the CA.
    NotChained { subject: String, reason: String },
}

impl CertificateStatus {
    fn accepted(&self) -> bool {
        matches!(
            self,
            CertificateStatus::NotChecked
                | CertificateStatus::Matched { .. }
                | CertificateStatus::Chained { .. }
        )
    }
}

/// A board member's line in the package, checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignerReport {
    pub id: String,
    pub name: String,
    /// The uppercase hex SHA-256 of the member's key, when readable.
    pub key_sha256: Option<String>,
    pub signature: SignatureStatus,
    pub certificate: CertificateStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateRef {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionCount {
    pub candidates: Vec<CandidateRef>,
    pub valid_votes: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CountMetricValue {
    pub id: String,
    pub kind: String,
    pub value: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContestCount {
    pub election_id: String,
    pub election_name: String,
    pub id: String,
    pub name: String,
    pub metrics: Vec<CountMetricValue>,
    pub selections: Vec<SelectionCount>,
}

/// What the package holds, and how its checks went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransmissionContents {
    /// The destination's folder; `None` for a lone `er_<station>.zip`.
    pub destination: Option<String>,
    pub destination_key_sha256: String,
    pub station_id: String,
    pub station_name: String,
    pub event_id: String,
    pub event_name: String,
    pub transaction_id: Option<String>,
    /// The SHA-256 the package declares for the EML.
    pub declared_sha256: String,
    /// The SHA-256 of the EML as opened, in uppercase hex.
    pub eml_sha256: String,
    pub signers: Vec<SignerReport>,
    /// Board members (distinct keys) whose signature is valid and accepted.
    pub valid_signatures: usize,
    pub contests: Vec<ContestCount>,
    pub eml: Vec<u8>,
}

/// Why a package is refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    UnreadablePackage(String),
    NoElectionReturns,
    UnknownDestination {
        requested: String,
        available: Vec<String>,
    },
    SealedForAnotherKey {
        destinations: Vec<String>,
    },
    UnreadableErFile(String),
    UnreadableEml(String),
    HashMismatch {
        declared: String,
        computed: String,
    },
    ExpectedHashMismatch {
        expected: String,
        computed: String,
    },
    OvcsKeyMismatch,
    OvcsSignatureInvalid,
    MemberSignatureInvalid {
        member: String,
    },
    MemberCertificateNotFound {
        member: String,
    },
    MemberCertificateNotChained {
        member: String,
        reason: String,
    },
    TooFewSignatures {
        required: usize,
        valid: usize,
    },
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Refusal::UnreadablePackage(reason) => {
                write!(f, "the package can't be read: {reason}")
            }
            Refusal::NoElectionReturns => {
                write!(f, "the package holds no election returns (er_*.zip)")
            }
            Refusal::UnknownDestination {
                requested,
                available,
            } => write!(
                f,
                "the package has no destination {requested:?}; it has {}",
                available.join(", ")
            ),
            Refusal::SealedForAnotherKey { destinations } => write!(
                f,
                "the package ({}) does not open with this key: it was sealed for another key, or changed",
                destinations.join(", ")
            ),
            Refusal::UnreadableErFile(reason) => {
                write!(f, "the election returns can't be opened: {reason}")
            }
            Refusal::UnreadableEml(reason) => {
                write!(f, "the election returns can't be read: {reason}")
            }
            Refusal::HashMismatch { declared, computed } => write!(
                f,
                "the election returns' SHA-256 is {computed}, not the {declared} the package declares"
            ),
            Refusal::ExpectedHashMismatch { expected, computed } => write!(
                f,
                "the election returns' SHA-256 is {computed}, not the {expected} expected"
            ),
            Refusal::OvcsKeyMismatch => {
                write!(f, "the package names an OVCS key other than the one given")
            }
            Refusal::OvcsSignatureInvalid => write!(
                f,
                "the OVCS signature is not valid for the election returns"
            ),
            Refusal::MemberSignatureInvalid { member } => write!(
                f,
                "the signature of {member} is not valid for the election returns"
            ),
            Refusal::MemberCertificateNotFound { member } => {
                write!(f, "no certificate given has the key of {member}")
            }
            Refusal::MemberCertificateNotChained { member, reason } => write!(
                f,
                "the certificate of {member} does not chain to the CA: {reason}"
            ),
            Refusal::TooFewSignatures { required, valid } => write!(
                f,
                "{valid} board member signature(s) accepted, {required} required"
            ),
        }
    }
}

/// A refused package: every reason found, and what could be read of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransmissionRefusal {
    pub refusals: Vec<Refusal>,
    pub contents: Option<Box<TransmissionContents>>,
}

impl TransmissionRefusal {
    fn only(refusal: Refusal) -> Self {
        TransmissionRefusal {
            refusals: vec![refusal],
            contents: None,
        }
    }
}

impl fmt::Display for TransmissionRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let reasons: Vec<String> = self.refusals.iter().map(ToString::to_string).collect();
        write!(f, "{}", reasons.join("; "))
    }
}

impl std::error::Error for TransmissionRefusal {}

/// Checks a transmission package for the destination whose private key is
/// in `options`.
pub fn verify_transmission_package(
    package: &[u8],
    options: &VerifyOptions,
) -> std::result::Result<TransmissionContents, TransmissionRefusal> {
    let opened = open_package(package, options)?;
    let mut refusals = vec![];

    let eml_sha256 = hex::encode_upper(Sha256::digest(&opened.eml));
    let declared_sha256 = opened.json.sha256_hash.trim().to_ascii_uppercase();
    if declared_sha256 != eml_sha256 {
        refusals.push(Refusal::HashMismatch {
            declared: declared_sha256.clone(),
            computed: eml_sha256.clone(),
        });
    }
    if let Some(expected) = &options.expected_sha256 {
        if *expected != eml_sha256 {
            refusals.push(Refusal::ExpectedHashMismatch {
                expected: expected.clone(),
                computed: eml_sha256.clone(),
            });
        }
    }

    if !same_key(&opened.json.publickey, &options.ovcs_public_key) {
        refusals.push(Refusal::OvcsKeyMismatch);
    }
    if !signature_is_valid(
        &options.ovcs_public_key,
        &opened.eml,
        &opened.json.signature,
    ) {
        refusals.push(Refusal::OvcsSignatureInvalid);
    }

    let signers: Vec<SignerReport> = opened
        .json
        .members
        .iter()
        .map(|member| check_member(member, &opened.eml, &options.member_trust))
        .collect();
    let mut signed_keys = BTreeSet::new();
    for signer in &signers {
        let member = format!("{} ({})", signer.name, signer.id);
        if signer.signature == SignatureStatus::Invalid {
            refusals.push(Refusal::MemberSignatureInvalid {
                member: member.clone(),
            });
        }
        match &signer.certificate {
            CertificateStatus::NotFound => {
                refusals.push(Refusal::MemberCertificateNotFound { member })
            }
            CertificateStatus::NotChained { reason, .. } => {
                refusals.push(Refusal::MemberCertificateNotChained {
                    member,
                    reason: reason.clone(),
                })
            }
            _ => {}
        }
        if signer.signature == SignatureStatus::Valid && signer.certificate.accepted() {
            if let Some(key) = &signer.key_sha256 {
                signed_keys.insert(key.clone());
            }
        }
    }
    let valid_signatures = signed_keys.len();
    if valid_signatures < options.required_signatures {
        refusals.push(Refusal::TooFewSignatures {
            required: options.required_signatures,
            valid: valid_signatures,
        });
    }

    let (transaction_id, contests) = match read_counts(&opened.eml) {
        Ok(counts) => counts,
        Err(reason) => {
            refusals.push(Refusal::UnreadableEml(reason));
            (None, vec![])
        }
    };

    let contents = TransmissionContents {
        destination: opened.destination,
        destination_key_sha256: options.destination_key_sha256.clone(),
        station_id: opened.json.station_id,
        station_name: opened.json.station_name,
        event_id: opened.json.event_id,
        event_name: opened.json.event_name,
        transaction_id,
        declared_sha256,
        eml_sha256,
        signers,
        valid_signatures,
        contests,
        eml: opened.eml,
    };
    if refusals.is_empty() {
        Ok(contents)
    } else {
        Err(TransmissionRefusal {
            refusals,
            contents: Some(Box::new(contents)),
        })
    }
}

struct OpenedPackage {
    destination: Option<String>,
    json: ACMJson,
    eml: Vec<u8>,
}

/// An ER file before its key is unsealed.
struct SealedReturns {
    json: ACMJson,
    exz: Vec<u8>,
}

fn unreadable(reason: impl fmt::Display) -> TransmissionRefusal {
    TransmissionRefusal::only(Refusal::UnreadablePackage(reason.to_string()))
}

fn open_zip(bytes: &[u8]) -> std::result::Result<ZipArchive<Cursor<&[u8]>>, TransmissionRefusal> {
    ZipArchive::new(Cursor::new(bytes)).map_err(|error| unreadable(format!("not a zip: {error}")))
}

fn read_member(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    name: &str,
) -> std::result::Result<Vec<u8>, TransmissionRefusal> {
    let file = archive
        .by_name(name)
        .map_err(|error| unreadable(format!("{name}: {error}")))?;
    if file.size() > MAX_PACKAGE_MEMBER_BYTES {
        return Err(unreadable(format!("{name} is too large")));
    }
    let mut bytes = vec![];
    file.take(MAX_PACKAGE_MEMBER_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| unreadable(format!("{name}: {error}")))?;
    if bytes.len() as u64 > MAX_PACKAGE_MEMBER_BYTES {
        return Err(unreadable(format!("{name} is too large")));
    }
    Ok(bytes)
}

fn file_names(archive: &ZipArchive<Cursor<&[u8]>>) -> Vec<String> {
    archive
        .file_names()
        .filter(|name| !name.ends_with('/'))
        .map(str::to_string)
        .collect()
}

fn base_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn is_er_file(path: &str, extension: &str) -> bool {
    let name = base_name(path);
    name.starts_with(ER_FILE_PREFIX) && name.ends_with(extension)
}

/// The one ER file with `extension` among `names`.
fn single_er_file(names: &[String], extension: &str) -> Option<String> {
    let mut matching = names.iter().filter(|name| is_er_file(name, extension));
    match (matching.next(), matching.next()) {
        (Some(name), None) => Some(name.clone()),
        _ => None,
    }
}

/// The destinations of an `all_servers.zip`: each folder's ER zip, by
/// folder name.
fn destinations(names: &[String]) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = names
        .iter()
        .filter(|name| is_er_file(name, ZIP_EXTENSION))
        .filter_map(|name| {
            name.rsplit_once('/')
                .map(|(folder, _)| (folder.to_string(), name.clone()))
        })
        .collect();
    found.sort();
    found
}

fn read_sealed_returns(er_zip: &[u8]) -> std::result::Result<SealedReturns, TransmissionRefusal> {
    let mut archive = open_zip(er_zip)?;
    let names = file_names(&archive);
    let json_name = single_er_file(&names, JSON_EXTENSION)
        .ok_or_else(|| unreadable("the returns' package has no single er_*.json"))?;
    let exz_name = single_er_file(&names, EXZ_EXTENSION)
        .ok_or_else(|| unreadable("the returns' package has no single er_*.exz"))?;
    let json_bytes = read_member(&mut archive, &json_name)?;
    let json: ACMJson = serde_json::from_slice(&json_bytes)
        .map_err(|error| unreadable(format!("{json_name}: {error}")))?;
    let exz = read_member(&mut archive, &exz_name)?;
    Ok(SealedReturns { json, exz })
}

fn unseal_password(
    sealed: &SealedReturns,
    key: &PKeyRef<Private>,
) -> std::result::Result<Vec<u8>, SealingError> {
    let encrypted = BASE64
        .decode(sealed.json.encrypted_key.trim())
        .map_err(|_| SealingError::TooShort)?;
    ecies_decrypt(key, &encrypted)
}

fn decompress(data: &[u8]) -> std::result::Result<Vec<u8>, String> {
    let mut eml = vec![];
    XzDecoder::new(Cursor::new(data))
        .take(MAX_PACKAGE_MEMBER_BYTES + 1)
        .read_to_end(&mut eml)
        .map_err(|error| format!("not xz-compressed: {error}"))?;
    if eml.len() as u64 > MAX_PACKAGE_MEMBER_BYTES {
        return Err("the election returns are too large".to_string());
    }
    Ok(eml)
}

fn open_returns(
    destination: Option<String>,
    sealed: SealedReturns,
    password: &[u8],
) -> std::result::Result<OpenedPackage, TransmissionRefusal> {
    let compressed = openssl_enc_decrypt(password, &sealed.exz)
        .map_err(|error| TransmissionRefusal::only(Refusal::UnreadableErFile(error.to_string())))?;
    let eml = decompress(&compressed)
        .map_err(|reason| TransmissionRefusal::only(Refusal::UnreadableErFile(reason)))?;
    Ok(OpenedPackage {
        destination,
        json: sealed.json,
        eml,
    })
}

fn open_package(
    package: &[u8],
    options: &VerifyOptions,
) -> std::result::Result<OpenedPackage, TransmissionRefusal> {
    let mut archive = open_zip(package)?;
    let names = file_names(&archive);
    let found = destinations(&names);
    if found.is_empty() {
        if single_er_file(&names, JSON_EXTENSION).is_none() {
            return Err(TransmissionRefusal::only(Refusal::NoElectionReturns));
        }
        let sealed = read_sealed_returns(package)?;
        let password = unseal_password(&sealed, &options.destination_key).map_err(|_| {
            TransmissionRefusal::only(Refusal::SealedForAnotherKey {
                destinations: vec![format!("{ER_FILE_PREFIX}{}", sealed.json.station_id)],
            })
        })?;
        return open_returns(None, sealed, &password);
    }

    let candidates: Vec<&(String, String)> = match &options.destination {
        Some(requested) => {
            let chosen: Vec<&(String, String)> = found
                .iter()
                .filter(|(destination, _)| destination == requested)
                .collect();
            if chosen.is_empty() {
                return Err(TransmissionRefusal::only(Refusal::UnknownDestination {
                    requested: requested.clone(),
                    available: found
                        .iter()
                        .map(|(destination, _)| destination.clone())
                        .collect(),
                }));
            }
            chosen
        }
        None => found.iter().collect(),
    };
    for (destination, entry) in &candidates {
        let er_zip = read_member(&mut archive, entry)?;
        let sealed = read_sealed_returns(&er_zip)?;
        if let Ok(password) = unseal_password(&sealed, &options.destination_key) {
            return open_returns(Some(destination.clone()), sealed, &password);
        }
    }
    Err(TransmissionRefusal::only(Refusal::SealedForAnotherKey {
        destinations: candidates
            .iter()
            .map(|(destination, _)| destination.clone())
            .collect(),
    }))
}

fn read_public_key(pem: &str) -> Option<PKey<Public>> {
    PKey::public_key_from_pem(pem.as_bytes()).ok()
}

fn same_key(pem: &str, key: &PKeyRef<Public>) -> bool {
    match (
        read_public_key(pem).and_then(|key| key.public_key_to_der().ok()),
        key.public_key_to_der().ok(),
    ) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

/// Whether `signature_base64` is a SHA-256 signature of `data` by `key`:
/// PKCS#1 v1.5 for RSA, DER ECDSA for P-256.
fn signature_is_valid(key: &PKeyRef<Public>, data: &[u8], signature_base64: &str) -> bool {
    let Ok(signature) = BASE64.decode(signature_base64.trim()) else {
        return false;
    };
    let Some(algorithm) = key_algorithm(key) else {
        return false;
    };
    let verify = || -> std::result::Result<bool, openssl::error::ErrorStack> {
        let mut verifier = Verifier::new(MessageDigest::sha256(), key)?;
        if algorithm == SignatureAlgorithm::RsaPkcs1Sha256 {
            verifier.set_rsa_padding(Padding::PKCS1)?;
        }
        verifier.update(data)?;
        verifier.verify(&signature)
    };
    verify().unwrap_or(false)
}

fn subject(certificate: &X509) -> String {
    certificate
        .to_der()
        .ok()
        .and_then(|der| {
            X509Certificate::from_der(&der)
                .ok()
                .map(|(_, parsed)| parsed.subject().to_string())
        })
        .unwrap_or_default()
}

fn certificate_with_key<'a>(certificates: &'a [X509], key_der: &[u8]) -> Option<&'a X509> {
    certificates.iter().find(|certificate| {
        certificate
            .public_key()
            .and_then(|key| key.public_key_to_der())
            .is_ok_and(|der| der == key_der)
    })
}

fn certificate_status(key: &PKeyRef<Public>, trust: &MemberTrust) -> CertificateStatus {
    let Ok(key_der) = key.public_key_to_der() else {
        return CertificateStatus::NotFound;
    };
    match trust {
        MemberTrust::PackageKeys => CertificateStatus::NotChecked,
        MemberTrust::Certificates(certificates) => {
            match certificate_with_key(certificates, &key_der) {
                Some(certificate) => CertificateStatus::Matched {
                    subject: subject(certificate),
                },
                None => CertificateStatus::NotFound,
            }
        }
        MemberTrust::ChainedCertificates { ca, certificates } => {
            let Some(certificate) = certificate_with_key(certificates, &key_der) else {
                return CertificateStatus::NotFound;
            };
            let subject = subject(certificate);
            match verified_path(certificate, ca, ca) {
                Ok(Ok(_)) => CertificateStatus::Chained { subject },
                Ok(Err(reason)) => CertificateStatus::NotChained { subject, reason },
                Err(error) => CertificateStatus::NotChained {
                    subject,
                    reason: error.to_string(),
                },
            }
        }
    }
}

fn check_member(member: &ACMTrustee, eml: &[u8], trust: &MemberTrust) -> SignerReport {
    let key = member.publickey.as_deref().and_then(read_public_key);
    let key_sha256 = key.as_ref().and_then(|key| public_key_sha256(key).ok());
    let (signature, certificate) = match (&member.signature, &key) {
        (None, _) => (SignatureStatus::Missing, CertificateStatus::NotChecked),
        (Some(_), None) => (SignatureStatus::Invalid, CertificateStatus::NotChecked),
        (Some(signature), Some(key)) => {
            let status = if signature_is_valid(key, eml, signature) {
                SignatureStatus::Valid
            } else {
                SignatureStatus::Invalid
            };
            (status, certificate_status(key, trust))
        }
    };
    SignerReport {
        id: member.id.clone(),
        name: member.name.clone(),
        key_sha256,
        signature,
        certificate,
    }
}

fn is_element(node: &Node, name: &str) -> bool {
    node.is_element() && node.tag_name().name() == name
}

fn child<'a, 'input>(node: &Node<'a, 'input>, name: &str) -> Option<Node<'a, 'input>> {
    node.children().find(|child| is_element(child, name))
}

fn text(node: Option<Node>) -> String {
    node.and_then(|node| node.text())
        .map(|text| text.trim().to_string())
        .unwrap_or_default()
}

fn identifier(node: &Node, identifier_tag: &str, name_tag: &str) -> (String, String) {
    let identifier = child(node, identifier_tag);
    let id = identifier
        .and_then(|identifier| identifier.attribute(EML_ID_NUMBER))
        .unwrap_or_default()
        .to_string();
    let name = text(identifier.and_then(|identifier| child(&identifier, name_tag)));
    (id, name)
}

fn number(value: &str, what: &str) -> std::result::Result<i64, String> {
    value
        .parse()
        .map_err(|_| format!("{what} is {value:?}, not a number"))
}

fn read_contest(
    election_id: &str,
    election_name: &str,
    contest: &Node,
) -> std::result::Result<ContestCount, String> {
    let (id, name) = identifier(contest, EML_CONTEST_IDENTIFIER, EML_CONTEST_NAME);
    let metrics = contest
        .descendants()
        .filter(|node| is_element(node, EML_COUNT_METRIC))
        .map(|metric| {
            let metric_id = metric.attribute(EML_METRIC_ID).unwrap_or_default();
            Ok(CountMetricValue {
                id: metric_id.to_string(),
                kind: metric
                    .attribute(EML_METRIC_TYPE)
                    .unwrap_or_default()
                    .to_string(),
                value: number(
                    &text(Some(metric)),
                    &format!("the count {metric_id} of contest {id}"),
                )?,
            })
        })
        .collect::<std::result::Result<Vec<_>, String>>()?;
    let selections = contest
        .descendants()
        .filter(|node| is_element(node, EML_SELECTION))
        .map(|selection| {
            let candidates = selection
                .children()
                .filter(|node| is_element(node, EML_CANDIDATE))
                .map(|candidate| {
                    let (id, name) =
                        identifier(&candidate, EML_CANDIDATE_IDENTIFIER, EML_CANDIDATE_NAME);
                    CandidateRef { id, name }
                })
                .collect();
            Ok(SelectionCount {
                candidates,
                valid_votes: number(
                    &text(child(&selection, EML_VALID_VOTES)),
                    &format!("a selection's valid votes in contest {id}"),
                )?,
            })
        })
        .collect::<std::result::Result<Vec<_>, String>>()?;
    Ok(ContestCount {
        election_id: election_id.to_string(),
        election_name: election_name.to_string(),
        id,
        name,
        metrics,
        selections,
    })
}

/// The transaction id and the counts per contest of an EML 510 document.
pub fn read_counts(eml: &[u8]) -> std::result::Result<(Option<String>, Vec<ContestCount>), String> {
    let text_eml = std::str::from_utf8(eml).map_err(|_| "not UTF-8 text".to_string())?;
    let document = Document::parse(text_eml).map_err(|error| format!("not XML: {error}"))?;
    let root = document.root_element();
    if !is_element(&root, EML_ROOT) {
        return Err("not an EML document".to_string());
    }
    let transaction_id = root
        .descendants()
        .find(|node| is_element(node, EML_TRANSACTION_ID))
        .map(|node| text(Some(node)))
        .filter(|id| !id.is_empty());
    let mut contests = vec![];
    for election in root
        .descendants()
        .filter(|node| is_element(node, EML_ELECTION))
    {
        let (election_id, election_name) =
            identifier(&election, EML_ELECTION_IDENTIFIER, EML_ELECTION_NAME);
        for contest in election
            .descendants()
            .filter(|node| is_element(node, EML_CONTEST))
        {
            contests.push(read_contest(&election_id, &election_name, &contest)?);
        }
    }
    Ok((transaction_id, contests))
}

#[cfg(test)]
mod tests {
    use super::*;

    const EML: &str = r#"<EML Id="510" xmlns="urn:oasis:names:tc:evs:schema:eml">
  <EMLHeader><TransactionId> 42 </TransactionId></EMLHeader>
  <Count><Election>
    <ElectionIdentifier IdNumber="E1"><ElectionName>General</ElectionName></ElectionIdentifier>
    <Contests><Contest>
      <ContestIdentifier IdNumber="C1"><ContestName>Mayor</ContestName></ContestIdentifier>
      <TotalVotes>
        <CountMetric Type="Total Number of Registered Voters" Id="RV">10</CountMetric>
        <Selection>
          <Candidate><CandidateIdentifier IdNumber="K1"><CandidateName>Kim &amp; Lee</CandidateName></CandidateIdentifier></Candidate>
          <ValidVotes>7</ValidVotes>
        </Selection>
      </TotalVotes>
    </Contest></Contests>
  </Election></Count>
</EML>"#;

    #[test]
    fn counts_are_read_per_contest() {
        let (transaction, contests) = read_counts(EML.as_bytes()).unwrap();
        assert_eq!(transaction.as_deref(), Some("42"));
        assert_eq!(
            contests,
            vec![ContestCount {
                election_id: "E1".into(),
                election_name: "General".into(),
                id: "C1".into(),
                name: "Mayor".into(),
                metrics: vec![CountMetricValue {
                    id: "RV".into(),
                    kind: "Total Number of Registered Voters".into(),
                    value: 10,
                }],
                selections: vec![SelectionCount {
                    candidates: vec![CandidateRef {
                        id: "K1".into(),
                        name: "Kim & Lee".into(),
                    }],
                    valid_votes: 7,
                }],
            }]
        );
    }

    #[test]
    fn returns_that_are_not_eml_are_refused() {
        assert!(read_counts(b"\xff\xfe").is_err());
        assert!(read_counts(b"<EML><unclosed></EML>").is_err());
        assert!(read_counts(b"<Other/>").is_err());
        assert!(read_counts(EML.replace(">7<", ">seven<").as_bytes()).is_err());
        assert!(read_counts(EML.replace(">10<", ">-<").as_bytes()).is_err());
    }

    #[test]
    fn returns_without_a_transaction_have_none() {
        let (transaction, contests) = read_counts(b"<EML/>").unwrap();
        assert_eq!(transaction, None);
        assert!(contests.is_empty());
    }

    #[test]
    fn a_sha256_is_read_as_people_copy_it() {
        let hex = "ab".repeat(32);
        assert_eq!(normalize_sha256(&hex).unwrap(), hex.to_uppercase());
        assert_eq!(
            normalize_sha256(&grouped_sha256(&hex.to_uppercase())).unwrap(),
            hex.to_uppercase()
        );
        assert!(normalize_sha256("abc").is_err());
        assert!(normalize_sha256(&"zz".repeat(32)).is_err());
    }

    #[test]
    fn a_sha256_is_shown_in_groups_of_eight() {
        assert_eq!(grouped_sha256("0123456789ABCDEF01"), "01234567 89ABCDEF 01");
    }

    #[test]
    fn refusals_say_why() {
        let refusal = TransmissionRefusal {
            refusals: vec![
                Refusal::OvcsSignatureInvalid,
                Refusal::TooFewSignatures {
                    required: 2,
                    valid: 1,
                },
            ],
            contents: None,
        };
        let message = refusal.to_string();
        assert!(message.contains("OVCS signature"));
        assert!(message.contains("2 required"));
    }

    #[test]
    fn er_files_are_found_by_name() {
        let names: Vec<String> = [
            "ccs-1/",
            "ccs-1/er_1.zip",
            "ccs-1/al_1.zip",
            "ccs-2/er_1.zip",
            "er_1.zip",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        assert_eq!(
            destinations(&names),
            vec![
                ("ccs-1".to_string(), "ccs-1/er_1.zip".to_string()),
                ("ccs-2".to_string(), "ccs-2/er_1.zip".to_string()),
            ]
        );
        assert_eq!(single_er_file(&names, ".json"), None);
        assert_eq!(
            single_er_file(&["er_1.json".to_string()], ".json").as_deref(),
            Some("er_1.json")
        );
    }
}
