// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use csv::ReaderBuilder;
use std::{cmp::Ordering, fs::File};
use tracing::{event, info, instrument, Level};

#[instrument(skip_all, err)]
pub fn merge_join_csv(
    ballots_file: &File,
    voters_file: &File,
    ballots_voter_id_index: usize,
    voters_id_index: usize,
    ballots_content_index: usize,
) -> Result<(Vec<String>, u64, u64, u64)> {
    info!("START merge_join_csv");

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
        let Some(Ok(ballot)) = ballots_record.as_ref() else {
            ballots_record = ballots_iterator.next();
            continue;
        };
        let Some(Ok(voter)) = voters_record.as_ref() else {
            voters_record = voters_iterator.next();
            continue;
        };

        // Extract the join keys.
        let Some(ballot_voter_id) = ballot.get(ballots_voter_id_index) else {
            // Advance file1.
            ballots_record = ballots_iterator.next();
            continue;
        };
        // Ignore ballots with an empty key.
        if ballot_voter_id.is_empty() {
            ballots_record = ballots_iterator.next();
            continue;
        }

        // Extract the join keys.
        let Some(voter_id) = voter.get(voters_id_index) else {
            // Advance file1.
            voters_record = voters_iterator.next();
            continue;
        };

        // Ignore users with an empty key.
        if voter_id.is_empty() {
            voters_record = voters_iterator.next();
            continue;
        }

        // Compare the join keys lexicographically.
        match ballot_voter_id.cmp(&voter_id) {
            Ordering::Less => {
                // If the ballot has no voter.
                ballots_without_voter += 1;
                // Advance file1.
                ballots_record = ballots_iterator.next();
                casted_ballots += 1;
            }
            Ordering::Greater => {
                // Advance file2.
                voters_record = voters_iterator.next();
                elegible_voters += 1;
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
                casted_ballots += 1;
                voters_record = voters_iterator.next();
                elegible_voters += 1;
            }
        }
    }

    // Count the rest of the voters
    while voters_record.is_some() {
        elegible_voters += 1;
        voters_record = voters_iterator.next();
    }

    // Count the rest of the ballots
    while ballots_record.is_some() {
        casted_ballots += 1;
        ballots_without_voter += 1;
        ballots_record = ballots_iterator.next();
    }

    info!("ballots_to_be_tallied: {}, elegible_voters: {}, ballots_without_voter: {}, casted_ballots: {}", result.len(), elegible_voters, ballots_without_voter, casted_ballots);

    Ok((
        result,
        elegible_voters,
        ballots_without_voter,
        casted_ballots,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Helper function to run tests for `merge_join_csv`.
    fn run_merge_join_test(
        ballots_csv: &str,
        users_csv: &str,
    ) -> Result<(Vec<String>, u64, u64, u64)> {
        let mut ballots_file = NamedTempFile::new()?;
        write!(ballots_file, "{}", ballots_csv)?;
        ballots_file.flush()?;

        let mut users_file = NamedTempFile::new()?;
        write!(users_file, "{}", users_csv)?;
        users_file.flush()?;

        let ballots_ro = ballots_file.reopen()?;
        let users_ro = users_file.reopen()?;

        // Assumes standard test indexes:
        // join_index=0, output_index=2_index=1
        let (ballot_contents, elegible_voters, ballots_without_voter, casted_ballots) =
            merge_join_csv(&ballots_ro, &users_ro, 0, 0, 1)?;
        Ok((
            ballot_contents,
            elegible_voters,
            ballots_without_voter,
            casted_ballots,
        ))
    }

    #[test]
    fn test_basic_auditable_ballot() -> Result<()> {
        // user_C's ballot should be counted as auditable as they are not in the users file.
        let ballots = "user_A,content_A\nuser_B,content_B\nuser_C,content_C";
        let users = "user_A\nuser_B";
        let (_, elegible_voters, ballots_without_voter, casted_ballots) =
            run_merge_join_test(ballots, users)?;
        assert_eq!(elegible_voters, 2);
        assert_eq!(ballots_without_voter, 1);
        assert_eq!(casted_ballots, 3);
        Ok(())
    }

    #[test]
    fn test_no_auditable_ballots_all_match() -> Result<()> {
        // All users who voted are in the enabled users list.
        let ballots = "user_A,content_A\nuser_B,content_B";
        let users = "user_A\nuser_B";
        let (_, elegible_voters, ballots_without_voter, casted_ballots) =
            run_merge_join_test(ballots, users)?;
        assert_eq!(elegible_voters, 2);
        assert_eq!(ballots_without_voter, 0);
        assert_eq!(casted_ballots, 2);
        Ok(())
    }

    #[test]
    fn test_auditable_ballots_at_end_of_file() -> Result<()> {
        // This specifically tests the bug fix. user_C and user_D's ballots are after
        // the last user in the users file. The old buggy code would miss these.
        let ballots = "user_A,content_A\nuser_C,content_C\nuser_D,content_D";
        let users = "user_A\nuser_B";
        let (_, elegible_voters, ballots_without_voter, casted_ballots) =
            run_merge_join_test(ballots, users)?;
        assert_eq!(elegible_voters, 2);
        assert_eq!(ballots_without_voter, 2);
        assert_eq!(casted_ballots, 3);
        Ok(())
    }

    #[test]
    fn test_empty_ballot_file() -> Result<()> {
        // If there are no ballots, the count must be 0.
        let ballots = "";
        let users = "user_A\nuser_B";
        let (_, elegible_voters, ballots_without_voter, casted_ballots) =
            run_merge_join_test(ballots, users)?;
        assert_eq!(elegible_voters, 2);
        assert_eq!(ballots_without_voter, 0);
        assert_eq!(casted_ballots, 0);
        Ok(())
    }

    #[test]
    fn test_empty_enabled_users_file() -> Result<()> {
        // If the enabled users list is empty, all ballots should be counted as auditable.
        let ballots = "user_A,content_A\nuser_B,content_B\nuser_C,content_C";
        let users = "";
        let (_, elegible_voters, ballots_without_voter, casted_ballots) =
            run_merge_join_test(ballots, users)?;
        assert_eq!(elegible_voters, 0);
        assert_eq!(ballots_without_voter, 3);
        assert_eq!(casted_ballots, 3);
        Ok(())
    }

    #[test]
    fn test_both_files_empty() -> Result<()> {
        // If both files are empty, the count is 0.
        let ballots = "";
        let users = "";
        let (_, elegible_voters, ballots_without_voter, casted_ballots) =
            run_merge_join_test(ballots, users)?;
        assert_eq!(elegible_voters, 0);
        assert_eq!(ballots_without_voter, 0);
        assert_eq!(casted_ballots, 0);
        Ok(())
    }

    #[test]
    fn test_mixed_scenario_with_gaps() -> Result<()> {
        // A more complex real-world scenario.
        // user_A: match
        // user_C: auditable
        // user_E: match
        // user_F: auditable
        // user_H: auditable
        let ballots = "user_A,content_A\nuser_C,content_C\nuser_E,content_E\nuser_F,content_F\nuser_H,content_H";
        let users = "user_A\nuser_B\nuser_D\nuser_E\nuser_G";
        let (_, elegible_voters, ballots_without_voter, casted_ballots) =
            run_merge_join_test(ballots, users)?;
        assert_eq!(elegible_voters, 5);
        assert_eq!(ballots_without_voter, 3);
        assert_eq!(casted_ballots, 5);
        Ok(())
    }

    #[test]
    fn test_handles_malformed_but_consistent_columns() -> Result<()> {
        // This test has consistent column counts, but contains invalid data
        // like empty strings for keys, which should be skipped by the function's logic.
        //
        // - Row 1: ``,content_A` -> Skipped (empty key1)
        // - Row 2: `user_B,content_B` -> VALID AUDITABLE BALLOT
        // - Row 3: `user_C,content_C` -> VALID AUDITABLE BALLOT
        // - Row 4: `user_D,content_D` -> Valid, but matches user_D, so not auditable.
        let ballots = ",content_A\nuser_B,content_B\nuser_C,content_C\nuser_D,content_D";
        let users = "user_A\nuser_D";
        let (_, elegible_voters, ballots_without_voter, casted_ballots) =
            run_merge_join_test(ballots, users)?;
        assert_eq!(elegible_voters, 2);
        assert_eq!(ballots_without_voter, 2);
        assert_eq!(casted_ballots, 3);
        Ok(())
    }

    #[test]
    fn test_large_scale_auditable_count() -> Result<()> {
        const TOTAL_ENTRIES: u64 = 500;
        const EXPECTED_AUDITABLE_COUNT: u64 = (TOTAL_ENTRIES / 2) as u64; // We will add only even users, so odds are auditable.

        let mut ballots_csv = String::new();
        let mut users_csv = String::new();

        // Generate hundreds of "random-like" but deterministic entries.
        // The user IDs are padded with zeros to ensure correct lexicographical sorting.
        for i in 0..TOTAL_ENTRIES {
            let user_id = format!("user-{:04}", i);

            // 1. Add a ballot for every single user.
            ballots_csv.push_str(&format!("{},content_{}\n", user_id, i));

            // 2. Add only users with an even index to the "enabled users" file.
            if i % 2 == 0 {
                users_csv.push_str(&format!("{}\n", user_id));
            }
        }

        // Run the test with the generated data.
        let (_, elegible_voters, ballots_without_voter, casted_ballots) =
            run_merge_join_test(&ballots_csv, &users_csv)?;

        // 3. The function should count exactly half the entries—the ones we omitted (the odds).
        assert_eq!(elegible_voters, EXPECTED_AUDITABLE_COUNT);
        assert_eq!(ballots_without_voter, EXPECTED_AUDITABLE_COUNT);
        assert_eq!(casted_ballots, TOTAL_ENTRIES);

        Ok(())
    }

    #[test]
    fn test_merge_join_basic_join() -> Result<()> {
        // Both ballots have a corresponding enabled user, so both contents should be returned.
        let ballots = "user_A,content_A\nuser_B,content_B";
        let users = "user_A\nuser_B";
        let (result, _, _, _) = run_merge_join_test(ballots, users)?;
        assert_eq!(result, vec!["content_A", "content_B"]);
        Ok(())
    }

    #[test]
    fn test_merge_join_partial_join() -> Result<()> {
        // Only user_A exists in both files. user_C's ballot should be ignored.
        let ballots = "user_A,content_A\nuser_C,content_C";
        let users = "user_A\nuser_B";
        let (result, _, _, _) = run_merge_join_test(ballots, users)?;
        assert_eq!(result, vec!["content_A"]);
        Ok(())
    }

    #[test]
    fn test_merge_join_no_matches() -> Result<()> {
        // No common users between the two files.
        let ballots = "user_A,content_A";
        let users = "user_B\nuser_C";
        let (result, _, _, _) = run_merge_join_test(ballots, users)?;
        assert!(result.is_empty());
        Ok(())
    }

    #[test]
    fn test_merge_join_ignores_empty_keys() -> Result<()> {
        // *** CRITICAL TEST ***
        // This confirms the fix for the empty key bug.
        // The empty keys in both files should NOT result in a successful join.
        let ballots = "user_A,content_A\n,bad_content";
        let users = "user_A\n"; // Note the empty user record
        let (result, _, _, _) = run_merge_join_test(ballots, users)?;
        assert_eq!(result, vec!["content_A"]);
        Ok(())
    }

    #[test]
    fn test_merge_join_handles_malformed_csv() -> Result<()> {
        // This confirms the function skips malformed rows gracefully.
        // The "user_B" record is missing columns and should be ignored.
        let ballots = "user_A,content_A\nuser_B\nuser_C,content_C";
        let users = "user_A\nuser_C";
        let (result, _, _, _) = run_merge_join_test(ballots, users)?;
        assert_eq!(result, vec!["content_A", "content_C"]);
        Ok(())
    }

    #[test]
    fn test_merge_join_large_scale() -> Result<()> {
        // Stress test with a larger data set.
        const TOTAL_ENTRIES: i32 = 500;
        const EXPECTED_JOIN_COUNT: usize = (TOTAL_ENTRIES / 2) as usize;

        let mut ballots_csv = String::new();
        let mut users_csv = String::new();

        for i in 0..TOTAL_ENTRIES {
            let user_id = format!("user-{:04}", i);
            // Add a ballot for every user.
            ballots_csv.push_str(&format!("{},content_{}\n", user_id, i));
            // Add only even-indexed users to the enabled list.
            if i % 2 == 0 {
                users_csv.push_str(&format!("{}\n", user_id));
            }
        }

        let (result, _, _, _) = run_merge_join_test(&ballots_csv, &users_csv)?;

        // The function should join and return only the 250 ballots from the even users.
        assert_eq!(result.len(), EXPECTED_JOIN_COUNT);
        // Spot check the first and last expected content.
        assert_eq!(result.first().unwrap(), "content_0");
        assert_eq!(result.last().unwrap(), "content_498");

        Ok(())
    }
}
