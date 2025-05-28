// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use csv::ReaderBuilder;
use std::{cmp::Ordering, fs::File};

use tracing::{info, instrument};

use anyhow::{anyhow, Result};

#[instrument(skip_all, err)]
pub fn merge_join_csv(
    file1: &File,
    file2: &File,
    file1_join_index: usize,
    file2_join_index: usize,
    file1_output_index: usize,
    ballot_election_id_index: usize,
    election_id: &str,
) -> Result<Vec<String>> {
    // Initialize the result vector
    let mut result = Vec::new();

    // Assume the CSV files do not have headers.
    let mut rdr1 = ReaderBuilder::new().has_headers(false).from_reader(file1);
    let mut rdr2 = ReaderBuilder::new().has_headers(false).from_reader(file2);

    // Create iterators over CSV records.
    let mut iter1 = rdr1.records();
    let mut iter2 = rdr2.records();

    // Read the first record from each file.
    let mut rec1_opt = iter1.next();
    let mut rec2_opt = iter2.next();

    // Continue while both files still have records.
    while rec1_opt.is_some() && rec2_opt.is_some() {
        // Unwrap the current records.
        let rec1 = rec1_opt
            .as_ref()
            .and_then(|res| res.as_ref().ok())
            .ok_or(anyhow!("Could not unwrap record"))?;
        let rec2 = rec2_opt
            .as_ref()
            .and_then(|res| res.as_ref().ok())
            .ok_or(anyhow!("Could not unwrap record"))?;

        // Extract the join keys.
        let Some(key1) = rec1.get(file1_join_index) else {
            // Advance file1.
            rec1_opt = iter1.next();
            continue;
        };

        // Extract the join keys.
        let Some(key2) = rec2.get(file2_join_index) else {
            // Advance file1.
            rec2_opt = iter2.next();
            continue;
        };

        let Some(ballot_election_id) = rec1.get(ballot_election_id_index) else {
            // Advance file1.
            rec1_opt = iter1.next();
            continue;
        };

        if ballot_election_id != election_id {
            // Advance file1.
            rec1_opt = iter1.next();
            continue;
        }

        // Compare the join keys lexicographically.
        match key1.cmp(&key2) {
            Ordering::Less => {
                // Advance file1.
                rec1_opt = iter1.next();
            }
            Ordering::Greater => {
                // Advance file2.
                rec2_opt = iter2.next();
            }
            Ordering::Equal => {
                let value = rec1.get(file1_output_index).ok_or_else(|| {
                    anyhow!(
                        "Output column index {} out of bounds in file1",
                        file1_output_index
                    )
                })?;

                result.push(value.to_string());

                // Advance both iterators.
                rec1_opt = iter1.next();
                rec2_opt = iter2.next();
            }
        }
    }

    Ok(result)
}

#[instrument(skip_all, err)]
pub fn count_unique_csv(
    file1: &File,
    file2: &File,
    file1_join_index: usize,
    file2_join_index: usize,
    ballot_election_id_index: usize,
    election_id: &str,
) -> Result<usize> {
    // Initialize the result vector
    let mut count = 0;

    // Assume the CSV files do not have headers.
    let mut rdr1 = ReaderBuilder::new().has_headers(false).from_reader(file1);
    let mut rdr2 = ReaderBuilder::new().has_headers(false).from_reader(file2);

    // Create iterators over CSV records.
    let mut iter1 = rdr1.records();
    let mut iter2 = rdr2.records();

    // Read the first record from each file.
    let mut rec1_opt = iter1.next();
    let mut rec2_opt = iter2.next();

    // Continue while both files still have records.
    while rec1_opt.is_some() && rec2_opt.is_some() {
        // Unwrap the current records.
        let rec1 = rec1_opt
            .as_ref()
            .and_then(|res| res.as_ref().ok())
            .ok_or(anyhow!("Could not unwrap record"))?;
        let rec2 = rec2_opt
            .as_ref()
            .and_then(|res| res.as_ref().ok())
            .ok_or(anyhow!("Could not unwrap record"))?;

        // Extract the join keys.
        let Some(key1) = rec1.get(file1_join_index) else {
            count = count + 1;
            // Advance file1.
            rec1_opt = iter1.next();
            continue;
        };

        // Extract the join keys.
        let Some(key2) = rec2.get(file2_join_index) else {
            count = count + 1;
            // Advance file1.
            rec2_opt = iter2.next();
            continue;
        };

        let Some(ballot_election_id) = rec1.get(ballot_election_id_index) else {
            count = count + 1;
            // Advance file1.
            rec1_opt = iter1.next();
            continue;
        };

        if ballot_election_id != election_id {
            count = count + 1;
            // Advance file1.
            rec1_opt = iter1.next();
            continue;
        }

        // Compare the join keys lexicographically.
        match key1.cmp(&key2) {
            Ordering::Less => {
                count = count + 1;
                // Advance file1.
                rec1_opt = iter1.next();
            }
            Ordering::Greater => {
                // Advance file2.
                rec2_opt = iter2.next();
            }
            Ordering::Equal => {
                // Advance both iterators.
                rec1_opt = iter1.next();
                rec2_opt = iter2.next();
            }
        }
    }

    Ok(count)
}
