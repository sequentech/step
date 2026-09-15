// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Exercise the real SQLite writer, including its transaction boundary. A
//! reported failure must not leave a newly committed partial results event.

use rusqlite::{params, Connection};
use std::fs;
use tempfile::tempdir;
use velvet::pipes::generate_db::{
    populate_results_tables, process_decoded_ballots, PipeConfigGenerateDatabase,
};
use velvet::pipes::generate_reports::ElectionReportDataComputed;

#[derive(Debug, PartialEq)]
struct StoredElection {
    tenant: String,
    event: String,
    election: String,
    census: i64,
    voters: i64,
    participation: f64,
    blank_ballots: i64,
    blank_fraction: f64,
}

fn config() -> PipeConfigGenerateDatabase {
    PipeConfigGenerateDatabase {
        include_decoded_ballots: false,
        tenant_id: "test-tenant".into(),
        election_event_id: "test-event".into(),
        database_filename: "results.db".into(),
    }
}

fn election() -> ElectionReportDataComputed {
    ElectionReportDataComputed {
        election_id: "council".into(),
        area: None,
        census: 20,
        total_votes: 10,
        blank_ballots: Some(2),
        reports: vec![],
    }
}

#[test]
fn a_new_results_database_persists_the_event_election_and_correct_denominators() {
    let input = tempdir().unwrap();
    let output = tempdir().unwrap();
    populate_results_tables(input.path(), output.path(), vec![election()], &config()).unwrap();

    let database = Connection::open(output.path().join("results.db")).unwrap();
    let stored = database
        .query_row(
            "SELECT tenant_id, election_event_id, election_id, elegible_census,
                total_voters, total_voters_percent, blank_ballots, blank_ballots_percent
         FROM results_election",
            [],
            |row| {
                Ok(StoredElection {
                    tenant: row.get("tenant_id")?,
                    event: row.get("election_event_id")?,
                    election: row.get("election_id")?,
                    census: row.get("elegible_census")?,
                    voters: row.get("total_voters")?,
                    participation: row.get("total_voters_percent")?,
                    blank_ballots: row.get("blank_ballots")?,
                    blank_fraction: row.get("blank_ballots_percent")?,
                })
            },
        )
        .unwrap();
    assert_eq!(
        stored,
        StoredElection {
            tenant: "test-tenant".into(),
            event: "test-event".into(),
            election: "council".into(),
            census: 20,
            voters: 10,
            participation: 0.5,
            blank_ballots: 2,
            blank_fraction: 0.2,
        }
    );

    // Result rows must reference the event committed in the same transaction.
    let joined: i64 = database.query_row(
        "SELECT count(*) FROM results_election AS election
         JOIN results_event AS event ON election.results_event_id = event.id
         WHERE election.tenant_id = event.tenant_id AND election.election_event_id = event.election_event_id",
        [], |row| row.get(0),
    ).unwrap();
    assert_eq!(joined, 1);
}

#[test]
fn a_database_write_failure_rolls_back_the_partial_results_event() {
    let input = tempdir().unwrap();
    let output = tempdir().unwrap();
    let source = Connection::open(input.path().join("results.db")).unwrap();
    // An incompatible copied schema forces a real SQL error after the results
    // event has been inserted. An early validation error would miss this bug.
    source
        .execute_batch(
            "CREATE TABLE results_election (unexpected TEXT);
                          INSERT INTO results_election VALUES ('keep this');",
        )
        .unwrap();
    drop(source);

    let error = populate_results_tables(input.path(), output.path(), vec![election()], &config());
    assert!(error.is_err(), "the incompatible schema must fail");
    let output_database = Connection::open(output.path().join("results.db")).unwrap();
    let partial_event_tables: i64 = output_database
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'results_event'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        partial_event_tables, 0,
        "failure must roll back the newly created event table and its row"
    );

    let preserved: String = output_database
        .query_row("SELECT unexpected FROM results_election", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(preserved, "keep this");
    let source_database = Connection::open(input.path().join("results.db")).unwrap();
    assert_eq!(
        source_database
            .query_row::<String, _, _>("SELECT unexpected FROM results_election", [], |row| row
                .get(0))
            .unwrap(),
        "keep this"
    );
}

#[tokio::test]
async fn decoded_ballots_preserve_bytes_and_replace_only_the_matching_area() {
    let directory = tempdir().unwrap();
    let north = directory
        .path()
        .join("election__e1/contest__c1/area__north");
    let south = directory
        .path()
        .join("election__e1/contest__c1/area__south");
    fs::create_dir_all(&north).unwrap();
    fs::create_dir_all(&south).unwrap();
    fs::write(north.join("decoded_ballots.json"), b"[ 1, 2 ]\n").unwrap();
    fs::write(south.join("decoded_ballots.json"), b"[]").unwrap();
    fs::write(south.join("unrelated.txt"), b"not a ballot file").unwrap();

    let mut database = Connection::open_in_memory().unwrap();
    let transaction = database.transaction().unwrap();
    process_decoded_ballots(&transaction, directory.path())
        .await
        .unwrap();
    transaction.commit().unwrap();
    let read = |database: &Connection, area: &str| {
        database.query_row::<Vec<u8>, _, _>(
        "SELECT decoded_ballot_json FROM ballot WHERE election_id = ?1 AND contest_id = ?2 AND area_id = ?3",
        params!["e1", "c1", area], |row| row.get(0),
    ).unwrap()
    };
    assert_eq!(read(&database, "north"), b"[ 1, 2 ]\n");
    assert_eq!(read(&database, "south"), b"[]");

    fs::write(north.join("decoded_ballots.json"), b"[3]").unwrap();
    let transaction = database.transaction().unwrap();
    process_decoded_ballots(&transaction, directory.path())
        .await
        .unwrap();
    transaction.commit().unwrap();
    assert_eq!(read(&database, "north"), b"[3]");
    assert_eq!(read(&database, "south"), b"[]");
    assert_eq!(
        database
            .query_row::<i64, _, _>("SELECT count(*) FROM ballot", [], |row| row.get(0))
            .unwrap(),
        2
    );
}

#[tokio::test]
async fn decoded_ballots_without_an_area_fail_and_can_be_rolled_back() {
    let directory = tempdir().unwrap();
    let incomplete = directory.path().join("election__e1/contest__c1");
    fs::create_dir_all(&incomplete).unwrap();
    fs::write(incomplete.join("decoded_ballots.json"), b"[]").unwrap();
    let mut database = Connection::open_in_memory().unwrap();
    let transaction = database.transaction().unwrap();
    assert!(process_decoded_ballots(&transaction, directory.path())
        .await
        .is_err());
    transaction.rollback().unwrap();

    let tables: i64 = database
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE name = 'ballot'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(tables, 0);
}
