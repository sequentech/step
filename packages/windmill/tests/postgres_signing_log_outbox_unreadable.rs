// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! An outbox row this version can't read (a kind a newer version queued)
//! fails and stops its event, like an entry the board refused. A binary of
//! its own: the test drops a CHECK in its transaction, which would hold up
//! every other test on the table.

#[path = "support/schema.rs"]
mod schema;

use anyhow::Result;
use deadpool_postgres::Transaction;
use electoral_log::messages::message::SigningData;
use electoral_log::messages::newtypes::SigningStatementKind;
use serde_json::json;
use std::time::Duration;
use strand::signature::StrandSignatureSk;
use uuid::Uuid;
use windmill::postgres::signing::{SigningLogOutboxRow, SigningLogOutboxUser};
use windmill::services::signing::log::{
    outbox_snapshot, post_outbox_snapshot, stage, Actor, LogScope, LogStep, OutboxBatch,
    OutboxSnapshot, SigningLogBoard, SigningLogDelivery, SigningLogKeys, SystemOutcome,
};

/// Counts the entries written.
#[derive(Default)]
struct CountingBoard {
    written: Vec<i64>,
}

impl SigningLogBoard for CountingBoard {
    async fn deliver(&mut self, _: &str, deliveries: &[SigningLogDelivery]) -> Result<Vec<bool>> {
        self.written
            .extend(deliveries.iter().map(|delivery| delivery.row_id));
        Ok(vec![true; deliveries.len()])
    }
}

struct TestKeys(SigningData);

impl SigningLogKeys for TestKeys {
    async fn prepare(
        &mut self,
        _: &mut deadpool_postgres::Client,
        _: Uuid,
        _: Uuid,
        _: &[SigningLogOutboxUser],
    ) -> Result<String> {
        Ok("board".to_string())
    }

    fn signing_data(&self, _: &SigningLogOutboxRow) -> Result<&SigningData> {
        Ok(&self.0)
    }
}

fn log_step(tenant: Uuid, event: Uuid, n: u32) -> LogStep {
    LogStep {
        kind: SigningStatementKind::SigningRequestCreated,
        user: Actor {
            user_id: format!("user-{n}"),
            username: format!("sbei-{n}"),
        },
        system: SystemOutcome::Info,
        scope: LogScope {
            tenant_id: tenant,
            election_event_id: event,
            election_id: None,
            area_id: None,
        },
        description: format!("Step {n}"),
        details: json!({ "n": n }),
    }
}

#[tokio::test]
async fn an_unreadable_row_fails_and_stops_its_event() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let (tenant, event) = (Uuid::new_v4(), Uuid::new_v4());
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, 'unreadable')",
        &[&tenant],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol)
         VALUES ($1, $2, 'RSA256')",
        &[&event, &tenant],
    )
    .await
    .unwrap();

    stage(&tx, &log_step(tenant, event, 1)).await.unwrap();
    tx.batch_execute(
        "ALTER TABLE sequent_backend.signing_log_outbox
         DROP CONSTRAINT signing_log_outbox_statement_kind_known",
    )
    .await
    .unwrap();
    let unreadable: i64 = tx
        .query_one(
            "INSERT INTO sequent_backend.signing_log_outbox
                 (tenant_id, election_event_id, step_id, entry, statement_kind, event_type,
                  log_type, body)
             VALUES ($1, $2, $3, 1, 'SigningFromANewerVersion', 'SYSTEM', 'INFO', '{}')
             RETURNING id",
            &[&tenant, &event, &Uuid::new_v4()],
        )
        .await
        .unwrap()
        .get(0);
    stage(&tx, &log_step(tenant, event, 2)).await.unwrap();

    let mut board = CountingBoard::default();
    let OutboxSnapshot::Ready(up_to_id) =
        outbox_snapshot(&tx, tenant, event, Duration::from_secs(2))
            .await
            .unwrap()
    else {
        panic!("no snapshot");
    };
    let sk = StrandSignatureSk::generate().unwrap();
    let keys = TestKeys(SigningData::new(sk.clone(), "", sk));
    let batch = OutboxBatch {
        up_to_id,
        rows: 200,
        per_transaction: 16,
    };
    let pass = post_outbox_snapshot(&tx, tenant, event, &keys, &mut board, "board", batch)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(pass.posted, 2);
    assert_eq!(pass.failed, Some(unreadable));
    assert_eq!(board.written.len(), 2);

    let rows: Vec<(i64, bool, i32, Option<String>)> = tx
        .query(
            "SELECT id, posted_at IS NOT NULL, attempts, last_error
             FROM sequent_backend.signing_log_outbox
             WHERE election_event_id = $1 ORDER BY id",
            &[&event],
        )
        .await
        .unwrap()
        .iter()
        .map(|row| (row.get(0), row.get(1), row.get(2), row.get(3)))
        .collect();
    assert_eq!(rows.len(), 5);
    assert!(rows[..2].iter().all(|row| row.1 && row.2 == 0));
    let failed = &rows[2];
    assert_eq!((failed.0, failed.1, failed.2), (unreadable, false, 1));
    let error = failed.3.as_deref().unwrap();
    assert!(error.contains("SigningFromANewerVersion"), "{error}");
    // The steps after it wait.
    assert!(rows[3..].iter().all(|row| !row.1 && row.2 == 0));
    tx.rollback().await.unwrap();
}
