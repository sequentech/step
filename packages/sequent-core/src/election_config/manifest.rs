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

/// What `report-manifest.json` is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReportManifestFormat {
    #[serde(rename = "sequent.report-manifest/1")]
    V1,
}

/// The configuration a report was generated from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigurationStamp {
    pub external_id: String,
    pub revision: u64,
    pub manifest_sha256: String,
    /// The digest of the template the report was drawn with.
    pub template_sha256: String,
}

/// A generated report's hash manifest: the configuration it came from and
/// every file it produced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportManifest {
    pub format: ReportManifestFormat,
    pub report_type: String,
    pub configuration: ConfigurationStamp,
    pub files: Vec<FileEntry>,
}

/// The file a generation writes its hash manifest to, beside the report's
/// files.
pub const REPORT_MANIFEST_NAME: &str = "report-manifest.json";

impl ReportManifest {
    /// `report-manifest.json` as written: pretty, with a trailing newline.
    pub fn to_bytes(&self) -> serde_json::Result<Vec<u8>> {
        serde_json::to_vec_pretty(self).map(|mut bytes| {
            bytes.push(b'\n');
            bytes
        })
    }
}

impl ConfigurationStamp {
    /// The line a report names its configuration with, where it has no
    /// template to print it: a comment in an SQL or XML file.
    pub fn line(&self) -> String {
        format!(
            "Configuration revision {}, manifest SHA-256 {}",
            self.revision, self.manifest_sha256
        )
    }
}

/// The stamp of a report of `report_type` drawn with `template`, or a
/// refusal when the signed configuration sets a design for that report and
/// `template` is not it. A type may be set more than once, for the event and
/// for an election: the template must then be one of the designs set, unless
/// one of the settings leaves the design to the platform.
pub fn report_stamp(
    manifest: &Manifest,
    manifest_sha256: &str,
    report_type: &str,
    template: &str,
) -> Result<ConfigurationStamp, Problem> {
    let template_sha256 = sha256_hex(template.as_bytes());
    let designs: Vec<Option<&String>> = manifest
        .content
        .reports
        .iter()
        .filter(|setting| setting.report_type == report_type)
        .map(|setting| setting.template_sha256.as_ref())
        .collect();
    let approved: Vec<&str> = designs
        .iter()
        .flatten()
        .map(|digest| digest.as_str())
        .collect();
    if !approved.is_empty()
        && approved.len() == designs.len()
        && !approved.contains(&template_sha256.as_str())
    {
        let approved = approved.join(", ");
        return Err(Problem::error(
            Code::IntegrityMismatch,
            "reports",
            format!(
                "the {report_type} report's template is not the one the \
                 signed configuration approved: its digest is \
                 {template_sha256}, and the configuration says {approved}"
            ),
        )
        .id("package.report-template-changed")
        .detail("report", report_type)
        .detail("expected", approved)
        .detail("actual", &template_sha256));
    }
    Ok(ConfigurationStamp {
        external_id: manifest.configuration.external_id.clone(),
        revision: manifest.configuration.revision,
        manifest_sha256: manifest_sha256.to_string(),
        template_sha256,
    })
}

/// The hash manifest of a generated report's files.
pub fn report_manifest(
    report_type: &str,
    stamp: &ConfigurationStamp,
    files: &[Artifact],
) -> ReportManifest {
    let mut entries: Vec<FileEntry> = files
        .iter()
        .map(|file| FileEntry {
            path: file.name.clone(),
            size: file.bytes.len() as u64,
            sha256: sha256_hex(&file.bytes),
            members: Vec::new(),
        })
        .collect();
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    ReportManifest {
        format: ReportManifestFormat::V1,
        report_type: report_type.to_string(),
        configuration: stamp.clone(),
        files: entries,
    }
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
    compiled: &super::architect::Compiled,
    previous_designs: &[BallotDesign],
) -> Result<RevisionContent, super::problem::Report> {
    use super::problem::Report;

    let delivery = super::archive::delivery(&compiled.layout)
        .map_err(Report::from_problem)?;
    let Payload { members, files } =
        expand(&delivery.bytes, "delivery", ArchiveLimits::PACKAGE)
            .map_err(Report::from_problem)?;
    let digests = super::preview::ballot_design_digests(&compiled.bundle)?;
    let ballot_designs = super::design::versioned(previous_designs, digests);
    let reports = report_settings(&compiled.bundle)?;

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

const REPORT_TYPE_COLUMN: &str = "report_type";
const REPORT_TEMPLATE_COLUMN: &str = "template_alias";
const REPORT_COPIES_COLUMN: &str = "copies";
const REPORT_FORMATS_COLUMN: &str = "output_formats";

/// How many copies a report prints when its row doesn't say.
const DEFAULT_REPORT_COPIES: u32 = 1;

/// The setting of every report the bundle's reports file creates: the plan's
/// own and the rows of a Reports sheet the plan carries. Read from the file
/// the importer reads, so no report is created that the approvers didn't see.
fn report_settings(
    bundle: &super::build::Bundle,
) -> Result<Vec<ReportSetting>, super::problem::Report> {
    use std::str::FromStr;

    use super::report::{parse_copies, parse_output_formats, ReportType};

    let mut report = super::problem::Report::default();
    let mut settings = Vec::new();
    let Some(table) = &bundle.reports else {
        return Ok(settings);
    };
    let cell = |row: &[String], column: &str| -> Option<String> {
        let at = table.columns.iter().position(|name| name == column)?;
        let text = row.get(at)?.trim();
        (!text.is_empty()).then(|| text.to_string())
    };

    for (index, row) in table.rows.iter().enumerate() {
        let path = format!("reports[{index}]");
        let Some(report_type) = cell(row, REPORT_TYPE_COLUMN) else {
            report.push(
                Problem::error(
                    Code::MissingField,
                    path,
                    "a report in the configuration has no type, so it can't \
                     be signed",
                )
                .id("package.report-unreadable"),
            );
            continue;
        };
        let unreadable = |reason: String| {
            Problem::error(
                Code::InvalidValue,
                path.clone(),
                format!("the {report_type} report can't be signed: {reason}"),
            )
            .id("package.report-unreadable")
            .detail("report", &report_type)
            .detail("reason", reason)
        };

        let copies =
            match parse_copies(cell(row, REPORT_COPIES_COLUMN).as_deref()) {
                Ok(copies) => copies.unwrap_or(DEFAULT_REPORT_COPIES),
                Err(reason) => {
                    report.push(unreadable(reason));
                    continue;
                }
            };
        let formats = match parse_output_formats(
            cell(row, REPORT_FORMATS_COLUMN).as_deref(),
        ) {
            Ok(Some(formats)) => formats,
            Ok(None) => ReportType::from_str(&report_type)
                .ok()
                .and_then(|known| known.formats().first().copied())
                .into_iter()
                .collect(),
            Err(reason) => {
                report.push(unreadable(reason));
                continue;
            }
        };

        let template = cell(row, REPORT_TEMPLATE_COLUMN);
        let template_sha256 = match &template {
            None => None,
            Some(alias) => {
                let found = bundle
                    .templates
                    .iter()
                    .find(|template| template.alias.trim() == alias);
                if found.is_none() {
                    report.push(
                        Problem::error(
                            Code::DanglingReference,
                            path.clone(),
                            format!(
                                "the {report_type} report is drawn with \
                                 template '{alias}', which isn't in the \
                                 configuration, so its design can't be signed"
                            ),
                        )
                        .id("package.report-template-missing")
                        .detail("report", &report_type)
                        .detail("template", alias),
                    );
                    continue;
                }
                found.map(|template| sha256_hex(template.document.as_bytes()))
            }
        };

        settings.push(ReportSetting {
            report_type,
            formats,
            copies,
            template,
            template_sha256,
        });
    }

    if report.has_errors() {
        return Err(report);
    }
    Ok(settings)
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

const MEBIBYTE: u64 = 1024 * 1024;

/// The most members a package may hold, its nested zips' members included.
pub const MAX_PACKAGE_MEMBERS: usize = 10_000;

/// The most one member may expand to. The largest member of a delivery is
/// its voters file.
pub const MAX_MEMBER_BYTES: u64 = 1024 * MEBIBYTE;

/// The most a package's members may expand to together, its nested zips'
/// members included.
pub const MAX_PACKAGE_BYTES: u64 = 2048 * MEBIBYTE;

/// How deep a zip may sit inside other zips. A delivery's sit one deep.
pub const MAX_NESTING_DEPTH: usize = 3;

/// The most `manifest.json`, `manifest.sig` or `manifest.chain.pem` may
/// expand to. They are read before anything vouches for the package.
pub const MAX_SIGNATURE_MEMBER_BYTES: u64 = 16 * MEBIBYTE;

/// How much of an archive a reader expands. A zip says how large its members
/// are and can lie, so the limits are kept while reading, not taken from its
/// directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArchiveLimits {
    pub members: usize,
    pub member_bytes: u64,
    pub total_bytes: u64,
    pub nesting_depth: usize,
}

impl ArchiveLimits {
    /// What a package and the delivery inside it are held to.
    pub const PACKAGE: ArchiveLimits = ArchiveLimits {
        members: MAX_PACKAGE_MEMBERS,
        member_bytes: MAX_MEMBER_BYTES,
        total_bytes: MAX_PACKAGE_BYTES,
        nesting_depth: MAX_NESTING_DEPTH,
    };

    /// What the three signature members are held to.
    pub const SIGNATURE: ArchiveLimits = ArchiveLimits {
        members: MAX_PACKAGE_MEMBERS,
        member_bytes: MAX_SIGNATURE_MEMBER_BYTES,
        total_bytes: 3 * MAX_SIGNATURE_MEMBER_BYTES,
        nesting_depth: 0,
    };
}

/// What is left of an [`ArchiveLimits`] while a package is read: one budget
/// for the package and every zip nested in it.
#[derive(Debug, Clone)]
struct Budget {
    limits: ArchiveLimits,
    members_left: usize,
    bytes_left: u64,
}

impl Budget {
    fn new(limits: ArchiveLimits) -> Self {
        Budget {
            limits,
            members_left: limits.members,
            bytes_left: limits.total_bytes,
        }
    }

    fn take_members(
        &mut self,
        count: usize,
        what: &str,
    ) -> Result<(), Problem> {
        match self.members_left.checked_sub(count) {
            Some(left) => {
                self.members_left = left;
                Ok(())
            }
            None => Err(self.too_many_members(what)),
        }
    }

    fn too_many_members(&self, what: &str) -> Problem {
        Problem::error(
            Code::Unreadable,
            what,
            format!(
                "{what} holds more files than the {} a package may hold",
                self.limits.members
            ),
        )
        .id("package.too-many-members")
        .detail("archive", what)
        .detail("limit", self.limits.members)
    }

    /// Reads one member, stopping as soon as it passes what is allowed.
    fn read(
        &mut self,
        entry: &mut impl Read,
        declared: u64,
        name: &str,
        what: &str,
    ) -> Result<Vec<u8>, Problem> {
        let allowed = self.limits.member_bytes.min(self.bytes_left);
        if declared > allowed {
            return Err(self.too_large(declared, name, what));
        }
        let mut contents = Vec::new();
        entry
            .take(allowed.saturating_add(1))
            .read_to_end(&mut contents)
            .map_err(|error| unreadable_zip(what, error.to_string()))?;
        let size = contents.len() as u64;
        if size > allowed {
            return Err(self.too_large(size, name, what));
        }
        self.bytes_left -= size;
        Ok(contents)
    }

    fn too_large(&self, size: u64, name: &str, what: &str) -> Problem {
        if size > self.limits.member_bytes {
            Problem::error(
                Code::Unreadable,
                what,
                format!(
                    "'{name}' in {what} expands to more than the {} bytes a \
                     file may",
                    self.limits.member_bytes
                ),
            )
            .id("package.member-too-large")
            .detail("file", name)
            .detail("archive", what)
            .detail("limit", self.limits.member_bytes)
        } else {
            Problem::error(
                Code::Unreadable,
                what,
                format!(
                    "the package expands to more than the {} bytes it may: \
                     '{name}' in {what} is past them",
                    self.limits.total_bytes
                ),
            )
            .id("package.too-large")
            .detail("file", name)
            .detail("archive", what)
            .detail("limit", self.limits.total_bytes)
        }
    }
}

/// Every member's entry, with a nested zip's members listed under it, in
/// path order. Nested zips are expanded within [`ArchiveLimits::PACKAGE`].
pub fn file_entries(members: &[Artifact]) -> Result<Vec<FileEntry>, Problem> {
    entries_at(members, 1, &mut Budget::new(ArchiveLimits::PACKAGE))
}

/// The entries of members that sit `depth` zips deep, the package being the
/// first.
fn entries_at(
    members: &[Artifact],
    depth: usize,
    budget: &mut Budget,
) -> Result<Vec<FileEntry>, Problem> {
    let mut entries = Vec::with_capacity(members.len());
    for member in members {
        let nested = if member.name.ends_with(NESTED_ZIP_SUFFIX) {
            if depth > budget.limits.nesting_depth {
                return Err(Problem::error(
                    Code::Unreadable,
                    member.name.as_str(),
                    format!(
                        "'{}' is nested in more than the {} zips a file may be",
                        member.name, budget.limits.nesting_depth
                    ),
                )
                .id("package.nested-too-deep")
                .detail("file", &member.name)
                .detail("limit", budget.limits.nesting_depth));
            }
            let inner =
                read_members(&member.bytes, &member.name, budget, |_| true)?;
            entries_at(&inner, depth + 1, budget)?
        } else {
            Vec::new()
        };
        entries.push(FileEntry {
            path: member.name.clone(),
            size: member.bytes.len() as u64,
            sha256: sha256_hex(&member.bytes),
            members: nested,
        });
    }
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(entries)
}

/// The members of a zip, in the order they are stored, within
/// [`ArchiveLimits::PACKAGE`]. A name that appears twice is refused: which
/// of the two a reader takes is the reader's choice, and the `zip` crate
/// silently keeps only the last.
pub fn read_zip(bytes: &[u8], what: &str) -> Result<Vec<Artifact>, Problem> {
    read_members(
        bytes,
        what,
        &mut Budget::new(ArchiveLimits::PACKAGE),
        |_| true,
    )
}

/// The members of a zip that `wanted` names, read out of `budget`. The
/// others are counted and not expanded.
fn read_members(
    bytes: &[u8],
    what: &str,
    budget: &mut Budget,
    wanted: impl Fn(&str) -> bool,
) -> Result<Vec<Artifact>, Problem> {
    let names = central_directory_names(bytes, budget.members_left)
        .map_err(|reason| unreadable_zip(what, reason))?;
    if names.len() > budget.members_left {
        return Err(budget.too_many_members(what));
    }
    let mut seen = std::collections::BTreeSet::new();
    for name in &names {
        if !seen.insert(name.as_str()) {
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
    budget.take_members(archive.len().max(names.len()), what)?;
    let mut members: Vec<Artifact> = Vec::new();
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| unreadable_zip(what, error.to_string()))?;
        if entry.is_dir() || !wanted(entry.name()) {
            continue;
        }
        let name = entry.name().to_string();
        let declared = entry.size();
        let contents = budget.read(&mut entry, declared, &name, what)?;
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

/// The names in a zip's central directory, duplicates included. It stops
/// one past `most`, which is enough to tell that there are too many.
fn central_directory_names(
    bytes: &[u8],
    most: usize,
) -> Result<Vec<String>, String> {
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
        if names.len() > most {
            break;
        }
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

/// Opens a package, or a plain delivery, without checking anything. A
/// verifier reads [`signature_members`] first and [`payload`] only once the
/// signature holds.
pub fn open_package(bytes: &[u8]) -> Result<OpenedPackage, Problem> {
    let mut opened = OpenedPackage {
        members: Vec::new(),
        manifest: None,
        signature: None,
        chain: None,
    };
    for member in read_zip(bytes, PACKAGE)? {
        match member.name.as_str() {
            MANIFEST_MEMBER => opened.manifest = Some(member.bytes),
            SIGNATURE_MEMBER => opened.signature = Some(member.bytes),
            CHAIN_MEMBER => opened.chain = Some(member.bytes),
            _ => opened.members.push(member),
        }
    }
    Ok(opened)
}

/// What a package is called in a problem.
const PACKAGE: &str = "package";

fn is_signature_member(name: &str) -> bool {
    [MANIFEST_MEMBER, SIGNATURE_MEMBER, CHAIN_MEMBER].contains(&name)
}

/// Whether a file carries any of a package's signature members, going by
/// its directory alone: nothing is expanded.
pub fn has_signature_members(bytes: &[u8]) -> bool {
    central_directory_names(bytes, MAX_PACKAGE_MEMBERS)
        .is_ok_and(|names| names.iter().any(|name| is_signature_member(name)))
}

/// A package's manifest, signature and chain, without the delivery.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SignatureMembers {
    pub manifest: Option<Vec<u8>>,
    pub signature: Option<Vec<u8>>,
    pub chain: Option<Vec<u8>>,
}

impl SignatureMembers {
    /// The signature members the package lacks.
    pub fn missing(&self) -> Vec<&'static str> {
        [
            (self.manifest.is_none(), MANIFEST_MEMBER),
            (self.signature.is_none(), SIGNATURE_MEMBER),
            (self.chain.is_none(), CHAIN_MEMBER),
        ]
        .into_iter()
        .filter_map(|(absent, name)| absent.then_some(name))
        .collect()
    }
}

/// Reads a package's three signature members and nothing else, within
/// [`ArchiveLimits::SIGNATURE`], so the signature can be checked before the
/// delivery is expanded.
pub fn signature_members(bytes: &[u8]) -> Result<SignatureMembers, Problem> {
    let mut found = SignatureMembers::default();
    let mut budget = Budget::new(ArchiveLimits::SIGNATURE);
    for member in
        read_members(bytes, PACKAGE, &mut budget, is_signature_member)?
    {
        match member.name.as_str() {
            MANIFEST_MEMBER => found.manifest = Some(member.bytes),
            SIGNATURE_MEMBER => found.signature = Some(member.bytes),
            CHAIN_MEMBER => found.chain = Some(member.bytes),
            _ => {}
        }
    }
    Ok(found)
}

/// A package's delivery: its members, in the order they are stored, and
/// their entries as a manifest lists them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Payload {
    pub members: Vec<Artifact>,
    pub files: Vec<FileEntry>,
}

/// Expands a package's delivery, nested zips included, within
/// [`ArchiveLimits::PACKAGE`] for all of it together.
pub fn payload(bytes: &[u8]) -> Result<Payload, Problem> {
    expand(bytes, PACKAGE, ArchiveLimits::PACKAGE)
}

fn expand(
    bytes: &[u8],
    what: &str,
    limits: ArchiveLimits,
) -> Result<Payload, Problem> {
    let mut budget = Budget::new(limits);
    let members = read_members(bytes, what, &mut budget, |name| {
        !is_signature_member(name)
    })?;
    let files = entries_at(&members, 1, &mut budget)?;
    Ok(Payload { members, files })
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
