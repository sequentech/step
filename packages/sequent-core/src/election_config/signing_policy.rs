// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The v2 part of a tenant's configuration-signing policy: who may approve or
//! sign, identified by a stable signer ID that holds one or more enrolled keys.
//!
//! A certificate is not a person: one person may hold several certificates, or
//! a reissued one. So a v2 package's approvals and final signature are counted
//! per **signer ID**, and a key reaches an ID only through an SPKI pin (SHA-256
//! of the certificate's SubjectPublicKeyInfo) enrolled in the importing
//! tenant's own policy. A pin belongs to one ID only, and the package never
//! says who is trusted: the tenant does.
//!
//! The policy's digest ([`policy_sha256`]) is what approvers and the final
//! signer bind to, so a policy changed after approval no longer matches.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::json;

use super::manifest::sha256_hex;
use super::package_verify::{canonical_certificates_pem, ConfigurationSigning};
use crate::signing::canonical_json;

/// What [`policy_sha256`] hashes, named so a future form can't collide.
pub const SIGNING_POLICY_FORMAT: &str = "sequent.configuration-signing-policy/2";

/// The oldest package format a tenant still imports.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum PackageFormat {
    /// Format 1 and 2. Format 1 is checked as it always was.
    #[default]
    V1,
    /// Format 2 only: a format-1 package is refused (no downgrade).
    V2,
}

/// What an enrolled signer may do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SignerRole {
    Approver,
    FinalSigner,
}

impl SignerRole {
    pub fn as_str(self) -> &'static str {
        match self {
            SignerRole::Approver => "approver",
            SignerRole::FinalSigner => "final-signer",
        }
    }
}

/// Whether the final signer may also be one of the approvers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinalSignerSeparation {
    /// The final signer is not one of the package's approvers.
    #[default]
    Separate,
    Allowed,
}

/// One key a signer holds: the SHA-256 of its certificate's
/// SubjectPublicKeyInfo, lowercase hex. A reissued certificate for the same
/// key keeps the pin; a new key is a new pin under the same signer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnrolledKey {
    pub spki_sha256: String,
    /// What the key is to a person (a token, a file). Not checked.
    #[serde(default)]
    pub label: String,
}

/// A person or an organization's key custodian, with a stable ID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnrolledSigner {
    pub id: String,
    pub name: String,
    pub roles: Vec<SignerRole>,
    pub keys: Vec<EnrolledKey>,
}

/// A problem with the v2 part of a policy, as `check` reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyProblem {
    pub id: &'static str,
    pub message: String,
}

fn problem(id: &'static str, message: impl Into<String>) -> PolicyProblem {
    PolicyProblem {
        id,
        message: message.into(),
    }
}

/// A signer ID: what approvals and manifests name, so plain and stable.
fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_' || c == '.')
}

fn valid_pin(pin: &str) -> bool {
    pin.len() == 64 && pin.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
}

/// The enrolled signers, indexed by key and by ID. Built only from a policy
/// that passed [`check_signers`], so a pin has exactly one owner.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Enrollment {
    by_pin: BTreeMap<String, String>,
    by_id: BTreeMap<String, EnrolledSigner>,
}

impl Enrollment {
    pub fn from_signers(signers: &[EnrolledSigner]) -> Result<Self, Vec<PolicyProblem>> {
        let problems = check_signers(signers);
        if !problems.is_empty() {
            return Err(problems);
        }
        let mut enrollment = Enrollment::default();
        for signer in signers {
            for key in &signer.keys {
                enrollment.by_pin.insert(key.spki_sha256.clone(), signer.id.clone());
            }
            enrollment.by_id.insert(signer.id.clone(), signer.clone());
        }
        Ok(enrollment)
    }

    /// The signer a key belongs to, by its SPKI pin.
    pub fn signer_of(&self, spki_sha256: &str) -> Option<&EnrolledSigner> {
        self.by_pin.get(spki_sha256).and_then(|id| self.by_id.get(id))
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

/// Each signer is well formed, IDs are unique, and every pin belongs to one
/// signer only.
pub fn check_signers(signers: &[EnrolledSigner]) -> Vec<PolicyProblem> {
    let mut problems = Vec::new();
    let mut ids = BTreeSet::new();
    let mut owners: BTreeMap<&str, &str> = BTreeMap::new();
    for signer in signers {
        if !valid_id(&signer.id) {
            problems.push(problem(
                "configuration-signing.signer-id",
                format!(
                    "signer ID '{}' must be 1–64 characters of a–z, 0–9, '-', '_' or '.'",
                    signer.id
                ),
            ));
        } else if !ids.insert(signer.id.as_str()) {
            problems.push(problem(
                "configuration-signing.duplicate-signer",
                format!("signer ID '{}' is enrolled twice", signer.id),
            ));
        }
        if signer.name.trim().is_empty() {
            problems.push(problem(
                "configuration-signing.signer-name",
                format!("signer '{}' has no name", signer.id),
            ));
        }
        if signer.roles.is_empty() {
            problems.push(problem(
                "configuration-signing.signer-roles",
                format!("signer '{}' has no role", signer.id),
            ));
        }
        if signer.keys.is_empty() {
            problems.push(problem(
                "configuration-signing.signer-keys",
                format!("signer '{}' has no enrolled key", signer.id),
            ));
        }
        for key in &signer.keys {
            if !valid_pin(&key.spki_sha256) {
                problems.push(problem(
                    "configuration-signing.key-pin",
                    format!(
                        "signer '{}': '{}' is not a SHA-256 SPKI pin (64 lowercase hex)",
                        signer.id, key.spki_sha256
                    ),
                ));
                continue;
            }
            match owners.get(key.spki_sha256.as_str()) {
                Some(owner) if *owner == signer.id => problems.push(problem(
                    "configuration-signing.duplicate-key",
                    format!("signer '{}' enrolls key {} twice", signer.id, key.spki_sha256),
                )),
                Some(owner) => problems.push(problem(
                    "configuration-signing.shared-key",
                    format!(
                        "key {} is enrolled for both '{}' and '{}': a key belongs to one signer",
                        key.spki_sha256, owner, signer.id
                    ),
                )),
                None => {
                    owners.insert(key.spki_sha256.as_str(), signer.id.as_str());
                }
            }
        }
    }
    problems
}

/// The policy's v2 requirements beyond each signer being well formed: a
/// tenant that requires format 2 can actually be satisfied.
pub fn check_policy(settings: &ConfigurationSigning) -> Vec<PolicyProblem> {
    let mut problems = check_signers(&settings.signers);
    // A new policy states its minimum format: leaving it out must never
    // quietly let format 1 bypass the enrollment it sets up.
    if settings.minimum_package_format.is_none()
        && (!settings.signers.is_empty() || settings.final_signer_separation.is_some())
    {
        problems.push(problem(
            "configuration-signing.minimum-format-required",
            "a policy that enrolls signers must state its minimum package format",
        ));
    }
    let minimum = settings.effective_minimum_format();
    // Optional signatures with format 2 required: an unsigned file would
    // bypass the enrollment, so the two can't be set together.
    if minimum == PackageFormat::V2
        && settings.policy == super::package_verify::ConfigurationSigningPolicy::Optional
    {
        problems.push(problem(
            "configuration-signing.optional-with-v2",
            "format 2 is required, so signatures must be required too",
        ));
    }
    if minimum == PackageFormat::V2 || !settings.signers.is_empty() {
        let required = usize::from(settings.required_approvals);
        let approver_ids: Vec<&str> = settings
            .signers
            .iter()
            .filter(|s| s.roles.contains(&SignerRole::Approver))
            .map(|s| s.id.as_str())
            .collect();
        let approvers = approver_ids.len();
        let finals = settings
            .signers
            .iter()
            .filter(|s| s.roles.contains(&SignerRole::FinalSigner))
            .collect::<Vec<_>>();
        // Under `separate`, some final signer must leave `required` approvers
        // who are other people: a signer with both roles can't be counted on
        // both sides of the same package.
        if settings.effective_separation() == FinalSignerSeparation::Separate
            && !finals.is_empty()
            && approvers >= required
            && !finals.iter().any(|f| {
                approver_ids.iter().filter(|id| **id != f.id).count() >= required
            })
        {
            problems.push(problem(
                "configuration-signing.separation-unsatisfiable",
                format!(
                    "the final signer must not approve, and no final signer \
                     leaves {required} other enrolled approvers"
                ),
            ));
        }
        if approvers < required {
            problems.push(problem(
                "configuration-signing.too-few-approvers",
                format!(
                    "{approvers} enrolled approvers, and {} approvals are required",
                    settings.required_approvals
                ),
            ));
        }
        if finals.is_empty() {
            problems.push(problem(
                "configuration-signing.no-final-signer",
                "no enrolled signer may make the final signature",
            ));
        }
    }
    if minimum == PackageFormat::V2 && settings.signers.is_empty() {
        problems.push(problem(
            "configuration-signing.no-signers",
            "format 2 is required, and no signer is enrolled",
        ));
    }
    problems
}

/// The digest approvers and the final signer bind to: SHA-256 of the
/// canonical JSON (sorted keys, no whitespace) of everything in the policy
/// that decides what imports, in exactly this form:
///
/// - `format`: [`SIGNING_POLICY_FORMAT`];
/// - `policy`, `required_approvals`, the *effective* `minimum_package_format`
///   and `final_signer_separation`;
/// - `package_roots` and `staff_roots`: each root set as the **sorted, deduplicated
///   list of the SHA-256 of each certificate's DER**, so PEM order,
///   line wrapping, whitespace and repeats don't matter;
/// - `signers`: sorted by `id`, each with `name`, its roles as a sorted set,
///   and its key pins as a sorted set (key labels are notes, not policy).
///
/// Two installations holding the same policy compute the same digest, whatever
/// the order or formatting of their files.
pub fn policy_sha256(settings: &ConfigurationSigning) -> Result<String, String> {
    let roots = |pem: &str| -> Result<Vec<String>, String> {
        if pem.trim().is_empty() {
            return Ok(Vec::new());
        }
        // Read the same way the verifier reads them; refuse what isn't only
        // certificates.
        canonical_certificates_pem(pem)?;
        let set: BTreeSet<String> = super::package_verify::certificates_from_pem(pem)?
            .iter()
            .map(|der| sha256_hex(der))
            .collect();
        Ok(set.into_iter().collect())
    };
    let mut signers = settings.signers.clone();
    signers.sort_by(|a, b| a.id.cmp(&b.id));
    let signers: Vec<_> = signers
        .into_iter()
        .map(|signer| {
            let roles: BTreeSet<_> = signer.roles.iter().map(|r| r.as_str()).collect();
            let keys: BTreeSet<_> = signer.keys.iter().map(|k| k.spki_sha256.clone()).collect();
            json!({
                "id": signer.id,
                "name": signer.name,
                "roles": roles,
                "keys": keys,
            })
        })
        .collect();
    let value = json!({
        "format": SIGNING_POLICY_FORMAT,
        "policy": settings.policy,
        "package_roots": roots(&settings.package_roots)?,
        "staff_roots": roots(&settings.staff_roots)?,
        "required_approvals": settings.required_approvals,
        "minimum_package_format": settings.effective_minimum_format(),
        "final_signer_separation": settings.effective_separation(),
        "signers": signers,
    });
    canonical_json(&value)
        .map(|canonical| sha256_hex(canonical.as_bytes()))
        .map_err(|error| error.to_string())
}

#[cfg(test)]
#[path = "signing_policy_tests.rs"]
mod signing_policy_tests;
