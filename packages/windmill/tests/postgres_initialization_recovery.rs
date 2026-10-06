// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The all-event sweep owns a separate fixture database so it cannot stage
//! records belonging to another integration test.

#[path = "support/schema.rs"]
mod schema;

use deadpool_postgres::Transaction;
use uuid::Uuid;
use windmill::services::initialization_record::{
    stage_initialization_log, sweep_initialization_logs,
};

async fn recorded(tx: &Transaction<'_>) -> (Uuid, Uuid, Uuid) {
    let (tenant, event, post, initialization) = (
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
        &[&tenant, &tenant.to_string()],
    )
    .await
    .unwrap();
    tx.execute("INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol) VALUES ($1, $2, 'RSA256')", &[&event, &tenant]).await.unwrap();
    tx.execute("INSERT INTO sequent_backend.election (id, tenant_id, election_event_id) VALUES ($1, $2, $3)", &[&post, &tenant, &event]).await.unwrap();
    tx.execute("INSERT INTO sequent_backend.election_initialization (id, tenant_id, election_event_id, election_id, tally_session_id, election_name) VALUES ($1, $2, $3, $4, $5, 'Test Post')", &[&initialization, &tenant, &event, &post, &Uuid::new_v4()]).await.unwrap();
    (tenant, event, initialization)
}

#[tokio::test]
async fn periodic_recovery_retries_failed_staging_without_another_tally_and_does_not_starve_other_events(
) {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let (tenant, event, pending) = recorded(&tx).await;
    let (_, _, healthy) = recorded(&tx).await;
    tx.batch_execute(&format!("CREATE FUNCTION sequent_backend.reject_recovery_test() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.election_event_id = '{event}'::uuid THEN RAISE EXCEPTION 'injected staging failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_recovery_test BEFORE INSERT ON sequent_backend.signing_log_outbox FOR EACH ROW EXECUTE FUNCTION sequent_backend.reject_recovery_test();")).await.unwrap();
    tx.commit().await.unwrap();

    let error = stage_initialization_log(&mut client, tenant, event)
        .await
        .unwrap_err();
    assert!(format!("{error:?}").contains("injected staging failure"));
    assert_eq!(sweep_initialization_logs(&mut client).await.unwrap(), 1);
    for (id, expected) in [(pending, false), (healthy, true)] {
        let staged: bool = client.query_one("SELECT log_staged_at IS NOT NULL FROM sequent_backend.election_initialization WHERE id = $1", &[&id]).await.unwrap().get(0);
        assert_eq!(staged, expected);
    }
    client.batch_execute("DROP TRIGGER reject_recovery_test ON sequent_backend.signing_log_outbox; DROP FUNCTION sequent_backend.reject_recovery_test();").await.unwrap();
    assert_eq!(sweep_initialization_logs(&mut client).await.unwrap(), 1);
    assert_eq!(sweep_initialization_logs(&mut client).await.unwrap(), 0);
    for id in [pending, healthy] {
        let logs: i64 = client.query_one("SELECT count(*) FROM sequent_backend.signing_log_outbox WHERE step_id = $1 AND statement_kind = 'ElectionInitialized'", &[&id]).await.unwrap().get(0);
        assert_eq!(logs, 2, "one USER and one SYSTEM entry, exactly once");
    }
}
