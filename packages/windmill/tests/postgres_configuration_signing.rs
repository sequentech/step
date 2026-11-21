// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The database keeps a tenant's `settings.configuration_signing` from any
//! write but the validated setter's, and a restored backup keeps it.
//!
//! Needs the PostgreSQL server `HASURA_DB__*` names; every test rolls back.

#[path = "support/schema.rs"]
mod schema;
#[allow(dead_code)]
#[path = "support/signing_pki.rs"]
mod signing_pki;

use deadpool_postgres::Transaction;
use serde_json::{json, Value};
use uuid::Uuid;
use windmill::postgres::tenant::{
    allow_configuration_signing_change, delete_tenant, write_configuration_signing,
};

const REQUIRED: &str =
    r#"{"policy": "required", "package_roots": "", "staff_roots": "", "required_approvals": 2}"#;

fn required() -> Value {
    serde_json::from_str(REQUIRED).unwrap()
}

/// A tenant with `settings`, inserted as the setter would.
async fn tenant(tx: &Transaction<'_>, settings: Value) -> Uuid {
    let id = Uuid::new_v4();
    allow_configuration_signing_change(tx, &id.to_string())
        .await
        .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug, settings) VALUES ($1, $2, $3)",
        &[&id, &format!("tenant-{id}"), &settings],
    )
    .await
    .unwrap();
    // Drop the permission again: what follows runs as any other write would.
    tx.execute(
        "SELECT set_config('sequent.configuration_signing_tenant', '', true)",
        &[],
    )
    .await
    .unwrap();
    id
}

async fn settings(tx: &Transaction<'_>, id: Uuid) -> Value {
    tx.query_one(
        "SELECT settings FROM sequent_backend.tenant WHERE id = $1",
        &[&id],
    )
    .await
    .unwrap()
    .get(0)
}

/// Runs `sql` inside a savepoint and reports whether the guard refused it,
/// rolling back either way so the next statement runs on the same state.
async fn refused(tx: &Transaction<'_>, sql: &str, id: Uuid) -> bool {
    tx.execute("SAVEPOINT attempt", &[]).await.unwrap();
    let result = tx.execute(sql, &[&id]).await;
    tx.execute("ROLLBACK TO SAVEPOINT attempt", &[])
        .await
        .unwrap();
    match result {
        Ok(_) => false,
        Err(error) => {
            let message = error
                .as_db_error()
                .map(|db| db.message().to_string())
                .unwrap_or_default();
            assert!(
                message.contains("configuration_signing is set only through its own setting"),
                "refused for another reason: {error:?}"
            );
            true
        }
    }
}

#[tokio::test]
async fn every_direct_write_of_the_policy_is_refused() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let id = tenant(
        &tx,
        json!({"configuration_signing": required(), "theme": "dark"}),
    )
    .await;

    // What a GraphQL update, _append, _delete_key or a restore would run.
    for sql in [
        r#"UPDATE sequent_backend.tenant SET settings = '{"theme": "dark"}' WHERE id = $1"#,
        r#"UPDATE sequent_backend.tenant SET settings = settings - 'configuration_signing' WHERE id = $1"#,
        r#"UPDATE sequent_backend.tenant SET settings = settings || '{"configuration_signing": {"policy": "optional"}}' WHERE id = $1"#,
        r#"UPDATE sequent_backend.tenant SET settings = settings || '{"configuration_signing": null}' WHERE id = $1"#,
        r#"UPDATE sequent_backend.tenant SET settings = jsonb_set(settings, '{configuration_signing,required_approvals}', '0') WHERE id = $1"#,
        r#"UPDATE sequent_backend.tenant SET settings = NULL WHERE id = $1"#,
        r#"DELETE FROM sequent_backend.tenant WHERE id = $1"#,
        r#"UPDATE sequent_backend.tenant SET id = gen_random_uuid() WHERE id = $1"#,
    ] {
        assert!(refused(&tx, sql, id).await, "{sql} was allowed");
    }
    assert_eq!(settings(&tx, id).await["configuration_signing"], required());

    // A new row can't bring its own trust either.
    let other = Uuid::new_v4();
    tx.execute("SAVEPOINT insert", &[]).await.unwrap();
    let inserted = tx
        .execute(
            "INSERT INTO sequent_backend.tenant (id, slug, settings) VALUES ($1, $2, $3)",
            &[
                &other,
                &format!("tenant-{other}"),
                &json!({"configuration_signing": required()}),
            ],
        )
        .await;
    tx.execute("ROLLBACK TO SAVEPOINT insert", &[])
        .await
        .unwrap();
    assert!(inserted.is_err(), "an insert with a policy was allowed");
}

#[tokio::test]
async fn other_settings_change_as_before() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let id = tenant(
        &tx,
        json!({"configuration_signing": required(), "theme": "dark"}),
    )
    .await;
    // The settings tabs write the whole object back with the key unchanged.
    let whole = format!(
        r#"UPDATE sequent_backend.tenant SET settings = '{{"configuration_signing": {REQUIRED}, "theme": "light"}}' WHERE id = $1"#
    );
    assert!(!refused(&tx, &whole, id).await);
    assert!(!refused(
        &tx,
        r#"UPDATE sequent_backend.tenant SET settings = settings || '{"theme": "light"}', labels = '{"a": 1}' WHERE id = $1"#,
        id
    )
    .await);

    // A tenant without a policy stays without one, and can be created and
    // deleted as before.
    let plain = tenant(&tx, json!({"theme": "dark"})).await;
    assert!(
        !refused(
            &tx,
            r#"UPDATE sequent_backend.tenant SET settings = '{"theme": "light"}' WHERE id = $1"#,
            plain
        )
        .await
    );
    assert!(refused(
        &tx,
        r#"UPDATE sequent_backend.tenant SET settings = settings || '{"configuration_signing": {"policy": "optional"}}' WHERE id = $1"#,
        plain
    )
    .await);
    assert!(
        !refused(
            &tx,
            "DELETE FROM sequent_backend.tenant WHERE id = $1",
            plain
        )
        .await
    );
}

#[tokio::test]
async fn the_permission_covers_one_tenant_for_one_transaction() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let ours = tenant(&tx, json!({"theme": "dark"})).await;
    let theirs = tenant(&tx, json!({"configuration_signing": required()})).await;

    allow_configuration_signing_change(&tx, &ours.to_string())
        .await
        .unwrap();
    // Allowed for ours, not for theirs.
    assert!(
        refused(
            &tx,
            r#"UPDATE sequent_backend.tenant SET settings = '{}' WHERE id = $1"#,
            theirs
        )
        .await
    );
    let previous = write_configuration_signing(&tx, &ours.to_string(), &required())
        .await
        .unwrap();
    assert_eq!(previous, None);
    assert_eq!(
        settings(&tx, ours).await,
        json!({"configuration_signing": required(), "theme": "dark"})
    );
    tx.commit().await.unwrap();

    // The next transaction on the same pooled connection has no permission.
    let tx = client.transaction().await.unwrap();
    let carried: Option<String> = tx
        .query_one(
            "SELECT current_setting('sequent.configuration_signing_tenant', true)",
            &[],
        )
        .await
        .unwrap()
        .get(0);
    assert!(carried.unwrap_or_default().is_empty());
    assert!(
        refused(
            &tx,
            r#"UPDATE sequent_backend.tenant SET settings = '{"theme": "dark"}' WHERE id = $1"#,
            ours
        )
        .await
    );
    // Clean up the committed rows through the paths allowed to.
    delete_tenant(&tx, &ours.to_string()).await.unwrap();
    delete_tenant(&tx, &theirs.to_string()).await.unwrap();
    tx.commit().await.unwrap();
}

#[tokio::test]
async fn a_restored_backup_keeps_the_installed_policy_and_its_absence() {
    use windmill::services::import::import_tenant::process_record;
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();

    let backup = |settings: &str| {
        {
            // As the tenant export writes a row: its dates are RFC 3339.
            csv::StringRecord::from(vec![
                "source",
                "slug",
                "2026-01-01T00:00:00+00:00",
                "2026-01-02T00:00:00+00:00",
                "",
                "",
                "true",
                "",
                settings,
                "null",
            ])
        }
    };
    let guarded = tenant(
        &tx,
        json!({"configuration_signing": required(), "theme": "dark"}),
    )
    .await;
    // A backup that says signatures are optional.
    process_record(
        &tx,
        &guarded.to_string(),
        &backup(r#"{"configuration_signing": {"policy": "optional"}, "theme": "light"}"#),
    )
    .await
    .unwrap();
    assert_eq!(
        settings(&tx, guarded).await,
        json!({"configuration_signing": required(), "theme": "light"})
    );
    // An old backup from before any policy.
    process_record(&tx, &guarded.to_string(), &backup(r#"{"theme": "blue"}"#))
        .await
        .unwrap();
    assert_eq!(
        settings(&tx, guarded).await,
        json!({"configuration_signing": required(), "theme": "blue"})
    );

    // A tenant without a policy restores as before, and a backup can't add one.
    let plain = tenant(&tx, json!({"theme": "dark"})).await;
    process_record(
        &tx,
        &plain.to_string(),
        &backup(r#"{"configuration_signing": {"policy": "optional"}, "theme": "light"}"#),
    )
    .await
    .unwrap();
    assert_eq!(settings(&tx, plain).await, json!({"theme": "light"}));
}

#[tokio::test]
async fn the_setting_does_not_change_under_a_running_application_or_import() {
    use sequent_core::election_config::package_verify::{
        ConfigurationSigning, ConfigurationSigningPolicy,
    };
    use windmill::postgres::configuration_packages::{application_lock_key, hold_imports};
    use windmill::services::configuration_signing::{save, SaveOutcome};

    let pool = schema::pool().await;
    let optional = ConfigurationSigning {
        policy: ConfigurationSigningPolicy::Optional,
        ..ConfigurationSigning::default()
    };
    let mut client = pool.get().await.unwrap();
    let setup = client.transaction().await.unwrap();
    let id = tenant(&setup, json!({"theme": "dark"})).await;
    setup.commit().await.unwrap();
    let tenant_id = id.to_string();

    // An application holds its claim on a connection of its own for its run.
    let mut runner = pool.get().await.unwrap();
    let run = runner.transaction().await.unwrap();
    run.query_one(
        "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
        &[&application_lock_key(&tenant_id)],
    )
    .await
    .unwrap();
    let tx = client.transaction().await.unwrap();
    let outcome = save(&tx, &tenant_id, &optional, chrono::Utc::now())
        .await
        .unwrap();
    assert!(matches!(outcome, SaveOutcome::Busy), "{outcome:?}");
    assert_eq!(settings(&tx, id).await, json!({"theme": "dark"}));
    drop(tx);
    drop(run);

    // So does an election event import of a signed package.
    let held = hold_imports(&pool, &tenant_id).await.unwrap();
    let tx = client.transaction().await.unwrap();
    let outcome = save(&tx, &tenant_id, &optional, chrono::Utc::now())
        .await
        .unwrap();
    assert!(matches!(outcome, SaveOutcome::Busy), "{outcome:?}");
    drop(tx);
    drop(held);

    // With neither running it saves, and while it is saving an application
    // can't claim the tenant.
    let mut saved = false;
    for _ in 0..50 {
        let tx = client.transaction().await.unwrap();
        match save(&tx, &tenant_id, &optional, chrono::Utc::now())
            .await
            .unwrap()
        {
            SaveOutcome::Saved { .. } => {
                let claimed: bool = runner
                    .query_one(
                        "SELECT pg_try_advisory_lock(hashtextextended($1, 0))",
                        &[&application_lock_key(&tenant_id)],
                    )
                    .await
                    .unwrap()
                    .get(0);
                assert!(!claimed, "an application claimed the tenant mid-change");
                tx.commit().await.unwrap();
                saved = true;
                break;
            }
            // The import hold's connection is closing; its lock goes with it.
            SaveOutcome::Busy => {
                drop(tx);
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
            other => panic!("{other:?}"),
        }
    }
    assert!(saved, "the setting was never saved");
    let tx = client.transaction().await.unwrap();
    assert_eq!(
        settings(&tx, id).await["configuration_signing"]["policy"],
        "optional"
    );
    delete_tenant(&tx, &tenant_id).await.unwrap();
    tx.commit().await.unwrap();
}

/// Format 2: a policy that enrolls signers saves only when it states its
/// minimum format, requires signatures with format 2, gives each key one
/// owner and can be satisfied; it is stored canonically. A legacy setting
/// is stored without any format-2 field.
#[tokio::test]
async fn format_2_policies_save_only_when_complete_and_legacy_ones_stay_legacy() {
    use sequent_core::election_config::package_verify::{
        ConfigurationSigning, ConfigurationSigningPolicy,
    };
    use sequent_core::election_config::signing_policy::{
        EnrolledKey, EnrolledSigner, FinalSignerSeparation, PackageFormat, SignerRole,
    };
    use windmill::services::configuration_signing::{save, SaveOutcome};

    let signer = |id: &str, role: SignerRole, pin: u8| EnrolledSigner {
        id: id.to_string(),
        name: format!("DEMO {id}"),
        roles: vec![role],
        keys: vec![EnrolledKey {
            spki_sha256: format!("{pin:02x}").repeat(32),
            label: String::new(),
        }],
    };
    let root = signing_pki::Pki::get().root.pem();
    let v2 = ConfigurationSigning {
        policy: ConfigurationSigningPolicy::Required,
        package_roots: root.clone(),
        staff_roots: root.clone(),
        required_approvals: 2,
        minimum_package_format: Some(PackageFormat::V2),
        final_signer_separation: Some(FinalSignerSeparation::Separate),
        signers: vec![
            signer("custodian", SignerRole::FinalSigner, 3),
            signer("approver-2", SignerRole::Approver, 2),
            signer("approver-1", SignerRole::Approver, 1),
        ],
    };

    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let id = tenant(&tx, json!({})).await;
    let tenant_id = id.to_string();

    let refused_with = |outcome: SaveOutcome, expected: &str| match outcome {
        SaveOutcome::Refused { check } => assert!(
            check
                .report
                .errors()
                .any(|p| p.id.as_deref() == Some(expected)),
            "{expected} missing"
        ),
        other => panic!("expected {expected}, got {other:?}"),
    };
    let mut no_minimum = v2.clone();
    no_minimum.minimum_package_format = None;
    refused_with(
        save(&tx, &tenant_id, &no_minimum, chrono::Utc::now())
            .await
            .unwrap(),
        "configuration-signing.minimum-format-required",
    );
    let mut optional = v2.clone();
    optional.policy = ConfigurationSigningPolicy::Optional;
    refused_with(
        save(&tx, &tenant_id, &optional, chrono::Utc::now())
            .await
            .unwrap(),
        "configuration-signing.optional-with-v2",
    );
    let mut shared = v2.clone();
    shared.signers[1].keys[0].spki_sha256 = "01".repeat(32);
    refused_with(
        save(&tx, &tenant_id, &shared, chrono::Utc::now())
            .await
            .unwrap(),
        "configuration-signing.shared-key",
    );
    assert_eq!(
        settings(&tx, id).await,
        json!({}),
        "nothing refused was written"
    );

    let outcome = save(&tx, &tenant_id, &v2, chrono::Utc::now())
        .await
        .unwrap();
    assert!(matches!(outcome, SaveOutcome::Saved { .. }), "{outcome:?}");
    let stored = settings(&tx, id).await["configuration_signing"].clone();
    assert_eq!(stored["minimum_package_format"], "v2");
    assert_eq!(stored["final_signer_separation"], "separate");
    let ids: Vec<&str> = stored["signers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        ids,
        ["approver-1", "approver-2", "custodian"],
        "stored canonically"
    );
    let read: ConfigurationSigning = serde_json::from_value(stored).unwrap();
    assert_eq!(read.policy_sha256().unwrap(), v2.policy_sha256().unwrap());

    // A legacy setting: no format-2 field is stored.
    let legacy = ConfigurationSigning {
        policy: ConfigurationSigningPolicy::Required,
        package_roots: root.clone(),
        staff_roots: root,
        required_approvals: 2,
        ..ConfigurationSigning::default()
    };
    let outcome = save(&tx, &tenant_id, &legacy, chrono::Utc::now())
        .await
        .unwrap();
    assert!(matches!(outcome, SaveOutcome::Saved { .. }), "{outcome:?}");
    let stored = settings(&tx, id).await["configuration_signing"].clone();
    for field in ["minimum_package_format", "final_signer_separation"] {
        assert!(
            stored.get(field).is_none(),
            "{field} stored for a legacy setting"
        );
    }
    assert!(
        stored.get("signers").is_none(),
        "signers stored for a legacy setting"
    );
}
