// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! A signed configuration package: a delivery, with a manifest of what it
//! holds, the approvals it was given and the signature over it.
//!
//! The package is the delivery's zip with three more members at its root:
//!
//! * `manifest.json`: [`Manifest`], the configuration's revision, every
//!   member's size and SHA-256 (the nested zips' members too), each ballot
//!   design's version and digest, each report setting and the approvals.
//! * `manifest.sig`: the signature over the bytes of `manifest.json`, made by
//!   the organization's key in its HSM.
//! * `manifest.chain.pem`: the signing certificate and its issuers.
//!
//! Readers of a delivery ignore root members they don't know, so a package
//! still opens wherever a delivery did.
//!
//! Every digest is of bytes as emitted, never of a re-serialization: map order
//! and deflate output differ between builds, and a digest that depends on
//! them stops matching for no reason anyone could see.
//!
//! Checking a package is in [`super::package_verify`].

use std::collections::BTreeMap;
use std::io::{Cursor, Read};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::signing::{canonical_json, crockford_code, SignatureAlgorithm};

use super::archive::{zip, Artifact};
use super::design::BallotDesign;
use super::problem::{Code, Problem};

pub const MANIFEST_MEMBER: &str = "manifest.json";
pub const SIGNATURE_MEMBER: &str = "manifest.sig";
pub const CHAIN_MEMBER: &str = "manifest.chain.pem";

/// Members whose contents are hashed as a list of their own members too.
const NESTED_ZIP_SUFFIX: &str = ".zip";

/// What `manifest.json` is. A reader refuses a format it doesn't know rather
/// than guessing at its fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ManifestFormat {
    #[serde(rename = "sequent.configuration-manifest/1")]
    V1,
}

impl ManifestFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            ManifestFormat::V1 => "sequent.configuration-manifest/1",
        }
    }
}

/// What an approver signs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalFormat {
    #[serde(rename = "sequent.configuration-approval/1")]
    V1,
}

impl ApprovalFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            ApprovalFormat::V1 => "sequent.configuration-approval/1",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub format: ManifestFormat,
    pub configuration: ConfigurationRevision,
    /// SHA-256 of the canonical JSON of [`Manifest::content`]: what the
    /// approvers sign.
    pub content_sha256: String,
    pub content: Content,
    pub approvals: Vec<Approval>,
    pub produced: Produced,
    /// The newest revocation lists the producer had, PEM, so an offline
    /// importer learns of a revoked key with the next package.
    #[serde(default)]
    pub revocation_lists: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigurationRevision {
    pub external_id: String,
    pub name: String,
    /// Starts at 1, goes up by one and is never reused.
    pub revision: u64,
    pub previous: Option<PreviousRevision>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviousRevision {
    pub revision: u64,
    pub manifest_sha256: String,
}

/// Everything the approvers approve.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Content {
    pub files: Vec<FileEntry>,
    pub ballot_designs: Vec<BallotDesign>,
    pub reports: Vec<ReportSetting>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub size: u64,
    pub sha256: String,
    /// A nested zip's own members, each hashed as stored uncompressed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<FileEntry>,
}

pub use super::report::ReportFormat;

/// How one report is generated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportSetting {
    /// The platform's report type, as `report.report_type` stores it.
    pub report_type: String,
    pub formats: Vec<ReportFormat>,
    pub copies: u32,
    /// The template's alias, when the report uses the organization's design.
    pub template: Option<String>,
    pub template_sha256: Option<String>,
}

/// One approver's signature over [`approval_payload`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Approval {
    pub name: String,
    pub role: String,
    /// RFC 3339, UTC.
    pub signed_at: String,
    pub algorithm: SignatureAlgorithm,
    /// The approver's certificate and its issuers, PEM, leaf first.
    pub certificate_chain: String,
    /// The signature, base64 DER.
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Produced {
    /// RFC 3339, UTC: when the package was signed.
    pub at: String,
    pub custodian: String,
    pub key_label: String,
    /// The versions of what produced it, by name.
    pub producer: BTreeMap<String, String>,
}

impl Manifest {
    /// `manifest.json` as written: pretty, with a trailing newline. Hash and
    /// sign these bytes, never a re-serialization of a parsed manifest.
    pub fn to_bytes(&self) -> Result<Vec<u8>, Problem> {
        let mut bytes = serde_json::to_vec_pretty(self).map_err(|error| {
            Problem::error(
                Code::InvalidValue,
                MANIFEST_MEMBER,
                format!("the manifest could not be written: {error}"),
            )
            .id("package.unwritable-manifest")
            .detail("reason", error)
        })?;
        bytes.push(b'\n');
        Ok(bytes)
    }
}

impl Content {
    /// SHA-256 of the content's canonical JSON.
    pub fn sha256(&self) -> Result<String, Problem> {
        let value = serde_json::to_value(self)
            .map_err(|error| unwritable_content(error.to_string()))?;
        let canonical = canonical_json(&value)
            .map_err(|error| unwritable_content(error.to_string()))?;
        Ok(sha256_hex(canonical.as_bytes()))
    }
}

fn unwritable_content(reason: String) -> Problem {
    Problem::error(
        Code::InvalidValue,
        "content",
        format!("the configuration's content could not be hashed: {reason}"),
    )
    .id("package.unhashable-content")
    .detail("reason", reason)
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// The bytes every approver of a revision signs: the same for all of them,
/// so they can compare its code aloud.
pub fn approval_payload(
    external_id: &str,
    revision: u64,
    content_sha256: &str,
) -> Result<String, Problem> {
    canonical_json(&json!({
        "format": ApprovalFormat::V1.as_str(),
        "external_id": external_id,
        "revision": revision,
        "content_sha256": content_sha256,
    }))
    .map_err(|error| unwritable_content(error.to_string()))
}

/// The payload's signing code: the first 40 bits of its SHA-256, as
/// Crockford base32 `XXXX-XXXX`.
pub fn approval_code(payload: &str) -> String {
    let digest = Sha256::digest(payload.as_bytes());
    let mut leading = [0u8; 5];
    leading.copy_from_slice(&digest[..5]);
    crockford_code(leading)
}

/// What a revision of a compiled plan holds: the delivery's members and the
/// content the approvers approve.
#[derive(Debug, Clone)]
pub struct RevisionContent {
    /// The delivery zip's name, which the package keeps.
    pub delivery_name: String,
    pub members: Vec<Artifact>,
    pub content: Content,
}

/// A compiled plan as a revision: its delivery's members, its ballot designs
/// numbered against the last signed revision's, and its report settings
/// with the digest of each report's template.
pub fn revision_content(
    plan: &super::architect::Blueprint,
    compiled: &super::architect::Compiled,
    previous_designs: &[BallotDesign],
) -> Result<RevisionContent, super::problem::Report> {
    use super::problem::Report;

    let delivery = super::archive::delivery(&compiled.layout)
        .map_err(Report::from_problem)?;
    let members =
        read_zip(&delivery.bytes, "delivery").map_err(Report::from_problem)?;
    let files = file_entries(&members).map_err(Report::from_problem)?;
    let digests = super::preview::ballot_design_digests(&compiled.bundle)?;
    let ballot_designs = super::design::versioned(previous_designs, digests);

    let mut report = Report::default();
    let reports = plan
        .reports
        .iter()
        .map(|planned| {
            let template_sha256 = planned.template.as_ref().and_then(|alias| {
                let found = compiled
                    .bundle
                    .templates
                    .iter()
                    .find(|template| template.alias == *alias);
                if found.is_none() {
                    report.push(
                        Problem::error(
                            Code::DanglingReference,
                            "reports",
                            format!(
                                "the {} report is drawn with template '{alias}', \
                                 which isn't in the configuration, so its design \
                                 can't be signed",
                                planned.report_type
                            ),
                        )
                        .id("package.report-template-missing")
                        .detail("report", &planned.report_type)
                        .detail("template", alias),
                    );
                }
                found.map(|template| sha256_hex(template.document.as_bytes()))
            });
            let formats = if planned.formats.is_empty() {
                planned.report_type.formats()[..1].to_vec()
            } else {
                planned.formats.clone()
            };
            ReportSetting {
                report_type: planned.report_type.to_string(),
                formats,
                copies: planned.copies,
                template: planned.template.clone(),
                template_sha256,
            }
        })
        .collect();
    if report.has_errors() {
        return Err(report);
    }

    Ok(RevisionContent {
        delivery_name: delivery.name,
        members,
        content: Content {
            files,
            ballot_designs,
            reports,
        },
    })
}

/// What is added, changed or removed between two revisions.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    Added,
    Changed,
    Removed,
}

/// What a change is about.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ChangeSubject {
    File,
    BallotDesign,
    Report,
}

/// One difference between a revision and the one it is compared with.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Change {
    pub subject: ChangeSubject,
    pub kind: ChangeKind,
    /// A file's path (`outer.zip/inner.csv` inside a nested zip), a design's
    /// `area / election`, or a report's type.
    pub name: String,
    /// A design's version before and after.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub versions: Option<(u32, u32)>,
}

/// What changed from `before` to `after`, in a stable order.
pub fn changes(before: &Content, after: &Content) -> Vec<Change> {
    let mut all = Vec::new();
    file_changes(&before.files, &after.files, "", &mut all);

    let design_name = |design: &BallotDesign| {
        format!("{} / {}", design.area, design.election)
    };
    for design in &after.ballot_designs {
        let old = before.ballot_designs.iter().find(|old| {
            old.area == design.area && old.election == design.election
        });
        match old {
            None => all.push(Change {
                subject: ChangeSubject::BallotDesign,
                kind: ChangeKind::Added,
                name: design_name(design),
                versions: None,
            }),
            Some(old) if old.sha256 != design.sha256 => all.push(Change {
                subject: ChangeSubject::BallotDesign,
                kind: ChangeKind::Changed,
                name: design_name(design),
                versions: Some((old.version, design.version)),
            }),
            Some(_) => {}
        }
    }
    for old in &before.ballot_designs {
        if !after.ballot_designs.iter().any(|design| {
            design.area == old.area && design.election == old.election
        }) {
            all.push(Change {
                subject: ChangeSubject::BallotDesign,
                kind: ChangeKind::Removed,
                name: design_name(old),
                versions: None,
            });
        }
    }

    for report in &after.reports {
        match before
            .reports
            .iter()
            .find(|old| old.report_type == report.report_type)
        {
            None => all.push(report_change(ChangeKind::Added, report)),
            Some(old) if old != report => {
                all.push(report_change(ChangeKind::Changed, report))
            }
            Some(_) => {}
        }
    }
    for old in &before.reports {
        if !after
            .reports
            .iter()
            .any(|report| report.report_type == old.report_type)
        {
            all.push(report_change(ChangeKind::Removed, old));
        }
    }
    all
}

fn report_change(kind: ChangeKind, report: &ReportSetting) -> Change {
    Change {
        subject: ChangeSubject::Report,
        kind,
        name: report.report_type.clone(),
        versions: None,
    }
}

fn file_changes(
    before: &[FileEntry],
    after: &[FileEntry],
    within: &str,
    all: &mut Vec<Change>,
) {
    let full = |path: &str| {
        if within.is_empty() {
            path.to_string()
        } else {
            format!("{within}/{path}")
        }
    };
    for entry in after {
        match before.iter().find(|old| old.path == entry.path) {
            None => all.push(Change {
                subject: ChangeSubject::File,
                kind: ChangeKind::Added,
                name: full(&entry.path),
                versions: None,
            }),
            Some(old) if old.sha256 != entry.sha256 => {
                // A nested zip is described by what changed inside it, and
                // by itself only when nothing inside did.
                let before_inner = all.len();
                file_changes(
                    &old.members,
                    &entry.members,
                    &full(&entry.path),
                    all,
                );
                if all.len() == before_inner {
                    all.push(Change {
                        subject: ChangeSubject::File,
                        kind: ChangeKind::Changed,
                        name: full(&entry.path),
                        versions: None,
                    });
                }
            }
            Some(_) => {}
        }
    }
    for old in before {
        if !after.iter().any(|entry| entry.path == old.path) {
            all.push(Change {
                subject: ChangeSubject::File,
                kind: ChangeKind::Removed,
                name: full(&old.path),
                versions: None,
            });
        }
    }
}

/// Every member's entry, with a nested zip's members listed under it, in
/// path order.
pub fn file_entries(members: &[Artifact]) -> Result<Vec<FileEntry>, Problem> {
    let mut entries = members
        .iter()
        .map(|member| {
            let nested = if member.name.ends_with(NESTED_ZIP_SUFFIX) {
                file_entries(&read_zip(&member.bytes, &member.name)?)?
            } else {
                Vec::new()
            };
            Ok(FileEntry {
                path: member.name.clone(),
                size: member.bytes.len() as u64,
                sha256: sha256_hex(&member.bytes),
                members: nested,
            })
        })
        .collect::<Result<Vec<_>, Problem>>()?;
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(entries)
}

/// The members of a zip, in the order they are stored. A name that appears
/// twice is refused: which of the two a reader takes is the reader's choice,
/// and the `zip` crate silently keeps only the last.
pub fn read_zip(bytes: &[u8], what: &str) -> Result<Vec<Artifact>, Problem> {
    let names = central_directory_names(bytes)
        .map_err(|reason| unreadable_zip(what, reason))?;
    for (index, name) in names.iter().enumerate() {
        if names[..index].contains(name) {
            return Err(Problem::error(
                Code::Unreadable,
                what,
                format!("'{name}' appears twice in {what}"),
            )
            .id("package.duplicate-member")
            .detail("file", name)
            .detail("archive", what));
        }
    }

    let mut archive = ::zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| unreadable_zip(what, error.to_string()))?;
    let mut members: Vec<Artifact> = Vec::with_capacity(archive.len());
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| unreadable_zip(what, error.to_string()))?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().to_string();
        let mut contents = Vec::new();
        entry
            .read_to_end(&mut contents)
            .map_err(|error| unreadable_zip(what, error.to_string()))?;
        members.push(Artifact {
            name,
            bytes: contents,
        });
    }
    Ok(members)
}

const END_OF_CENTRAL_DIRECTORY: &[u8] = b"PK\x05\x06";
const ZIP64_LOCATOR: &[u8] = b"PK\x06\x07";
const ZIP64_END_OF_CENTRAL_DIRECTORY: &[u8] = b"PK\x06\x06";
const CENTRAL_DIRECTORY_HEADER: &[u8] = b"PK\x01\x02";

/// Every name in a zip's central directory, duplicates included.
fn central_directory_names(bytes: &[u8]) -> Result<Vec<String>, String> {
    let read = |at: usize, width: usize| -> Result<u64, String> {
        let field = bytes
            .get(at..at + width)
            .ok_or_else(|| "it ends early".to_string())?;
        Ok(field
            .iter()
            .rev()
            .fold(0u64, |acc, byte| (acc << 8) | u64::from(*byte)))
    };

    let last = bytes
        .len()
        .checked_sub(22)
        .ok_or_else(|| "it is too short".to_string())?;
    let end = (last.saturating_sub(u16::MAX as usize)..=last)
        .rev()
        .find(|at| bytes[*at..].starts_with(END_OF_CENTRAL_DIRECTORY))
        .ok_or_else(|| "it has no central directory".to_string())?;

    let mut count = read(end + 10, 2)?;
    let mut offset = read(end + 16, 4)?;
    if count == u64::from(u16::MAX) || offset == u64::from(u32::MAX) {
        let locator = end
            .checked_sub(20)
            .filter(|at| bytes[*at..].starts_with(ZIP64_LOCATOR))
            .ok_or_else(|| "its zip64 directory is missing".to_string())?;
        let zip64 = usize::try_from(read(locator + 8, 8)?)
            .map_err(|_| "its zip64 directory is out of range".to_string())?;
        if !bytes.get(zip64..).is_some_and(|rest| {
            rest.starts_with(ZIP64_END_OF_CENTRAL_DIRECTORY)
        }) {
            return Err("its zip64 directory is missing".to_string());
        }
        count = read(zip64 + 32, 8)?;
        offset = read(zip64 + 48, 8)?;
    }

    let mut at = usize::try_from(offset)
        .map_err(|_| "its central directory is out of range".to_string())?;
    let mut names = Vec::new();
    for _ in 0..count {
        if !bytes
            .get(at..)
            .is_some_and(|rest| rest.starts_with(CENTRAL_DIRECTORY_HEADER))
        {
            return Err("its central directory is damaged".to_string());
        }
        let name_length = read(at + 28, 2)? as usize;
        let extra_length = read(at + 30, 2)? as usize;
        let comment_length = read(at + 32, 2)? as usize;
        let name = bytes
            .get(at + 46..at + 46 + name_length)
            .ok_or_else(|| "it ends early".to_string())?;
        names.push(String::from_utf8_lossy(name).into_owned());
        at += 46 + name_length + extra_length + comment_length;
    }
    Ok(names)
}

fn unreadable_zip(what: &str, reason: String) -> Problem {
    Problem::error(
        Code::Unreadable,
        what,
        format!("{what} could not be read as a zip: {reason}"),
    )
    .id("package.unreadable-zip")
    .detail("archive", what)
    .detail("reason", reason)
}

/// A package's members, separated from its signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenedPackage {
    /// The delivery's members, in the order they are stored.
    pub members: Vec<Artifact>,
    pub manifest: Option<Vec<u8>>,
    pub signature: Option<Vec<u8>>,
    pub chain: Option<Vec<u8>>,
}

impl OpenedPackage {
    pub fn is_signed(&self) -> bool {
        self.manifest.is_some()
            || self.signature.is_some()
            || self.chain.is_some()
    }
}

/// Opens a package, or a plain delivery, without checking anything.
pub fn open_package(bytes: &[u8]) -> Result<OpenedPackage, Problem> {
    let mut opened = OpenedPackage {
        members: Vec::new(),
        manifest: None,
        signature: None,
        chain: None,
    };
    for member in read_zip(bytes, "package")? {
        match member.name.as_str() {
            MANIFEST_MEMBER => opened.manifest = Some(member.bytes),
            SIGNATURE_MEMBER => opened.signature = Some(member.bytes),
            CHAIN_MEMBER => opened.chain = Some(member.bytes),
            _ => opened.members.push(member),
        }
    }
    Ok(opened)
}

/// The signed package: the delivery's members, then the manifest, its
/// signature and the signer's chain.
pub fn package(
    name: &str,
    members: &[Artifact],
    manifest: &[u8],
    signature: &[u8],
    chain_pem: &str,
) -> Result<Artifact, Problem> {
    let mut all = members.to_vec();
    all.push(Artifact {
        name: MANIFEST_MEMBER.to_string(),
        bytes: manifest.to_vec(),
    });
    all.push(Artifact {
        name: SIGNATURE_MEMBER.to_string(),
        bytes: signature.to_vec(),
    });
    all.push(Artifact {
        name: CHAIN_MEMBER.to_string(),
        bytes: chain_pem.as_bytes().to_vec(),
    });
    Ok(Artifact {
        name: name.to_string(),
        bytes: zip(&all)?,
    })
}

/// The format a `manifest.json` says it is, read before the rest so an
/// unknown one is reported as such rather than as a missing field.
pub fn declared_format(manifest: &[u8]) -> Option<String> {
    serde_json::from_slice::<Value>(manifest)
        .ok()?
        .get("format")?
        .as_str()
        .map(str::to_string)
}

#[cfg(test)]
#[path = "manifest_tests.rs"]
mod manifest_tests;
