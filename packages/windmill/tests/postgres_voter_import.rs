// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Durable completed-import evidence, audit identity and transaction rollback
//! against the same migrated schema used by the services.

#[path = "support/schema.rs"]
mod schema;

use sequent_core::types::voter_import::VotersImportedEvidence;
use uuid::Uuid;
use windmill::postgres::voter_import::{
    complete_import_task, fail_import_task, latest_voter_import, record_voter_import,
};
use windmill::services::audit_log::AuditActor;

fn evidence() -> VotersImportedEvidence {
    VotersImportedEvidence {
        import_id: String::new(),
        document_id: "certified-document".into(),
        file_name: "certified.csv".into(),
        sha256: "ab".repeat(32),
        declared_sha256: Some("ab".repeat(32)),
        counts_by_post: [("North".into(), 2), ("South".into(), 1)].into(),
        imported_at: "2026-10-14T10:00:00Z".into(),
    }
}

#[tokio::test]
async fn completed_import_receipts_share_audit_identity_and_latest_never_hides_uncertified_imports()
{
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let tenant = Uuid::new_v4();
    let event = Uuid::new_v4();
    let task = Uuid::new_v4();
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
        &[&tenant, &format!("import-{tenant}")],
    )
    .await
    .unwrap();
    tx.execute("INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol) VALUES ($1, $2, 'RSA256')", &[&event, &tenant]).await.unwrap();
    tx.execute("INSERT INTO sequent_backend.tasks_execution (id, tenant_id, election_event_id, name, type, execution_status, executed_by_user) VALUES ($1, $2, $3, 'Import voters', 'IMPORT_USERS', 'IN_PROGRESS', 'ada')", &[&task, &tenant, &event]).await.unwrap();
    let tenant_id = tenant.to_string();
    let event_id = event.to_string();
    let task_id = task.to_string();
    let actor = AuditActor {
        user_id: Some("admin-id".into()),
        username: Some("ada".into()),
        role: Some("admin-user".into()),
    };
    assert!(latest_voter_import(&tx, &tenant_id, &event_id)
        .await
        .unwrap()
        .is_none());
    let completed = record_voter_import(
        &tx,
        &tenant_id,
        &event_id,
        &task_id,
        Some(&actor),
        evidence(),
    )
    .await
    .unwrap();
    complete_import_task(&tx, &tenant_id, &task_id)
        .await
        .unwrap();
    let id = Uuid::parse_str(&completed.import_id).unwrap();
    let row = tx.query_one("SELECT statement_kind, details, user_id, username FROM sequent_backend.audit_log_outbox WHERE id = $1", &[&id]).await.unwrap();
    assert_eq!(row.get::<_, String>(0), "VotersImported");
    let mut signed_details = serde_json::to_value(&completed).unwrap();
    signed_details["actor"] =
        serde_json::json!({"user_id":"admin-id", "username":"ada", "role":"admin-user"});
    assert_eq!(row.get::<_, serde_json::Value>(1), signed_details);
    assert_eq!(
        row.get::<_, serde_json::Value>(1)["actor"]["user_id"],
        "admin-id"
    );
    assert_eq!(row.get::<_, String>(2), "admin-id");
    assert_eq!(row.get::<_, String>(3), "ada");
    let task_row = tx.query_one("SELECT execution_status, annotations FROM sequent_backend.tasks_execution WHERE tenant_id = $1 AND id = $2", &[&tenant, &task]).await.unwrap();
    assert_eq!(task_row.get::<_, String>(0), "SUCCESS");
    assert_eq!(
        task_row.get::<_, serde_json::Value>(1)["voter_import_receipt"]["import_id"],
        completed.import_id
    );
    assert!(!fail_import_task(
        &tx,
        &tenant_id,
        Some(&event_id),
        &task_id,
        serde_json::json!([]),
        serde_json::json!({"problems": []})
    )
    .await
    .unwrap());
    let terminal: String = tx.query_one("SELECT execution_status FROM sequent_backend.tasks_execution WHERE tenant_id = $1 AND id = $2", &[&tenant, &task]).await.unwrap().get(0);
    assert_eq!(terminal, "SUCCESS");
    assert_eq!(
        latest_voter_import(&tx, &tenant_id, &event_id)
            .await
            .unwrap(),
        Some(completed.clone())
    );
    assert!(
        latest_voter_import(&tx, &Uuid::new_v4().to_string(), &event_id)
            .await
            .unwrap()
            .is_none()
    );

    tx.batch_execute("SAVEPOINT incomplete_receipt")
        .await
        .unwrap();
    let mut uncertified = evidence();
    uncertified.declared_sha256 = None;
    let newer = record_voter_import(
        &tx,
        &tenant_id,
        &event_id,
        &task_id,
        Some(&actor),
        uncertified,
    )
    .await
    .unwrap();
    assert!(!newer.is_certified());
    assert_eq!(
        latest_voter_import(&tx, &tenant_id, &event_id)
            .await
            .unwrap(),
        Some(newer)
    );
    tx.batch_execute("ROLLBACK TO SAVEPOINT incomplete_receipt")
        .await
        .unwrap();
    assert_eq!(
        latest_voter_import(&tx, &tenant_id, &event_id)
            .await
            .unwrap(),
        Some(completed)
    );
    let failed_task = Uuid::new_v4();
    let original_logs = serde_json::json!([{"message":"Original queue failure"}]);
    tx.execute("INSERT INTO sequent_backend.tasks_execution (id, tenant_id, election_event_id, name, type, execution_status, executed_by_user, logs) VALUES ($1, $2, $3, 'Import voters', 'IMPORT_USERS', 'FAILED', 'ada', $4)", &[&failed_task, &tenant, &event, &original_logs]).await.unwrap();
    assert!(!fail_import_task(
        &tx,
        &tenant_id,
        Some(&event_id),
        &failed_task.to_string(),
        serde_json::json!([]),
        serde_json::json!({"problems":[]})
    )
    .await
    .unwrap());
    let retained_logs: serde_json::Value = tx
        .query_one(
            "SELECT logs FROM sequent_backend.tasks_execution WHERE tenant_id = $1 AND id = $2",
            &[&tenant, &failed_task],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(retained_logs, original_logs);
    tx.rollback().await.unwrap();
}
