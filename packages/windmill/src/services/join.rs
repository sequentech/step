// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use csv::ReaderBuilder;
use std::{cmp::Ordering, fs::File};

use tracing::{info, instrument};

use anyhow::{anyhow, Result};

#[instrument(skip_all, err)]
pub fn merge_join_csv(
    ballots_file: &File,
    voters_file: &File,
    ballots_voter_id_index: usize,
    voters_id_index: usize,
    ballots_content_index: usize,
    ballots_election_id_index: usize,
    election_id: &str,
) -> Result<(Vec<String>, u64, u64, u64)> {
    info!("START merge_join_csv election_id: {election_id}");

    // Initialize the result vector
    let mut result = Vec::new();
    let mut ballots_without_voter = 0;
    let mut elegible_voters = 0;
    let mut casted_ballots = 0;

    // Assume the CSV files do not have headers.
    let mut ballots_reader = ReaderBuilder::new()
        .has_headers(false)
        .from_reader(ballots_file);
    let mut voters_reader = ReaderBuilder::new()
        .has_headers(false)
        .from_reader(voters_file);

    // Create iterators over CSV records.
    let mut ballots_iterator = ballots_reader.records();
    let mut voters_iterator = voters_reader.records();

    // Read the first record from each file.
    let mut ballots_record = ballots_iterator.next();
    let mut voters_record = voters_iterator.next();

    // Continue while both files still have records.
    while ballots_record.is_some() && voters_record.is_some() {
        // Unwrap the current records.
        let ballot = ballots_record
            .as_ref()
            .and_then(|res| res.as_ref().ok())
            .ok_or(anyhow!("Could not unwrap record"))?;
        let voter = voters_record
            .as_ref()
            .and_then(|res| res.as_ref().ok())
            .ok_or(anyhow!("Could not unwrap record"))?;

        // Extract the join keys.
        let Some(ballot_voter_id) = ballot.get(ballots_voter_id_index) else {
            // Advance file1.
            ballots_record = ballots_iterator.next();
            continue;
        };

        // Extract the join keys.
        let Some(voter_id) = voter.get(voters_id_index) else {
            elegible_voters = elegible_voters + 1;
            // Advance file1.
            voters_record = voters_iterator.next();
            continue;
        };

        let Some(ballot_election_id) = ballot.get(ballots_election_id_index) else {
            // Advance file1.
            ballots_record = ballots_iterator.next();
            continue;
        };

        if ballot_election_id == election_id {
            casted_ballots = casted_ballots + 1;
        } else {
            // Advance file1.
            ballots_record = ballots_iterator.next();
            continue;
        }

        // Compare the join keys lexicographically.
        match ballot_voter_id.cmp(&voter_id) {
            Ordering::Less => {
                // If the ballot has no voter.
                ballots_without_voter = ballots_without_voter + 1;
                // Advance file1.
                ballots_record = ballots_iterator.next();
            }
            Ordering::Greater => {
                // Advance file2.
                voters_record = voters_iterator.next();
            }
            Ordering::Equal => {
                let ballot_content = ballot.get(ballots_content_index).ok_or_else(|| {
                    anyhow!(
                        "Output column index {} out of bounds in file1",
                        ballots_content_index
                    )
                })?;

                result.push(ballot_content.to_string());

                // Advance both iterators.
                ballots_record = ballots_iterator.next();
                voters_record = voters_iterator.next();
            }
        }
    }

    info!("ballots_to_be_tallied: {}, elegible_voters: {}, ballots_without_voter: {}, casted_ballots: {}", result.len(), elegible_voters, ballots_without_voter, casted_ballots);

    Ok((
        result,
        elegible_voters,
        ballots_without_voter,
        casted_ballots,
    ))
}
