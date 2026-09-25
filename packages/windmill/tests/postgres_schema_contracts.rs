// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The migrated-schema fixture the PostgreSQL adapter tests build on.

#[path = "support/schema.rs"]
mod schema;

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
