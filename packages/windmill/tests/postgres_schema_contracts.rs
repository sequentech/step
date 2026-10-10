// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The migrated-schema fixture the PostgreSQL adapter tests build on.

#[path = "support/schema.rs"]
mod schema;

#[tokio::test]
async fn creation_lock_protects_a_database_until_its_anchor_is_connected() {
    use deadpool_postgres::Runtime;
    use std::time::Duration;
    use tokio_postgres::NoTls;
    use windmill::services::database::PgConfig;

    let mut config = PgConfig::from_env().unwrap().hasura_db;
    config.dbname = Some("postgres".into());
    let inspector_pool = config.create_pool(Some(Runtime::Tokio1), NoTls).unwrap();
    let inspector = inspector_pool.get().await.unwrap();
    let (database, admin) = schema::create_database().await;
    let mut competing = tokio::spawn(schema::create_database());

    let premature = tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            tokio::select! {
                result = &mut competing => break Some(result.unwrap()),
                row = inspector.query_one(
                    "SELECT EXISTS (SELECT 1 FROM pg_locks
                     WHERE locktype = 'advisory' AND NOT granted
                     AND database = (SELECT oid FROM pg_database WHERE datname = 'postgres')
                     AND classid = ((hashtextextended('windmill_schema_', 0) >> 32)
                                    & 4294967295)::oid
                     AND objid = (hashtextextended('windmill_schema_', 0)
                                  & 4294967295)::oid)",
                    &[],
                ) => {
                    if row.unwrap().get::<_, bool>(0) {
                        break None;
                    }
                }
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("competing initializer either waits for the lock or finishes");

    if let Some((other, other_admin)) = premature {
        let exists: bool = inspector
            .query_one(
                "SELECT EXISTS (SELECT 1 FROM pg_database WHERE datname = $1)",
                &[&database],
            )
            .await
            .unwrap()
            .get(0);
        inspector
            .batch_execute(&format!("DROP DATABASE \"{other}\""))
            .await
            .unwrap();
        drop(other_admin);
        panic!("competing cleanup ran before the anchor; original database exists: {exists}");
    }

    schema::anchor(&database).await;
    drop(admin);
    let (other, other_admin) = tokio::time::timeout(Duration::from_secs(30), competing)
        .await
        .expect("competing initializer resumes after the anchor is ready")
        .unwrap();
    inspector
        .batch_execute(&format!("DROP DATABASE \"{other}\""))
        .await
        .unwrap();
    drop(other_admin);

    let mut config = PgConfig::from_env().unwrap().hasura_db;
    config.dbname = Some(database);
    let pool = config.create_pool(Some(Runtime::Tokio1), NoTls).unwrap();
    let connection = pool
        .get()
        .await
        .expect("competing cleanup preserves the anchored database");
    assert_eq!(
        connection
            .query_one("SELECT 42", &[])
            .await
            .unwrap()
            .get::<_, i32>(0),
        42
    );
}

#[tokio::test]
async fn every_backend_migration_builds_the_schema_the_adapters_query() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    for table in [
        "tenant",
        "election_event",
        "election",
        "contest",
        "candidate",
        "area",
        "ballot_publication",
        "ballot_style",
        "cast_vote",
        "keys_ceremony",
        "tally_session",
        "tally_session_execution",
        "scheduled_event",
        "tasks_execution",
        "election_voting_window",
    ] {
        let row = transaction
            .query_one(
                "SELECT to_regclass('sequent_backend.' || $1) IS NOT NULL",
                &[&table],
            )
            .await
            .unwrap();
        assert!(row.get::<_, bool>(0), "missing table {table}");
    }
    transaction.rollback().await.unwrap();
}
