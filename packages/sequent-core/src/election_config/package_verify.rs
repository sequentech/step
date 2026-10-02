// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Checking a signed configuration package, before anything in it is read.
//!
//! One function, [`verify_package`], used wherever a package is opened: the
//! Election Architect in the browser, `step-cli verify-package` and windmill's
//! import. It refuses:
//!
//! * a package with no signature, or one whose manifest can't be read;
//! * a signer outside the trust anchors, revoked by a list the verifier has
//!   seen or carried in the package, or whose certificate isn't made for
//!   signing;
//! * a signature that doesn't verify;
//! * a changed, missing or extra file, and a content digest that doesn't
//!   match the content;
//! * too few valid approvals from distinct people.
//!
//! Whether the revision is newer than the last one imported is [`freshness`],
//! separate because the caller needs the manifest's external id to look the
//! last one up.
//!
//! Certificate paths, revocation lists and signatures are checked with
//! `rustls-webpki` over `ring`, which build for the browser too.

use std::time::Duration;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use chrono::DateTime;
use rustls_pki_types::pem::PemObject;
use rustls_pki_types::{
    CertificateDer, CertificateRevocationListDer,
    SignatureVerificationAlgorithm, UnixTime,
};
use serde::{Deserialize, Serialize};
use webpki::{
    CertRevocationList, EndEntityCert, ExpirationPolicy, KeyPurposeIdIter,
    OwnedCertRevocationList, RevocationCheckDepth, RevocationOptionsBuilder,
    UnknownStatusPolicy,
};
use x509_parser::prelude::{FromDer, X509Certificate};

use crate::signing::SignatureAlgorithm;

use super::archive::Artifact;
use super::manifest::{
    approval_payload, declared_format, file_entries, open_package, sha256_hex,
    Approval, FileEntry, Manifest, ManifestFormat,
};
use super::problem::{Code, Problem, Report};

/// Extended key usages that sign documents, besides anyExtendedKeyUsage,
/// clientAuth and emailProtection: documentSigning (RFC 9336), Adobe
/// authentic documents and Microsoft document signing. The same list the
/// staff certificate checks use.
const SIGNING_EXTENDED_KEY_USAGES: [&str; 3] = [
    "1.3.6.1.5.5.7.3.36",
    "1.2.840.113583.1.1.5",
    "1.3.6.1.4.1.311.10.3.12",
];

/// Algorithms a certificate in a path may be signed with.
static PATH_ALGORITHMS: &[&dyn SignatureVerificationAlgorithm] = &[
    webpki::ring::ECDSA_P256_SHA256,
    webpki::ring::ECDSA_P256_SHA384,
    webpki::ring::ECDSA_P384_SHA256,
    webpki::ring::ECDSA_P384_SHA384,
    webpki::ring::ED25519,
    webpki::ring::RSA_PKCS1_2048_8192_SHA256,
    webpki::ring::RSA_PKCS1_2048_8192_SHA384,
    webpki::ring::RSA_PKCS1_2048_8192_SHA512,
    webpki::ring::RSA_PSS_2048_8192_SHA256_LEGACY_KEY,
    webpki::ring::RSA_PSS_2048_8192_SHA384_LEGACY_KEY,
    webpki::ring::RSA_PSS_2048_8192_SHA512_LEGACY_KEY,
];

/// Algorithms the organization's key may sign `manifest.json` with: ECDSA
/// P-256, or RSA-PSS where its CA requires RSA.
static MANIFEST_ALGORITHMS: &[&dyn SignatureVerificationAlgorithm] = &[
    webpki::ring::ECDSA_P256_SHA256,
    webpki::ring::RSA_PSS_2048_8192_SHA256_LEGACY_KEY,
];

/// What a verifier trusts, set when it is deployed.
#[derive(Debug, Clone, Default)]
pub struct PackageTrust {
    /// The roots a package's signing certificate must chain to, DER.
    pub package_roots: Vec<Vec<u8>>,
    /// The roots an approver's certificate must chain to, DER.
    pub staff_roots: Vec<Vec<u8>>,
    /// Revocation lists the verifier has seen, DER. A package's own lists
    /// are added to these.
    pub revocation_lists: Vec<Vec<u8>>,
    /// How many approvals from distinct people a package needs.
    pub required_approvals: u16,
}

impl PackageTrust {
    /// Trust from PEM: certificates for the roots, and revocation lists.
    pub fn from_pem(
        package_roots: &str,
        staff_roots: &str,
        revocation_lists: &[String],
        required_approvals: u16,
    ) -> Result<Self, Problem> {
        let mut lists = Vec::new();
        for list in revocation_lists {
            lists.extend(crls_from_pem(list).map_err(|reason| {
                unreadable_trust("revocation_lists", reason)
            })?);
        }
        Ok(PackageTrust {
            package_roots: certificates_from_pem(package_roots)
                .map_err(|reason| unreadable_trust("package_roots", reason))?,
            staff_roots: certificates_from_pem(staff_roots)
                .map_err(|reason| unreadable_trust("staff_roots", reason))?,
            revocation_lists: lists,
            required_approvals,
        })
    }
}

/// Who signed a certificate-checked signature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CertificateIdentity {
    pub subject: String,
    pub issuer: String,
    /// Lowercase hex.
    pub serial: String,
    /// SHA-256 of the certificate's DER, lowercase hex.
    pub fingerprint_sha256: String,
}

/// A package that passed every check. Only now may its members be read.
#[derive(Debug, Clone)]
pub struct VerifiedPackage {
    pub manifest: Manifest,
    pub manifest_sha256: String,
    pub signer: CertificateIdentity,
    pub approvers: Vec<CertificateIdentity>,
    /// The delivery's members, as checked.
    pub members: Vec<Artifact>,
}

/// The last package imported for a configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LastImport {
    pub revision: u64,
    pub manifest_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Freshness {
    /// Newer than anything imported for this configuration.
    New,
    /// The very package imported last.
    AlreadyImported,
}

/// Every check except the revision's freshness.
pub fn verify_package(
    bytes: &[u8],
    trust: &PackageTrust,
) -> Result<VerifiedPackage, Report> {
    let opened = open_package(bytes).map_err(Report::from_problem)?;
    let (manifest_bytes, signature, chain) =
        match (&opened.manifest, &opened.signature, &opened.chain) {
            (Some(manifest), Some(signature), Some(chain)) => {
                (manifest, signature, chain)
            }
            _ => {
                let missing: Vec<&str> = [
                    (
                        opened.manifest.is_none(),
                        super::manifest::MANIFEST_MEMBER,
                    ),
                    (
                        opened.signature.is_none(),
                        super::manifest::SIGNATURE_MEMBER,
                    ),
                    (opened.chain.is_none(), super::manifest::CHAIN_MEMBER),
                ]
                .into_iter()
                .filter_map(|(absent, name)| absent.then_some(name))
                .collect();
                return Err(Report::from_problem(unsigned(&missing)));
            }
        };

    let manifest =
        read_manifest(manifest_bytes).map_err(Report::from_problem)?;
    let produced_at =
        unix_time(&manifest.produced.at).map_err(Report::from_problem)?;

    let mut crls = trust.revocation_lists.clone();
    for list in &manifest.revocation_lists {
        // A list that doesn't parse protects nothing; one that does is
        // checked against its issuer by the path check.
        if let Ok(parsed) = crls_from_pem(list) {
            crls.extend(parsed);
        }
    }

    let chain = certificates_from_pem(&String::from_utf8_lossy(chain))
        .map_err(|reason| Report::from_problem(unreadable_chain(reason)))?;
    let signer = check_certificate(
        &chain,
        &trust.package_roots,
        &crls,
        produced_at,
        Role::PackageSigner,
    )
    .map_err(Report::from_problem)?;
    verify_with(&chain[0], MANIFEST_ALGORITHMS, manifest_bytes, signature)
        .map_err(|reason| Report::from_problem(bad_signature(reason)))?;

    let mut report = Report::default();
    check_files(&opened.members, &manifest.content.files, &mut report);
    match manifest.content.sha256() {
        Ok(digest) if digest == manifest.content_sha256 => {}
        Ok(digest) => {
            report.push(content_mismatch(&manifest.content_sha256, &digest))
        }
        Err(problem) => report.push(problem),
    }
    let approvers = check_approvals(&manifest, trust, &crls, &mut report);
    if report.has_errors() {
        return Err(report);
    }

    Ok(VerifiedPackage {
        manifest_sha256: sha256_hex(manifest_bytes),
        manifest,
        signer,
        approvers,
        members: opened.members,
    })
}

/// Whether a verified package may be imported after `last`, the last one
/// imported for the same configuration: a newer revision may, the same
/// package again is reported as such, and anything else is a rollback.
pub fn freshness(
    package: &VerifiedPackage,
    last: Option<&LastImport>,
) -> Result<Freshness, Problem> {
    let Some(last) = last else {
        return Ok(Freshness::New);
    };
    if last.manifest_sha256 == package.manifest_sha256 {
        return Ok(Freshness::AlreadyImported);
    }
    let revision = package.manifest.configuration.revision;
    if revision > last.revision {
        Ok(Freshness::New)
    } else {
        Err(Problem::error(
            Code::Rollback,
            "manifest.configuration.revision",
            format!(
                "revision {revision} is not newer than revision {}, the last \
                 one imported",
                last.revision
            ),
        )
        .id("package.rollback")
        .detail("revision", revision)
        .detail("last", last.revision))
    }
}

/// Checks a package signing certificate before it is used: its path to one
/// of `roots` at `at` (RFC 3339), its revocation status in `revocation_lists`
/// and its key usage, as [`verify_package`] will check them.
pub fn verify_signing_certificate(
    chain_pem: &str,
    roots: &[Vec<u8>],
    revocation_lists: &[Vec<u8>],
    at: &str,
) -> Result<CertificateIdentity, Problem> {
    let chain = certificates_from_pem(chain_pem).map_err(unreadable_chain)?;
    check_certificate(
        &chain,
        roots,
        revocation_lists,
        unix_time(at)?,
        Role::PackageSigner,
    )
}

/// Checks one approval on its own: its certificate against the staff roots
/// at the time it was given, and its signature over the revision's payload.
/// Whether the approvers are distinct and enough is the caller's.
pub fn verify_approval(
    approval: &Approval,
    payload: &str,
    staff_roots: &[Vec<u8>],
    revocation_lists: &[Vec<u8>],
) -> Result<CertificateIdentity, String> {
    let at =
        unix_time(&approval.signed_at).map_err(|problem| problem.message)?;
    let chain = certificates_from_pem(&approval.certificate_chain)?;
    let identity = check_certificate(
        &chain,
        staff_roots,
        revocation_lists,
        at,
        Role::Approver,
    )
    .map_err(|problem| problem.message)?;
    let signature = STANDARD
        .decode(approval.signature.trim())
        .map_err(|error| format!("the signature is not base64: {error}"))?;
    let algorithms: &[&dyn SignatureVerificationAlgorithm] =
        match approval.algorithm {
            SignatureAlgorithm::EcdsaP256Sha256 => {
                &[webpki::ring::ECDSA_P256_SHA256]
            }
            SignatureAlgorithm::RsaPkcs1Sha256 => {
                &[webpki::ring::RSA_PKCS1_2048_8192_SHA256]
            }
        };
    verify_with(&chain[0], algorithms, payload.as_bytes(), &signature)?;
    Ok(identity)
}

fn check_approvals(
    manifest: &Manifest,
    trust: &PackageTrust,
    crls: &[Vec<u8>],
    report: &mut Report,
) -> Vec<CertificateIdentity> {
    let payload = match approval_payload(
        &manifest.configuration.external_id,
        manifest.configuration.revision,
        &manifest.content_sha256,
    ) {
        Ok(payload) => payload,
        Err(problem) => {
            report.push(problem);
            return Vec::new();
        }
    };

    let mut approvers: Vec<CertificateIdentity> = Vec::new();
    for (index, approval) in manifest.approvals.iter().enumerate() {
        match verify_approval(approval, &payload, &trust.staff_roots, crls) {
            Ok(identity) => {
                let repeated = approvers.iter().any(|seen| {
                    seen.subject == identity.subject
                        || seen.fingerprint_sha256
                            == identity.fingerprint_sha256
                });
                if repeated {
                    report.push(
                        Problem::warning(
                            Code::NotApproved,
                            format!("manifest.approvals[{index}]"),
                            format!(
                                "{} approved more than once; it counts once",
                                approval.name
                            ),
                        )
                        .id("package.approval-repeated")
                        .detail("name", &approval.name),
                    );
                } else {
                    approvers.push(identity);
                }
            }
            Err(reason) => report.push(
                Problem::error(
                    Code::NotApproved,
                    format!("manifest.approvals[{index}]"),
                    format!(
                        "the approval by {} doesn't count: {reason}",
                        approval.name
                    ),
                )
                .id("package.approval-invalid")
                .detail("name", &approval.name)
                .detail("reason", reason),
            ),
        }
    }

    let required = usize::from(trust.required_approvals);
    if approvers.len() < required {
        report.push(
            Problem::error(
                Code::NotApproved,
                "manifest.approvals",
                format!(
                    "{} valid approvals from different people, and {required} \
                     are needed",
                    approvers.len()
                ),
            )
            .id("package.too-few-approvals")
            .detail("count", approvers.len())
            .detail("required", required),
        );
    }
    approvers
}

fn check_files(
    members: &[Artifact],
    listed: &[FileEntry],
    report: &mut Report,
) {
    match file_entries(members) {
        Ok(actual) => compare_entries(&actual, listed, "", report),
        Err(problem) => report.push(problem),
    }
}

fn compare_entries(
    actual: &[FileEntry],
    listed: &[FileEntry],
    within: &str,
    report: &mut Report,
) {
    let full = |path: &str| {
        if within.is_empty() {
            path.to_string()
        } else {
            format!("{within}/{path}")
        }
    };
    for expected in listed {
        match actual.iter().find(|entry| entry.path == expected.path) {
            None => report.push(
                Problem::error(
                    Code::IntegrityMismatch,
                    full(&expected.path),
                    format!(
                        "{} is in the manifest and not in the package",
                        full(&expected.path)
                    ),
                )
                .id("package.file-missing")
                .detail("file", full(&expected.path)),
            ),
            Some(found) if found.sha256 != expected.sha256 => {
                report.push(
                    Problem::error(
                        Code::IntegrityMismatch,
                        full(&expected.path),
                        format!(
                            "{} changed after signing: its SHA-256 is {}, \
                             and the manifest says {}",
                            full(&expected.path),
                            found.sha256,
                            expected.sha256
                        ),
                    )
                    .id("package.file-changed")
                    .detail("file", full(&expected.path))
                    .detail("expected", &expected.sha256)
                    .detail("actual", &found.sha256),
                );
                compare_entries(
                    &found.members,
                    &expected.members,
                    &full(&expected.path),
                    report,
                );
            }
            Some(_) => {}
        }
    }
    for extra in actual
        .iter()
        .filter(|entry| !listed.iter().any(|listed| listed.path == entry.path))
    {
        report.push(
            Problem::error(
                Code::IntegrityMismatch,
                full(&extra.path),
                format!(
                    "{} is in the package and not in the manifest",
                    full(&extra.path)
                ),
            )
            .id("package.file-extra")
            .detail("file", full(&extra.path)),
        );
    }
}

fn read_manifest(bytes: &[u8]) -> Result<Manifest, Problem> {
    match declared_format(bytes) {
        Some(format) if format == ManifestFormat::V1.as_str() => {}
        Some(format) => {
            return Err(Problem::error(
                Code::IncompatibleVersion,
                super::manifest::MANIFEST_MEMBER,
                format!(
                    "the manifest is in format '{format}', which this version \
                     can't read"
                ),
            )
            .id("package.unknown-format")
            .detail("format", format))
        }
        None => {}
    }
    serde_json::from_slice(bytes).map_err(|error| {
        Problem::error(
            Code::Unreadable,
            super::manifest::MANIFEST_MEMBER,
            format!("the manifest could not be read: {error}"),
        )
        .id("package.unreadable-manifest")
        .detail("reason", error)
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    PackageSigner,
    Approver,
}

/// The certificate's path to one of `roots` at `at`, its revocation status
/// in `crls` and its key usage. `chain` is the leaf, then its issuers.
fn check_certificate(
    chain: &[Vec<u8>],
    roots: &[Vec<u8>],
    crls: &[Vec<u8>],
    at: UnixTime,
    role: Role,
) -> Result<CertificateIdentity, Problem> {
    let untrusted = |reason: String| untrusted_signer(role, reason);
    let Some((leaf, intermediates)) = chain.split_first() else {
        return Err(untrusted("there is no certificate".to_string()));
    };
    if roots.is_empty() {
        return Err(untrusted("no trusted roots are configured".to_string()));
    }

    let root_ders: Vec<CertificateDer<'_>> = roots
        .iter()
        .map(|der| CertificateDer::from(der.as_slice()))
        .collect();
    let anchors = root_ders
        .iter()
        .map(|der| {
            webpki::anchor_from_trusted_cert(der).map_err(|error| {
                untrusted(format!("a trusted root is unreadable: {error:?}"))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let intermediate_ders: Vec<CertificateDer<'_>> = intermediates
        .iter()
        .map(|der| CertificateDer::from(der.as_slice()))
        .collect();
    let leaf_der = CertificateDer::from(leaf.as_slice());
    let end_entity = EndEntityCert::try_from(&leaf_der).map_err(|error| {
        untrusted(format!("the certificate is unreadable: {error:?}"))
    })?;

    let lists: Vec<CertRevocationList<'_>> = crls
        .iter()
        .filter_map(|der| OwnedCertRevocationList::from_der(der).ok())
        .map(CertRevocationList::from)
        .collect();
    let list_refs: Vec<&CertRevocationList<'_>> = lists.iter().collect();
    let revocation =
        RevocationOptionsBuilder::new(&list_refs)
            .ok()
            .map(|builder| {
                builder
                    .with_depth(RevocationCheckDepth::Chain)
                    .with_status_policy(UnknownStatusPolicy::Allow)
                    .with_expiration_policy(ExpirationPolicy::Ignore)
                    .build()
            });

    end_entity
        .verify_for_usage(
            PATH_ALGORITHMS,
            &anchors,
            &intermediate_ders,
            at,
            AnyUsage,
            revocation,
            None,
        )
        .map_err(|error| match error {
            webpki::Error::CertRevoked => revoked_signer(role),
            other => untrusted(path_failure(&other)),
        })?;

    let (_, parsed) = X509Certificate::from_der(leaf).map_err(|error| {
        untrusted(format!("the certificate is unreadable: {error}"))
    })?;
    if !made_for_signing(&parsed) {
        let problem = Problem::error(
            Code::UntrustedSigner,
            role.path(),
            format!("{}'s certificate is not made for signing", role.who()),
        );
        return Err(match role {
            Role::PackageSigner => problem.id("package.signer-key-usage"),
            Role::Approver => problem.id("package.approver-key-usage"),
        });
    }

    Ok(CertificateIdentity {
        subject: parsed.subject().to_string(),
        issuer: parsed.issuer().to_string(),
        serial: hex::encode(parsed.raw_serial()),
        fingerprint_sha256: sha256_hex(leaf),
    })
}

/// Key usage allows digitalSignature or nonRepudiation, and an extended key
/// usage, if there is one, allows signing documents.
fn made_for_signing(certificate: &X509Certificate<'_>) -> bool {
    let signs = matches!(
        certificate.key_usage(),
        Ok(Some(usage)) if usage.value.digital_signature() || usage.value.non_repudiation()
    );
    let extended = match certificate.extended_key_usage() {
        Ok(None) => true,
        Ok(Some(usage)) => {
            let usage = usage.value;
            usage.any
                || usage.client_auth
                || usage.email_protection
                || usage.other.iter().any(|oid| {
                    SIGNING_EXTENDED_KEY_USAGES
                        .contains(&oid.to_id_string().as_str())
                })
        }
        Err(_) => false,
    };
    signs && extended
}

/// The extended key usage is checked on the leaf by [`made_for_signing`],
/// with the same rule the staff certificate checks use; the path check
/// accepts any.
struct AnyUsage;

impl webpki::ExtendedKeyUsageValidator for AnyUsage {
    fn validate(
        &self,
        _: KeyPurposeIdIter<'_, '_>,
    ) -> Result<(), webpki::Error> {
        Ok(())
    }
}

impl Role {
    fn path(self) -> &'static str {
        match self {
            Role::PackageSigner => super::manifest::CHAIN_MEMBER,
            Role::Approver => "manifest.approvals",
        }
    }

    fn who(self) -> &'static str {
        match self {
            Role::PackageSigner => "the package signer",
            Role::Approver => "the approver",
        }
    }
}

fn path_failure(error: &webpki::Error) -> String {
    match error {
        webpki::Error::UnknownIssuer => {
            "it is not issued by a trusted authority".to_string()
        }
        webpki::Error::CertExpired { .. } => {
            "it had expired when it was used".to_string()
        }
        webpki::Error::CertNotValidYet { .. } => {
            "it was not valid yet when it was used".to_string()
        }
        webpki::Error::CaUsedAsEndEntity => {
            "it is a certificate authority, which doesn't sign as a person"
                .to_string()
        }
        other => format!("its certificate path doesn't verify ({other:?})"),
    }
}

fn verify_with(
    leaf: &[u8],
    algorithms: &[&dyn SignatureVerificationAlgorithm],
    message: &[u8],
    signature: &[u8],
) -> Result<(), String> {
    let leaf_der = CertificateDer::from(leaf);
    let end_entity = EndEntityCert::try_from(&leaf_der)
        .map_err(|error| format!("the certificate is unreadable: {error:?}"))?;
    if algorithms.iter().any(|algorithm| {
        end_entity
            .verify_signature(*algorithm, message, signature)
            .is_ok()
    }) {
        Ok(())
    } else {
        Err("the signature does not match the signed bytes".to_string())
    }
}

fn unix_time(rfc3339: &str) -> Result<UnixTime, Problem> {
    let parsed = DateTime::parse_from_rfc3339(rfc3339).map_err(|error| {
        Problem::error(
            Code::InvalidValue,
            super::manifest::MANIFEST_MEMBER,
            format!("'{rfc3339}' is not a time: {error}"),
        )
        .id("package.invalid-time")
        .detail("value", rfc3339)
    })?;
    let seconds = u64::try_from(parsed.timestamp()).map_err(|_| {
        Problem::error(
            Code::InvalidValue,
            super::manifest::MANIFEST_MEMBER,
            format!("'{rfc3339}' is before 1970"),
        )
        .id("package.invalid-time")
        .detail("value", rfc3339)
    })?;
    Ok(UnixTime::since_unix_epoch(Duration::from_secs(seconds)))
}

/// Every certificate in a PEM bundle, DER, in order.
pub fn certificates_from_pem(pem: &str) -> Result<Vec<Vec<u8>>, String> {
    let certificates = CertificateDer::pem_slice_iter(pem.as_bytes())
        .map(|certificate| certificate.map(|der| der.to_vec()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("unreadable PEM: {error}"))?;
    if certificates.is_empty() {
        return Err("there are no certificates in it".to_string());
    }
    Ok(certificates)
}

/// Every revocation list in a PEM bundle, DER.
pub fn crls_from_pem(pem: &str) -> Result<Vec<Vec<u8>>, String> {
    CertificateRevocationListDer::pem_slice_iter(pem.as_bytes())
        .map(|list| list.map(|der| der.to_vec()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("unreadable PEM: {error}"))
}

fn unsigned(missing: &[&str]) -> Problem {
    Problem::error(
        Code::Unsigned,
        super::archive::IMPORTABLE_MEMBER,
        format!(
            "the package is not signed: it has no {}",
            missing.join(", ")
        ),
    )
    .id("package.unsigned")
    .detail("missing", missing.join(", "))
}

fn unreadable_chain(reason: String) -> Problem {
    Problem::error(
        Code::Unreadable,
        super::manifest::CHAIN_MEMBER,
        format!("the signer's certificates could not be read: {reason}"),
    )
    .id("package.unreadable-chain")
    .detail("reason", reason)
}

fn unreadable_trust(what: &str, reason: String) -> Problem {
    Problem::error(
        Code::InvalidValue,
        what,
        format!("the trusted {what} could not be read: {reason}"),
    )
    .id("package.unreadable-trust")
    .detail("setting", what)
    .detail("reason", reason)
}

fn untrusted_signer(role: Role, reason: String) -> Problem {
    let problem = Problem::error(
        Code::UntrustedSigner,
        role.path(),
        format!("{} is not trusted: {reason}", role.who()),
    );
    match role {
        Role::PackageSigner => problem.id("package.untrusted-signer"),
        Role::Approver => problem.id("package.untrusted-approver"),
    }
    .detail("reason", reason)
}

fn revoked_signer(role: Role) -> Problem {
    let problem = Problem::error(
        Code::UntrustedSigner,
        role.path(),
        format!("{}'s certificate has been revoked", role.who()),
    );
    match role {
        Role::PackageSigner => problem.id("package.revoked-signer"),
        Role::Approver => problem.id("package.revoked-approver"),
    }
}

fn bad_signature(reason: String) -> Problem {
    Problem::error(
        Code::BadSignature,
        super::manifest::SIGNATURE_MEMBER,
        format!("the package's signature doesn't verify: {reason}"),
    )
    .id("package.bad-signature")
    .detail("reason", reason)
}

fn content_mismatch(listed: &str, actual: &str) -> Problem {
    Problem::error(
        Code::IntegrityMismatch,
        "manifest.content_sha256",
        format!(
            "the manifest's content digest is {listed}, and its content hashes \
             to {actual}"
        ),
    )
    .id("package.content-digest")
    .detail("expected", listed)
    .detail("actual", actual)
}

#[cfg(test)]
#[path = "package_verify_tests.rs"]
mod package_verify_tests;
