// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Format-2 signing without a service: what a standalone Election Architect
//! (no login) and the local PKCS#11 signing companion share, so both build and
//! check exactly the same bytes the importer verifies.
//!
//! 1. The editor's compiled revision becomes an [`ApprovalRequest`]: the
//!    configuration, its content (files, ballot designs, reports), and the
//!    binding every approver signs (target tenant, revision, content digest,
//!    policy digest), with the code they read aloud.
//! 2. Each approver checks the request against the delivery
//!    ([`check_request`]) and signs [`super::manifest::approval_payload_v2`].
//! 3. The final signer assembles the manifest ([`assemble_manifest`]), which
//!    first checks every approval, role and the separation rule against the
//!    installed policy, then signs its bytes; [`assemble_package`] zips it.
//!
//! The policy is the installation's ([`InstalledSigning`]), never the
//! package's. The importer checks everything again against its own tenant's
//! current policy, at its own time.

use std::collections::BTreeMap;
use std::time::Duration;

use chrono::{DateTime, SecondsFormat, Utc};
use rustls_pki_types::UnixTime;
use serde::{Deserialize, Serialize};

use super::archive::Artifact;
use super::manifest::{
    approval_code_v2, file_entries, package, payload, Approval,
    ApprovalBinding, ConfigurationRevision, Content, Manifest, ManifestFormat,
    Produced, RevisionContent, SigningBinding,
};
use super::package_verify::{
    check_approvals_v2, ConfigurationSigning, ConfigurationSigningPolicy,
};
use super::problem::{Code, Problem, Report};
use super::signing_policy::{check_policy, PackageFormat, SignerRole};

/// What [`ApprovalRequest`] is.
pub const APPROVAL_REQUEST_FORMAT: &str = "sequent.approval-request/2";

/// The installation's signing policy, installed beside the delivery profile
/// and never edited in the app: the target tenant and its format-2 policy,
/// the same content the tenant's administrator installs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstalledSigning {
    pub tenant_id: String,
    pub signing: ConfigurationSigning,
}

impl InstalledSigning {
    /// Reads and checks it: a format-2 policy that requires signatures,
    /// enrolls signers and can be satisfied.
    pub fn read(json: &str) -> Result<Self, Report> {
        let installed: InstalledSigning = serde_json::from_str(json).map_err(|error| {
            Report::from_problem(
                Problem::error(
                    Code::Unreadable,
                    "signing-policy.json",
                    format!("the installed signing policy can't be read: {error}"),
                )
                .id("signing-policy.unreadable"),
            )
        })?;
        let mut report = Report::default();
        if installed.tenant_id.trim().is_empty() {
            report.push(
                Problem::error(
                    Code::MissingField,
                    "tenant_id",
                    "the policy names no tenant",
                )
                .id("signing-policy.no-tenant"),
            );
        }
        if installed.signing.minimum_package_format != Some(PackageFormat::V2)
            || installed.signing.policy != ConfigurationSigningPolicy::Required
            || installed.signing.signers.is_empty()
        {
            report.push(
                Problem::error(
                    Code::InvalidValue,
                    "signing",
                    "standalone signing needs a format-2 policy that requires \
                     signatures and enrolls signers",
                )
                .id("signing-policy.not-v2"),
            );
        }
        for found in check_policy(&installed.signing) {
            report.push(
                Problem::error(
                    Code::InvalidValue,
                    "signing.signers",
                    found.message,
                )
                .id(found.id),
            );
        }
        if report.has_errors() {
            Err(report)
        } else {
            Ok(installed)
        }
    }

    pub fn policy_sha256(&self) -> Result<String, Problem> {
        self.signing.policy_sha256()
    }
}

/// What the editor hands every approver and the final signer: what is to be
/// approved, and what they bind to. Travels with the delivery zip.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApprovalRequest {
    pub format: String,
    pub configuration: ConfigurationRevision,
    pub delivery_name: String,
    pub content: Content,
    pub binding: ApprovalBinding,
    /// [`approval_code_v2`] of the binding: read aloud and compared.
    pub code: String,
}

/// The request for a compiled revision, bound to the installed policy.
pub fn approval_request(
    installed: &InstalledSigning,
    configuration: ConfigurationRevision,
    revision: &RevisionContent,
) -> Result<ApprovalRequest, Report> {
    let binding = ApprovalBinding {
        tenant_id: installed.tenant_id.clone(),
        external_id: configuration.external_id.clone(),
        revision: configuration.revision,
        content_sha256: revision
            .content
            .sha256()
            .map_err(Report::from_problem)?,
        policy_sha256: installed
            .policy_sha256()
            .map_err(Report::from_problem)?,
    };
    let code = approval_code_v2(&binding).map_err(Report::from_problem)?;
    Ok(ApprovalRequest {
        format: APPROVAL_REQUEST_FORMAT.to_string(),
        configuration,
        delivery_name: revision.delivery_name.clone(),
        content: revision.content.clone(),
        binding,
        code,
    })
}

fn request_problem(
    id: &str,
    path: &str,
    message: impl Into<String>,
) -> Problem {
    Problem::error(Code::InvalidValue, path, message).id(id)
}

/// A request that passed [`check_request`] against the installed policy and
/// its delivery. Only `check_request` makes one (the fields are private), so
/// nothing can assemble or sign a request that wasn't checked.
#[derive(Debug, Clone)]
pub struct CheckedRequest {
    request: ApprovalRequest,
    delivery: Vec<u8>,
    members: Vec<Artifact>,
}

impl CheckedRequest {
    pub fn request(&self) -> &ApprovalRequest {
        &self.request
    }
    pub fn members(&self) -> &[Artifact] {
        &self.members
    }
}

/// The request is what it says: its delivery's files are the listed ones,
/// its digest is its content's, it is for the installed tenant and policy,
/// its binding names its own configuration, and its code is its binding's.
pub fn check_request(
    installed: &InstalledSigning,
    request: &ApprovalRequest,
    delivery: &[u8],
) -> Result<CheckedRequest, Report> {
    let mut report = Report::default();
    if request.format != APPROVAL_REQUEST_FORMAT {
        report.push(request_problem(
            "request.format",
            "format",
            "this is not a format-2 approval request",
        ));
    }
    let expanded = payload(delivery).map_err(Report::from_problem)?;
    if expanded.files != request.content.files {
        report.push(request_problem(
            "request.files-differ",
            "content.files",
            "the delivery's files are not the ones the request lists",
        ));
    }
    match request.content.sha256() {
        Ok(digest) if digest == request.binding.content_sha256 => {}
        Ok(_) => report.push(request_problem(
            "request.content-digest",
            "binding.content_sha256",
            "the request's content digest is not its content's",
        )),
        Err(problem) => report.push(problem),
    }
    if request.binding.tenant_id != installed.tenant_id {
        report.push(request_problem(
            "request.wrong-tenant",
            "binding.tenant_id",
            "the request is for another tenant than this installation's",
        ));
    }
    match installed.policy_sha256() {
        Ok(digest) if digest == request.binding.policy_sha256 => {}
        Ok(_) => report.push(request_problem(
            "request.policy-changed",
            "binding.policy_sha256",
            "the request was made under another signing policy than the installed one",
        )),
        Err(problem) => report.push(problem),
    }
    if request.binding.external_id != request.configuration.external_id
        || request.binding.revision != request.configuration.revision
    {
        report.push(request_problem(
            "request.configuration",
            "binding",
            "the binding names another configuration or revision",
        ));
    }
    match approval_code_v2(&request.binding) {
        Ok(code) if code == request.code => {}
        Ok(_) => report.push(request_problem(
            "request.code",
            "code",
            "the request's code is not its binding's",
        )),
        Err(problem) => report.push(problem),
    }
    if report.has_errors() {
        Err(report)
    } else {
        Ok(CheckedRequest {
            request: request.clone(),
            delivery: delivery.to_vec(),
            members: expanded.members,
        })
    }
}

/// Who makes the final signature, and how it is recorded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinalSigner {
    /// The enrolled signer ID of the final signer.
    pub signer_id: String,
    /// What signs: `p12` in a browser, `pkcs11:<provider>`; recorded, never
    /// trusted (only the enrolled key decides who signed).
    pub key_label: String,
    /// The tool and version, by name.
    pub producer: BTreeMap<String, String>,
}

fn unix(now: DateTime<Utc>) -> UnixTime {
    UnixTime::since_unix_epoch(Duration::from_secs(
        u64::try_from(now.timestamp()).unwrap_or(0),
    ))
}

/// The format-2 manifest for `request`, once every approval counts under the
/// installed policy at `now`, the final signer may sign, and separation holds.
/// The caller signs its bytes ([`Manifest::to_bytes`]) and hands them to
/// [`assemble_package`].
pub fn assemble_manifest(
    installed: &InstalledSigning,
    checked: &CheckedRequest,
    approvals: Vec<Approval>,
    final_signer: &FinalSigner,
    now: DateTime<Utc>,
) -> Result<Manifest, Report> {
    // Re-checked against *this* installed policy: a request checked under
    // another one doesn't pass.
    let checked =
        check_request(installed, &checked.request, &checked.delivery)?;
    let request = checked.request();
    let trust = installed
        .signing
        .trust_for(&installed.tenant_id, Vec::new())
        .map_err(Report::from_problem)?;
    let Some(v2) = trust.v2.as_ref() else {
        return Err(Report::from_problem(request_problem(
            "signing-policy.not-v2",
            "signing",
            "the installed policy enrolls no signers",
        )));
    };
    let mut report = Report::default();
    let Some(enrolled) = v2_signer(installed, &final_signer.signer_id) else {
        return Err(Report::from_problem(request_problem(
            "package.final-signer-not-enrolled",
            "signer_id",
            format!("{} is not an enrolled signer", final_signer.signer_id),
        )));
    };
    if !enrolled.roles.contains(&SignerRole::FinalSigner) {
        report.push(request_problem(
            "package.final-signer-role",
            "signer_id",
            format!("{} may not make the final signature", enrolled.id),
        ));
    }
    let manifest = Manifest {
        format: ManifestFormat::V2,
        configuration: request.configuration.clone(),
        content_sha256: request.binding.content_sha256.clone(),
        content: request.content.clone(),
        approvals,
        produced: Produced {
            at: now.to_rfc3339_opts(SecondsFormat::Secs, true),
            custodian: enrolled.name.clone(),
            key_label: final_signer.key_label.clone(),
            producer: final_signer.producer.clone(),
        },
        revocation_lists: Vec::new(),
        signing: Some(SigningBinding {
            tenant_id: request.binding.tenant_id.clone(),
            policy_sha256: request.binding.policy_sha256.clone(),
            final_signer_id: final_signer.signer_id.clone(),
            role: SignerRole::FinalSigner.as_str().to_string(),
        }),
    };
    let (_, approver_ids) = check_approvals_v2(
        &manifest,
        &request.binding,
        &trust,
        v2,
        &[],
        unix(now),
        now,
        &mut report,
    );
    if installed.signing.effective_separation()
        == super::signing_policy::FinalSignerSeparation::Separate
        && approver_ids.contains(&final_signer.signer_id)
    {
        report.push(request_problem(
            "package.final-signer-approved",
            "signer_id",
            format!(
                "{} approved this revision and may not also make its final signature",
                final_signer.signer_id
            ),
        ));
    }
    if report.has_errors() {
        Err(report)
    } else {
        Ok(manifest)
    }
}

/// One approval, checked as the importer will count it: its signature over
/// this request's payload, its chain at `now`, its key enrolled for the
/// signer ID it names, with the approver role. What an approver's page runs
/// before handing the approval file out. Returns the signer ID.
pub fn check_approval(
    installed: &InstalledSigning,
    checked: &CheckedRequest,
    approval: &Approval,
    now: DateTime<Utc>,
) -> Result<String, Report> {
    let checked =
        check_request(installed, &checked.request, &checked.delivery)?;
    let request = checked.request();
    let mut trust = installed
        .signing
        .trust_for(&installed.tenant_id, Vec::new())
        .map_err(Report::from_problem)?;
    // One approval at a time: the count is the final signer's to check.
    trust.required_approvals = 1;
    let Some(v2) = trust.v2.clone() else {
        return Err(Report::from_problem(request_problem(
            "signing-policy.not-v2",
            "signing",
            "the installed policy enrolls no signers",
        )));
    };
    let manifest = Manifest {
        format: ManifestFormat::V2,
        configuration: request.configuration.clone(),
        content_sha256: request.binding.content_sha256.clone(),
        content: request.content.clone(),
        approvals: vec![approval.clone()],
        produced: Produced {
            at: now.to_rfc3339_opts(SecondsFormat::Secs, true),
            custodian: String::new(),
            key_label: String::new(),
            producer: BTreeMap::new(),
        },
        revocation_lists: Vec::new(),
        signing: None,
    };
    let mut report = Report::default();
    let (_, ids) = check_approvals_v2(
        &manifest,
        &request.binding,
        &trust,
        &v2,
        &[],
        unix(now),
        now,
        &mut report,
    );
    match (report.has_errors(), ids.into_iter().next()) {
        (false, Some(id)) => Ok(id),
        _ => Err(report),
    }
}

fn v2_signer<'a>(
    installed: &'a InstalledSigning,
    id: &str,
) -> Option<&'a super::signing_policy::EnrolledSigner> {
    installed.signing.signers.iter().find(|s| s.id == id)
}

/// The signed package, returned only once the importer's own full check
/// passes against the installed tenant and policy at `now`: the delivery's
/// members, the manifest, its signature and the signer's chain (PEM, leaf
/// first). Nothing that fails verification is ever handed out.
pub fn assemble_verified_package(
    installed: &InstalledSigning,
    checked: &CheckedRequest,
    manifest_bytes: &[u8],
    signature: &[u8],
    chain_pem: &str,
    now: DateTime<Utc>,
) -> Result<Artifact, Report> {
    let checked =
        check_request(installed, &checked.request, &checked.delivery)?;
    let request = checked.request();
    if file_entries(checked.members()).map_err(Report::from_problem)?
        != request.content.files
    {
        return Err(Report::from_problem(request_problem(
            "request.files-differ",
            "content.files",
            "the delivery's files are not the ones the request lists",
        )));
    }
    let name = format!(
        "{}-r{}.zip",
        request.delivery_name.trim_end_matches(".zip"),
        request.configuration.revision
    );
    let artifact = package(
        &name,
        checked.members(),
        manifest_bytes,
        signature,
        chain_pem,
    )
    .map_err(Report::from_problem)?;
    let trust = installed
        .signing
        .trust_for(&installed.tenant_id, Vec::new())
        .map_err(Report::from_problem)?;
    let verified =
        super::package_verify::verify_package_at(&artifact.bytes, &trust, now)?;
    // And it is the request's revision, not some other manifest signed here.
    if verified.manifest.content_sha256 != request.binding.content_sha256
        || verified.manifest.configuration != request.configuration
    {
        return Err(Report::from_problem(request_problem(
            "request.manifest-differs",
            "manifest.json",
            "the signed manifest is not the request's",
        )));
    }
    Ok(artifact)
}
