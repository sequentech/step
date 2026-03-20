// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use b3::messages::artifact::Configuration;
use b3::messages::newtypes::{TrusteeSet, MAX_TRUSTEES, NULL_TRUSTEE};
use csv::ReaderBuilder;
use std::{cmp::Ordering, fs::File};
use strand::context::Ctx;
use strand::signature::StrandSignaturePk;
use tracing::{event, info, instrument, Level};

/// Performs a sorted merge join on two CSV files and returns the matched ballot contents
/// together with voter and ballot statistics.
///
/// Both files must be sorted lexicographically on their join key column. The function
/// yields `(matched_ballot_contents, eligible_voters, ballots_without_voter, casted_ballots)`.
#[instrument(skip_all, err)]
pub fn merge_join_csv(
    ballots_file: &File,
    voters_file: &File,
    ballots_voter_id_index: usize,
    voters_id_index: usize,
    ballots_content_index: usize,
    delegate_count_index: Option<usize>,
) -> Result<(Vec<String>, u64, u64, u64)> {
    info!("START merge_join_csv");

    let mut result = Vec::new();
    let mut ballots_without_voter: u64 = 0;
    let mut elegible_voters: u64 = 0;
    let mut casted_ballots: u64 = 0;

    let mut ballots_reader = ReaderBuilder::new()
        .has_headers(false)
        .from_reader(ballots_file);
    let mut voters_reader = ReaderBuilder::new()
        .has_headers(false)
        .from_reader(voters_file);

    let mut ballots_iterator = ballots_reader.records();
    let mut voters_iterator = voters_reader.records();

    let mut ballots_record = ballots_iterator.next();
    let mut voters_record = voters_iterator.next();

    while ballots_record.is_some() && voters_record.is_some() {
        let Some(Ok(ballot)) = ballots_record.as_ref() else {
            ballots_record = ballots_iterator.next();
            continue;
        };
        let Some(Ok(voter)) = voters_record.as_ref() else {
            voters_record = voters_iterator.next();
            continue;
        };

        let Some(ballot_voter_id) = ballot.get(ballots_voter_id_index) else {
            ballots_record = ballots_iterator.next();
            continue;
        };
        if ballot_voter_id.is_empty() {
            ballots_record = ballots_iterator.next();
            continue;
        }

        let Some(voter_id) = voter.get(voters_id_index) else {
            voters_record = voters_iterator.next();
            continue;
        };
        if voter_id.is_empty() {
            voters_record = voters_iterator.next();
            continue;
        }

        let delegate_count: usize = if let Some(index) = delegate_count_index {
            let Some(delegate_count_str) = voter.get(index) else {
                voters_record = voters_iterator.next();
                continue;
            };
            if delegate_count_str.is_empty() {
                voters_record = voters_iterator.next();
                continue;
            }
            let Ok(count) = delegate_count_str.parse() else {
                voters_record = voters_iterator.next();
                continue;
            };
            count
        } else {
            0
        };

        match ballot_voter_id.cmp(voter_id) {
            Ordering::Less => {
                ballots_without_voter += 1;
                ballots_record = ballots_iterator.next();
                casted_ballots += 1;
            }
            Ordering::Greater => {
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
                result.extend(std::iter::repeat(ballot_content.to_string()).take(delegate_count));

                ballots_record = ballots_iterator.next();
                voters_record = voters_iterator.next();

                casted_ballots += 1 + (delegate_count as u64);
                elegible_voters += 1;
            }
        }
    }

    while voters_record.is_some() {
        elegible_voters += 1;
        voters_record = voters_iterator.next();
    }

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

/// Maps trustee public keys to their 1-indexed positions in the board configuration's trustee
/// list and returns a fixed-size `TrusteeSet` array.
#[instrument(skip_all)]
pub fn generate_trustee_set<C: Ctx>(
    configuration: &Configuration<C>,
    trustee_pks: Vec<StrandSignaturePk>,
) -> TrusteeSet {
    let result = find_trustee_positions(&configuration.trustees, trustee_pks);
    event!(Level::INFO, "TrusteeSet: {:?}", result);
    result
}

/// Returns a `TrusteeSet` array where each slot holds the 1-based position of the corresponding
/// `trustee_pks` entry within `trustees`, or `NULL_TRUSTEE` if not found.
fn find_trustee_positions(
    trustees: &[StrandSignaturePk],
    trustee_pks: Vec<StrandSignaturePk>,
) -> TrusteeSet {
    let mut selected_trustees: TrusteeSet = [NULL_TRUSTEE; MAX_TRUSTEES];
    let trustee_ids: Vec<usize> = trustee_pks
        .into_iter()
        .map(|trustee_pk| {
            let position = trustees.iter().position(|trustee| trustee == &trustee_pk);
            match position {
                Some(value) => value + 1,
                None => NULL_TRUSTEE,
            }
        })
        .collect();
    for i in 0..trustee_ids.len() {
        selected_trustees[i] = trustee_ids[i];
    }
    selected_trustees
}

#[cfg(test)]
mod tests {
    use super::*;
    use b3::messages::newtypes::NULL_TRUSTEE;
    use std::io::Write;
    use strand::signature::{StrandSignaturePk, StrandSignatureSk};
    use tempfile::NamedTempFile;

    fn make_pk() -> StrandSignaturePk {
        let sk = StrandSignatureSk::gen().unwrap();
        StrandSignaturePk::from_sk(&sk).unwrap()
    }

    #[test]
    fn test_all_trustees_found() {
        let pks: Vec<StrandSignaturePk> = (0..3).map(|_| make_pk()).collect();
        let result = find_trustee_positions(&pks, pks.clone());
        assert_eq!(result[0], 1);
        assert_eq!(result[1], 2);
        assert_eq!(result[2], 3);
        assert!(result[3..].iter().all(|&x| x == NULL_TRUSTEE));
    }

    #[test]
    fn test_trustee_not_in_config() {
        let config_pks: Vec<StrandSignaturePk> = (0..2).map(|_| make_pk()).collect();
        let unknown_pk = make_pk();
        let result = find_trustee_positions(&config_pks, vec![config_pks[0].clone(), unknown_pk]);
        assert_eq!(result[0], 1);
        assert_eq!(result[1], NULL_TRUSTEE);
        assert!(result[2..].iter().all(|&x| x == NULL_TRUSTEE));
    }

    #[test]
    fn test_empty_pks() {
        let config_pks: Vec<StrandSignaturePk> = (0..2).map(|_| make_pk()).collect();
        let result = find_trustee_positions(&config_pks, vec![]);
        assert!(result.iter().all(|&x| x == NULL_TRUSTEE));
    }

    #[test]
    fn test_out_of_order_pks() {
        let pks: Vec<StrandSignaturePk> = (0..3).map(|_| make_pk()).collect();
        // Provide pks in reverse order
        let reversed = vec![pks[2].clone(), pks[0].clone(), pks[1].clone()];
        let result = find_trustee_positions(&pks, reversed);
        assert_eq!(result[0], 3);
        assert_eq!(result[1], 1);
        assert_eq!(result[2], 2);
        assert!(result[3..].iter().all(|&x| x == NULL_TRUSTEE));
    }

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

        let (ballot_contents, elegible_voters, ballots_without_voter, casted_ballots) =
            merge_join_csv(
                &ballots_ro,
                &users_ro,
                0,
                0,
                1,
                None,
            )?;
        Ok((
            ballot_contents,
            elegible_voters,
            ballots_without_voter,
            casted_ballots,
        ))
    }

    #[test]
    fn test_basic_auditable_ballot() -> Result<()> {
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
        const EXPECTED_AUDITABLE_COUNT: u64 = (TOTAL_ENTRIES / 2) as u64;

        let mut ballots_csv = String::new();
        let mut users_csv = String::new();

        for i in 0..TOTAL_ENTRIES {
            let user_id = format!("user-{:04}", i);
            ballots_csv.push_str(&format!("{},content_{}\n", user_id, i));
            if i % 2 == 0 {
                users_csv.push_str(&format!("{}\n", user_id));
            }
        }

        let (_, elegible_voters, ballots_without_voter, casted_ballots) =
            run_merge_join_test(&ballots_csv, &users_csv)?;

        assert_eq!(elegible_voters, EXPECTED_AUDITABLE_COUNT);
        assert_eq!(ballots_without_voter, EXPECTED_AUDITABLE_COUNT);
        assert_eq!(casted_ballots, TOTAL_ENTRIES);

        Ok(())
    }

    #[test]
    fn test_merge_join_basic_join() -> Result<()> {
        let ballots = "user_A,content_A\nuser_B,content_B";
        let users = "user_A\nuser_B";
        let (result, _, _, _) = run_merge_join_test(ballots, users)?;
        assert_eq!(result, vec!["content_A", "content_B"]);
        Ok(())
    }

    #[test]
    fn test_merge_join_partial_join() -> Result<()> {
        let ballots = "user_A,content_A\nuser_C,content_C";
        let users = "user_A\nuser_B";
        let (result, _, _, _) = run_merge_join_test(ballots, users)?;
        assert_eq!(result, vec!["content_A"]);
        Ok(())
    }

    #[test]
    fn test_merge_join_no_matches() -> Result<()> {
        let ballots = "user_A,content_A";
        let users = "user_B\nuser_C";
        let (result, _, _, _) = run_merge_join_test(ballots, users)?;
        assert!(result.is_empty());
        Ok(())
    }

    #[test]
    fn test_merge_join_ignores_empty_keys() -> Result<()> {
        let ballots = "user_A,content_A\n,bad_content";
        let users = "user_A\n";
        let (result, _, _, _) = run_merge_join_test(ballots, users)?;
        assert_eq!(result, vec!["content_A"]);
        Ok(())
    }

    #[test]
    fn test_merge_join_handles_malformed_csv() -> Result<()> {
        let ballots = "user_A,content_A\nuser_B\nuser_C,content_C";
        let users = "user_A\nuser_C";
        let (result, _, _, _) = run_merge_join_test(ballots, users)?;
        assert_eq!(result, vec!["content_A", "content_C"]);
        Ok(())
    }

    #[test]
    fn test_merge_join_large_scale() -> Result<()> {
        const TOTAL_ENTRIES: i32 = 500;
        const EXPECTED_JOIN_COUNT: usize = (TOTAL_ENTRIES / 2) as usize;

        let mut ballots_csv = String::new();
        let mut users_csv = String::new();

        for i in 0..TOTAL_ENTRIES {
            let user_id = format!("user-{:04}", i);
            ballots_csv.push_str(&format!("{},content_{}\n", user_id, i));
            if i % 2 == 0 {
                users_csv.push_str(&format!("{}\n", user_id));
            }
        }

        let (result, _, _, _) = run_merge_join_test(&ballots_csv, &users_csv)?;

        assert_eq!(result.len(), EXPECTED_JOIN_COUNT);
        assert_eq!(result.first().unwrap(), "content_0");
        assert_eq!(result.last().unwrap(), "content_498");

        Ok(())
    }

    fn run_merge_join_delegates_test(
        ballots_csv: &str,
        voters_csv: &str,
    ) -> Result<(Vec<String>, u64, u64, u64)> {
        let mut ballots_file = NamedTempFile::new()?;
        write!(ballots_file, "{}", ballots_csv)?;
        ballots_file.flush()?;

        let mut voters_file = NamedTempFile::new()?;
        write!(voters_file, "{}", voters_csv)?;
        voters_file.flush()?;

        let ballots_ro = ballots_file.reopen()?;
        let voters_ro = voters_file.reopen()?;

        let (ballot_contents, elegible_voters, ballots_without_voter, casted_ballots) =
            merge_join_csv(
                &ballots_ro,
                &voters_ro,
                0,
                0,
                1,
                Some(1),
            )?;

        Ok((
            ballot_contents,
            elegible_voters,
            ballots_without_voter,
            casted_ballots,
        ))
    }

    #[test]
    fn test_basic_delegate_counts() -> Result<()> {
        let ballots = "\
            user_A,content_A
            user_B,content_B
            user_C,content_C
            user_D,content_D
            user_E,content_E";

        let voters = "\
            user_A,1
            user_B,0
            user_D,3
            user_E,2
            user_F,1";

        let (result, elegible_voters, ballots_without_voter, casted_ballots) =
            run_merge_join_delegates_test(ballots, voters)?;

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
        assert_eq!(ballots_without_voter, 1);
        assert_eq!(casted_ballots, 11);
        assert_eq!(elegible_voters, 5);

        Ok(())
    }

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
        assert_eq!(ballots_without_voter, 2);
        assert_eq!(elegible_voters, 3);
        assert_eq!(casted_ballots, 7);

        Ok(())
    }

    #[test]
    fn test_invalid_delegate_count() -> Result<()> {
        let ballots = "\
            user_A,content_A
            user_G,content_G";

        let voters = "\
            user_A,1
            user_G,not_a_number";

        let (result, elegible_voters, ballots_without_voter, casted_ballots) =
            run_merge_join_delegates_test(ballots, voters)?;

        assert_eq!(result.len(), 2);
        assert_eq!(result, vec!["content_A", "content_A"]);
        assert_eq!(ballots_without_voter, 1);
        assert_eq!(casted_ballots, 3);
        assert_eq!(elegible_voters, 1);

        Ok(())
    }
}
