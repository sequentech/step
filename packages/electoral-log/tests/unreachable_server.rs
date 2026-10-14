// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The log databases of a server that is down: names are derived without it,
//! and every operation that needs it fails instead of answering as if a log
//! were empty. This binary points `ELECTORAL_LOG_PG_*` at a port nothing
//! listens on, so it holds this one test.

use electoral_log::adapters::events::{
    event_of_board, owner_of_board, tenant_of_board, BoardOwner, EventDatabases,
};
use electoral_log::ports::ElectoralLogStore;
use electoral_log::{ElectoralLogMessage, LogEntry, LogQuery};

const TENANT: &str = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";
const EVENT: &str = "0d1b2c3d-4e5f-4071-8293-a4b5c6d7e8f9";
const TENANT_BOARD: &str = "devtenant90505c8a23a94cdfa26b4e19f6a097d5";
const EVENT_BOARD: &str = "devtenant90505c8a23a94cdfaevent0d1b2c3d4e5f40718293a4b5c6d7e8f9";

fn entry() -> LogEntry {
    LogEntry {
        delivery_id: "delivery".into(),
        message: ElectoralLogMessage {
            id: 0,
            created: 1,
            sender_pk: "sender".into(),
            statement_timestamp: 1,
            statement_kind: "AdminChange".into(),
            message: vec![0],
            version: "1".into(),
            user_id: None,
            username: None,
            election_id: None,
            area_id: None,
            ballot_id: None,
        },
    }
}

#[tokio::test]
async fn a_server_that_is_down_names_its_databases_and_fails_everything_else() {
    for (name, value) in [
        ("ELECTORAL_LOG_PG_HOST", "127.0.0.1"),
        ("ELECTORAL_LOG_PG_PORT", "1"),
        ("ELECTORAL_LOG_PG_USER", "nobody"),
        ("ELECTORAL_LOG_PG_PASSWORD", "nothing"),
        ("ELECTORAL_LOG_PG_DATABASE", "electoral_log"),
        ("ELECTORAL_LOG_PG_SSLMODE", "disable"),
    ] {
        std::env::set_var(name, value);
    }
    for name in [
        "ELECTORAL_LOG_PG_READER_USER",
        "ELECTORAL_LOG_PG_BACKUP_USER",
        "ELECTORAL_LOG_PG_PROVISIONING_USER",
    ] {
        std::env::remove_var(name);
    }
    let databases = EventDatabases::from_env().unwrap();

    // Names need no server.
    assert_eq!(
        databases.database_name(EVENT).unwrap(),
        "electoral_log_0d1b2c3d4e5f40718293a4b5c6d7e8f9"
    );
    let tenant_database = databases.tenant_database_name(TENANT).unwrap();
    assert_eq!(
        tenant_database,
        "electoral_log_t90505c8a23a94cdfa26b4e19f6a097d5"
    );
    let retired = databases
        .retired_database_name(EVENT, 1_791_000_000)
        .unwrap();
    assert_eq!(
        retired,
        "electoral_log_r1791000000_0d1b2c3d4e5f40718293a4b5c6d7e8f9"
    );
    assert!(databases.is_tenant_database(&tenant_database));
    assert!(!databases.is_event_database(&tenant_database));
    assert!(!databases.is_tenant_database(&retired));
    assert!(!databases.is_event_database(&retired));
    for invalid in ["", "tenant", "90505c8a"] {
        assert!(databases.tenant_database_name(invalid).is_err());
        assert!(databases.retired_database_name(invalid, 1).is_err());
    }
    assert_eq!(tenant_of_board(TENANT_BOARD).unwrap(), TENANT);
    assert_eq!(event_of_board(EVENT_BOARD).unwrap(), EVENT);
    assert_eq!(
        owner_of_board(TENANT_BOARD).unwrap(),
        BoardOwner::Tenant(TENANT.into())
    );
    assert_eq!(
        owner_of_board(EVENT_BOARD).unwrap(),
        BoardOwner::Event(EVENT.into())
    );
    assert!(owner_of_board("board").is_err());

    // Everything else needs the server, and says so.
    assert!(databases.initialize().await.is_err());
    assert!(databases.create_tenant(TENANT).await.is_err());
    assert!(databases.tenant(TENANT).await.is_err());
    assert!(databases.tenants().await.is_err());
    assert!(databases.tenant_store(TENANT).await.is_err());
    assert!(databases.retire_event(TENANT, EVENT, 1).await.is_err());
    assert!(databases.retired_events(TENANT).await.is_err());
    assert!(databases.create_event(TENANT, EVENT).await.is_err());
    assert!(databases.event(EVENT).await.is_err());
    assert!(databases.events().await.is_err());
    assert!(databases.has_event(EVENT).await.is_err());
    assert!(databases.store(EVENT).await.is_err());
    assert!(databases.drop_event(TENANT, EVENT).await.is_err());
    assert!(databases.apply_schema(EVENT).await.is_err());
    assert!(databases.board_store(TENANT_BOARD).await.is_err());
    assert!(databases.board_store(EVENT_BOARD).await.is_err());
    assert!(databases.board_store("board").await.is_err());
    for board in [TENANT_BOARD, EVENT_BOARD] {
        assert!(databases.create_board(board).await.is_err());
        assert!(databases.has_board(board).await.is_err());
        assert!(databases.delete_board(board).await.is_err());
        assert!(databases.query(board, &LogQuery::default()).await.is_err());
        assert!(databases.count(board, &LogQuery::default()).await.is_err());
        let mut entries = std::iter::once(Ok(entry()));
        assert!(databases.append(board, &mut entries).await.is_err());
    }
    // An invalid ID is refused before the server is asked.
    assert!(databases.create_tenant("tenant").await.is_err());
    assert!(databases.retire_event("tenant", EVENT, 1).await.is_err());
}
