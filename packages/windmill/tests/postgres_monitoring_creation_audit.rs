// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Creation defaults use the event's real system identity and audit board.

#[path = "support/event_database.rs"]
mod event_database;
#[path = "support/schema.rs"]
mod schema;

use deadpool_postgres::Transaction;
use electoral_log::messages::message::Message;
use electoral_log::messages::newtypes::MonitoringDashboardMode;
use electoral_log::messages::statement::StatementBody;
use electoral_log::LogQuery;
use event_database::{run, EventDatabase};
use sequent_core::monitoring::revision::DashboardMode;
use serde_json::json;
use std::sync::Once;
use uuid::Uuid;
use voting_crypto::signing::VerificationKey;
use voting_crypto::RecordDeserialize;
use windmill::services::electoral_log_board::{
    create_protocol_manager_keys, get_board_client, get_board_signing_key,
};
use windmill::services::monitoring::audit::initialize_new_event;
use windmill::services::monitoring::config_store::{get_live_config, EventRef};
use windmill::services::vault::admin_user_signing_key_exists;

fn environment() {
    static SET: Once = Once::new();
    SET.call_once(|| std::env::set_var("MASTER_SECRET", "5a".repeat(32)));
}

async fn insert_event(tx: &Transaction<'_>, event: EventRef, board: &str) {
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
        &[&event.tenant_id, &event.tenant_id.to_string()],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.election_event
             (id, tenant_id, encryption_protocol, bulletin_board_reference)
         VALUES ($1, $2, 'RSA256', $3)",
        &[
            &event.election_event_id,
            &event.tenant_id,
            &json!({"id": 0, "database_name": board, "is_archived": false}),
        ],
    )
    .await
    .unwrap();
}

#[test]
fn creation_defaults_are_signed_by_the_existing_system_key_and_recorded_once() {
    run(async {
        environment();
        let event = EventRef {
            tenant_id: Uuid::new_v4(),
            election_event_id: Uuid::new_v4(),
        };
        let database = EventDatabase::create(event.tenant_id, event.election_event_id).await;
        let pool = schema::pool().await;
        let mut client = pool.get().await.unwrap();
        let tenant = event.tenant_id.to_string();
        let event_id = event.election_event_id.to_string();
        let tx = client.transaction().await.unwrap();
        insert_event(&tx, event, &database.board).await;
        create_protocol_manager_keys(&tx, &tenant, &event_id, &database.board)
            .await
            .unwrap();
        let key = get_board_signing_key(&tx, &tenant, Some(&event_id), &database.board)
            .await
            .unwrap();

        initialize_new_event(&tx, &tenant, &event_id).await.unwrap();
        let live = get_live_config(&tx, event).await.unwrap().unwrap();
        assert_eq!(live.mode, DashboardMode::Configured);
        assert_eq!(live.generation, 1);
        assert!(!live.assembled.set.dashboards.is_empty());
        assert!(!admin_user_signing_key_exists(&tx, &tenant, "system")
            .await
            .unwrap());
        // The creation transaction owns the defaults until it commits.
        let mut observer = pool.get().await.unwrap();
        let read = observer.transaction().await.unwrap();
        assert!(get_live_config(&read, event).await.unwrap().is_none());
        read.rollback().await.unwrap();
        tx.commit().await.unwrap();

        let board = get_board_client().await.unwrap();
        let entries = board
            .query(&database.board, &LogQuery::default())
            .await
            .unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].user_id.is_none());
        let message = Message::record_deserialize(&entries[0].message).unwrap();
        let system = VerificationKey::from_sk(&key).unwrap();
        message.verify(&system).unwrap();
        assert_eq!(
            message.sender.pk.to_b64_string().unwrap(),
            system.to_b64_string().unwrap()
        );
        let StatementBody::MonitoringConfigChanged(recorded_event, details) =
            message.statement.body
        else {
            panic!("Expected the creation defaults' audit entry");
        };
        assert_eq!(recorded_event.0, event_id);
        assert_eq!(details.mode, MonitoringDashboardMode::Configured);
        assert_eq!(details.generation, 1);
        assert_eq!(details.preset.unwrap().id.0, "comelec");
        assert_eq!(details.revisions.len(), live.documents.len());

        let tx = client.transaction().await.unwrap();
        initialize_new_event(&tx, &tenant, &event_id).await.unwrap();
        assert_eq!(get_live_config(&tx, event).await.unwrap().unwrap(), live);
        tx.commit().await.unwrap();
        assert_eq!(
            board
                .query(&database.board, &LogQuery::default())
                .await
                .unwrap()
                .len(),
            1
        );
        database.drop().await;
    });
}

#[test]
fn missing_system_identity_refuses_defaults_and_allows_creation_to_roll_back() {
    run(async {
        environment();
        let event = EventRef {
            tenant_id: Uuid::new_v4(),
            election_event_id: Uuid::new_v4(),
        };
        let database = EventDatabase::create(event.tenant_id, event.election_event_id).await;
        let pool = schema::pool().await;
        let mut client = pool.get().await.unwrap();
        let tenant = event.tenant_id.to_string();
        let event_id = event.election_event_id.to_string();
        let tx = client.transaction().await.unwrap();
        insert_event(&tx, event, &database.board).await;
        let error = initialize_new_event(&tx, &tenant, &event_id)
            .await
            .unwrap_err();
        assert!(format!("{error:#}").contains("audit board signing key not found"));
        tx.rollback().await.unwrap();
        let tx = client.transaction().await.unwrap();
        assert!(get_live_config(&tx, event).await.unwrap().is_none());
        assert_eq!(
            tx.query_one(
                "SELECT count(*) FROM sequent_backend.election_event WHERE id = $1",
                &[&event.election_event_id]
            )
            .await
            .unwrap()
            .get::<_, i64>(0),
            0
        );
        assert!(initialize_new_event(&tx, "invalid", &event_id)
            .await
            .is_err());
        assert!(initialize_new_event(&tx, &tenant, "invalid").await.is_err());
        tx.rollback().await.unwrap();
        assert!(get_board_client()
            .await
            .unwrap()
            .query(&database.board, &LogQuery::default())
            .await
            .unwrap()
            .is_empty());
        database.drop().await;
    });
}
