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
use sequent_core::election_config::archive::{Artifact, ADMIN_PORTAL_MEMBER, IMPORTABLE_MEMBER};
use sequent_core::election_config::manifest::{file_entries, read_zip};
use sequent_core::election_config::package_contract::{
    coverage, nested_disposition, Coverage, Disposition, Requirements, Section,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
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
pub fn plan_package(bytes: &[u8], requirements: &Requirements) -> Result<PackagePlan> {
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
            let inner = read_zip(&member.bytes, ADMIN_PORTAL_MEMBER).map_err(|p| anyhow!("{p}"))?;
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

/// What became of a submission an earlier run began, as its task row says.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskState {
    /// Still running: it may yet change the target.
    Running,
    Succeeded,
    /// Ended without succeeding, or interrupted with the run that held it.
    Failed,
}

/// The platform side: previewing and applying each section through the
/// existing importers and with the requesting administrator's permissions.
#[async_trait]
pub trait PackageTarget: Send + Sync {
    async fn preview(&self, section: Section, files: &[Artifact]) -> Result<SectionPreview>;
    /// Record a submission before it starts (its task row), so an
    /// interruption leaves its identity behind. `None` for a section whose
    /// submission is one transaction with nothing to wait for.
    async fn begin(&self, section: Section) -> Result<Option<String>>;
    async fn apply(
        &self,
        section: Section,
        files: &[Artifact],
        task_id: Option<&str>,
    ) -> Result<SectionSubmission>;
    /// The state of a submission an earlier run began.
    async fn submission_state(&self, task_id: &str) -> Result<TaskState>;
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
    /// Begun and not yet confirmed. Saved before the submission starts, with
    /// its task, so an interrupted run finds it.
    Submitting { task_id: Option<String> },
    /// The target matches the package after a submission. `reconciled` when
    /// that was confirmed from the target rather than by the submission's own
    /// reply (a later run, or a reply that was lost).
    Applied {
        submission: Option<SectionSubmission>,
        reconciled: bool,
    },
    Failed {
        error: String,
        task_id: Option<String>,
    },
    /// The target, or an earlier submission, could not be settled. Nothing
    /// further is submitted for it until a later run settles it.
    Unknown {
        error: String,
        task_id: Option<String>,
    },
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
    pub tenant_id: String,
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
    /// A required section is missing or a preview found a blocker before
    /// anything was submitted; nothing was changed.
    Refused,
    /// The run stopped at a section after earlier sections may have been
    /// applied; each section's status says which were.
    Failed,
    /// A section, or an earlier submission of it, could not be settled;
    /// later sections were not started.
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
        if let Some(record) = self.sections.iter_mut().find(|r| r.section == section) {
            record.status = status;
        }
    }
}

/// The target matches the package: nothing left to apply and nothing that
/// would prevent it.
fn converged(preview: &SectionPreview) -> bool {
    !preview.needs_apply && preview.blockers.is_empty()
}

/// The submission an earlier run made of a section, to keep its identity
/// when the section is confirmed again.
fn earlier_submission(
    prior: Option<&ApplicationRecord>,
    section: Section,
) -> Option<SectionSubmission> {
    match prior?.status(section)? {
        SectionStatus::Applied { submission, .. } => submission.clone(),
        SectionStatus::Submitting { task_id }
        | SectionStatus::Failed { task_id, .. }
        | SectionStatus::Unknown { task_id, .. } => task_id.as_ref().map(|id| SectionSubmission {
            task_id: Some(id.clone()),
            detail: Value::Null,
        }),
        _ => None,
    }
}

/// The earlier run's unsettled submission of a section, if any.
fn in_flight(prior: Option<&ApplicationRecord>, section: Section) -> Option<String> {
    match prior?.status(section)? {
        SectionStatus::Submitting { task_id } | SectionStatus::Unknown { task_id, .. } => {
            task_id.clone()
        }
        _ => None,
    }
}

/// Preview, and in [`ApplyMode::Apply`] apply, every section in order.
///
/// `prior` is the saved record of an earlier run of the same package on the
/// same tenant. A submission it left unsettled is never repeated while its
/// task may still be running: the caller holds the application's claim, so
/// the target can tell a running task from one interrupted with its run.
pub async fn run(
    plan: &PackagePlan,
    tenant_id: &str,
    mode: ApplyMode,
    prior: Option<&ApplicationRecord>,
    target: &dyn PackageTarget,
    store: &dyn ProgressStore,
) -> Result<ApplicationRecord> {
    if let Some(prior) = prior {
        if prior.package_sha256 != plan.package_sha256 || prior.tenant_id != tenant_id {
            return Err(anyhow!(
                "the saved application is of a different package or tenant; start a new one"
            ));
        }
    }
    let mut record = ApplicationRecord {
        format: APPLICATION_FORMAT.to_string(),
        tenant_id: tenant_id.to_string(),
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
    // The target answers each preview knowing what earlier sections of the
    // same package will create (the groups the roles section adds).
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
            Ok(preview) => record.set(section, SectionStatus::Previewed { preview }),
            Err(error) => {
                record.set(
                    section,
                    SectionStatus::Unknown {
                        error: format!("{error:#}"),
                        task_id: None,
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

    let mut changed = false;
    for section in Section::APPLY_ORDER {
        let Some(files) = plan.payloads.get(&section) else {
            continue;
        };
        // An earlier submission left unsettled: settle it before anything
        // else, and never submit again while it may still be running.
        if let Some(task_id) = in_flight(prior, section) {
            match target.submission_state(&task_id).await {
                Ok(TaskState::Running) => {
                    return stop(
                        record,
                        section,
                        Outcome::Unknown,
                        format!("an earlier submission (task {task_id}) is still running"),
                        Some(task_id),
                        store,
                    )
                    .await;
                }
                Ok(TaskState::Succeeded | TaskState::Failed) => {}
                Err(error) => {
                    return stop(
                        record,
                        section,
                        Outcome::Unknown,
                        format!(
                            "the earlier submission (task {task_id}) cannot be checked: {error:#}"
                        ),
                        Some(task_id),
                        store,
                    )
                    .await;
                }
            }
        }
        // Re-preview: an earlier section may have changed what this one
        // needs.
        let preview = match target.preview(section, files).await {
            Ok(preview) => preview,
            Err(error) => {
                let task_id = in_flight(prior, section);
                return stop(
                    record,
                    section,
                    Outcome::Unknown,
                    format!("{error:#}"),
                    task_id,
                    store,
                )
                .await;
            }
        };
        if !preview.blockers.is_empty() {
            record.set(section, SectionStatus::Blocked { preview });
            record.outcome = if changed {
                Outcome::Failed
            } else {
                Outcome::Refused
            };
            store.save(&record).await?;
            return Ok(record);
        }
        if !preview.needs_apply {
            let status = match earlier_submission(prior, section) {
                Some(submission) => SectionStatus::Applied {
                    submission: Some(submission),
                    reconciled: true,
                },
                None => SectionStatus::Unchanged { preview },
            };
            record.set(section, status);
            store.save(&record).await?;
            continue;
        }
        let task_id = match target.begin(section).await {
            Ok(task_id) => task_id,
            Err(error) => {
                return stop(
                    record,
                    section,
                    failed_or_refused(changed),
                    format!("{error:#}"),
                    None,
                    store,
                )
                .await;
            }
        };
        record.set(
            section,
            SectionStatus::Submitting {
                task_id: task_id.clone(),
            },
        );
        store.save(&record).await?;
        changed = true;
        let submitted = target.apply(section, files, task_id.as_deref()).await;
        // Success is what the target now says, not what the submission said;
        // a submission can also fail after its effect landed (a lost reply).
        let after = target.preview(section, files).await;
        let status = match (submitted, after) {
            (Ok(submission), Ok(after)) if converged(&after) => SectionStatus::Applied {
                submission: Some(submission),
                reconciled: false,
            },
            (Err(error), Ok(after)) if converged(&after) => SectionStatus::Applied {
                submission: Some(SectionSubmission {
                    task_id: task_id.clone(),
                    detail: json!({"reply": format!("{error:#}")}),
                }),
                reconciled: true,
            },
            (Ok(_), Ok(_)) => SectionStatus::Failed {
                error: "the import finished but the target still differs from the package".into(),
                task_id: task_id.clone(),
            },
            (Err(error), Ok(_)) => SectionStatus::Failed {
                error: format!("{error:#}"),
                task_id: task_id.clone(),
            },
            (submitted, Err(check)) => SectionStatus::Unknown {
                error: match submitted {
                    Ok(_) => format!("the target could not be checked afterwards: {check:#}"),
                    Err(error) => {
                        format!("{error:#}; the target could not be checked afterwards: {check:#}")
                    }
                },
                task_id: task_id.clone(),
            },
        };
        let stop_with = match status {
            SectionStatus::Applied { .. } => None,
            SectionStatus::Unknown { .. } => Some(Outcome::Unknown),
            _ => Some(Outcome::Failed),
        };
        record.set(section, status);
        store.save(&record).await?;
        if let Some(outcome) = stop_with {
            record.outcome = outcome;
            store.save(&record).await?;
            return Ok(record);
        }
    }
    record.outcome = Outcome::Applied;
    store.save(&record).await?;
    Ok(record)
}

fn failed_or_refused(changed: bool) -> Outcome {
    if changed {
        Outcome::Failed
    } else {
        Outcome::Refused
    }
}

async fn stop(
    mut record: ApplicationRecord,
    section: Section,
    outcome: Outcome,
    error: String,
    task_id: Option<String>,
    store: &dyn ProgressStore,
) -> Result<ApplicationRecord> {
    record.set(
        section,
        match outcome {
            Outcome::Unknown => SectionStatus::Unknown { error, task_id },
            _ => SectionStatus::Failed { error, task_id },
        },
    );
    record.outcome = outcome;
    store.save(&record).await?;
    Ok(record)
}

#[cfg(test)]
#[path = "configuration_package_apply_tests.rs"]
mod tests;
