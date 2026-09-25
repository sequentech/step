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
        if i % PROGRESS_INTERVAL == 0 {
            println!("Generated {} users...", i);
        }
    }
    wtr.flush()?;
    Ok(())
}
