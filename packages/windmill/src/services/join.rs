// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, ensure, Result};
use csv::{ReaderBuilder, StringRecord};
use sequent_core::types::keycloak::{MAX_VOTE_WEIGHT, MIN_VOTE_WEIGHT};
use sequent_core::types::participation::{ParticipationChannel, VotesByChannel};
use std::{cmp::Ordering, fs::File};
use tracing::{info, instrument};

/// Where a voter's ballot multiplicity comes from, i.e. how many times the
/// voter's ciphertext enters the mix batch. The payload is the index of the
/// column carrying the value in the voters CSV.
///
/// The two variants are mutually exclusive by construction: delegated voting
/// and voter-weighted voting cannot be enabled on the same election event, so
/// the voters CSV carries at most one such column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultiplicitySource {
    /// Column holds the number of voters who delegated to this voter.
    /// Multiplicity is `1 + delegate_count`.
    DelegateCount(usize),
    /// Column holds the voter's own vote weight. Multiplicity is that weight.
    VoteWeight(usize),
}

/// Reads `(multiplicity, census_count, cast_count)` for a voter row.
///
/// `multiplicity` is how many ciphertexts the voter contributes.
/// `census_count` is how much the voter contributes to `eligible_voters`.
/// `cast_count` is how much a matched ballot contributes to participation.
/// Voter weights only affect `multiplicity`; census and election-level turnout
/// remain voter headcounts. Delegated voting preserves its existing behavior,
/// where a delegate's matched ballot represents the voter and all delegators.
///
/// A missing, empty or unparseable column is an error rather than a skipped
/// voter: skipping would silently discard a cast ballot and under-count the
/// census.
fn read_multiplicity(
    voter: &StringRecord,
    source: Option<MultiplicitySource>,
) -> Result<(u64, u64, u64)> {
    let (index, is_weight) = match source {
        None => return Ok((1, 1, 1)),
        Some(MultiplicitySource::DelegateCount(index)) => (index, false),
        Some(MultiplicitySource::VoteWeight(index)) => (index, true),
    };

    let raw = voter
        .get(index)
        .ok_or_else(|| anyhow!("Voter row is missing multiplicity column {index}: {voter:?}"))?;
    let value: u64 = raw.trim().parse().map_err(|_| {
        anyhow!("Invalid multiplicity {raw:?} in column {index} for voter row {voter:?}")
    })?;

    if is_weight {
        ensure!(
            (MIN_VOTE_WEIGHT..=MAX_VOTE_WEIGHT).contains(&value),
            "Vote weight {value} is out of range, must be between \
             {MIN_VOTE_WEIGHT} and {MAX_VOTE_WEIGHT}"
        );
        Ok((value, 1, 1))
    } else {
        let multiplicity = 1 + value;
        Ok((multiplicity, 1, multiplicity))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct MergeJoinResult {
    /// `(ballot content, multiplicity)`. The content is stored once with its
    /// multiplicity rather than repeated, so memory stays proportional to the
    /// number of distinct ballots. Callers expand after parsing, where the
    /// repeated value is a small ciphertext instead of the ballot payload.
    pub ballot_contents: Vec<(String, u64)>,
    pub eligible_voters: u64,
    pub ballots_without_voter: u64,
    pub casted_ballots: u64,
    pub casted_ballots_by_channel: VotesByChannel,
}

fn count_ballot_channel(
    counts: &mut VotesByChannel,
    ballot: &StringRecord,
    channel_index: Option<usize>,
    count: u64,
) -> Result<()> {
    let Some(channel_index) = channel_index else {
        return Ok(());
    };
    let channel = ballot
        .get(channel_index)
        .filter(|channel| !channel.is_empty())
        .ok_or_else(|| anyhow!("Ballot channel column {channel_index} is missing or empty"))?;
    *counts
        .entry(ParticipationChannel::from(channel))
        .or_default() += count;
    Ok(())
}

#[instrument(skip_all, err)]
pub fn merge_join_csv(
    ballots_file: &File,
    voters_file: &File,
    ballots_voter_id_index: usize,
    voters_id_index: usize,
    ballots_content_index: usize,
    ballots_channel_index: Option<usize>,
    multiplicity_source: Option<MultiplicitySource>,
) -> Result<MergeJoinResult> {
    info!("START merge_join_csv");

    // Initialize the result vector and counters
    let mut result = Vec::new();
    let mut ballots_without_voter: u64 = 0;
    let mut elegible_voters: u64 = 0;
    let mut casted_ballots: u64 = 0;
    let mut casted_ballots_by_channel = VotesByChannel::new();

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

        // Extract the ballot join key.
        let Some(ballot_voter_id) = ballot.get(ballots_voter_id_index) else {
            // Advance ballots file.
            ballots_record = ballots_iterator.next();
            continue;
        };
        // Ignore ballots with an empty key.
        if ballot_voter_id.is_empty() {
            ballots_record = ballots_iterator.next();
            continue;
        }

        // Extract the voter join key.
        let Some(voter_id) = voter.get(voters_id_index) else {
            // Advance voters file.
            voters_record = voters_iterator.next();
            continue;
        };
        // Ignore users with an empty key.
        if voter_id.is_empty() {
            voters_record = voters_iterator.next();
            continue;
        }

        let (multiplicity, census_count, cast_count) =
            read_multiplicity(voter, multiplicity_source)?;

        // Compare the join keys lexicographically.
        match ballot_voter_id.cmp(voter_id) {
            Ordering::Less => {
                // If the ballot has no voter.
                ballots_without_voter += 1;
                count_ballot_channel(
                    &mut casted_ballots_by_channel,
                    ballot,
                    ballots_channel_index,
                    1,
                )?;
                // Advance ballots file.
                ballots_record = ballots_iterator.next();
                casted_ballots += 1;
            }
            Ordering::Greater => {
                // Advance voters file.
                voters_record = voters_iterator.next();
                elegible_voters += census_count;
            }
            Ordering::Equal => {
                // Match found.
                let ballot_content = ballot.get(ballots_content_index).ok_or_else(|| {
                    anyhow!(
                        "Output column index {} out of bounds in file1",
                        ballots_content_index
                    )
                })?;

                // Store the ballot once with its multiplicity; it is expanded
                // by the caller after parsing.
                result.push((ballot_content.to_string(), multiplicity));

                casted_ballots += cast_count;
                count_ballot_channel(
                    &mut casted_ballots_by_channel,
                    ballot,
                    ballots_channel_index,
                    multiplicity,
                )?;

                // Advance both iterators.
                ballots_record = ballots_iterator.next();
                voters_record = voters_iterator.next();

                elegible_voters += census_count;
            }
        }
    }

    // Count the rest of the voters. A record that will not parse still counts
    // as one voter, exactly as it did before multiplicities existed, so this
    // cannot move a census for an election that does not use them.
    while let Some(voter_record) = voters_record {
        let census_count = match &voter_record {
            Ok(voter) => read_multiplicity(voter, multiplicity_source)?.1,
            Err(_) => 1,
        };
        elegible_voters += census_count;
        voters_record = voters_iterator.next();
    }

    // Count the rest of the ballots
    while let Some(ballot_record) = ballots_record {
        casted_ballots += 1;
        ballots_without_voter += 1;
        if ballots_channel_index.is_some() {
            let ballot = ballot_record?;
            count_ballot_channel(
                &mut casted_ballots_by_channel,
                &ballot,
                ballots_channel_index,
                1,
            )?;
        }
        ballots_record = ballots_iterator.next();
    }

    let ballots_to_be_tallied: u64 = result.iter().map(|(_, multiplicity)| multiplicity).sum();
    if ballots_channel_index.is_some() {
        let channel_total: u64 = casted_ballots_by_channel.values().sum();
        let weighted_participation_total = ballots_to_be_tallied
            .checked_add(ballots_without_voter)
            .ok_or_else(|| anyhow!("Weighted participation total overflow"))?;
        ensure!(
            channel_total == weighted_participation_total,
            "Ballot channel total {channel_total} does not match weighted participation total \
             {weighted_participation_total}"
        );
    }

    info!("ballots_to_be_tallied: {}, distinct_ballots: {}, elegible_voters: {}, ballots_without_voter: {}, casted_ballots: {}", ballots_to_be_tallied, result.len(), elegible_voters, ballots_without_voter, casted_ballots);

    Ok(MergeJoinResult {
        ballot_contents: result,
        eligible_voters: elegible_voters,
        ballots_without_voter,
        casted_ballots,
        casted_ballots_by_channel,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use sequent_core::ballot::VotingStatusChannel;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Expands `(content, multiplicity)` pairs into the flat vector the merge
    /// used to return, so the pre-existing assertions still describe the
    /// ballots that reach the mix batch.
    fn expand(contents: Vec<(String, u64)>) -> Vec<String> {
        contents
            .into_iter()
            .flat_map(|(content, multiplicity)| std::iter::repeat_n(content, multiplicity as usize))
            .collect()
    }

    /// Helper function to run tests for `merge_join_csv` (non-delegate mode).
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
        // ballots_voter_id_index=0, voters_id_index=0, ballots_content_index=1
        // Pass `None` for multiplicity_source to run in standard mode.
        let result = merge_join_csv(
            &ballots_ro,
            &users_ro,
            0,    // ballots_voter_id_index
            0,    // voters_id_index
            1,    // ballots_content_index
            None, // ballots_channel_index
            None, // multiplicity_source
        )?;
        Ok((
            expand(result.ballot_contents),
            result.eligible_voters,
            result.ballots_without_voter,
            result.casted_ballots,
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

    /// Helper that writes the two CSV strings to temporary files,
    /// reopens them for reading and then calls `merge_join_csv` in delegate mode.
    ///
    /// The index arguments are the *standard* test indexes used by the
    /// original function:
    ///   ballots_voter_id_index = 0
    ///   voters_id_index        = 0
    ///   ballots_content_index  = 1
    ///   delegate_count_index   = 1
    fn run_merge_join_delegates_test(
        ballots_csv: &str,
        voters_csv: &str,
    ) -> Result<(Vec<String>, u64, u64, u64)> {
        // Write the CSV strings to temporary files
        let mut ballots_file = NamedTempFile::new()?;
        write!(ballots_file, "{}", ballots_csv)?;
        ballots_file.flush()?;

        let mut voters_file = NamedTempFile::new()?;
        write!(voters_file, "{}", voters_csv)?;
        voters_file.flush()?;

        // Reopen the files read‑only – the original function expects `&File`
        let ballots_ro = ballots_file.reopen()?;
        let voters_ro = voters_file.reopen()?;

        // Call the function under test
        // Pass a DelegateCount source to run in delegate mode.
        let result = merge_join_csv(
            &ballots_ro,
            &voters_ro,
            /* ballots_voter_id_index   */ 0,
            /* voters_id_index        */ 0,
            /* ballots_content_index  */ 1,
            /* ballots_channel_index  */ None,
            /* multiplicity_source    */ Some(MultiplicitySource::DelegateCount(1)),
        )?;

        Ok((
            expand(result.ballot_contents),
            result.eligible_voters,
            result.ballots_without_voter,
            result.casted_ballots,
        ))
    }

    /// ------------------------------------------------------------------
    /// 1. Basic delegate counting
    /// ------------------------------------------------------------------
    #[test]
    fn test_basic_delegate_counts() -> Result<()> {
        // ballots: voter_id, content
        let ballots = "\
            user_A,content_A
            user_B,content_B
            user_C,content_C
            user_D,content_D
            user_E,content_E";

        // voters: voter_id, delegate_count
        let voters = "\
            user_A,1
            user_B,0
            user_D,3
            user_E,2
            user_F,1"; // user_F has no ballot -> eligible voter

        let (result, elegible_voters, ballots_without_voter, casted_ballots) =
            run_merge_join_delegates_test(ballots, voters)?;

        // 10 entries in the result vector:
        //   user_A → 1 (own) + 1 (delegate) = 2 copies
        //   user_B → 1 (own) + 0 (delegate) = 1 copy
        //   user_D → 1 (own) + 3 (delegate) = 4 copies
        //   user_E → 1 (own) + 2 (delegate) = 3 copies
        assert_eq!(result.len(), 10);
        assert_eq!(
            result,
            vec![
                "content_A",
                "content_A",
                "content_B",
                "content_D",
                "content_D",
                "content_D",
                "content_D",
                "content_E",
                "content_E",
                "content_E"
            ]
        );

        // User_C has no matching voter: counted as “without voter”
        assert_eq!(ballots_without_voter, 1);

        // Total ballots cast = 2 (A) + 1 (B) + 1 (C) + 4 (D) + 3 (E) = 11
        assert_eq!(casted_ballots, 11);

        // 5 eligible voters (A, B, D, E, F)
        assert_eq!(elegible_voters, 5);

        Ok(())
    }

    /// ------------------------------------------------------------------
    /// 2. Empty / missing keys
    /// ------------------------------------------------------------------
    #[test]
    fn test_missing_and_empty_keys() -> Result<()> {
        let ballots = "\
        user_A,content_A
        ,content_B     # empty voter id
        user_C,content_C
        user_D,content_D";

        let voters = "\
        user_A,1
        user_B,0
        # missing user_C
        user_D,2";

        let (result, elegible_voters, ballots_without_voter, casted_ballots) =
            run_merge_join_delegates_test(ballots, voters)?;

        // Only user_A and user_D should be matched
        // user_A -> 1 + 1 = 2 copies
        // user_D -> 1 + 2 = 3 copies
        assert_eq!(result.len(), 5);
        assert_eq!(
            result,
            vec![
                "content_A",
                "content_A",
                "content_D",
                "content_D",
                "content_D"
            ]
        );

        // Ballot 2 (empty voter id) and 3 (user_C) are "without voter"
        assert_eq!(ballots_without_voter, 2);

        // 3 eligible voters (A, B, D)
        assert_eq!(elegible_voters, 3);

        // Total ballots cast = 2 (A) + 1 (B) + 1 (C) + 3 (D) = 7
        assert_eq!(casted_ballots, 7);

        Ok(())
    }

    /// ------------------------------------------------------------------
    /// 3. Invalid multiplicity is fatal, never a skipped voter
    /// ------------------------------------------------------------------
    /// Skipping the voter would drop their cast ballot from the tally and
    /// under-count the census, both silently.
    #[test]
    fn test_invalid_delegate_count_is_an_error() {
        let ballots = "\
            user_A,content_A
            user_G,content_G";

        let voters = "\
            user_A,1
            user_G,not_a_number"; // invalid count

        let error = run_merge_join_delegates_test(ballots, voters)
            .expect_err("an unparseable multiplicity must fail the merge");
        assert!(
            error.to_string().contains("Invalid multiplicity"),
            "unexpected error: {error}"
        );
    }

    /// ------------------------------------------------------------------
    /// Voter-weighted voting
    /// ------------------------------------------------------------------
    ///
    /// Runs the merge in voter-weighted mode. The voters file is
    /// `voter_id,vote_weight`.
    fn run_merge_join_weights_test(
        ballots: &str,
        voters: &str,
        channel_index: Option<usize>,
    ) -> Result<MergeJoinResult> {
        let mut ballots_file = NamedTempFile::new()?;
        write!(ballots_file, "{ballots}")?;
        ballots_file.flush()?;

        let mut voters_file = NamedTempFile::new()?;
        write!(voters_file, "{voters}")?;
        voters_file.flush()?;

        merge_join_csv(
            &ballots_file.reopen()?,
            &voters_file.reopen()?,
            0,
            0,
            1,
            channel_index,
            Some(MultiplicitySource::VoteWeight(1)),
        )
    }

    /// A voter with weight `w` contributes `w` ballots to the tally, while
    /// census and participation remain voter headcounts.
    #[test]
    fn test_vote_weight_only_multiplies_tallied_ballots() -> Result<()> {
        let ballots = "user_A,content_A\nuser_B,content_B";
        // user_C is eligible but did not vote.
        let voters = "user_A,3\nuser_B,1\nuser_C,5";

        let result = run_merge_join_weights_test(ballots, voters, None)?;

        assert_eq!(
            result.ballot_contents,
            vec![("content_A".to_string(), 3), ("content_B".to_string(), 1)]
        );
        assert_eq!(result.casted_ballots, 2);
        assert_eq!(result.eligible_voters, 3);
        assert_eq!(result.ballots_without_voter, 0);
        Ok(())
    }

    /// An absent weight column value can never reach here: the SQL COALESCEs it
    /// to the default. If it somehow does, it must fail loudly.
    #[test]
    fn test_empty_vote_weight_is_an_error() {
        let error = run_merge_join_weights_test("user_A,content_A", "user_A,", None)
            .expect_err("an empty weight must fail the merge");
        assert!(
            error.to_string().contains("Invalid multiplicity"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn test_out_of_range_vote_weights_are_errors() {
        for weight in ["0", "-1", "1.5", "abc", "100001"] {
            let voters = format!("user_A,{weight}");
            let error = run_merge_join_weights_test("user_A,content_A", &voters, None)
                .unwrap_err()
                .to_string();
            assert!(
                error.contains("Invalid multiplicity") || error.contains("out of range"),
                "vote weight {weight} must be rejected as a weight, got: {error}"
            );
        }
    }

    /// Defensive: the dump now emits a canonical number, so padding cannot
    /// reach here from that path. It can from a hand-written voters file in a
    /// test or tool, and failing on it would abort the tally after voting
    /// closed, so parsing stays lenient about surrounding whitespace.
    #[test]
    fn test_padded_vote_weight_is_accepted() -> Result<()> {
        let result = run_merge_join_weights_test("user_A,content_A", "user_A, 3 ", None)?;
        assert_eq!(result.ballot_contents, vec![("content_A".to_string(), 3)]);
        Ok(())
    }

    /// Channel totals stay in the same weighted units as the tally input so
    /// downstream tally validation can compare like with like.
    #[test]
    fn test_vote_weight_channel_totals_remain_weighted() -> Result<()> {
        let ballots = "user_A,content_A,ONLINE\nuser_B,content_B,TELEPHONE";
        let voters = "user_A,11\nuser_B,2";

        let result = run_merge_join_weights_test(ballots, voters, Some(2))?;

        assert_eq!(result.casted_ballots, 2);
        assert_eq!(
            result
                .casted_ballots_by_channel
                .get(&VotingStatusChannel::ONLINE.into()),
            Some(&11)
        );
        assert_eq!(
            result
                .casted_ballots_by_channel
                .get(&VotingStatusChannel::TELEPHONE.into()),
            Some(&2)
        );
        Ok(())
    }

    /// Weight 1 everywhere must be indistinguishable from the feature being off.
    #[test]
    fn test_unit_weights_match_unweighted_behaviour() -> Result<()> {
        let ballots = "user_A,content_A\nuser_B,content_B";
        let weighted = run_merge_join_weights_test(ballots, "user_A,1\nuser_B,1", None)?;
        let (plain_contents, plain_eligible, plain_without, plain_casted) =
            run_merge_join_test(ballots, "user_A\nuser_B")?;

        assert_eq!(expand(weighted.ballot_contents), plain_contents);
        assert_eq!(weighted.eligible_voters, plain_eligible);
        assert_eq!(weighted.ballots_without_voter, plain_without);
        assert_eq!(weighted.casted_ballots, plain_casted);
        Ok(())
    }

    #[test]
    fn test_counts_channels_for_matched_auditable_and_delegated_ballots() -> Result<()> {
        let mut ballots_file = NamedTempFile::new()?;
        write!(
            ballots_file,
            "user_A,content_A,ONLINE\nuser_B,content_B,TELEPHONE\nuser_Z,content_Z,KIOSK"
        )?;
        ballots_file.flush()?;

        let mut voters_file = NamedTempFile::new()?;
        write!(voters_file, "user_A,2\nuser_B,0")?;
        voters_file.flush()?;

        let result = merge_join_csv(
            &ballots_file.reopen()?,
            &voters_file.reopen()?,
            0,
            0,
            1,
            Some(2),
            Some(MultiplicitySource::DelegateCount(1)),
        )?;

        assert_eq!(expand(result.ballot_contents.clone()).len(), 4);
        assert_eq!(result.casted_ballots, 5);
        assert_eq!(result.ballots_without_voter, 1);
        assert_eq!(
            result
                .casted_ballots_by_channel
                .get(&VotingStatusChannel::ONLINE.into()),
            Some(&3)
        );
        assert_eq!(
            result
                .casted_ballots_by_channel
                .get(&VotingStatusChannel::TELEPHONE.into()),
            Some(&1)
        );
        assert_eq!(
            result
                .casted_ballots_by_channel
                .get(&VotingStatusChannel::KIOSK.into()),
            Some(&1)
        );
        assert_eq!(
            result
                .casted_ballots_by_channel
                .values()
                .copied()
                .sum::<u64>(),
            result.casted_ballots
        );

        Ok(())
    }
}
