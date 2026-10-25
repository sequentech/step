// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use sequent_core::election_config::package_contract::CensusRequirement;
use serde_json::json;
use std::collections::BTreeSet;
use std::sync::Mutex;

fn zip(members: &[(&str, &[u8])]) -> Vec<u8> {
    use std::io::Write;
    let mut out = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, bytes) in members {
        out.start_file(
            *name,
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored),
        )
        .unwrap();
        out.write_all(bytes).unwrap();
    }
    out.finish().unwrap().into_inner()
}

/// A synthetic delivery with every section.
fn delivery() -> Vec<u8> {
    let event = zip(&[
        ("export_election_event-e1.json", b"{}"),
        ("export_voters-e1.csv", b"username\nv1\n"),
    ]);
    let portal = zip(&[
        ("keycloak_admin-t1.json", br#"{"displayName":"X"}"#),
        ("export_permissions-t1.csv", b"role,permissions\nadmin,a\n"),
        ("admin_users.csv", b"username,group_name\nop,admin\n"),
        ("trustees_public.json", b"[]"),
        ("export_templates-t1.csv", b"alias\n"),
    ]);
    zip(&[
        ("official_election_setup.zip", &event),
        ("admin_portal_settings.zip", &portal),
        ("blueprint.json", b"{}"),
    ])
}

fn all_required() -> Requirements {
    Requirements {
        sections: Section::APPLY_ORDER.to_vec(),
        census: CensusRequirement::Required,
    }
}

/// A target that holds one "applied" flag per section.
#[derive(Default)]
struct Fake {
    applied: Mutex<BTreeSet<Section>>,
    submissions: Mutex<Vec<Section>>,
    fail: Option<Section>,
    /// The submission lands but its reply is lost.
    lost_reply: Option<Section>,
    /// Every preview of this section errors.
    unreachable: Option<Section>,
    blocked: Option<Section>,
    /// The submission "succeeds" without changing the target.
    no_effect: Option<Section>,
}

#[async_trait]
impl PackageTarget for Fake {
    async fn preview(&self, section: Section, files: &[Artifact]) -> Result<SectionPreview> {
        assert!(!files.is_empty(), "a section is previewed with its files");
        if self.unreachable == Some(section) {
            return Err(anyhow!("target unreachable"));
        }
        let applied = self.applied.lock().unwrap().contains(&section);
        Ok(SectionPreview {
            needs_apply: !applied,
            changes: json!({"files": files.iter().map(|f| f.name.clone()).collect::<Vec<_>>()}),
            blockers: if self.blocked == Some(section) {
                vec!["unknown role".into()]
            } else {
                vec![]
            },
        })
    }

    async fn apply(&self, section: Section, _: &[Artifact]) -> Result<SectionSubmission> {
        self.submissions.lock().unwrap().push(section);
        if self.fail == Some(section) {
            return Err(anyhow!("import failed"));
        }
        if self.no_effect != Some(section) {
            self.applied.lock().unwrap().insert(section);
        }
        if self.lost_reply == Some(section) {
            return Err(anyhow!("connection reset"));
        }
        Ok(SectionSubmission {
            task_id: Some(format!("task-{section:?}")),
            detail: json!({}),
        })
    }
}

#[derive(Default)]
struct Saved(Mutex<Vec<ApplicationRecord>>);

#[async_trait]
impl ProgressStore for Saved {
    async fn save(&self, record: &ApplicationRecord) -> Result<()> {
        self.0.lock().unwrap().push(record.clone());
        Ok(())
    }
}

fn status(record: &ApplicationRecord, section: Section) -> &SectionStatus {
    record.status(section).unwrap()
}

fn plan() -> PackagePlan {
    plan_package(&delivery(), &all_required()).unwrap()
}

#[test]
fn a_delivery_is_read_into_each_sections_files() {
    let plan = plan();
    let names = |s: Section| -> Vec<String> {
        plan.payloads[&s].iter().map(|f| f.name.clone()).collect()
    };
    assert_eq!(names(Section::TenantSettings), ["keycloak_admin-t1.json"]);
    assert_eq!(names(Section::RolePermissions), ["export_permissions-t1.csv"]);
    assert_eq!(names(Section::AdminAccounts), ["admin_users.csv"]);
    assert_eq!(names(Section::Trustees), ["trustees_public.json"]);
    assert_eq!(names(Section::ElectionEvent), ["official_election_setup.zip"]);
    // Templates are manual: no section applies them.
    assert!(plan
        .payloads
        .values()
        .flatten()
        .all(|f| f.name != "export_templates-t1.csv"));
    assert!(plan.coverage.is_complete());
}

#[tokio::test]
async fn applying_runs_every_section_in_order_and_confirms_each_from_the_target() {
    let target = Fake::default();
    let store = Saved::default();
    let record = run(&plan(), ApplyMode::Apply, None, &target, &store).await.unwrap();
    assert_eq!(record.outcome, Outcome::Applied);
    assert_eq!(*target.submissions.lock().unwrap(), Section::APPLY_ORDER.to_vec());
    for section in Section::APPLY_ORDER {
        assert_eq!(
            *status(&record, section),
            SectionStatus::Applied {
                submission: Some(SectionSubmission {
                    task_id: Some(format!("task-{section:?}")),
                    detail: json!({}),
                }),
                reconciled: false
            }
        );
    }
    // Progress is saved before each submission, so an interruption leaves a
    // section marked as submitting.
    let saved = store.0.lock().unwrap();
    assert!(saved.iter().any(|r| *status(r, Section::Trustees) == SectionStatus::Submitting));
}

#[tokio::test]
async fn preview_changes_nothing() {
    let target = Fake::default();
    let record = run(&plan(), ApplyMode::Preview, None, &target, &Saved::default())
        .await
        .unwrap();
    assert_eq!(record.outcome, Outcome::Previewed);
    assert!(target.submissions.lock().unwrap().is_empty());
    assert!(matches!(
        status(&record, Section::ElectionEvent),
        SectionStatus::Previewed { .. }
    ));
}

#[tokio::test]
async fn one_blocker_refuses_the_whole_application_before_any_change() {
    // The blocked section is the fourth; nothing before it may be applied.
    let target = Fake {
        blocked: Some(Section::Trustees),
        ..Fake::default()
    };
    let record = run(&plan(), ApplyMode::Apply, None, &target, &Saved::default())
        .await
        .unwrap();
    assert_eq!(record.outcome, Outcome::Refused);
    assert!(target.submissions.lock().unwrap().is_empty());
    assert!(matches!(status(&record, Section::Trustees), SectionStatus::Blocked { .. }));
}

#[tokio::test]
async fn a_missing_required_section_refuses_and_is_named() {
    let without_trustees = {
        let event = zip(&[
            ("export_election_event-e1.json", b"{}"),
            ("export_voters-e1.csv", b"username\nv1\n"),
        ]);
        let portal = zip(&[("keycloak_admin-t1.json", b"{}")]);
        zip(&[
            ("official_election_setup.zip", &event),
            ("admin_portal_settings.zip", &portal),
        ])
    };
    let plan = plan_package(&without_trustees, &all_required()).unwrap();
    let target = Fake::default();
    let record = run(&plan, ApplyMode::Apply, None, &target, &Saved::default())
        .await
        .unwrap();
    assert_eq!(record.outcome, Outcome::Refused);
    assert!(target.submissions.lock().unwrap().is_empty());
    assert_eq!(*status(&record, Section::Trustees), SectionStatus::Missing);
    assert_eq!(*status(&record, Section::AdminAccounts), SectionStatus::Missing);
}

#[tokio::test]
async fn a_failure_stops_later_sections_and_a_rerun_resumes_without_resubmitting() {
    let store = Saved::default();
    let failing = Fake {
        fail: Some(Section::AdminAccounts),
        ..Fake::default()
    };
    let first = run(&plan(), ApplyMode::Apply, None, &failing, &store).await.unwrap();
    assert_eq!(first.outcome, Outcome::Failed);
    assert!(matches!(status(&first, Section::AdminAccounts), SectionStatus::Failed { .. }));
    // Previewed up front, never started.
    assert!(matches!(
        status(&first, Section::Trustees),
        SectionStatus::Previewed { preview } if preview.needs_apply
    ));
    assert_eq!(
        *failing.submissions.lock().unwrap(),
        [Section::TenantSettings, Section::RolePermissions, Section::AdminAccounts]
    );

    // The same target, fixed: the first two sections are already in place
    // and must not be submitted again.
    let fixed = Fake::default();
    *fixed.applied.lock().unwrap() = failing.applied.lock().unwrap().clone();
    let second = run(&plan(), ApplyMode::Apply, Some(&first), &fixed, &store)
        .await
        .unwrap();
    assert_eq!(second.outcome, Outcome::Applied);
    assert_eq!(
        *fixed.submissions.lock().unwrap(),
        [Section::AdminAccounts, Section::Trustees, Section::ElectionEvent]
    );
    assert_eq!(
        *status(&second, Section::TenantSettings),
        SectionStatus::Applied {
            submission: None,
            reconciled: true
        }
    );
}

#[tokio::test]
async fn an_interrupted_submission_is_reconciled_from_the_target_not_resubmitted() {
    // A run stopped after saving "submitting" for the event, whose import had
    // in fact landed. The rerun asks the target and records it applied.
    let target = Fake::default();
    let store = Saved::default();
    let completed = run(&plan(), ApplyMode::Apply, None, &target, &store).await.unwrap();
    let mut interrupted = completed.clone();
    interrupted.set(Section::ElectionEvent, SectionStatus::Submitting);
    interrupted.outcome = Outcome::Running;
    let rerun_target = Fake::default();
    *rerun_target.applied.lock().unwrap() = target.applied.lock().unwrap().clone();
    let rerun = run(&plan(), ApplyMode::Apply, Some(&interrupted), &rerun_target, &store)
        .await
        .unwrap();
    assert_eq!(rerun.outcome, Outcome::Applied);
    assert!(rerun_target.submissions.lock().unwrap().is_empty());
    assert_eq!(
        *status(&rerun, Section::ElectionEvent),
        SectionStatus::Applied {
            submission: None,
            reconciled: true
        }
    );
}

#[tokio::test]
async fn a_lost_reply_whose_effect_landed_is_applied_and_not_retried() {
    let target = Fake {
        lost_reply: Some(Section::RolePermissions),
        ..Fake::default()
    };
    let record = run(&plan(), ApplyMode::Apply, None, &target, &Saved::default())
        .await
        .unwrap();
    assert_eq!(record.outcome, Outcome::Applied);
    assert_eq!(
        target
            .submissions
            .lock()
            .unwrap()
            .iter()
            .filter(|s| **s == Section::RolePermissions)
            .count(),
        1
    );
    assert_eq!(
        *status(&record, Section::RolePermissions),
        SectionStatus::Applied {
            submission: None,
            reconciled: true
        }
    );
}

#[tokio::test]
async fn a_section_the_target_cannot_be_asked_about_stops_as_unknown() {
    let target = Fake {
        unreachable: Some(Section::ElectionEvent),
        ..Fake::default()
    };
    let record = run(&plan(), ApplyMode::Apply, None, &target, &Saved::default())
        .await
        .unwrap();
    assert_eq!(record.outcome, Outcome::Unknown);
    // Found during the up-front preview: nothing at all was submitted.
    assert!(target.submissions.lock().unwrap().is_empty());
}

#[tokio::test]
async fn an_import_that_reports_success_without_effect_is_a_failure() {
    let target = Fake {
        no_effect: Some(Section::Trustees),
        ..Fake::default()
    };
    let record = run(&plan(), ApplyMode::Apply, None, &target, &Saved::default())
        .await
        .unwrap();
    assert_eq!(record.outcome, Outcome::Failed);
    assert!(matches!(status(&record, Section::Trustees), SectionStatus::Failed { .. }));
    assert!(matches!(
        status(&record, Section::ElectionEvent),
        SectionStatus::Previewed { preview } if preview.needs_apply
    ));
}

#[tokio::test]
async fn a_saved_application_of_another_package_is_not_resumed() {
    let target = Fake::default();
    let mut other = run(&plan(), ApplyMode::Preview, None, &target, &Saved::default())
        .await
        .unwrap();
    other.package_sha256 = "different".into();
    assert!(run(&plan(), ApplyMode::Apply, Some(&other), &target, &Saved::default())
        .await
        .is_err());
}
