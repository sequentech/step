// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The result check of `trustee verify --result`: the board's result
//! statement against the votes the transcript's verified decryption yields.

use anyhow::{anyhow, bail, ensure, Context as _, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use protocol_board::verification::TranscriptAnchors;
use sequent_core::ballot::BallotStyle;
use sequent_core::encrypt::hash_ballot_style;
use sequent_core::result_count::count_contest_votes;
use sequent_core::types::result_statement::{
    ResultCounts, ResultStatement, CHECKED_COUNTING_ALGORITHM,
    RESULT_STATEMENT_VERSION,
};
use serde_json::{json, Value};

/// Reads a statement the verifier can check; anything else is an input
/// error, like an unreadable transcript.
pub fn parse_statement(content: &str) -> Result<ResultStatement> {
    let statement: ResultStatement = serde_json::from_str(content)
        .context("the result statement cannot be parsed")?;
    ensure!(
        statement.version == RESULT_STATEMENT_VERSION,
        "unsupported result statement version"
    );
    ensure!(
        statement.contests.len() == 1,
        "the result statement must name exactly one contest"
    );
    for contest in &statement.contests {
        ensure!(
            contest.counting_algorithm == CHECKED_COUNTING_ALGORITHM,
            "the result check supports only the counting algorithm {CHECKED_COUNTING_ALGORITHM}"
        );
    }
    Ok(statement)
}

pub struct ResultCheck {
    pub ballot_style_hash: String,
    pub report: Value,
}

fn compare(
    contest: &str,
    what: &str,
    stated: &ResultCounts,
    counted: &ResultCounts,
) -> Result<()> {
    for (field, stated, counted) in stated.fields(counted) {
        ensure!(
            stated == counted,
            "result statement, contest {contest}: {what} {field} stated {stated}, counted {counted}"
        );
    }
    Ok(())
}

/// Checks `statement` for the board `tally_id` (decimal), whose verified
/// plaintexts (base64, as the report holds them) are `plaintexts`.
pub fn check_result(
    statement: &ResultStatement,
    configuration_hash: &str,
    tally_id: &str,
    anchors: &TranscriptAnchors,
    plaintexts: &[String],
) -> Result<ResultCheck> {
    ensure!(
        statement.configuration_hash.eq_ignore_ascii_case(configuration_hash),
        "result statement names another configuration hash"
    );
    let ballot_style: BallotStyle =
        serde_json::from_value(statement.ballot_style.clone())
            .map_err(|_| anyhow!("the ballot style of the result statement cannot be read"))?;
    let ballot_style_hash = hash_ballot_style(&ballot_style)
        .map_err(|error| anyhow!("ballot style hash: {error:?}"))?;
    let key = ballot_style
        .public_key
        .as_ref()
        .ok_or_else(|| anyhow!("the ballot style of the result statement has no public key"))?;
    ensure!(
        key.public_key == anchors.public_key,
        "the ballot style of the result statement has another public key"
    );
    ensure!(
        key.configuration_id.as_deref()
            == Some(anchors.configuration_id.to_string().as_str()),
        "the ballot style of the result statement names another Configuration"
    );
    let payloads = plaintexts
        .iter()
        .map(|encoded| {
            <[u8; 30]>::try_from(STANDARD.decode(encoded)?.as_slice())
                .map_err(|_| anyhow!("a plaintext is not 30 bytes"))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut contests = vec![];
    for statement_contest in &statement.contests {
        let id = &statement_contest.contest_id;
        let Some(contest) = ballot_style.contests.iter().find(|c| &c.id == id)
        else {
            bail!("result statement, contest {id}: not in its ballot style");
        };
        let mine: Vec<_> = statement_contest
            .batches
            .iter()
            .filter(|batch| batch.tally_id == tally_id)
            .collect();
        ensure!(
            mine.len() == 1,
            "result statement, contest {id}: lists {} batches with the supplied tally identifier, not one",
            mine.len()
        );
        let counted = count_contest_votes(contest, &payloads);
        compare(id, "batch", &mine[0].counts, &counted)?;
        let mut sum = ResultCounts::default();
        for batch in &statement_contest.batches {
            // Voter-weighted and delegated voting, the only sources of a
            // multiplier, are not part of the checked configuration.
            ensure!(
                batch.multiplier == 1,
                "result statement, contest {id}: batch {} has multiplier {}; only 1 is checked",
                batch.tally_id,
                batch.multiplier
            );
            sum.add_scaled(&batch.counts, 1)
                .ok_or_else(|| anyhow!("result statement counts overflow"))?;
        }
        for sheet in &statement_contest.tally_sheets {
            sum.add_scaled(&sheet.counts, 1)
                .ok_or_else(|| anyhow!("result statement counts overflow"))?;
        }
        for (field, stated, parts) in statement_contest.result.fields(&sum) {
            ensure!(
                stated == parts,
                "result statement, contest {id}: result {field} stated {stated}, sum of its parts {parts}"
            );
        }
        contests.push(json!({
            "contest_id": id,
            "stated": mine[0].counts,
            "counted": counted,
            "result": statement_contest.result,
            "sum_of_parts": sum,
        }));
    }
    Ok(ResultCheck {
        ballot_style_hash,
        report: json!({ "contests": contests }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequent_core::ballot::PublicKeyConfig;
    use sequent_core::ballot_codec::PlaintextCodec;
    use sequent_core::fixtures::ballot_codec::get_test_contest;
    use sequent_core::plaintext::{DecodedVoteChoice, DecodedVoteContest};
    use sequent_core::types::result_statement::{
        BatchCounts, ContestStatement,
    };

    const HASH: &str = "ab";
    const TALLY: &str = "42";

    fn anchors() -> TranscriptAnchors {
        TranscriptAnchors {
            protocol_manager: "manager".into(),
            configuration_id: 7,
            public_key: "joint".into(),
        }
    }

    fn vote(selected: Option<usize>) -> String {
        let contest = get_test_contest();
        let decoded = DecodedVoteContest {
            contest_id: contest.id.clone(),
            is_explicit_invalid: false,
            is_decline_to_vote: false,
            is_blank_ballot: false,
            invalid_errors: vec![],
            invalid_alerts: vec![],
            choices: contest
                .candidates
                .iter()
                .enumerate()
                .map(|(index, candidate)| DecodedVoteChoice {
                    id: candidate.id.clone(),
                    selected: if Some(index) == selected { 0 } else { -1 },
                    write_in_text: None,
                })
                .collect(),
        };
        STANDARD.encode(contest.encode_plaintext_contest(&decoded).unwrap())
    }

    fn counts(first: u64, valid: u64) -> ResultCounts {
        let contest = get_test_contest();
        let mut counts = ResultCounts {
            total_valid_votes: valid,
            ..Default::default()
        };
        counts.candidates.insert(contest.candidates[0].id.clone(), first);
        counts
    }

    fn statement(batch: ResultCounts, result: ResultCounts) -> ResultStatement {
        let contest = get_test_contest();
        let ballot_style = json!({
            "id": "style", "tenant_id": "tenant", "election_event_id": "event",
            "election_id": "election", "area_id": "area",
            "public_key": PublicKeyConfig {
                public_key: "joint".into(),
                is_demo: false,
                keys_ceremony_id: None,
                configuration_id: Some("7".into()),
            },
            "contests": [contest.clone()],
        });
        ResultStatement {
            version: 1,
            configuration_hash: HASH.into(),
            election_id: "election".into(),
            area_id: "area".into(),
            ballot_style,
            contests: vec![ContestStatement {
                contest_id: contest.id,
                counting_algorithm: "plurality-at-large".into(),
                result,
                batches: vec![BatchCounts {
                    tally_id: TALLY.into(),
                    multiplier: 1,
                    counts: batch,
                }],
                tally_sheets: vec![],
            }],
        }
    }

    fn run(statement: &ResultStatement) -> Result<ResultCheck> {
        let plaintexts = [vote(Some(0)), vote(Some(0))];
        check_result(statement, HASH, TALLY, &anchors(), &plaintexts)
    }

    #[test]
    fn a_statement_that_matches_the_decrypted_votes_passes() {
        let check = run(&statement(counts(2, 2), counts(2, 2))).unwrap();
        assert_eq!(check.report["contests"][0]["counted"]["total_valid_votes"], 2);
        assert!(!check.ballot_style_hash.is_empty());
    }

    #[test]
    fn an_undecodable_vote_counts_as_an_implicit_invalid_vote() {
        // A length byte beyond the payload: the codec cannot read it, and
        // the tally counts it as an implicit invalid ballot.
        let undecodable = STANDARD.encode([0xff_u8; 30]);
        let plaintexts = [vote(Some(0)), vote(Some(0)), undecodable];
        let mut with_invalid = counts(2, 2);
        with_invalid.implicit_invalid_votes = 1;
        let check = check_result(
            &statement(with_invalid.clone(), with_invalid.clone()),
            HASH,
            TALLY,
            &anchors(),
            &plaintexts,
        )
        .unwrap();
        assert_eq!(
            check.report["contests"][0]["counted"]["implicit_invalid_votes"],
            1
        );
        // A statement that left the undecodable vote out fails.
        let error = check_result(
            &statement(counts(2, 2), counts(2, 2)),
            HASH,
            TALLY,
            &anchors(),
            &plaintexts,
        )
        .err()
        .unwrap()
        .to_string();
        assert!(
            error.contains("batch implicit_invalid_votes stated 0, counted 1"),
            "{error}"
        );
        // So does a result that left it out of the stored count.
        let error = check_result(
            &statement(with_invalid, counts(2, 2)),
            HASH,
            TALLY,
            &anchors(),
            &plaintexts,
        )
        .err()
        .unwrap()
        .to_string();
        assert!(
            error.contains("result implicit_invalid_votes stated 0, sum of its parts 1"),
            "{error}"
        );
    }

    #[test]
    fn batch_counts_that_differ_from_the_votes_fail() {
        let error = run(&statement(counts(3, 3), counts(3, 3)))
            .err()
            .unwrap()
            .to_string();
        assert!(error.contains("batch total_valid_votes stated 3, counted 2"), "{error}");
    }

    #[test]
    fn a_result_that_is_not_the_sum_of_its_parts_fails() {
        let error = run(&statement(counts(2, 2), counts(5, 2)))
            .err()
            .unwrap()
            .to_string();
        assert!(error.contains("stated 5, sum of its parts 2"), "{error}");
    }

    #[test]
    fn another_key_configuration_or_board_fails() {
        let good = statement(counts(2, 2), counts(2, 2));
        let mut other = good.clone();
        other.configuration_hash = "cd".into();
        assert!(run(&other).is_err());
        let mut other = good.clone();
        other.ballot_style["public_key"]["public_key"] = "other".into();
        assert_eq!(
            run(&other).err().unwrap().to_string(),
            "the ballot style of the result statement has another public key"
        );
        let mut other = good.clone();
        other.contests[0].batches[0].tally_id = "1".into();
        assert!(run(&other).is_err());
        let mut other = good;
        other.contests[0].batches[0].multiplier = 2;
        assert!(run(&other)
            .err()
            .unwrap()
            .to_string()
            .contains("multiplier 2; only 1 is checked"));
    }

    #[test]
    fn statements_outside_the_checked_configuration_are_unreadable() {
        let mut other = statement(counts(2, 2), counts(2, 2));
        other.contests[0].counting_algorithm = "instant-runoff".into();
        assert!(parse_statement(&serde_json::to_string(&other).unwrap()).is_err());
        let mut other = statement(counts(2, 2), counts(2, 2));
        other.version = 2;
        assert!(parse_statement(&serde_json::to_string(&other).unwrap()).is_err());
    }
}
