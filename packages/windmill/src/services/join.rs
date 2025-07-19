// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use csv::ReaderBuilder;
use std::{cmp::Ordering, fs::File};
use tracing::{event, info, instrument, Level};

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
        let rec1_res = rec1_opt.as_ref().unwrap();
        let Ok(rec1) = rec1_res else {
            rec1_opt = iter1.next();
            continue;
        };

        let rec2_res = rec2_opt.as_ref().unwrap();
        let Ok(rec2) = rec2_res else {
            rec2_opt = iter2.next();
            continue;
        };

        // Extract the join keys.
        let Some(key1) = rec1.get(file1_join_index) else {
            // Advance file1.
            rec1_opt = iter1.next();
            continue;
        };
        // Ignore ballots with an empty key.
        if key1.is_empty() {
            rec1_opt = iter1.next();
            continue;
        }

        // Extract the join keys.
        let Some(key2) = rec2.get(file2_join_index) else {
            // Advance file1.
            rec2_opt = iter2.next();
            continue;
        };

        // Ignore users with an empty key.
        if key2.is_empty() {
            rec2_opt = iter2.next();
            continue;
        }

        // Also ignore empty keys from the users file for robustness.
        if key2.is_empty() {
            rec2_opt = iter2.next();
            continue;
        }

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
    while rec1_opt.is_some() {
        // Unwrap the current records.
        // Check for parsing errors in the ballot file (file1).
        // If a record is malformed, `rec1_res` will be an `Err`.
        let rec1_res = rec1_opt.as_ref().unwrap(); // Safe due to the while loop.
        let Ok(rec1) = rec1_res else {
            // This is a CSV parsing error. Log it and skip to the next record.
            event!(
                Level::WARN,
                "Skipping malformed record in ballots file: {:?}",
                rec1_res.as_ref().err()
            );
            rec1_opt = iter1.next();
            continue;
        };

        // Extract the join keys.
        let Some(key1) = rec1.get(file1_join_index) else {
            // Advance file1.
            rec1_opt = iter1.next();
            continue;
        };

        // Ignore records with an empty join key.
        if key1.is_empty() {
            rec1_opt = iter1.next();
            continue;
        }

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

        // If file2 is exhausted, all remaining valid ballots in file1 are auditable.
        if rec2_opt.is_none() {
            count += 1;
            rec1_opt = iter1.next();
            continue;
        }

        // Check for parsing errors in the users file (file2).
        let rec2_res = rec2_opt.as_ref().unwrap(); // Safe because we checked is_none().
        let Ok(rec2) = rec2_res else {
            // This is a CSV parsing error. Log it and skip to the next user record.
            event!(
                Level::WARN,
                "Skipping malformed record in users file: {:?}",
                rec2_res.as_ref().err()
            );
            rec2_opt = iter2.next();
            continue;
        };

        // Extract the join keys.
        let Some(key2) = rec2.get(file2_join_index) else {
            // Advance file1.
            rec2_opt = iter2.next();
            continue;
        };

        // Ignore records with an empty join key.
        if key2.is_empty() {
            rec2_opt = iter2.next();
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Helper function to create temporary CSV files and run the test.
    /// This reduces boilerplate code in each test case.
    fn run_test(ballots_csv: &str, users_csv: &str, election_id_to_check: &str) -> Result<usize> {
        let mut ballots_file = NamedTempFile::new()?;
        write!(ballots_file, "{}", ballots_csv)?;
        ballots_file.flush()?;

        let mut users_file = NamedTempFile::new()?;
        write!(users_file, "{}", users_csv)?;
        users_file.flush()?;

        // Reopen files for reading, as the function expects a read-only handle
        let ballots_ro = ballots_file.reopen()?;
        let users_ro = users_file.reopen()?;

        count_unique_csv(&ballots_ro, &users_ro, 0, 0, 1, election_id_to_check)
    }

    #[test]
    fn test_basic_auditable_ballot() -> Result<()> {
        // user_C's ballot should be counted as auditable as they are not in the users file.
        let ballots =
            "user_A,election_1,content_A\nuser_B,election_1,content_B\nuser_C,election_1,content_C";
        let users = "user_A\nuser_B";
        let count = run_test(ballots, users, "election_1")?;
        assert_eq!(count, 1);
        Ok(())
    }

    #[test]
    fn test_no_auditable_ballots_all_match() -> Result<()> {
        // All users who voted are in the enabled users list.
        let ballots = "user_A,election_1,content_A\nuser_B,election_1,content_B";
        let users = "user_A\nuser_B";
        let count = run_test(ballots, users, "election_1")?;
        assert_eq!(count, 0);
        Ok(())
    }

    #[test]
    fn test_auditable_ballots_at_end_of_file() -> Result<()> {
        // This specifically tests the bug fix. user_C and user_D's ballots are after
        // the last user in the users file. The old buggy code would miss these.
        let ballots =
            "user_A,election_1,content_A\nuser_C,election_1,content_C\nuser_D,election_1,content_D";
        let users = "user_A\nuser_B";
        let count = run_test(ballots, users, "election_1")?;
        assert_eq!(count, 2);
        Ok(())
    }

    #[test]
    fn test_empty_ballot_file() -> Result<()> {
        // If there are no ballots, the count must be 0.
        let ballots = "";
        let users = "user_A\nuser_B";
        let count = run_test(ballots, users, "election_1")?;
        assert_eq!(count, 0);
        Ok(())
    }

    #[test]
    fn test_empty_enabled_users_file() -> Result<()> {
        // If the enabled users list is empty, all ballots should be counted as auditable.
        let ballots =
            "user_A,election_1,content_A\nuser_B,election_1,content_B\nuser_C,election_1,content_C";
        let users = "";
        let count = run_test(ballots, users, "election_1")?;
        assert_eq!(count, 3);
        Ok(())
    }

    #[test]
    fn test_both_files_empty() -> Result<()> {
        // If both files are empty, the count is 0.
        let ballots = "";
        let users = "";
        let count = run_test(ballots, users, "election_1")?;
        assert_eq!(count, 0);
        Ok(())
    }

    #[test]
    fn test_filters_by_election_id() -> Result<()> {
        // Ballots for 'election_2' and 'election_3' should be ignored.
        // user_B (election_1) and user_D (election_1) are auditable.
        // user_A (election_1) is valid.
        // user_C (election_2) is ignored.
        // user_E (election_3) is ignored.
        let ballots = "user_A,election_1,content_A\nuser_B,election_1,content_B\nuser_C,election_2,content_C\nuser_D,election_1,content_D\nuser_E,election_3,content_E";
        let users = "user_A";
        let count = run_test(ballots, users, "election_1")?;
        assert_eq!(count, 2);
        Ok(())
    }

    #[test]
    fn test_no_ballots_for_specified_election() -> Result<()> {
        // No ballots match the desired election_id, so the count should be 0.
        let ballots = "user_A,election_2,content_A\nuser_B,election_3,content_B";
        let users = "user_C";
        let count = run_test(ballots, users, "election_1")?;
        assert_eq!(count, 0);
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
        let ballots = "user_A,election_1,content_A\nuser_C,election_1,content_C\nuser_E,election_1,content_E\nuser_F,election_1,content_F\nuser_H,election_1,content_H";
        let users = "user_A\nuser_B\nuser_D\nuser_E\nuser_G";
        let count = run_test(ballots, users, "election_1")?;
        assert_eq!(count, 3);
        Ok(())
    }

    #[test]
    fn test_handles_malformed_but_consistent_columns() -> Result<()> {
        // This test has consistent column counts, but contains invalid data
        // like empty strings for keys, which should be skipped by the function's logic.
        //
        // - Row 1: ``,election_1,content_A` -> Skipped (empty key1)
        // - Row 2: `user_B,election_2,content_B` -> Skipped (wrong election_id)
        // - Row 3: `user_C,election_1,content_C` -> VALID AUDITABLE BALLOT
        // - Row 4: `user_D,election_1,content_D` -> Valid, but matches user_D, so not auditable.
        let ballots = ",election_1,content_A\nuser_B,election_2,content_B\nuser_C,election_1,content_C\nuser_D,election_1,content_D";
        let users = "user_A\nuser_D";
        let count = run_test(ballots, users, "election_1")?;
        assert_eq!(count, 1);
        Ok(())
    }

    #[test]
    fn test_large_scale_auditable_count() -> Result<()> {
        const TOTAL_ENTRIES: i32 = 500;
        const EXPECTED_AUDITABLE_COUNT: usize = (TOTAL_ENTRIES / 2) as usize; // We will add only even users, so odds are auditable.

        let mut ballots_csv = String::new();
        let mut users_csv = String::new();
        let election_id = "large_election";

        // Generate hundreds of "random-like" but deterministic entries.
        // The user IDs are padded with zeros to ensure correct lexicographical sorting.
        for i in 0..TOTAL_ENTRIES {
            let user_id = format!("user-{:04}", i);

            // 1. Add a ballot for every single user.
            ballots_csv.push_str(&format!("{},{},content_{}\n", user_id, election_id, i));

            // 2. Add only users with an even index to the "enabled users" file.
            if i % 2 == 0 {
                users_csv.push_str(&format!("{}\n", user_id));
            }
        }

        // Run the test with the generated data.
        let count = run_test(&ballots_csv, &users_csv, election_id)?;

        // 3. The function should count exactly half the entries—the ones we omitted (the odds).
        assert_eq!(count, EXPECTED_AUDITABLE_COUNT);

        Ok(())
    }

    /// Helper function to run tests for `merge_join_csv`.
    fn run_merge_join_test(
        ballots_csv: &str,
        users_csv: &str,
        election_id_to_check: &str,
    ) -> Result<Vec<String>> {
        let mut ballots_file = NamedTempFile::new()?;
        write!(ballots_file, "{}", ballots_csv)?;
        ballots_file.flush()?;

        let mut users_file = NamedTempFile::new()?;
        write!(users_file, "{}", users_csv)?;
        users_file.flush()?;

        let ballots_ro = ballots_file.reopen()?;
        let users_ro = users_file.reopen()?;

        // Assumes standard test indexes:
        // join_index=0, output_index=2, election_id_index=1
        merge_join_csv(&ballots_ro, &users_ro, 0, 0, 2, 1, election_id_to_check)
    }

    #[test]
    fn test_merge_join_basic_join() -> Result<()> {
        // Both ballots have a corresponding enabled user, so both contents should be returned.
        let ballots = "user_A,election_1,content_A\nuser_B,election_1,content_B";
        let users = "user_A\nuser_B";
        let result = run_merge_join_test(ballots, users, "election_1")?;
        assert_eq!(result, vec!["content_A", "content_B"]);
        Ok(())
    }

    #[test]
    fn test_merge_join_partial_join() -> Result<()> {
        // Only user_A exists in both files. user_C's ballot should be ignored.
        let ballots = "user_A,election_1,content_A\nuser_C,election_1,content_C";
        let users = "user_A\nuser_B";
        let result = run_merge_join_test(ballots, users, "election_1")?;
        assert_eq!(result, vec!["content_A"]);
        Ok(())
    }

    #[test]
    fn test_merge_join_no_matches() -> Result<()> {
        // No common users between the two files.
        let ballots = "user_A,election_1,content_A";
        let users = "user_B\nuser_C";
        let result = run_merge_join_test(ballots, users, "election_1")?;
        assert!(result.is_empty());
        Ok(())
    }

    #[test]
    fn test_merge_join_filters_by_election_id() -> Result<()> {
        // The user matches, but the ballot is for a different election, so it should be filtered out.
        let ballots = "user_A,election_2,content_A";
        let users = "user_A";
        let result = run_merge_join_test(ballots, users, "election_1")?;
        assert!(result.is_empty());
        Ok(())
    }

    #[test]
    fn test_merge_join_ignores_empty_keys() -> Result<()> {
        // *** CRITICAL TEST ***
        // This confirms the fix for the empty key bug.
        // The empty keys in both files should NOT result in a successful join.
        let ballots = "user_A,election_1,content_A\n,election_1,bad_content";
        let users = "user_A\n"; // Note the empty user record
        let result = run_merge_join_test(ballots, users, "election_1")?;
        assert_eq!(result, vec!["content_A"]);
        Ok(())
    }

    #[test]
    fn test_merge_join_handles_malformed_csv() -> Result<()> {
        // This confirms the function skips malformed rows gracefully.
        // The "user_B" record is missing columns and should be ignored.
        let ballots = "user_A,election_1,content_A\nuser_B\nuser_C,election_1,content_C";
        let users = "user_A\nuser_C";
        let result = run_merge_join_test(ballots, users, "election_1")?;
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
        let election_id = "large_election";

        for i in 0..TOTAL_ENTRIES {
            let user_id = format!("user-{:04}", i);
            // Add a ballot for every user.
            ballots_csv.push_str(&format!("{},{},content_{}\n", user_id, election_id, i));
            // Add only even-indexed users to the enabled list.
            if i % 2 == 0 {
                users_csv.push_str(&format!("{}\n", user_id));
            }
        }

        let result = run_merge_join_test(&ballots_csv, &users_csv, election_id)?;

        // The function should join and return only the 250 ballots from the even users.
        assert_eq!(result.len(), EXPECTED_JOIN_COUNT);
        // Spot check the first and last expected content.
        assert_eq!(result.first().unwrap(), "content_0");
        assert_eq!(result.last().unwrap(), "content_498");

        Ok(())
    }
}
