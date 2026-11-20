// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Browser-signed (WebAuthn) configuration packages: a versioned envelope
//! beside `manifest.json`, checked offline against a pinned roster.
//!
//! A package of this kind is the delivery, `manifest.json` and one
//! [`ENVELOPE_MEMBER`]: no `manifest.sig` and no `manifest.chain.pem`. The
//! envelope names its format, the tenant, the digest of `manifest.json`'s
//! exact bytes and the officials' [`Bundle`]. It is admitted only by
//! [`verify_browser_package`], with a [`PinnedRoster`] the verifier was
//! provisioned with: never a roster or threshold from the package, and never
//! from the tenant's imported signing setting.
//!
//! Dispatch is by member name, before anything is read:
//!
//! * a package with any member named like a browser envelope (any name, at
//!   any depth, that starts with `manifest.webauthn`, in any case) is a
//!   browser package. [`super::package_verify`] refuses it outright, so it is
//!   never taken for an unsigned file nor read as format 1 or 2;
//! * here, it must hold exactly [`ENVELOPE_MEMBER`], `manifest.json`, and
//!   neither of the certificate signature's members (mixed is refused);
//! * the envelope's format must be [`ENVELOPE_V1`]; another version of the
//!   same family is refused as unsupported, anything else as unreadable.
//!
//! Then, before the delivery is expanded: the envelope's manifest digest is
//! the recomputed one, its tenant is the pinned roster's, the manifest is a
//! plain one (no certificate approvals, no format-2 binding, no revocation
//! lists), its revision and previous revision are coherent, it was not
//! produced in the future, and the bundle's approvals and final signature
//! verify, the final one over the whole manifest
//! ([`super::webauthn_proof::package_seal_sha256`]). Only then is the
//! delivery expanded and checked file by file against the manifest, and the
//! content digest recomputed.

use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;

use super::archive::Artifact;
use super::manifest::{
    member_names, named_members, payload_without, sha256_hex, Manifest,
    ManifestFormat, CHAIN_MEMBER, MANIFEST_MEMBER, SIGNATURE_MEMBER,
};
use super::package_verify::{
    compare_entries, is_after, read_manifest, signed_in_the_future, unix_time,
    CertificateIdentity, Freshness, LastImport, VerificationScheme,
    VerifiedPackage,
};
use super::problem::{Code, Problem, Report};
use super::webauthn_proof::{
    verify_bundle_for_package, Bundle, PinnedRoster, ProofError, SealedManifest,
};

/// The envelope's member name.
pub const ENVELOPE_MEMBER: &str = "manifest.webauthn.json";
/// The one envelope format read.
pub const ENVELOPE_V1: &str = "sequent.configuration-webauthn-envelope/1";
/// The envelope family: a format in it that isn't [`ENVELOPE_V1`] is a
/// version this verifier doesn't read.
const ENVELOPE_FAMILY: &str = "sequent.configuration-webauthn-envelope/";
/// What every name a browser envelope might go by starts with.
const BROWSER_PREFIX: &str = "manifest.webauthn";

/// Whether a member's name is, or looks like, a browser envelope: by its
/// last path segment, in any case.
pub fn is_browser_member(name: &str) -> bool {
    name.rsplit(['/', '\\'])
        .next()
        .unwrap_or(name)
        .to_ascii_lowercase()
        .starts_with(BROWSER_PREFIX)
}

/// Whether a file carries anything named like a browser envelope, going by
/// its directory alone. A file whose directory can't be read has none that
/// can be found, and is left to the ordinary checks.
pub fn has_browser_members(bytes: &[u8]) -> bool {
    member_names(bytes)
        .is_ok_and(|names| names.iter().any(|name| is_browser_member(name)))
}

/// The refusal of a browser-signed package by the certificate verifier.
pub fn needs_browser_verifier() -> Problem {
    Problem::error(
        Code::Unsigned,
        ENVELOPE_MEMBER,
        "this package is signed with officials' security keys, which this \
         importer is not set up to verify, so nothing in it is imported",
    )
    .id("package.webauthn-not-enabled")
}

/// The envelope, format 1. Every field is required and no other is read.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvelopeV1 {
    pub format: String,
    pub tenant_id: String,
    pub manifest_sha256: String,
    pub bundle: Bundle,
}

/// A browser-signed package that passed every check. Only now may its
/// members be read.
#[derive(Debug, Clone)]
pub struct VerifiedBrowserPackage {
    pub manifest: Manifest,
    pub manifest_sha256: String,
    pub tenant_id: String,
    /// The pin of the roster it was checked against.
    pub roster_sha256: String,
    /// The approvers' and final signer's realm user ids.
    pub approvers: Vec<String>,
    pub final_signer: String,
    /// The delivery's members, as checked.
    pub members: Vec<Artifact>,
}

fn refused(code: Code, id: &str, message: impl Into<String>) -> Report {
    Report::from_problem(
        Problem::error(code, ENVELOPE_MEMBER, message.into()).id(id),
    )
}

fn proof_refused(error: &ProofError) -> Report {
    let code = match error {
        ProofError::Format(_) | ProofError::Encoding(_) => Code::Unreadable,
        ProofError::Roster(_) | ProofError::Policy(_) => Code::UntrustedSigner,
        ProofError::UnknownKey(_) | ProofError::WrongKey(_) => {
            Code::UntrustedSigner
        }
        ProofError::Binding(_) => Code::IntegrityMismatch,
        ProofError::ClientData(_)
        | ProofError::AuthenticatorData(_)
        | ProofError::Signature => Code::BadSignature,
        ProofError::TooFewApprovals { .. }
        | ProofError::Duplicate(_)
        | ProofError::SignerApproved => Code::NotApproved,
    };
    let reason = error.to_string();
    Report::from_problem(
        Problem::error(
            code,
            ENVELOPE_MEMBER,
            format!("the officials' signatures are not accepted: {reason}"),
        )
        .id("package.webauthn-refused")
        .detail("reason", reason),
    )
}

/// Reads the envelope by its declared format, strictly.
fn read_envelope(bytes: &[u8]) -> Result<EnvelopeV1, Report> {
    let value: Value = serde_json::from_slice(bytes).map_err(|error| {
        refused(
            Code::Unreadable,
            "package.webauthn-unreadable",
            format!("the envelope is not JSON: {error}"),
        )
    })?;
    match value.get("format").and_then(Value::as_str) {
        Some(ENVELOPE_V1) => {}
        Some(format) if format.starts_with(ENVELOPE_FAMILY) => {
            return Err(refused(
                Code::IncompatibleVersion,
                "package.webauthn-unknown-format",
                format!("the envelope is in format '{format}', which this version can't read"),
            ))
        }
        _ => {
            return Err(refused(
                Code::Unreadable,
                "package.webauthn-unreadable",
                "the envelope does not say it is a browser-signature envelope",
            ))
        }
    }
    serde_json::from_value(value).map_err(|error| {
        refused(
            Code::Unreadable,
            "package.webauthn-unreadable",
            format!("the envelope could not be read: {error}"),
        )
    })
}

/// A manifest a browser envelope may seal: format 1's fields only, with no
/// certificate approvals, format-2 binding or revocation lists to be
/// mistaken for signatures this verifier did not check.
fn check_plain(manifest: &Manifest) -> Result<(), Report> {
    if manifest.format != ManifestFormat::V1
        || !manifest.approvals.is_empty()
        || manifest.signing.is_some()
        || !manifest.revocation_lists.is_empty()
    {
        return Err(refused(
            Code::InvalidValue,
            "package.webauthn-mixed",
            "a browser-signed package's manifest carries certificate signatures or a format-2 binding",
        ));
    }
    let configuration = &manifest.configuration;
    if configuration.external_id.is_empty() || configuration.revision == 0 {
        return Err(refused(
            Code::InvalidValue,
            "package.webauthn-revision",
            "the manifest names no configuration or revision",
        ));
    }
    if let Some(previous) = &configuration.previous {
        let digest = &previous.manifest_sha256;
        let hex = digest.len() == 64
            && digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
        if previous.revision == 0
            || previous.revision >= configuration.revision
            || !hex
        {
            return Err(refused(
                Code::InvalidValue,
                "package.webauthn-previous",
                "the manifest's previous revision is not an earlier, identified one",
            ));
        }
    }
    Ok(())
}

/// Checks a browser-signed package against `pinned`, the roster and
/// threshold this verifier was provisioned with for its tenant, at `now`
/// where the caller has a clock. Nothing in the package is trusted until it
/// passes; see the module's documentation for the order.
pub fn verify_browser_package(
    bytes: &[u8],
    pinned: &PinnedRoster,
    now: Option<DateTime<Utc>>,
) -> Result<VerifiedBrowserPackage, Report> {
    let names = member_names(bytes).map_err(Report::from_problem)?;
    let browser: Vec<&String> = names
        .iter()
        .filter(|name| is_browser_member(name))
        .collect();
    if browser.is_empty() {
        return Err(refused(
            Code::Unsigned,
            "package.webauthn-no-envelope",
            "this package has no browser-signature envelope",
        ));
    }
    if browser.len() != 1 || browser[0].as_str() != ENVELOPE_MEMBER {
        return Err(refused(
            Code::InvalidValue,
            "package.webauthn-unexpected-member",
            format!(
                "the package has {} where exactly one {ENVELOPE_MEMBER} belongs",
                browser.iter().map(|name| name.as_str()).collect::<Vec<_>>().join(", ")
            ),
        ));
    }
    if names
        .iter()
        .any(|name| name == SIGNATURE_MEMBER || name == CHAIN_MEMBER)
    {
        return Err(refused(
            Code::InvalidValue,
            "package.webauthn-mixed",
            "a package is signed with certificates or with security keys, never both",
        ));
    }
    if !names.iter().any(|name| name == MANIFEST_MEMBER) {
        return Err(refused(
            Code::Unsigned,
            "package.webauthn-no-manifest",
            format!(
                "the package has no {MANIFEST_MEMBER} for its envelope to seal"
            ),
        ));
    }

    let mut manifest_bytes = None;
    let mut envelope_bytes = None;
    for member in named_members(bytes, &[MANIFEST_MEMBER, ENVELOPE_MEMBER])
        .map_err(Report::from_problem)?
    {
        match member.name.as_str() {
            MANIFEST_MEMBER => manifest_bytes = Some(member.bytes),
            ENVELOPE_MEMBER => envelope_bytes = Some(member.bytes),
            _ => {}
        }
    }
    let (Some(manifest_bytes), Some(envelope_bytes)) =
        (manifest_bytes, envelope_bytes)
    else {
        return Err(refused(
            Code::Unreadable,
            "package.webauthn-unreadable",
            "the manifest or its envelope could not be read",
        ));
    };

    let envelope = read_envelope(&envelope_bytes)?;
    let manifest_sha256 = sha256_hex(&manifest_bytes);
    if envelope.manifest_sha256 != manifest_sha256 {
        return Err(refused(
            Code::IntegrityMismatch,
            "package.webauthn-manifest-changed",
            "the envelope seals another manifest than this one",
        ));
    }
    if envelope.tenant_id != pinned.tenant() {
        return Err(refused(
            Code::UntrustedSigner,
            "package.webauthn-other-tenant",
            "the package was signed for another tenant",
        ));
    }

    let manifest =
        read_manifest(&manifest_bytes).map_err(Report::from_problem)?;
    check_plain(&manifest)?;
    if let Some(now) = now {
        let produced =
            unix_time(&manifest.produced.at).map_err(Report::from_problem)?;
        if is_after(produced, now) {
            return Err(Report::from_problem(signed_in_the_future(
                &manifest.produced.at,
                now,
            )));
        }
    }

    let configuration = &manifest.configuration;
    let sealed = SealedManifest {
        envelope: ENVELOPE_V1,
        external_id: &configuration.external_id,
        revision: configuration.revision,
        previous: configuration.previous.as_ref().map(|previous| {
            (previous.revision, previous.manifest_sha256.as_str())
        }),
        content_sha256: &manifest.content_sha256,
        manifest_sha256: &manifest_sha256,
    };
    let verified = verify_bundle_for_package(&envelope.bundle, pinned, sealed)
        .map_err(|error| proof_refused(&error))?;

    // Only now is the delivery expanded, and every member checked.
    let delivery = payload_without(bytes, &[ENVELOPE_MEMBER])
        .map_err(Report::from_problem)?;
    let mut report = Report::default();
    compare_entries(&delivery.files, &manifest.content.files, "", &mut report);
    match manifest.content.sha256() {
        Ok(digest) if digest == manifest.content_sha256 => {}
        Ok(digest) => report.push(
            Problem::error(
                Code::IntegrityMismatch,
                "manifest.content_sha256",
                format!(
                    "the manifest's content digest is {}, and its content's is {digest}",
                    manifest.content_sha256
                ),
            )
            .id("package.content-mismatch"),
        ),
        Err(problem) => report.push(problem),
    }
    if report.has_errors() {
        return Err(report);
    }

    Ok(VerifiedBrowserPackage {
        tenant_id: envelope.tenant_id,
        roster_sha256: pinned.sha256().to_string(),
        approvers: verified.approvers,
        final_signer: verified.final_signer,
        members: delivery.members,
        manifest_sha256,
        manifest,
    })
}

/// The label a browser-verified package's identities are recorded under, in
/// a [`VerifiedPackage`], before the roster's pin. A label for the record
/// only: what verified a package is its typed
/// [`VerifiedPackage::scheme`], never this.
pub const BROWSER_ISSUER_LABEL: &str = "sequent.webauthn-roster:";

impl VerifiedBrowserPackage {
    /// The package as importers record and track every verified package:
    /// the same manifest, digest and members, with
    /// [`VerificationScheme::Browser`] and the roster's pin. The signer and
    /// approvers are the officials' realm user ids, recorded under
    /// [`BROWSER_ISSUER_LABEL`] and the pin, with the pin as their
    /// fingerprint and no serial or key pin: they are not certificates.
    pub fn into_verified_package(self) -> VerifiedPackage {
        let issuer = format!("{BROWSER_ISSUER_LABEL}{}", self.roster_sha256);
        let identity = |subject: &str| CertificateIdentity {
            subject: subject.to_string(),
            issuer: issuer.clone(),
            serial: String::new(),
            fingerprint_sha256: self.roster_sha256.clone(),
            spki_sha256: String::new(),
        };
        VerifiedPackage {
            scheme: VerificationScheme::Browser {
                roster_sha256: self.roster_sha256.clone(),
            },
            signer: identity(&self.final_signer),
            approvers: self
                .approvers
                .iter()
                .map(|subject| identity(subject))
                .collect(),
            signer_id: Some(self.final_signer.clone()),
            approver_ids: self.approvers.clone(),
            manifest: self.manifest,
            manifest_sha256: self.manifest_sha256,
            members: self.members,
        }
    }
}

/// Whether a verified package was admitted on officials' security keys: by
/// its typed scheme, as the verifier set it.
pub fn is_browser_verified(package: &VerifiedPackage) -> bool {
    matches!(package.scheme, VerificationScheme::Browser { .. })
}

/// Whether a browser-verified package may be imported after `last`, the
/// tenant's last import of the same configuration, read under the tenant's
/// import lock. Stricter than the certificate rule: the package must name
/// exactly that import as its previous revision (its revision and manifest
/// digest), or name none where nothing was imported. The very package
/// imported last is reported as such; anything else is refused.
pub fn browser_freshness(
    package: &VerifiedPackage,
    last: Option<&LastImport>,
) -> Result<Freshness, Problem> {
    let configuration = &package.manifest.configuration;
    let previous = configuration
        .previous
        .as_ref()
        .map(|previous| (previous.revision, previous.manifest_sha256.as_str()));
    let refused = |message: String| {
        Problem::error(
            Code::Rollback,
            "manifest.configuration.previous",
            message,
        )
        .id("package.webauthn-not-next")
    };
    match last {
        None if previous.is_none() => Ok(Freshness::New),
        None => Err(refused(format!(
            "revision {} follows revision {}, which this tenant never imported",
            configuration.revision,
            previous.map(|(revision, _)| revision).unwrap_or_default()
        ))),
        Some(last) if last.manifest_sha256 == package.manifest_sha256 => {
            Ok(Freshness::AlreadyImported)
        }
        Some(last)
            if configuration.revision > last.revision
                && previous == Some((last.revision, last.manifest_sha256.as_str())) =>
        {
            Ok(Freshness::New)
        }
        Some(last) => Err(refused(format!(
            "revision {} does not follow revision {} (manifest {}), the last one this tenant \
             imported",
            configuration.revision, last.revision, last.manifest_sha256
        ))),
    }
}

#[cfg(test)]
#[path = "webauthn_package_tests.rs"]
mod webauthn_package_tests;
