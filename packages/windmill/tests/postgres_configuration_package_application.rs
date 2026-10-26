// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! A configuration package application as the task runs it
//! (`apply_configuration_package::execute`): the tenant's claim, the package
//! read from storage and admitted, the plan, the run through the platform
//! target, and the record saved on the task row; then a run that resumes from
//! that record, and one refused while another holds the claim. The package
//! carries the two sections that live in the platform's own database (the
//! tenant configuration and the trustees). The process-wide pools point at the
//! fixture database and a loopback S3 serves the package; everything is
//! synthetic and local.

// An application is one deeply nested future.
#![recursion_limit = "256"]

#[path = "support/s3_stub.rs"]
mod s3_stub;
#[path = "support/schema.rs"]
mod schema;

use deadpool_postgres::Pool;
use serde_json::{json, Value};
use std::ffi::OsString;
use std::future::Future;
use std::io::Write;
use std::sync::LazyLock;
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::sync::OnceCell;
use windmill::postgres::tasks_execution::get_task_by_id;
use windmill::services::database::{get_hasura_pool, get_keycloak_pool};
use windmill::services::import::configuration_package_apply::Outcome;
use windmill::tasks::apply_configuration_package::{execute, ApplyConfigurationPackageBody};

static FIXTURE: OnceCell<Pool> = OnceCell::const_new();

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

fn zip(members: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, bytes) in members {
        out.start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        out.write_all(bytes).unwrap();
    }
    out.finish().unwrap().into_inner()
}

/// An unsigned delivery with the tenant configuration and two trustees (the
/// public halves of election-architect's historical fixture's keys).
fn package() -> Vec<u8> {
    let tenant = json!({"settings": {"display_name": "Synthetic Applied"}});
    let trustees = json!([
        {"name": "trustee-a", "signingPublicKey": "J5CjY7Q9EgLPaK17Wu5ANhHwSWOX4d+wJLUS+Teevyc=",
         "shareEncryptionPublicKey": "ys1VtTvfwbBpHhjMHTNCUUnNr4RsBuYzQvMd0w83Gjg="},
        {"name": "trustee-b", "signingPublicKey": "093kVmcZJpRke1Ry/vYxm6cdsSNP0YA2K32WX88d2xo=",
         "shareEncryptionPublicKey": "XB5WQGFMeuKE44aGNR9x+bgrRxgUYMUAj+WfEt/mUys="},
    ]);
    let portal = zip(&[
        (
            "tenant_platform-synthetic.json",
            &serde_json::to_vec(&tenant).unwrap(),
        ),
        (
            "trustees_public.json",
            &serde_json::to_vec(&trustees).unwrap(),
        ),
    ]);
    zip(&[("admin_portal_settings.zip", &portal)])
}

/// A synthetic tenant, its uploaded package document, and a task row for an
/// application of it.
async fn tenant_with_document() -> (uuid::Uuid, uuid::Uuid) {
    let (tenant, document) = (uuid::Uuid::new_v4(), uuid::Uuid::new_v4());
    let client = fixture().await.get().await.unwrap();
    client
        .execute(
            "INSERT INTO sequent_backend.tenant (id, slug, is_active) VALUES ($1, $2, true)",
            &[&tenant, &format!("application-{tenant}")],
        )
        .await
        .unwrap();
    client
        .execute(
            "INSERT INTO sequent_backend.document (id, tenant_id, name) VALUES ($1, $2, 'delivery.zip')",
            &[&document, &tenant],
        )
        .await
        .unwrap();
    (tenant, document)
}

async fn task(tenant: uuid::Uuid) -> String {
    let id = uuid::Uuid::new_v4();
    fixture()
        .await
        .get()
        .await
        .unwrap()
        .execute(
            "INSERT INTO sequent_backend.tasks_execution (id, tenant_id, name, type, execution_status, executed_by_user)
             VALUES ($1, $2, 'Apply Configuration Package', 'APPLY_CONFIGURATION_PACKAGE', 'IN_PROGRESS', 'synthetic-operator')",
            &[&id, &tenant],
        )
        .await
        .unwrap();
    id.to_string()
}

fn body(tenant: uuid::Uuid, document: uuid::Uuid, mode: &str) -> ApplyConfigurationPackageBody {
    serde_json::from_value(json!({
        "tenant_id": tenant.to_string(),
        "document_id": document.to_string(),
        "mode": mode,
        "requirements": {"sections": ["tenant-configuration", "trustees"], "census": "optional"},
        "event_import": {
            "tenant_id": tenant.to_string(),
            "document_id": "",
            "password": null,
            "check_only": null,
            "sha256": null,
        },
        "audit_actor": null,
        "executed_by": "synthetic-operator",
    }))
    .unwrap()
}

async fn saved(task: &str) -> Value {
    fixture()
        .await
        .get()
        .await
        .unwrap()
        .query_one(
            "SELECT coalesce(annotations, '{}'::jsonb) FROM sequent_backend.tasks_execution WHERE id = $1::text::uuid",
            &[&task],
        )
        .await
        .unwrap()
        .get(0)
}

/// The included sections' states in the saved record (it lists every
/// section; the others are not included).
fn states(annotations: &Value) -> Vec<(String, String)> {
    annotations["application"]["sections"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| {
            (
                s["section"].as_str().unwrap().to_string(),
                s["status"]["state"].as_str().unwrap().to_string(),
            )
        })
        .filter(|(_, state)| state != "not-included")
        .collect()
}

async fn apply(tenant: uuid::Uuid, document: uuid::Uuid, mode: &str) -> (String, Outcome, Value) {
    let id = task(tenant).await;
    let row = get_task_by_id(&id).await.unwrap();
    let record = tokio::time::timeout(
        Duration::from_secs(60),
        execute(&body(tenant, document, mode), &row),
    )
    .await
    .expect("the application finished")
    .expect("the application ran");
    let annotations = saved(&id).await;
    (id, record.outcome, annotations)
}

#[test]
fn a_package_is_previewed_applied_and_then_found_applied() {
    run(async {
        fixture().await;
        let (endpoint, asked) = s3_stub::serve(package()).await;
        std::env::set_var("AWS_REGION", "us-east-1");
        std::env::set_var("AWS_S3_PRIVATE_URI", &endpoint);
        std::env::set_var("AWS_S3_BUCKET", "synthetic-documents");
        std::env::set_var("AWS_S3_ACCESS_KEY", "synthetic");
        std::env::set_var("AWS_S3_ACCESS_SECRET", "synthetic");
        let (tenant, document) = tenant_with_document().await;

        let (_, previewed, record) = apply(tenant, document, "preview").await;
        assert_eq!(previewed, Outcome::Previewed);
        assert_eq!(record["application"]["outcome"], "previewed", "{record}");
        assert_eq!(
            states(&record),
            [
                ("tenant-configuration".into(), "previewed".into()),
                ("trustees".into(), "previewed".into()),
            ],
            "{record}"
        );
        let client = fixture().await.get().await.unwrap();
        let display: Option<String> = client
            .query_one(
                "SELECT settings->>'display_name' FROM sequent_backend.tenant WHERE id = $1",
                &[&tenant],
            )
            .await
            .unwrap()
            .get(0);
        assert_ne!(
            display.as_deref(),
            Some("Synthetic Applied"),
            "a preview changes nothing"
        );

        let (_, applied, record) = apply(tenant, document, "apply").await;
        assert_eq!(applied, Outcome::Applied, "{record}");
        assert_eq!(
            states(&record),
            [
                ("tenant-configuration".into(), "applied".into()),
                ("trustees".into(), "applied".into()),
            ],
            "{record}"
        );
        let display: Option<String> = client
            .query_one(
                "SELECT settings->>'display_name' FROM sequent_backend.tenant WHERE id = $1",
                &[&tenant],
            )
            .await
            .unwrap()
            .get(0);
        assert_eq!(display.as_deref(), Some("Synthetic Applied"));
        let trustees: i64 = client
            .query_one(
                "SELECT count(*) FROM sequent_backend.trustee WHERE tenant_id = $1",
                &[&tenant],
            )
            .await
            .unwrap()
            .get(0);
        assert_eq!(trustees, 2);

        // Applied again: the earlier run's record is where it resumes, and
        // nothing is left to change.
        let (_, again, record) = apply(tenant, document, "apply").await;
        assert_eq!(again, Outcome::Applied, "{record}");
        let trustees: i64 = client
            .query_one(
                "SELECT count(*) FROM sequent_backend.trustee WHERE tenant_id = $1",
                &[&tenant],
            )
            .await
            .unwrap()
            .get(0);
        assert_eq!(trustees, 2, "no trustee registered twice");
        assert!(
            asked
                .lock()
                .unwrap()
                .iter()
                .any(|line| line.contains(&document.to_string())),
            "the package was read from storage"
        );

        // One application per tenant at a time: while another holds the
        // claim, a run is refused and does not touch the tenant.
        let mut holder = fixture().await.get().await.unwrap();
        let claim = holder.transaction().await.unwrap();
        let held: bool = claim
            .query_one(
                "SELECT pg_try_advisory_xact_lock(hashtextextended($1, 0))",
                &[&format!("package-application:{tenant}")],
            )
            .await
            .unwrap()
            .get(0);
        assert!(held);
        let id = task(tenant).await;
        let row = get_task_by_id(&id).await.unwrap();
        let refused = tokio::time::timeout(
            Duration::from_secs(60),
            execute(&body(tenant, document, "apply"), &row),
        )
        .await
        .expect("the refusal is immediate")
        .unwrap_err();
        assert!(
            format!("{refused:#}").contains("another configuration package application"),
            "{refused:#}"
        );
        claim.rollback().await.unwrap();
    })
}
