// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// This file contains unit tests for the generate_database module.
// The tests use an in-memory SQLite database and a temporary directory
// to simulate the file system, allowing for isolated and deterministic testing.

// We use the `#[cfg(test)]` attribute to ensure this module is only compiled
// when running tests.
#[cfg(test)]
mod tests {
    // Import necessary modules from the parent file.
    // The `super::*` allows us to access all public items from the main module.
    use super::*;
    use anyhow::Result;
    use rusqlite::Connection;
    use sequent_core::types::{
        hasura::{
            core::{Area, Contest, Election, ElectionType, TallySession, VotingType},
            enums::{CountingAlgorithm, TallySessionStatus},
        },
        report::{
            CandidateReportData, ContestReportDataComputed, ElectionReportDataComputed,
            InvalidVotesCount,
        },
    };
    use std::fs;
    use tempfile::tempdir;
    use uuid::Uuid;

    // A helper function to create an in-memory SQLite connection for testing.
    fn setup_database() -> Result<Connection> {
        let conn = Connection::open_in_memory()?;
        // The `rusqlite::Transaction` requires an existing database connection.
        Ok(conn)
    }

    // A simple mock for the data types needed by the tests.
    // In a real scenario, you might want to create a more robust mock data builder.
    fn mock_report_data() -> Vec<ElectionReportDataComputed> {
        let election_id_1 = Uuid::new_v4().to_string();
        let election_id_2 = Uuid::new_v4().to_string();
        let contest_id_1 = Uuid::new_v4().to_string();
        let contest_id_2 = Uuid::new_v4().to_string();
        let candidate_id_1 = Uuid::new_v4().to_string();
        let area_id_1 = Uuid::new_v4().to_string();

        let election_1 = ElectionReportDataComputed {
            election_id: election_id_1.clone(),
            census: 1000,
            total_votes: 500,
            reports: vec![
                ContestReportDataComputed {
                    contest: Contest {
                        id: contest_id_1.clone(),
                        ..Default::default()
                    },
                    candidate_result: vec![
                        CandidateReportData {
                            candidate: sequent_core::types::hasura::core::Candidate {
                                id: candidate_id_1.clone(),
                                ..Default::default()
                            },
                            total_count: 300,
                            ..Default::default()
                        },
                    ],
                    area: Some(Area {
                        id: area_id_1.clone(),
                        ..Default::default()
                    }),
                    contest_result: sequent_core::types::report::ContestResult {
                        census: 500,
                        total_votes: 250,
                        total_valid_votes: 240,
                        auditable_votes: 240,
                        total_invalid_votes: 5,
                        total_blank_votes: 5,
                        invalid_votes: InvalidVotesCount {
                            explicit: 3,
                            implicit: 2,
                        },
                        ..Default::default()
                    },
                },
            ],
            ..Default::default()
        };

        let election_2 = ElectionReportDataComputed {
            election_id: election_id_2.clone(),
            census: 2000,
            total_votes: 1500,
            reports: vec![
                ContestReportDataComputed {
                    contest: Contest {
                        id: contest_id_2.clone(),
                        ..Default::default()
                    },
                    candidate_result: vec![
                        CandidateReportData {
                            candidate: sequent_core::types::hasura::core::Candidate {
                                id: Uuid::new_v4().to_string(),
                                ..Default::default()
                            },
                            total_count: 1000,
                            ..Default::default()
                        },
                    ],
                    area: None, // This contest has no area
                    contest_result: sequent_core::types::report::ContestResult {
                        census: 1500,
                        total_votes: 750,
                        total_valid_votes: 700,
                        auditable_votes: 700,
                        total_invalid_votes: 25,
                        total_blank_votes: 25,
                        invalid_votes: InvalidVotesCount {
                            explicit: 15,
                            implicit: 10,
                        },
                        ..Default::default()
                    },
                },
            ],
            ..Default::default()
        };
        vec![election_1, election_2]
    }


    #[test]
    // Test case for successful extraction of IDs from a valid file path.
    fn test_extract_ids_from_path_success() -> Result<()> {
        let path = PathBuf::from(
            "/tmp/test_dir/election__e123/contest__c456/area__a789/decoded_ballots.json",
        );
        let (election_id, contest_id, area_id) = extract_ids_from_path(&path)?;

        assert_eq!(election_id, "e123");
        assert_eq!(contest_id, "c456");
        assert_eq!(area_id, "a789");

        Ok(())
    }

    #[test]
    // Test case for failed extraction when a file path is missing a required ID component.
    fn test_extract_ids_from_path_failure() {
        let path = PathBuf::from("/tmp/test_dir/contest__c456/area__a789/decoded_ballots.json");
        let result = extract_ids_from_path(&path);

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Could not extract all required IDs (election, contest, area) from path: \"/tmp/test_dir/contest__c456/area__a789/decoded_ballots.json\""
        );
    }

    #[tokio::test]
    // Test case for the `process_decoded_ballots` function.
    // This test creates a temporary directory structure and verifies that the data is correctly inserted into the in-memory database.
    async fn test_process_decoded_ballots() -> Result<()> {
        let dir = tempdir()?;
        let root_path = dir.path();
        let decoded_ballots_path = root_path.join("decoded_ballots");

        // Create a mock directory structure with UUIDs for each part.
        let election_id = Uuid::new_v4().to_string();
        let contest_id = Uuid::new_v4().to_string();
        let area_id = Uuid::new_v4().to_string();

        let ballot_dir = decoded_ballots_path
            .join(format!("election__{}", election_id))
            .join(format!("contest__{}", contest_id))
            .join(format!("area__{}", area_id));
        fs::create_dir_all(&ballot_dir)?;

        // Create the mock decoded_ballots.json file.
        let ballot_file_path = ballot_dir.join("decoded_ballots.json");
        let ballot_content = b"{\"ballot_id\": \"test_ballot\", \"votes\": [1, 2, 3]}";
        fs::write(&ballot_file_path, ballot_content)?;

        // Set up the in-memory database.
        let mut conn = setup_database()?;
        let tx = conn.transaction()?;

        // Call the function under test.
        process_decoded_ballots(&tx, &decoded_ballots_path).await?;
        tx.commit()?;

        // Verify the data was inserted correctly.
        let mut conn = setup_database()?;
        let result: String = conn.query_row(
            "SELECT decoded_ballot_json FROM ballot WHERE election_id = ? AND contest_id = ? AND area_id = ?",
            rusqlite::params![election_id, contest_id, area_id],
            |row| row.get(0),
        )?;

        // Check if the retrieved JSON matches the original content.
        assert_eq!(result.as_bytes(), ballot_content);

        Ok(())
    }

    #[tokio::test]
    // Test case for populating all results tables.
    // This test mocks the input data and verifies that the `save_results` function populates
    // all the expected tables with the correct number of entries.
    async fn test_populate_results_tables() -> Result<()> {
        // Set up the in-memory database.
        let mut conn = setup_database()?;
        let tx = conn.transaction()?;

        // Create the tables that `save_results` will insert into.
        tx.execute(
            "CREATE TABLE results_election (id TEXT PRIMARY KEY, tenant_id TEXT, election_event_id TEXT, election_id TEXT, results_event_id TEXT);",
            []
        )?;
        tx.execute(
            "CREATE TABLE results_contest (id TEXT PRIMARY KEY, tenant_id TEXT, election_event_id TEXT, election_id TEXT, contest_id TEXT, results_event_id TEXT);",
            []
        )?;
        tx.execute(
            "CREATE TABLE results_area_contest (id TEXT PRIMARY KEY, tenant_id TEXT, election_event_id TEXT, election_id TEXT, contest_id TEXT, area_id TEXT, results_event_id TEXT);",
            []
        )?;
        tx.execute(
            "CREATE TABLE results_contest_candidate (id TEXT PRIMARY KEY, tenant_id TEXT, election_event_id TEXT, election_id TEXT, contest_id TEXT, candidate_id TEXT, results_event_id TEXT);",
            []
        )?;
        tx.execute(
            "CREATE TABLE results_area_contest_candidate (id TEXT PRIMARY KEY, tenant_id TEXT, election_event_id TEXT, election_id TEXT, contest_id TEXT, area_id TEXT, candidate_id TEXT, results_event_id TEXT);",
            []
        )?;
        
        let mock_results = mock_report_data();
        let tenant_id = "test-tenant-1";
        let election_event_id = "test-event-1";
        let results_event_id = Uuid::new_v4().to_string();

        // Call the function under test.
        save_results(&tx, mock_results, tenant_id, election_event_id, &results_event_id).await?;
        tx.commit()?;

        // Check the number of rows in each table.
        // We expect one `results_election` entry for each election in the mock data.
        let election_count: i64 = conn.query_row("SELECT count(*) FROM results_election", [], |row| row.get(0))?;
        assert_eq!(election_count, 2);

        // We expect one `results_contest` entry for the contest with no area.
        let contest_count: i64 = conn.query_row("SELECT count(*) FROM results_contest", [], |row| row.get(0))?;
        assert_eq!(contest_count, 1);

        // We expect one `results_area_contest` entry for the contest with an area.
        let area_contest_count: i64 = conn.query_row("SELECT count(*) FROM results_area_contest", [], |row| row.get(0))?;
        assert_eq!(area_contest_count, 1);

        // We expect one `results_contest_candidate` entry for the contest with no area.
        let contest_candidate_count: i64 = conn.query_row("SELECT count(*) FROM results_contest_candidate", [], |row| row.get(0))?;
        assert_eq!(contest_candidate_count, 1);

        // We expect one `results_area_contest_candidate` entry for the contest with an area.
        let area_contest_candidate_count: i64 = conn.query_row("SELECT count(*) FROM results_area_contest_candidate", [], |row| row.get(0))?;
        assert_eq!(area_contest_candidate_count, 1);

        Ok(())
    }
}
