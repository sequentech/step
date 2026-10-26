// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The configuration package's platform target, through the `PackageTarget`
//! methods the application engine calls: the sections that live in the
//! platform's own database (the tenant configuration and the trustees), the
//! task rows a section begins, and their settlement. The target reaches the
//! database through the process-wide pools, so this binary points them at the
//! fixture database before their first use, as
//! postgres_task_adapters_global_pool.rs does. Writes commit: every test uses
//! a tenant of its own.

#[path = "support/schema.rs"]
mod schema;

use deadpool_postgres::Pool;
use sequent_core::election_config::archive::Artifact;
use sequent_core::election_config::package_contract::Section;
use serde_json::{json, Value};
use std::ffi::OsString;
use std::future::Future;
use std::sync::LazyLock;
use tokio::runtime::Runtime;
use tokio::sync::OnceCell;
use windmill::services::database::{get_hasura_pool, get_keycloak_pool};
use windmill::services::import::configuration_package_apply::{PackageTarget, TaskState};
use windmill::services::import::configuration_package_target::{ApplyContext, PlatformTarget};

static FIXTURE: OnceCell<Pool> = OnceCell::const_new();

/// One runtime for every test: the process-wide pools outlive any one test,
/// and a pooled connection is driven by the runtime that opened it.
static RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
});

fn run<F: Future>(test: F) -> F::Output {
    RUNTIME.block_on(test)
}

async fn current_database(pool: &Pool) -> String {
    pool.get()
        .await
        .unwrap()
        .query_one("SELECT current_database()", &[])
        .await
        .unwrap()
        .get(0)
}

fn pool_settings() -> Vec<(String, OsString)> {
    std::env::vars_os()
        .filter_map(|(key, value)| Some((key.into_string().ok()?, value)))
        .filter(|(key, _)| key.starts_with("HASURA_DB__") || key.starts_with("KEYCLOAK_DB__"))
        .collect()
}

/// The fixture pool, once both global pools connect to the fixture database.
async fn fixture() -> &'static Pool {
    FIXTURE
        .get_or_init(|| async {
            let pool = schema::pool().await;
            let database = current_database(&pool).await;
            let saved = pool_settings();
            for (key, _) in &saved {
                std::env::remove_var(key);
            }
            for (key, value) in &saved {
                if let Some(setting) = key.strip_prefix("HASURA_DB__") {
                    std::env::set_var(key, value);
                    std::env::set_var(format!("KEYCLOAK_DB__{setting}"), value);
                }
            }
            std::env::set_var("HASURA_DB__DBNAME", &database);
            std::env::set_var("KEYCLOAK_DB__DBNAME", &database);
            let (hasura, keycloak) = (get_hasura_pool().await, get_keycloak_pool().await);
            for (key, _) in pool_settings() {
                std::env::remove_var(key);
            }
            for (key, value) in &saved {
                std::env::set_var(key, value);
            }
            assert_eq!(current_database(&hasura).await, database);
            assert_eq!(current_database(&keycloak).await, database);
            pool
        })
        .await
}

/// A committed tenant of the test's own (unique id), with the target that
/// applies to it.
async fn tenant() -> (String, PlatformTarget) {
    let id = uuid::Uuid::new_v4().to_string();
    fixture()
        .await
        .get()
        .await
        .unwrap()
        .execute(
            "INSERT INTO sequent_backend.tenant (id, slug, is_active) VALUES ($1::text::uuid, $1, true)",
            &[&id],
        )
        .await
        .unwrap();
    // The event import's body as the server sends it: only the tenant set,
    // every right at its default.
    let event_import = serde_json::from_value(json!({
        "tenant_id": id,
        "document_id": "",
        "password": null,
        "check_only": null,
        "sha256": null,
    }))
    .unwrap();
    let target = PlatformTarget {
        context: ApplyContext {
            tenant_id: id.clone(),
            executed_by: "synthetic-operator".to_owned(),
            audit_actor: None,
            event_import,
            package_sha256: "ab".repeat(32),
            signed_package: None,
            package_groups: Default::default(),
            package_trustees: Default::default(),
            verified: None,
        },
    };
    (id, target)
}

fn file(name: &str, value: Value) -> Vec<Artifact> {
    vec![Artifact {
        name: name.to_owned(),
        bytes: serde_json::to_vec(&value).unwrap(),
    }]
}

async fn tenant_settings(id: &str) -> Value {
    fixture()
        .await
        .get()
        .await
        .unwrap()
        .query_one(
            "SELECT coalesce(settings, '{}'::jsonb) FROM sequent_backend.tenant WHERE id = $1::text::uuid",
            &[&id],
        )
        .await
        .unwrap()
        .get(0)
}

async fn task_status(id: &str) -> String {
    fixture()
        .await
        .get()
        .await
        .unwrap()
        .query_one(
            "SELECT execution_status FROM sequent_backend.tasks_execution WHERE id = $1::text::uuid",
            &[&id],
        )
        .await
        .unwrap()
        .get(0)
}

/// Two synthetic trustees' public keys (election-architect's
/// historical-fixture.mjs; the private halves were discarded).
fn trustees() -> Value {
    json!([
        {"name": "trustee-a", "signingPublicKey": "J5CjY7Q9EgLPaK17Wu5ANhHwSWOX4d+wJLUS+Teevyc=",
         "shareEncryptionPublicKey": "ys1VtTvfwbBpHhjMHTNCUUnNr4RsBuYzQvMd0w83Gjg="},
        {"name": "trustee-b", "signingPublicKey": "093kVmcZJpRke1Ry/vYxm6cdsSNP0YA2K32WX88d2xo=",
         "shareEncryptionPublicKey": "XB5WQGFMeuKE44aGNR9x+bgrRxgUYMUAj+WfEt/mUys="},
    ])
}

#[test]
fn the_tenant_configuration_previews_applies_and_then_needs_nothing() {
    run(async {
        let (id, target) = tenant().await;
        let wanted = file(
            "tenant_platform-t.json",
            json!({"settings": {"display_name": "Synthetic"}}),
        );
        let preview = target
            .preview(Section::TenantConfiguration, &wanted)
            .await
            .unwrap();
        assert!(preview.needs_apply, "{preview:?}");
        assert!(preview.blockers.is_empty(), "{preview:?}");

        let task = target
            .begin(Section::TenantConfiguration)
            .await
            .unwrap()
            .expect("the tenant configuration runs as a task");
        assert_eq!(
            target.submission_state(&task).await.unwrap(),
            TaskState::Interrupted,
            "an open row under the claim belongs to a run that stopped"
        );
        let submitted = target
            .apply(Section::TenantConfiguration, &wanted, Some(&task))
            .await
            .unwrap();
        assert_eq!(submitted.task_id.as_deref(), Some(task.as_str()));
        assert_eq!(tenant_settings(&id).await["display_name"], "Synthetic");
        assert_eq!(task_status(&task).await, "SUCCESS");
        assert_eq!(
            target.submission_state(&task).await.unwrap(),
            TaskState::Succeeded
        );

        let again = target
            .preview(Section::TenantConfiguration, &wanted)
            .await
            .unwrap();
        assert!(!again.needs_apply, "{again:?}");
    })
}

#[test]
fn a_tenant_configuration_carrying_a_credential_is_blocked() {
    run(async {
        let (_, target) = tenant().await;
        let wanted = file(
            "tenant_platform-t.json",
            json!({"settings": {"smtp": {"password": "synthetic"}}}),
        );
        let preview = target
            .preview(Section::TenantConfiguration, &wanted)
            .await
            .unwrap();
        assert!(
            preview
                .blockers
                .iter()
                .any(|b| b.contains("password") && b.contains("credentials")),
            "{preview:?}"
        );
    })
}

#[test]
fn a_section_that_was_not_begun_is_not_applied() {
    run(async {
        let (id, target) = tenant().await;
        let wanted = file(
            "tenant_platform-t.json",
            json!({"settings": {"display_name": "Never"}}),
        );
        let refused = target
            .apply(Section::TenantConfiguration, &wanted, None)
            .await
            .unwrap_err();
        assert!(refused.to_string().contains("not begun"), "{refused:#}");
        assert_ne!(tenant_settings(&id).await["display_name"], "Never");
    })
}

#[test]
fn trustees_register_once_and_conflicting_keys_are_blocked() {
    run(async {
        let (_, target) = tenant().await;
        let wanted = file("trustees_public.json", trustees());
        let preview = target.preview(Section::Trustees, &wanted).await.unwrap();
        assert!(preview.needs_apply, "{preview:?}");
        assert!(preview.blockers.is_empty(), "{preview:?}");
        // One transaction in the run's own process: no task row.
        assert_eq!(target.begin(Section::Trustees).await.unwrap(), None);
        let submitted = target
            .apply(Section::Trustees, &wanted, None)
            .await
            .unwrap();
        assert_eq!(submitted.detail["registered"].as_array().unwrap().len(), 2);

        let again = target.preview(Section::Trustees, &wanted).await.unwrap();
        assert!(!again.needs_apply, "{again:?}");
        let repeated = target
            .apply(Section::Trustees, &wanted, None)
            .await
            .unwrap();
        assert!(
            repeated.detail["registered"].as_array().unwrap().is_empty(),
            "a registered name is never registered twice"
        );

        let mut changed = trustees();
        changed[0]["signingPublicKey"] = json!("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=");
        let conflicting = target
            .preview(Section::Trustees, &file("trustees_public.json", changed))
            .await
            .unwrap();
        assert!(!conflicting.blockers.is_empty(), "{conflicting:?}");
    })
}

#[test]
fn a_begun_section_settles_as_its_run_decides() {
    run(async {
        let (_, target) = tenant().await;
        let succeeded = target
            .begin(Section::RolePermissions)
            .await
            .unwrap()
            .unwrap();
        target.settle(&succeeded, true).await.unwrap();
        assert_eq!(task_status(&succeeded).await, "SUCCESS");
        assert_eq!(
            target.submission_state(&succeeded).await.unwrap(),
            TaskState::Succeeded
        );
        // A settled row is left as it is.
        target.settle(&succeeded, false).await.unwrap();
        assert_eq!(task_status(&succeeded).await, "SUCCESS");

        let failed = target.begin(Section::AdminAccounts).await.unwrap().unwrap();
        target.settle(&failed, false).await.unwrap();
        assert_eq!(task_status(&failed).await, "FAILED");
        assert_eq!(
            target.submission_state(&failed).await.unwrap(),
            TaskState::Failed
        );
    })
}

#[test]
fn another_tenants_task_is_never_read_as_this_ones() {
    run(async {
        let (_, target) = tenant().await;
        let (_, other) = tenant().await;
        let theirs = other.begin(Section::TenantSettings).await.unwrap().unwrap();
        let refused = target.submission_state(&theirs).await.unwrap_err();
        assert!(
            refused.to_string().contains("another tenant"),
            "{refused:#}"
        );
    })
}

#[test]
fn imports_are_held_for_the_application_and_released_with_it() {
    run(async {
        let (_, target) = tenant().await;
        let held = target.hold_imports().await.unwrap();
        drop(held);
        // Released: holding again succeeds at once.
        let again = tokio::time::timeout(std::time::Duration::from_secs(5), target.hold_imports())
            .await
            .expect("the hold was not released")
            .unwrap();
        drop(again);
    })
}

#[test]
fn a_section_takes_exactly_one_file_and_refuses_what_it_cannot_read() {
    run(async {
        let (_, target) = tenant().await;
        let none = target
            .preview(Section::TenantConfiguration, &[])
            .await
            .unwrap_err();
        assert!(
            none.to_string().contains("0 tenant configuration files"),
            "{none:#}"
        );
        let mut two = file("a.json", json!({}));
        two.extend(file("b.json", json!({})));
        let both = target
            .preview(Section::TenantConfiguration, &two)
            .await
            .unwrap_err();
        assert!(
            both.to_string().contains("exactly one is applied"),
            "{both:#}"
        );
        let unknown = target
            .preview(
                Section::TenantConfiguration,
                &file("t.json", json!({"slug": "not-a-package-field"})),
            )
            .await
            .unwrap_err();
        assert!(
            format!("{unknown:#}").contains("not the package's tenant record"),
            "{unknown:#}"
        );
        let trustees = target
            .preview(
                Section::Trustees,
                &file("trustees_public.json", json!({"not": "a list"})),
            )
            .await
            .unwrap_err();
        assert!(
            format!("{trustees:#}").contains("not the public trustee list"),
            "{trustees:#}"
        );
        let event = target
            .preview(
                Section::ElectionEvent,
                &[Artifact {
                    name: "event.zip".to_owned(),
                    bytes: b"not a zip".to_vec(),
                }],
            )
            .await;
        assert!(
            event.is_err(),
            "an unreadable event zip is refused: {event:?}"
        );
    })
}

#[test]
fn a_tenant_that_does_not_exist_is_not_previewed() {
    run(async {
        let (_, mut target) = tenant().await;
        target.context.tenant_id = uuid::Uuid::new_v4().to_string();
        let missing = target
            .preview(
                Section::TenantConfiguration,
                &file("t.json", json!({"settings": {"display_name": "Nobody"}})),
            )
            .await;
        assert!(missing.is_err(), "{missing:?}");
    })
}

#[test]
fn one_tenants_trustees_are_not_anothers() {
    run(async {
        let (_, first) = tenant().await;
        let (_, second) = tenant().await;
        let wanted = file("trustees_public.json", trustees());
        first.apply(Section::Trustees, &wanted, None).await.unwrap();
        let theirs = second.preview(Section::Trustees, &wanted).await.unwrap();
        assert!(
            theirs.needs_apply,
            "another tenant's registration is not this one's: {theirs:?}"
        );
    })
}

#[test]
fn applying_keeps_unnamed_settings_and_never_the_packages_signing_trust() {
    run(async {
        let (id, target) = tenant().await;
        let signing = sequent_core::election_config::package_verify::CONFIGURATION_SIGNING_SETTING;
        fixture()
            .await
            .get()
            .await
            .unwrap()
            .execute(
                "UPDATE sequent_backend.tenant SET settings = $2 WHERE id = $1::text::uuid",
                &[
                    &id,
                    &json!({"kept": "installed", signing: {"installed": true}}),
                ],
            )
            .await
            .unwrap();
        let wanted = file(
            "tenant_platform-t.json",
            json!({"settings": {"display_name": "Synthetic", signing: {"from": "the package"}}}),
        );
        let preview = target
            .preview(Section::TenantConfiguration, &wanted)
            .await
            .unwrap();
        assert!(
            !preview.blockers.is_empty(),
            "a package that would change the signing trust is blocked: {preview:?}"
        );
        // Applied anyway (as if the installed value changed after the
        // preview): the installed trust stays, and so does what the package
        // does not name.
        let task = target
            .begin(Section::TenantConfiguration)
            .await
            .unwrap()
            .unwrap();
        target
            .apply(Section::TenantConfiguration, &wanted, Some(&task))
            .await
            .unwrap();
        let settings = tenant_settings(&id).await;
        assert_eq!(settings["display_name"], "Synthetic");
        assert_eq!(settings["kept"], "installed");
        assert_eq!(settings[signing], json!({"installed": true}));
    })
}
