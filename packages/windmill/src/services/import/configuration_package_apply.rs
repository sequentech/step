// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Applying a whole delivery: tenant settings, roles, administrators,
//! trustees and the election event, in that order.
//!
//! Every section is previewed against the target first, and only what the
//! preview says is missing is applied. After applying, the section is
//! previewed again and must have converged; that second preview, not the
//! absence of an error, is what records it as applied. Progress is saved
//! before and after each submission, so a run interrupted mid-section is
//! resumed by previewing again: a section whose effect already reached the
//! target is recorded as applied without being submitted twice, and one the
//! target cannot be asked about stops the run as unknown rather than being
//! retried blindly.
//!
//! The members each section reads, and which members nothing applies, come
//! from `sequent_core`'s shared [`package_contract`], the same table the
//! wizard's review shows.

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use sequent_core::election_config::archive::{
    Artifact, ADMIN_PORTAL_MEMBER, IMPORTABLE_MEMBER,
};
use sequent_core::election_config::manifest::{file_entries, read_zip};
use sequent_core::election_config::package_contract::{
    coverage, nested_disposition, Coverage, Disposition, Requirements, Section,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub const APPLICATION_FORMAT: &str = "sequent.package-application/1";

/// Preview only, or preview and apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ApplyMode {
    Preview,
    Apply,
}

/// What a delivery holds for each section, read once.
#[derive(Debug, Clone)]
pub struct PackagePlan {
    pub package_sha256: String,
    pub coverage: Coverage,
    pub payloads: BTreeMap<Section, Vec<Artifact>>,
}

/// Read a delivery (the outer zip; a signed package's verified members are
/// passed the same way) into the files each section applies.
pub fn plan_package(
    bytes: &[u8],
    requirements: &Requirements,
) -> Result<PackagePlan> {
    let members = read_zip(bytes, "package").map_err(|p| anyhow!("{p}"))?;
    plan_members(&members, sha256_hex(bytes), requirements)
}

pub fn plan_members(
    members: &[Artifact],
    package_sha256: String,
    requirements: &Requirements,
) -> Result<PackagePlan> {
    let entries = file_entries(members).map_err(|p| anyhow!("{p}"))?;
    let coverage = coverage(&entries, requirements);
    let mut payloads: BTreeMap<Section, Vec<Artifact>> = BTreeMap::new();
    for member in members {
        if member.name == IMPORTABLE_MEMBER {
            // The event importer takes its zip whole.
            payloads
                .entry(Section::ElectionEvent)
                .or_default()
                .push(member.clone());
        } else if member.name == ADMIN_PORTAL_MEMBER {
            let inner = read_zip(&member.bytes, ADMIN_PORTAL_MEMBER)
                .map_err(|p| anyhow!("{p}"))?;
            for file in inner {
                if let Disposition::Applied(section) =
                    nested_disposition(ADMIN_PORTAL_MEMBER, &file.name)
                {
                    payloads.entry(section).or_default().push(file);
                }
            }
        }
    }
    Ok(PackagePlan {
        package_sha256,
        coverage,
        payloads,
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// What the target says about one section.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionPreview {
    /// Whether anything remains to be applied.
    pub needs_apply: bool,
    /// The planned changes, for the operator: created, updated, unchanged.
    pub changes: Value,
    /// Reasons the section cannot be applied as it stands (an unknown role,
    /// a trustee whose registered keys differ). Any blocker stops the run.
    pub blockers: Vec<String>,
}

/// The outcome of submitting one section.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionSubmission {
    /// The task row that records the section's own import, when it has one.
    pub task_id: Option<String>,
    pub detail: Value,
}

/// The platform side: previewing and applying each section through the
/// existing importers and with the requesting administrator's permissions.
#[async_trait]
pub trait PackageTarget: Send + Sync {
    async fn preview(
        &self,
        section: Section,
        files: &[Artifact],
    ) -> Result<SectionPreview>;
    async fn apply(
        &self,
        section: Section,
        files: &[Artifact],
    ) -> Result<SectionSubmission>;
}

/// Where progress is saved: the task row of the application.
#[async_trait]
pub trait ProgressStore: Send + Sync {
    async fn save(&self, record: &ApplicationRecord) -> Result<()>;
}

/// Where one section stands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "state")]
pub enum SectionStatus {
    /// Not previewed yet.
    Pending,
    /// The package does not carry it and the profile does not require it.
    NotIncluded,
    /// Required and absent from the package.
    Missing,
    /// Previewed and, if the run stopped earlier, never started.
    Previewed { preview: SectionPreview },
    /// The preview found something that prevents applying it.
    Blocked { preview: SectionPreview },
    /// The target already matches; nothing was submitted.
    Unchanged { preview: SectionPreview },
    /// Submitted and not yet confirmed. Saved before submitting, so an
    /// interrupted run finds it.
    Submitting,
    /// Submitted, and a preview afterwards found nothing left to apply.
    /// `reconciled` when it was confirmed from the target on a later run
    /// rather than by the submission that applied it.
    Applied {
        submission: Option<SectionSubmission>,
        reconciled: bool,
    },
    Failed { error: String },
    /// The target could not be asked; nothing was submitted.
    Unknown { error: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionRecord {
    pub section: Section,
    pub members: Vec<String>,
    pub status: SectionStatus,
}

/// The saved state of one application of one package to one tenant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationRecord {
    pub format: String,
    pub package_sha256: String,
    pub mode: ApplyMode,
    pub coverage: Coverage,
    pub sections: Vec<SectionRecord>,
    pub outcome: Outcome,
}

/// How the run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Outcome {
    Running,
    /// Preview finished; nothing was changed.
    Previewed,
    /// Every included section matches the package.
    Applied,
    /// A required section is missing or a preview found a blocker; nothing
    /// was changed.
    Refused,
    /// A section failed; later sections were not started.
    Failed,
    /// A section's state could not be determined; later sections were not
    /// started.
    Unknown,
}

impl ApplicationRecord {
    fn status(&self, section: Section) -> Option<&SectionStatus> {
        self.sections
            .iter()
            .find(|r| r.section == section)
            .map(|r| &r.status)
    }

    fn set(&mut self, section: Section, status: SectionStatus) {
        if let Some(record) =
            self.sections.iter_mut().find(|r| r.section == section)
        {
            record.status = status;
        }
    }
}

/// Preview, and in [`ApplyMode::Apply`] apply, every section in order.
///
/// `prior` is the saved record of an earlier run of the same package on the
/// same tenant; it only changes how a confirmed section is labelled, since
/// every section is previewed against the target again.
pub async fn run(
    plan: &PackagePlan,
    mode: ApplyMode,
    prior: Option<&ApplicationRecord>,
    target: &dyn PackageTarget,
    store: &dyn ProgressStore,
) -> Result<ApplicationRecord> {
    if let Some(prior) = prior {
        if prior.package_sha256 != plan.package_sha256 {
            return Err(anyhow!(
                "the saved application is of a different package; start a new one"
            ));
        }
    }
    let mut record = ApplicationRecord {
        format: APPLICATION_FORMAT.to_string(),
        package_sha256: plan.package_sha256.clone(),
        mode,
        coverage: plan.coverage.clone(),
        sections: Section::APPLY_ORDER
            .iter()
            .map(|section| SectionRecord {
                section: *section,
                members: plan
                    .payloads
                    .get(section)
                    .map(|files| files.iter().map(|f| f.name.clone()).collect())
                    .unwrap_or_default(),
                status: SectionStatus::Pending,
            })
            .collect(),
        outcome: Outcome::Running,
    };

    // Every section is previewed before anything is applied, so the operator
    // sees the whole plan and one blocker stops the run before any change.
    let mut refused = !plan.coverage.is_complete();
    for section in Section::APPLY_ORDER {
        let Some(files) = plan.payloads.get(&section) else {
            let missing = plan.coverage.missing().contains(&section);
            record.set(
                section,
                if missing {
                    SectionStatus::Missing
                } else {
                    SectionStatus::NotIncluded
                },
            );
            continue;
        };
        match target.preview(section, files).await {
            Ok(preview) if !preview.blockers.is_empty() => {
                refused = true;
                record.set(section, SectionStatus::Blocked { preview });
            }
            Ok(preview) => {
                record.set(section, SectionStatus::Previewed { preview })
            }
            Err(error) => {
                record.set(
                    section,
                    SectionStatus::Unknown {
                        error: format!("{error:#}"),
                    },
                );
                record.outcome = Outcome::Unknown;
                store.save(&record).await?;
                return Ok(record);
            }
        }
    }
    if refused {
        record.outcome = Outcome::Refused;
        store.save(&record).await?;
        return Ok(record);
    }
    if mode == ApplyMode::Preview {
        record.outcome = Outcome::Previewed;
        store.save(&record).await?;
        return Ok(record);
    }
    store.save(&record).await?;

    for section in Section::APPLY_ORDER {
        let Some(files) = plan.payloads.get(&section) else {
            continue;
        };
        // Re-preview: an earlier section may have changed what this one
        // needs (administrators in a group the roles section created).
        let preview = match target.preview(section, files).await {
            Ok(preview) => preview,
            Err(error) => {
                return stop_unknown(record, section, error, store).await;
            }
        };
        if !preview.blockers.is_empty() {
            record.set(section, SectionStatus::Blocked { preview });
            record.outcome = Outcome::Refused;
            store.save(&record).await?;
            return Ok(record);
        }
        if !preview.needs_apply {
            let was_in_flight = prior.is_some_and(|prior| {
                matches!(
                    prior.status(section),
                    Some(SectionStatus::Submitting)
                        | Some(SectionStatus::Applied { .. })
                )
            });
            record.set(
                section,
                if was_in_flight {
                    SectionStatus::Applied {
                        submission: None,
                        reconciled: true,
                    }
                } else {
                    SectionStatus::Unchanged { preview }
                },
            );
            store.save(&record).await?;
            continue;
        }
        record.set(section, SectionStatus::Submitting);
        store.save(&record).await?;
        let submission = match target.apply(section, files).await {
            Ok(submission) => submission,
            Err(error) => {
                // A submission can fail after its effect landed (a lost
                // reply). Ask the target before calling it a failure.
                let status = match target.preview(section, files).await {
                    Ok(after) if !after.needs_apply => SectionStatus::Applied {
                        submission: None,
                        reconciled: true,
                    },
                    Ok(_) => SectionStatus::Failed {
                        error: format!("{error:#}"),
                    },
                    Err(check) => SectionStatus::Unknown {
                        error: format!(
                            "{error:#}; the target could not be checked afterwards: {check:#}"
                        ),
                    },
                };
                let stop = match status {
                    SectionStatus::Applied { .. } => None,
                    SectionStatus::Unknown { .. } => Some(Outcome::Unknown),
                    _ => Some(Outcome::Failed),
                };
                record.set(section, status);
                if let Some(outcome) = stop {
                    record.outcome = outcome;
                    store.save(&record).await?;
                    return Ok(record);
                }
                store.save(&record).await?;
                continue;
            }
        };
        // Success is what the target now says, not what the submission said.
        match target.preview(section, files).await {
            Ok(after) if !after.needs_apply => {
                record.set(
                    section,
                    SectionStatus::Applied {
                        submission: Some(submission),
                        reconciled: false,
                    },
                );
                store.save(&record).await?;
            }
            Ok(_) => {
                record.set(
                    section,
                    SectionStatus::Failed {
                        error: "the import finished but the target still differs from the package".to_string(),
                    },
                );
                record.outcome = Outcome::Failed;
                store.save(&record).await?;
                return Ok(record);
            }
            Err(error) => {
                return stop_unknown(record, section, error, store).await;
            }
        }
    }
    record.outcome = Outcome::Applied;
    store.save(&record).await?;
    Ok(record)
}

async fn stop_unknown(
    mut record: ApplicationRecord,
    section: Section,
    error: anyhow::Error,
    store: &dyn ProgressStore,
) -> Result<ApplicationRecord> {
    record.set(
        section,
        SectionStatus::Unknown {
            error: format!("{error:#}"),
        },
    );
    record.outcome = Outcome::Unknown;
    store.save(&record).await?;
    Ok(record)
}

#[cfg(test)]
#[path = "configuration_package_apply_tests.rs"]
mod tests;
