// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use chrono::{NaiveDate, Utc};
use clap::Args;
use colored::Colorize;
use csv::Writer;
use rand::Rng;
use sequent_core::util::external_config::GenerateVoters as VotersConfig;
use serde_json::Value;
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::PathBuf;

use crate::domain::generate_voters::{csv_file_name, output_columns, voter_record, ElectionIndex};
use crate::utils::read_config::load_external_config;

const PROGRESS_INTERVAL: usize = 10_000;

#[derive(Args)]
#[command(about)]
pub struct GenerateVoters {
    /// Working directory for input/output
    #[arg(long)]
    working_directory: String,

    #[arg(long)]
    num_users: usize,
}

impl GenerateVoters {
    /// Execute the rendering process
    pub fn run(&self) {
        match self.run_generate_voters(&self.working_directory, self.num_users) {
            Ok(_) => println!("{}", "Successfully generated voters into csv".green()),
            Err(err) => eprintln!("Error! Failed to generate voters: {err:?}"),
        }
    }

    fn run_generate_voters(
        &self,
        working_dir: &str,
        num_users: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let config = load_external_config(working_dir)?;

        // Get election event file path from config (or default).
        let election_event_file = config.election_event_json_file;
        let election_event_path = PathBuf::from(working_dir).join(election_event_file);
        let election_file = File::open(election_event_path)?;
        let election_data: Value = serde_json::from_reader(BufReader::new(election_file))?;

        let voters_config = config.generate_voters;
        let csv_file_path =
            PathBuf::from(working_dir).join(csv_file_name(&voters_config, num_users));
        let index = ElectionIndex::from_event(&election_data);

        let mut wtr = Writer::from_path(&csv_file_path)?;
        write_voters(
            &mut wtr,
            &index,
            &voters_config,
            num_users,
            &mut rand::rng(),
            || Utc::now().date_naive(),
        )?;

        println!(
            "Successfully generated {} users. CSV file created at: {}",
            num_users,
            csv_file_path.canonicalize()?.display()
        );
        Ok(())
    }
}

/// Writes the header and one record per voter, then flushes. `today` is read
/// again for every voter.
fn write_voters<W: Write>(
    wtr: &mut Writer<W>,
    index: &ElectionIndex,
    cfg: &VotersConfig,
    num_users: usize,
    rng: &mut impl Rng,
    today: impl Fn() -> NaiveDate,
) -> Result<(), Box<dyn std::error::Error>> {
    wtr.write_record(output_columns(cfg))?;
    for i in 0..num_users {
        let record = voter_record(i, index.area(i), index, cfg, rng, today());
        wtr.write_record(&record)?;

        // Optionally, log progress every so often rather than every record.
        let completed = i + 1;
        if completed % PROGRESS_INTERVAL == 0 {
            println!("Generated {} users...", completed);
        }
    }
    wtr.flush()?;
    Ok(())
}

#[cfg(test)]
#[path = "../../tests/support/voter_csv_boundaries.rs"]
mod boundary_tests;

#[cfg(test)]
mod username_start_regression {
    use super::*;
    use serde_json::json;

    #[test]
    fn generated_csv_honors_the_start_number_and_keeps_the_legacy_default() {
        for (start, expected) in [
            (Some(100), "username\n100\n101\n"),
            (None, "username\n0\n1\n"),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let mut config = json!({
                "election_event_json_file": "event.json", "realm_name": "synthetic",
                "tenant_id": "tenant", "election_event_id": "event", "area_id": "area", "election_id": "election",
                "generate_voters": {
                    "csv_file_name": "voters", "fields": ["username"], "excluded_columns": [],
                    "email_prefix": "voter", "domain": "example.test", "sequence_email_number": true,
                    "sequence_start_number": 0, "voter_password": "test", "password_salt": "salt",
                    "hashed_password": "hash", "overseas_reference": "B", "min_age": 18,
                    "max_age": 90, "authorized_elections_count": 0, "email_verified": true
                },
                "duplicate_votes": {"row_id_to_clone": "row"},
                "generate_applications": {"applicant_data": {}, "annotations": {}}
            });
            if let Some(start) = start {
                config["generate_voters"]["username_start_number"] = json!(start);
            }
            std::fs::write(dir.path().join("external_config.json"), config.to_string()).unwrap();
            std::fs::write(dir.path().join("event.json"), "{}").unwrap();
            let command = GenerateVoters {
                working_directory: dir.path().to_str().unwrap().into(),
                num_users: 2,
            };
            command
                .run_generate_voters(&command.working_directory, 2)
                .unwrap();
            assert_eq!(
                std::fs::read_to_string(dir.path().join("voters_2.csv")).unwrap(),
                expected
            );
        }
    }
    #[test]
    fn generated_csv_uses_mapper_external_keys_with_null_and_empty_id_fallbacks() {
        let dir = tempfile::tempdir().unwrap();
        let config = json!({
            "election_event_json_file":"event.json", "realm_name":"synthetic", "tenant_id":"tenant",
            "election_event_id":"event", "area_id":"area", "election_id":"election",
            "generate_voters":{"csv_file_name":"voters","fields":["area_name","authorized-election-ids"],
                "excluded_columns":[],"email_prefix":"voter","domain":"example.test","sequence_email_number":true,
                "sequence_start_number":0,"voter_password":"test","password_salt":"salt","hashed_password":"hash",
                "overseas_reference":"B","min_age":18,"max_age":90,"authorized_elections_count":0,"email_verified":true},
            "duplicate_votes":{"row_id_to_clone":"row"},"generate_applications":{"applicant_data":{},"annotations":{}}
        });
        let event = json!({
            "areas":[{"id":"a","name":"Eligible"},{"id":"b","name":"No elections"}],
            "elections":[{"id":"db-1","external_id":"external-1"},{"id":"db-2","external_id":null},
                {"id":"db-3","external_id":""},{"id":"db-4"}],
            "contests":[{"id":"c1","election_id":"db-1"},{"id":"c2","election_id":"db-2"},
                {"id":"c3","election_id":"db-3"},{"id":"c4","election_id":"db-4"}],
            "area_contests":[{"area_id":"a","contest_id":"c1"},{"area_id":"a","contest_id":"c2"},
                {"area_id":"a","contest_id":"c1"},{"area_id":"a","contest_id":"c3"},{"area_id":"a","contest_id":"c4"}]
        });
        std::fs::write(dir.path().join("external_config.json"), config.to_string()).unwrap();
        std::fs::write(dir.path().join("event.json"), event.to_string()).unwrap();
        let command = GenerateVoters {
            working_directory: dir.path().to_str().unwrap().into(),
            num_users: 2,
        };
        command
            .run_generate_voters(&command.working_directory, 2)
            .unwrap();
        assert_eq!(std::fs::read_to_string(dir.path().join("voters_2.csv")).unwrap(),
            "area_name,authorized-election-ids\nEligible,external-1|db-2|db-3|db-4\nNo elections,Unknown\n");
    }
}
