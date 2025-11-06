// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::fixtures::ballot_styles::generate_ballot_style;
use crate::fixtures::TestFixture;
use crate::pipes::pipe_inputs::BALLOTS_FILE;
use anyhow::{Error, Result};
use sequent_core::ballot::*;
use sequent_core::ballot_codec::multi_ballot::{BallotChoices, ContestChoices};
use sequent_core::ballot_codec::BigUIntCodec;
use sequent_core::plaintext::{DecodedVoteChoice, DecodedVoteContest};
use sequent_core::util::voting_screen::{
    check_voting_error_dialog_util, check_voting_not_allowed_next_util, get_contest_plurality,
    get_decoded_contest_plurality,
};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::str::FromStr;
use tracing::instrument;
use uuid::Uuid;

#[instrument(skip_all)]
pub fn generate_ballots(
    fixture: &TestFixture,
    election_num: u32,
    contest_num: u32,
    area_num: u32,
    ballots_num: u32,
) -> Result<()> {
    assert!(
        !(ballots_num > 0 && ballots_num < 20),
        "ballots_num should be at least 20"
    );

    let election_event_id = Uuid::new_v4();

    (0..election_num).try_for_each(|_| {
        let areas: Vec<Uuid> = (0..area_num).map(|_| Uuid::new_v4()).collect();
        let mut election = fixture.create_election_config(&election_event_id, areas)?;
        election.ballot_styles.clear();

        (0..contest_num).try_for_each(|_| {
            let contest = fixture.create_contest_config(
                &election.tenant_id,
                &election_event_id,
                &election.id,
            )?;
            (0..area_num).try_for_each(|index| {
                let area_config = fixture.create_area_config(
                    &election.tenant_id,
                    &election_event_id,
                    &election.id,
                    &Uuid::from_str(&contest.id).unwrap(),
                    100,
                    0,
                    None,
                    election
                        .areas
                        .get(index as usize)
                        .cloned()
                        .map(|val| val.id),
                )?;

                election.ballot_styles.push(generate_ballot_style(
                    &election.tenant_id,
                    &election.election_event_id,
                    &election.id,
                    &area_config.id,
                    vec![contest.clone()],
                ));

                let file = fixture
                    .input_dir_ballots
                    .join(format!("election__{}", &election.id))
                    .join(format!("contest__{}", &contest.id))
                    .join(format!("area__{}", area_config.id));

                if index == 1 {
                    // skip 1 ballot file
                    return Ok(());
                }

                let mut file = fs::OpenOptions::new()
                    .write(true)
                    .append(true)
                    .create(true)
                    .open(file.join(BALLOTS_FILE))?;

                (0..ballots_num).try_for_each(|i| {
                    let mut choices = vec![
                        DecodedVoteChoice {
                            id: "0".to_owned(),
                            selected: -1,
                            write_in_text: None,
                        },
                        DecodedVoteChoice {
                            id: "1".to_owned(),
                            selected: -1,
                            write_in_text: None,
                        },
                        DecodedVoteChoice {
                            id: "2".to_owned(),
                            selected: -1,
                            write_in_text: None,
                        },
                        DecodedVoteChoice {
                            id: "3".to_owned(),
                            selected: -1,
                            write_in_text: None,
                        },
                        DecodedVoteChoice {
                            id: "4".to_owned(),
                            selected: -1,
                            write_in_text: None,
                        },
                    ];

                    let mut plaintext_prepare = DecodedVoteContest {
                        contest_id: contest.id.clone(),
                        is_explicit_invalid: false,
                        invalid_errors: vec![],
                        invalid_alerts: vec![],
                        choices: vec![],
                    };

                    match i {
                        1 => choices[0].selected = 0,
                        2 => choices[1].selected = 0,
                        3 => choices[2].selected = 0,
                        4 => choices[3].selected = 0,
                        5 => choices[4].selected = 0,
                        6 => choices[0].selected = 0,
                        7 => choices[0].selected = 0,
                        8 => choices[3].selected = 0,
                        9 => choices[3].selected = 0,
                        10 => (),
                        11 => (),
                        12 => (),
                        14 => {
                            choices[2].selected = 0;
                            choices[3].selected = 42;
                        }
                        15 => {
                            choices[3].selected = 42;
                        }
                        _ => choices[1].selected = 0,
                    }

                    plaintext_prepare.choices = choices;

                    let plaintext = contest
                        .encode_plaintext_contest_bigint(&plaintext_prepare)
                        .unwrap();

                    writeln!(file, "{}", plaintext)?;

                    Ok::<(), Error>(())
                })?;

                Ok::<(), Error>(())
            })?;

            Ok::<(), Error>(())
        })?;

        Ok::<(), Error>(())
    })?;

    Ok(())
}

#[instrument(skip_all)]
pub fn generate_mcballots(
    fixture: &TestFixture,
    election_num: u32,
    contest_num: u32,
    area_num: u32,
    ballots_num: u32,
) -> Result<()> {
    assert!(
        !(ballots_num > 0 && ballots_num < 20),
        "ballots_num should be at least 20"
    );

    let election_event_id = Uuid::new_v4();

    (0..election_num).try_for_each(|_| {
        let areas: Vec<Uuid> = (0..area_num).map(|_| Uuid::new_v4()).collect();
        let mut election = fixture.create_election_config(&election_event_id, areas)?;
        election.ballot_styles.clear();

        let mut dvcs_by_area: HashMap<(String, u32), Vec<DecodedVoteContest>> = HashMap::new();
        let mut contests: Vec<Contest> = vec![];

        (0..contest_num).try_for_each(|_| {
            let contest = fixture.create_contest_config(
                &election.tenant_id,
                &election_event_id,
                &election.id,
            )?;
            contests.push(contest.clone());

            (0..area_num).try_for_each(|index| {
                let area_config = fixture.create_area_config(
                    &election.tenant_id,
                    &election_event_id,
                    &election.id,
                    &Uuid::from_str(&contest.id).unwrap(),
                    100,
                    0,
                    None,
                    election
                        .areas
                        .get(index as usize)
                        .cloned()
                        .map(|val| val.id),
                )?;

                // create the directory for multi-ballots
                let file = fixture
                    .input_dir_ballots
                    .join(format!("election__{}", &election.id))
                    .join(format!("area__{}", area_config.id));
                fs::create_dir_all(&file)?;

                election.ballot_styles.push(generate_ballot_style(
                    &election.tenant_id,
                    &election.election_event_id,
                    &election.id,
                    &area_config.id,
                    vec![contest.clone()],
                ));

                let file = fixture
                    .input_dir_ballots
                    .join(format!("election__{}", &election.id))
                    .join(format!("contest__{}", &contest.id))
                    .join(format!("area__{}", area_config.id));

                if index == 1 {
                    // skip 1 ballot file
                    return Ok(());
                }

                let mut file = fs::OpenOptions::new()
                    .write(true)
                    .append(true)
                    .create(true)
                    .open(file.join(BALLOTS_FILE))?;

                (0..ballots_num).try_for_each(|i| {
                    // (0..2).try_for_each(|i| {
                    let mut choices = vec![
                        DecodedVoteChoice {
                            id: "0".to_owned(),
                            selected: -1,
                            write_in_text: None,
                        },
                        DecodedVoteChoice {
                            id: "1".to_owned(),
                            selected: -1,
                            write_in_text: None,
                        },
                        DecodedVoteChoice {
                            id: "2".to_owned(),
                            selected: -1,
                            write_in_text: None,
                        },
                        DecodedVoteChoice {
                            id: "3".to_owned(),
                            selected: -1,
                            write_in_text: None,
                        },
                        DecodedVoteChoice {
                            id: "4".to_owned(),
                            selected: -1,
                            write_in_text: None,
                        },
                    ];

                    let mut plaintext_prepare = DecodedVoteContest {
                        contest_id: contest.id.clone(),
                        is_explicit_invalid: false,
                        invalid_errors: vec![],
                        invalid_alerts: vec![],
                        choices: vec![],
                    };

                    match i {
                        1 => choices[0].selected = 0,
                        2 => choices[1].selected = 0,
                        3 => choices[2].selected = 0,
                        4 => choices[3].selected = 0,
                        5 => choices[4].selected = 0,
                        6 => choices[0].selected = 0,
                        7 => choices[0].selected = 0,
                        8 => choices[3].selected = 0,
                        9 => choices[3].selected = 0,
                        10 => (),
                        11 => (),
                        12 => (),
                        14 => {
                            choices[2].selected = 0;
                            // We are not yet testing errors due to more than max allowed votes
                            // choices[3].selected = 42;
                        }
                        15 => {
                            choices[3].selected = 42;
                        }
                        _ => choices[1].selected = 0,
                    }

                    plaintext_prepare.choices = choices;

                    let area_id = area_config.id.to_string();
                    let key = (area_id, i);
                    if let Some(dvcs) = dvcs_by_area.get_mut(&key) {
                        dvcs.push(plaintext_prepare.clone());
                    } else {
                        dvcs_by_area.insert(key, vec![plaintext_prepare.clone()]);
                    }

                    let plaintext = contest
                        .encode_plaintext_contest_bigint(&plaintext_prepare)
                        .unwrap();

                    writeln!(file, "{}", plaintext)?;

                    Ok::<(), Error>(())
                })?;

                Ok::<(), Error>(())
            })?;

            Ok::<(), Error>(())
        })?;

        let mut ballots = vec![];
        for (key, choices) in dvcs_by_area {
            let contest_choices = choices
                .iter()
                .map(ContestChoices::from_decoded_vote_contest)
                .collect();
            let ballot = BallotChoices::new(false, contest_choices);

            let ballot_style = generate_ballot_style(
                &election.tenant_id,
                &election.election_event_id,
                &election.id,
                &Uuid::from_str(&key.0).unwrap(),
                contests.clone(),
            );

            let bigint = ballot.encode_to_bigint(&ballot_style).unwrap();

            ballots.push((key, bigint));
        }

        // This step is not essential, but it makes easier to maintain the order
        // of the ballots in the file to match the generated order
        ballots.sort_by(|a, b| a.0 .1.cmp(&b.0 .1));

        for (key, bigint) in ballots {
            let file = fixture
                .input_dir_ballots
                .join(format!("election__{}", &election.id))
                .join(format!("area__{}", key.0));

            let mut file = fs::OpenOptions::new()
                .write(true)
                .append(true)
                .create(true)
                .open(file.join(BALLOTS_FILE))?;

            writeln!(file, "{}", bigint)?;
        }

        Ok::<(), Error>(())
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::state::State;
    use crate::cli::CliRun;
    use crate::fixtures::ballot_styles::generate_ballot_style;
    use crate::fixtures::TestFixture;
    use crate::pipes::decode_ballots::decode_mcballots;
    use crate::pipes::decode_ballots::OUTPUT_DECODED_BALLOTS_FILE;
    use crate::pipes::do_tally::OUTPUT_CONTEST_RESULT_FILE;
    use crate::pipes::generate_reports::{ReportDataComputed, TemplateData};
    use crate::pipes::mark_winners::OUTPUT_WINNERS;
    use crate::pipes::pipe_inputs::{PREFIX_AREA, PREFIX_CONTEST, PREFIX_ELECTION};
    use crate::pipes::pipe_name::PipeNameOutputDir;
    use anyhow::{Error, Result};
    use sequent_core::ballot_codec::BigUIntCodec;
    use sequent_core::plaintext::{DecodedVoteChoice, DecodedVoteContest};
    use sequent_core::serialization::deserialize_with_path::deserialize_str;
    use sequent_core::util::init_log;
    use std::fs;
    use std::io::Read;
    use std::io::Write;
    use std::str::FromStr;
    use uuid::Uuid;
    use walkdir::WalkDir;

    #[test]
    fn test_create_configs() -> Result<()> {
        let fixture = TestFixture::new()?;

        let election_event_id = Uuid::new_v4();
        let areas: Vec<Uuid> = vec![Uuid::new_v4()];
        let election = fixture.create_election_config(&election_event_id, areas)?;
        let contest =
            fixture.create_contest_config(&election.tenant_id, &election_event_id, &election.id)?;

        let entries = fs::read_dir(&fixture.input_dir_configs)?;
        let count = entries.count();
        assert_eq!(count, 1);

        let entries = fs::read_dir(
            fixture
                .input_dir_configs
                .join(format!("election__{}", election.id)),
        )?;
        let count = entries.count();
        assert_eq!(count, 2);

        let entries = fs::read_dir(
            fixture
                .input_dir_configs
                .join(format!("election__{}", election.id))
                .join(format!("contest__{}", contest.id)),
        )?;
        let count = entries.count();
        assert_eq!(count, 1);

        Ok(())
    }

    #[test]
    fn test_create_ballots() -> Result<()> {
        let fixture = TestFixture::new()?;

        generate_ballots(&fixture, 5, 10, 3, 20)?;

        // count elections
        let entries = fs::read_dir(&fixture.input_dir_ballots)?;
        let count = entries.count();
        assert_eq!(count, 5);

        // count contests
        let mut entries = fs::read_dir(&fixture.input_dir_ballots)?;
        let entry = entries.next().unwrap()?;
        let contest_path = entry.path();
        let election_uuid = contest_path.components().last().unwrap();
        let entries = fs::read_dir(&contest_path)?;
        let count = entries.count();
        assert_eq!(count, 10);

        // count count areas
        let mut entries = fs::read_dir(fixture.input_dir_ballots.join(election_uuid))?;
        let entry = entries.next().unwrap()?;
        let contest_path = entry.path();
        let entries = fs::read_dir(contest_path)?;
        let count = entries.count();

        assert_eq!(count, 3);

        Ok(())
    }

    #[test]
    fn test_create_multi_ballots() -> Result<()> {
        let fixture = TestFixture::new()?;

        generate_mcballots(&fixture, 5, 10, 3, 20)?;

        // count elections
        let entries = fs::read_dir(&fixture.input_dir_ballots)?;
        let count = entries.count();
        assert_eq!(count, 5);

        // count contests
        let mut entries = fs::read_dir(&fixture.input_dir_ballots)?;
        let entry = entries.next().unwrap()?;
        let contest_path = entry.path();
        let election_uuid = contest_path.components().last().unwrap();
        let entries = fs::read_dir(&contest_path)?;
        let count = entries.count();
        // 13 = 10 contests + 3 areas (with multi contest ballots)
        assert_eq!(count, 13);

        Ok(())
    }

    #[test]
    fn test_pipes_exec_mcballots() -> Result<()> {
        //sequent_core::util::init_log::init_log(true);

        let election_num = 5;
        let contest_num = 10;
        let area_num = 3;
        let ballot_num = 20;

        let fixture = TestFixture::new_mc()?;

        generate_mcballots(&fixture, election_num, contest_num, area_num, ballot_num)?;

        let cli = CliRun {
            stage: "main".to_string(),
            pipe_id: "decode-ballots".to_string(),
            config: fixture.config_path.clone(),
            input_dir: fixture.root_dir.join("tests").join("input-dir"),
            output_dir: fixture.root_dir.join("tests").join("output-dir"),
        };

        let config = cli.validate()?;
        let mut state = State::new(&cli, &config)?;

        // DecodeBallots
        state.exec_next()?;

        // DecodeMCBallots
        state.exec_next()?;

        // Currently DecodeMCBallots outputs to both PipeNameOutputDir::DecodeBallots and
        // PipeNameOutputDir::DecodeMCBallots
        // So we do both checks here
        let output_dir = cli
            .output_dir
            .join(PipeNameOutputDir::DecodeBallots.as_ref());

        assert!(output_dir.exists());

        assert_eq!(
            WalkDir::new(output_dir.as_path())
                .into_iter()
                .filter_map(Result::ok)
                .filter(|e| {
                    e.path()
                        .file_name()
                        .map_or(false, |f| f == OUTPUT_DECODED_BALLOTS_FILE)
                })
                .count() as u32,
            election_num * contest_num * (area_num - 1)
        );

        let output_dir = cli
            .output_dir
            .join(PipeNameOutputDir::DecodeMCBallots.as_ref());

        assert!(output_dir.exists());

        assert_eq!(
            WalkDir::new(output_dir.as_path())
                .into_iter()
                .filter_map(Result::ok)
                .filter(|e| {
                    e.path().file_name().map_or(false, |f| {
                        f == decode_mcballots::OUTPUT_DECODED_BALLOTS_FILE
                    })
                })
                .count() as u32,
            election_num * (area_num - 1)
        );

        // VoteReceipts
        state.exec_next()?;

        // MultiBallotReceipts
        state.exec_next()?;

        // DoTally
        state.exec_next()?;

        assert_eq!(
            WalkDir::new(cli.output_dir.as_path())
                .into_iter()
                .filter_map(Result::ok)
                .filter(|e| {
                    e.path()
                        .file_name()
                        .map_or(false, |f| f == OUTPUT_CONTEST_RESULT_FILE)
                })
                .count() as u32,
            election_num * contest_num * area_num + election_num * contest_num
        );

        // MarkWinners
        state.exec_next()?;

        assert_eq!(
            WalkDir::new(cli.output_dir.as_path())
                .into_iter()
                .filter_map(Result::ok)
                .filter(|e| { e.path().file_name().map_or(false, |f| f == OUTPUT_WINNERS) })
                .count() as u32,
            election_num * contest_num * area_num + election_num * contest_num
        );

        // Generate reports
        state.exec_next()?;

        // Generate database
        state.exec_next()?;

        Ok(())
    }

    #[test]
    fn test_pipes_exec() -> Result<()> {
        sequent_core::util::init_log::init_log(true);

        let election_num = 5;
        let contest_num = 10;
        let area_num = 3;
        let ballot_num = 20;

        let fixture = TestFixture::new()?;

        generate_ballots(&fixture, election_num, contest_num, area_num, ballot_num)?;

        let cli = CliRun {
            stage: "main".to_string(),
            pipe_id: "decode-ballots".to_string(),
            config: fixture.config_path.clone(),
            input_dir: fixture.root_dir.join("tests").join("input-dir"),
            output_dir: fixture.root_dir.join("tests").join("output-dir"),
        };

        let config = cli.validate()?;
        let mut state = State::new(&cli, &config)?;

        // DecodeBallots
        state.exec_next()?;

        assert!(cli
            .output_dir
            .join(PipeNameOutputDir::DecodeBallots.as_ref())
            .exists());

        assert_eq!(
            WalkDir::new(cli.output_dir.as_path())
                .into_iter()
                .filter_map(Result::ok)
                .filter(|e| {
                    e.path()
                        .file_name()
                        .map_or(false, |f| f == OUTPUT_DECODED_BALLOTS_FILE)
                })
                .count() as u32,
            election_num * contest_num * (area_num - 1)
        );

        // VoteReceipts
        state.exec_next()?;

        // DoTally
        state.exec_next()?;

        assert_eq!(
            WalkDir::new(cli.output_dir.as_path())
                .into_iter()
                .filter_map(Result::ok)
                .filter(|e| {
                    e.path()
                        .file_name()
                        .map_or(false, |f| f == OUTPUT_CONTEST_RESULT_FILE)
                })
                .count() as u32,
            election_num * contest_num * area_num + election_num * contest_num
        );

        // MarkWinners
        state.exec_next()?;

        assert_eq!(
            WalkDir::new(cli.output_dir.as_path())
                .into_iter()
                .filter_map(Result::ok)
                .filter(|e| { e.path().file_name().map_or(false, |f| f == OUTPUT_WINNERS) })
                .count() as u32,
            election_num * contest_num * area_num + election_num * contest_num
        );

        Ok(())
    }

    #[test]
    fn test_generate_reports_normal() -> Result<()> {
        let fixture = TestFixture::new()?;

        generate_ballots(&fixture, 1, 2, 1, 50)?;

        let cli = CliRun {
            stage: "main".to_string(),
            pipe_id: "decode-ballots".to_string(),
            config: fixture.config_path.clone(),
            input_dir: fixture.root_dir.join("tests").join("input-dir"),
            output_dir: fixture.root_dir.join("tests").join("output-dir"),
        };

        let config = cli.validate()?;
        let mut state = State::new(&cli, &config)?;

        // DecodeBallots
        state.exec_next()?;

        // VoteReceipts
        state.exec_next()?;

        // DoTally
        state.exec_next()?;

        // MarkWinners
        state.exec_next()?;

        // Generate reports
        state.exec_next()?;

        // Generate database
        state.exec_next()?;

        Ok(())
    }

    #[test]
    fn test_generate_reports_without_ballots() -> Result<()> {
        // sequent_core::util::init_log::init_log(true);

        let fixture = TestFixture::new()?;

        generate_ballots(&fixture, 1, 2, 1, 0)?;

        let cli = CliRun {
            stage: "main".to_string(),
            pipe_id: "decode-ballots".to_string(),
            config: fixture.config_path.clone(),
            input_dir: fixture.root_dir.join("tests").join("input-dir"),
            output_dir: fixture.root_dir.join("tests").join("output-dir"),
        };

        let config = cli.validate()?;
        let mut state = State::new(&cli, &config)?;

        // DecodeBallots
        state.exec_next()?;

        // VoteReceipts
        state.exec_next()?;

        // DoTally
        state.exec_next()?;

        // MarkWinners
        state.exec_next()?;

        // Generate reports
        state.exec_next()?;

        // Generate database
        state.exec_next()?;

        Ok(())
    }

    #[test]
    fn test_results() -> Result<()> {
        let fixture = TestFixture::new()?;

        let election_event_id = Uuid::new_v4();
        let areas: Vec<Uuid> = vec![Uuid::new_v4(), Uuid::new_v4()];

        let mut election = fixture.create_election_config(&election_event_id, areas)?;
        election.ballot_styles.clear();

        // First ballot style
        let contest =
            fixture.create_contest_config(&election.tenant_id, &election_event_id, &election.id)?;
        let first_contest_id = contest.id.clone();

        // first area
        let area_config = fixture.create_area_config(
            &election.tenant_id,
            &election_event_id,
            &election.id,
            &Uuid::from_str(&contest.id).unwrap(),
            100,
            0,
            None,
            election.areas.first().cloned().map(|val| val.id),
        )?;
        let first_area_id = area_config.id;

        election.ballot_styles.push(generate_ballot_style(
            &election.tenant_id,
            &election.election_event_id,
            &election.id,
            &area_config.id,
            vec![contest.clone()],
        ));

        let ballot_file = fixture
            .input_dir_ballots
            .join(format!("election__{}", &election.id))
            .join(format!("contest__{}", &contest.id))
            .join(format!("area__{}", area_config.id));

        let mut file = fs::OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(ballot_file.join("ballots.csv"))?;

        (0..100).try_for_each(|i| {
            let mut choices = vec![
                DecodedVoteChoice {
                    id: "0".to_owned(),
                    selected: -1,
                    write_in_text: None,
                },
                DecodedVoteChoice {
                    id: "1".to_owned(),
                    selected: -1,
                    write_in_text: None,
                },
                DecodedVoteChoice {
                    id: "2".to_owned(),
                    selected: -1,
                    write_in_text: None,
                },
                DecodedVoteChoice {
                    id: "3".to_owned(),
                    selected: -1,
                    write_in_text: None,
                },
                DecodedVoteChoice {
                    id: "4".to_owned(),
                    selected: -1,
                    write_in_text: None,
                },
            ];

            let mut plaintext_prepare = DecodedVoteContest {
                contest_id: contest.id.clone(),
                is_explicit_invalid: false,
                invalid_errors: vec![],
                invalid_alerts: vec![],
                choices: vec![],
            };

            match i {
                1 => choices[0].selected = 0,
                2 => choices[1].selected = 0,
                3 => choices[2].selected = 0,
                4 => choices[3].selected = 0,
                5 => choices[4].selected = 0,
                6 => choices[0].selected = 0,
                7 => choices[0].selected = 0,
                8 => choices[3].selected = 0,
                9 => choices[3].selected = 0,
                10 => (),
                11 => (),
                12 => (),
                14 => {
                    choices[2].selected = 0;
                    choices[3].selected = 42;
                }
                15 => {
                    choices[3].selected = 42;
                }
                _ => choices[1].selected = 0,
            }

            plaintext_prepare.choices = choices;

            let plaintext = contest
                .encode_plaintext_contest_bigint(&plaintext_prepare)
                .unwrap();

            writeln!(file, "{}", plaintext)?;

            Ok::<(), Error>(())
        })?;

        // second area
        let area_config = fixture.create_area_config(
            &election.tenant_id,
            &election_event_id,
            &election.id,
            &Uuid::from_str(&contest.id).unwrap(),
            100,
            0,
            None,
            election.areas.get(1).cloned().map(|val| val.id),
        )?;
        election.ballot_styles.push(generate_ballot_style(
            &election.tenant_id,
            &election.election_event_id,
            &election.id,
            &area_config.id,
            vec![contest.clone()],
        ));

        let ballot_file = fixture
            .input_dir_ballots
            .join(format!("election__{}", &election.id))
            .join(format!("contest__{}", &contest.id))
            .join(format!("area__{}", &area_config.id));

        let mut file = fs::OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(ballot_file.join("ballots.csv"))?;

        (0..42).try_for_each(|i| {
            let mut choices = vec![
                DecodedVoteChoice {
                    id: "0".to_owned(),
                    selected: -1,
                    write_in_text: None,
                },
                DecodedVoteChoice {
                    id: "1".to_owned(),
                    selected: -1,
                    write_in_text: None,
                },
                DecodedVoteChoice {
                    id: "2".to_owned(),
                    selected: -1,
                    write_in_text: None,
                },
                DecodedVoteChoice {
                    id: "3".to_owned(),
                    selected: -1,
                    write_in_text: None,
                },
                DecodedVoteChoice {
                    id: "4".to_owned(),
                    selected: -1,
                    write_in_text: None,
                },
            ];

            let mut plaintext_prepare = DecodedVoteContest {
                contest_id: contest.id.clone(),
                is_explicit_invalid: false,
                invalid_errors: vec![],
                invalid_alerts: vec![],
                choices: vec![],
            };

            match i {
                1 => choices[0].selected = 0,
                2 => choices[1].selected = 0,
                3 => choices[2].selected = 0,
                4 => choices[3].selected = 0,
                5 => choices[4].selected = 0,
                6 => choices[0].selected = 0,
                7 => choices[0].selected = 0,
                8 => choices[3].selected = 0,
                9 => choices[3].selected = 0,
                10 => (),
                11 => (),
                12 => (),
                14 => {
                    choices[2].selected = 0;
                    choices[3].selected = 42;
                }
                15 => {
                    choices[3].selected = 42;
                }
                _ => choices[1].selected = 0,
            }

            plaintext_prepare.choices = choices;

            let plaintext = contest
                .encode_plaintext_contest_bigint(&plaintext_prepare)
                .unwrap();

            writeln!(file, "{}", plaintext)?;

            Ok::<(), Error>(())
        })?;

        // Second ballot style
        let contest =
            fixture.create_contest_config(&election.tenant_id, &election_event_id, &election.id)?;
        let second_contest_id = contest.id.clone();

        let area_config = fixture.create_area_config(
            &election.tenant_id,
            &election_event_id,
            &election.id,
            &Uuid::from_str(&contest.id).unwrap(),
            100,
            0,
            None,
            election.areas.first().cloned().map(|val| val.id),
        )?;

        election.ballot_styles.push(generate_ballot_style(
            &election.tenant_id,
            &election.election_event_id,
            &election.id,
            &area_config.id,
            vec![contest.clone()],
        ));

        let ballot_file = fixture
            .input_dir_ballots
            .join(format!("election__{}", &election.id))
            .join(format!("contest__{}", &contest.id))
            .join(format!("area__{}", area_config.id));

        let mut file = fs::OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(ballot_file.join("ballots.csv"))?;

        (0..20).try_for_each(|i| {
            let mut choices = vec![
                DecodedVoteChoice {
                    id: "0".to_owned(),
                    selected: -1,
                    write_in_text: None,
                },
                DecodedVoteChoice {
                    id: "1".to_owned(),
                    selected: -1,
                    write_in_text: None,
                },
                DecodedVoteChoice {
                    id: "2".to_owned(),
                    selected: -1,
                    write_in_text: None,
                },
                DecodedVoteChoice {
                    id: "3".to_owned(),
                    selected: -1,
                    write_in_text: None,
                },
                DecodedVoteChoice {
                    id: "4".to_owned(),
                    selected: -1,
                    write_in_text: None,
                },
            ];

            let mut plaintext_prepare = DecodedVoteContest {
                contest_id: contest.id.clone(),
                is_explicit_invalid: false,
                invalid_errors: vec![],
                invalid_alerts: vec![],
                choices: vec![],
            };

            match i {
                1 => choices[0].selected = 0,
                2 => choices[1].selected = 0,
                3 => choices[2].selected = 0,
                4 => choices[3].selected = 0,
                5 => choices[4].selected = 0,
                6 => choices[0].selected = 0,
                7 => choices[0].selected = 0,
                8 => choices[3].selected = 0,
                9 => choices[3].selected = 0,
                10 => (),
                11 => (),
                12 => (),
                14 => {
                    choices[2].selected = 0;
                    choices[3].selected = 42;
                }
                15 => {
                    choices[3].selected = 42;
                }
                _ => choices[1].selected = 0,
            }

            plaintext_prepare.choices = choices;

            let plaintext = contest
                .encode_plaintext_contest_bigint(&plaintext_prepare)
                .unwrap();

            writeln!(file, "{}", plaintext)?;

            Ok::<(), Error>(())
        })?;

        let cli = CliRun {
            stage: "main".to_string(),
            pipe_id: "decode-ballots".to_string(),
            config: fixture.config_path.clone(),
            input_dir: fixture.root_dir.join("tests").join("input-dir"),
            output_dir: fixture.root_dir.join("tests").join("output-dir"),
        };
        println!("cli config: {:?}", cli);

        let config = cli.validate()?;
        let mut state = State::new(&cli, &config)?;

        // DecodeBallots
        state.exec_next()?;

        // VoteReceipts
        state.exec_next()?;

        // DoTally
        state.exec_next()?;

        // MarkWinners
        state.exec_next()?;

        // Generate reports
        state.exec_next()?;

        // Generate database
        state.exec_next()?;

        // test first contest
        let mut path = cli.output_dir.clone();
        path.push("velvet-generate-reports");
        path.push(format!("{}{}", PREFIX_ELECTION, &election.id));
        path.push(format!("{}{}", PREFIX_CONTEST, &first_contest_id));
        path.push("report.json");

        let f = fs::File::open(&path)?;

        let reports: TemplateData = serde_json::from_reader(f)?;
        let report = &reports.reports[0];

        assert_eq!(report.contest_result.total_votes, 142, "total_votes");
        assert_eq!(
            report.contest_result.total_invalid_votes, 2,
            "total_invalid_votes"
        );
        assert_eq!(report.contest_result.auditable_votes, 0, "auditable_votes");
        assert_eq!(
            report.contest_result.total_valid_votes, 140,
            "total_valid_votes"
        );
        assert_eq!(
            report.contest_result.total_blank_votes, 6,
            "total_blank_votes"
        );
        assert_eq!(report.contest_result.census, 200, "census");
        assert_eq!(
            report
                .candidate_result
                .iter()
                .map(|cr| cr.total_count)
                .sum::<u64>(),
            134,
            "sum candidates"
        );

        let mut path = cli.output_dir.clone();
        path.push("velvet-generate-reports");
        path.push(format!("{}{}", PREFIX_ELECTION, &election.id));
        path.push(format!("{}{}", PREFIX_CONTEST, &first_contest_id));
        path.push(format!("{}{}", PREFIX_AREA, &first_area_id));
        path.push("report.json");

        let f = fs::File::open(&path)?;

        let reports: TemplateData = serde_json::from_reader(f)?;
        let report = &reports.reports[0];

        assert_eq!(report.contest_result.total_votes, 100);
        assert_eq!(
            report.contest_result.total_invalid_votes, 1,
            "total_invalid_votes"
        );
        assert_eq!(report.contest_result.total_valid_votes, 99);
        assert_eq!(report.contest_result.total_blank_votes, 3);
        assert_eq!(report.contest_result.census, 100);
        assert_eq!(
            report
                .candidate_result
                .iter()
                .map(|cr| cr.total_count)
                .sum::<u64>(),
            96
        );

        // test second contest

        let mut path = cli.output_dir.clone();
        path.push("velvet-generate-reports");
        path.push(format!("{}{}", PREFIX_ELECTION, &election.id));
        path.push(format!("{}{}", PREFIX_CONTEST, &second_contest_id));
        path.push("report.json");

        let f = fs::File::open(&path)?;

        let reports: TemplateData = serde_json::from_reader(f)?;
        let report = &reports.reports[0];

        assert_eq!(report.contest_result.total_votes, 20);
        assert_eq!(report.contest_result.total_valid_votes, 19);
        assert_eq!(report.contest_result.total_blank_votes, 3);
        assert_eq!(report.contest_result.total_invalid_votes, 1);
        assert_eq!(report.contest_result.census, 100);
        assert_eq!(
            report
                .candidate_result
                .iter()
                .map(|cr| cr.total_count)
                .sum::<u64>(),
            16
        );

        Ok(())
    }

    #[test]
    fn test_0_ballots() -> Result<()> {
        let fixture = TestFixture::new()?;

        let election_event_id = Uuid::new_v4();
        let areas: Vec<Uuid> = vec![Uuid::new_v4()];

        let mut election = fixture.create_election_config(&election_event_id, areas)?;
        election.ballot_styles.clear();

        // First ballot style
        let contest =
            fixture.create_contest_config(&election.tenant_id, &election_event_id, &election.id)?;

        // first area
        let area_config = fixture.create_area_config(
            &election.tenant_id,
            &election_event_id,
            &election.id,
            &Uuid::from_str(&contest.id).unwrap(),
            100,
            0,
            None,
            election.areas.first().cloned().map(|val| val.id),
        )?;
        election.ballot_styles.push(generate_ballot_style(
            &election.tenant_id,
            &election.election_event_id,
            &election.id,
            &area_config.id,
            vec![contest.clone()],
        ));

        let ballot_file = fixture
            .input_dir_ballots
            .join(format!("election__{}", &election.id))
            .join(format!("contest__{}", &contest.id))
            .join(format!("area__{}", area_config.id));

        let mut file = fs::OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(ballot_file.join("ballots.csv"))?;

        writeln!(file)?;

        let cli = CliRun {
            stage: "main".to_string(),
            pipe_id: "decode-ballots".to_string(),
            config: fixture.config_path.clone(),
            input_dir: fixture.root_dir.join("tests").join("input-dir"),
            output_dir: fixture.root_dir.join("tests").join("output-dir"),
        };

        let config = cli.validate()?;
        let mut state = State::new(&cli, &config)?;

        // DecodeBallots
        state.exec_next()?;

        // VoteReceipts
        state.exec_next()?;

        // DoTally
        state.exec_next()?;

        // MarkWinners
        state.exec_next()?;

        // Generate reports
        state.exec_next()?;

        // Generate database
        state.exec_next()?;

        let mut path = cli.output_dir.clone();
        path.push("velvet-generate-reports");
        path.push(format!("{}{}", PREFIX_ELECTION, &election.id));
        path.push(format!("{}{}", PREFIX_CONTEST, &contest.id));
        path.push(format!("{}{}", PREFIX_AREA, &area_config.id));
        path.push("report.json");

        let mut f = fs::File::open(&path)?;
        let mut buffer = String::new();
        f.read_to_string(&mut buffer)?;
        println!("{}", buffer);

        let reports: TemplateData = deserialize_str(&buffer)?;
        let report = &reports.reports[0];

        assert_eq!(report.contest_result.total_votes, 0);
        assert_eq!(report.contest_result.census, 100);
        assert_eq!(
            report
                .candidate_result
                .iter()
                .map(|cr| cr.total_count)
                .sum::<u64>(),
            0
        );

        Ok(())
    }

    #[test]
    fn test_blank_votes() -> Result<()> {
        let fixture = TestFixture::new()?;

        let election_event_id = Uuid::new_v4();
        let areas: Vec<Uuid> = vec![Uuid::new_v4()];

        let mut election = fixture.create_election_config(&election_event_id, areas)?;
        election.ballot_styles.clear();

        // First ballot style
        let contest =
            fixture.create_contest_config(&election.tenant_id, &election_event_id, &election.id)?;

        // first area
        let area_config = fixture.create_area_config(
            &election.tenant_id,
            &election_event_id,
            &election.id,
            &Uuid::from_str(&contest.id).unwrap(),
            100,
            0,
            None,
            election.areas.first().cloned().map(|val| val.id),
        )?;

        election.ballot_styles.push(generate_ballot_style(
            &election.tenant_id,
            &election.election_event_id,
            &election.id,
            &area_config.id,
            vec![contest.clone()],
        ));

        let ballot_file = fixture
            .input_dir_ballots
            .join(format!("election__{}", &election.id))
            .join(format!("contest__{}", &contest.id))
            .join(format!("area__{}", area_config.id));

        let mut file = fs::OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(ballot_file.join("ballots.csv"))?;

        for i in 0..10 {
            let mut choices = vec![
                DecodedVoteChoice {
                    id: "0".to_owned(),
                    selected: -1,
                    write_in_text: None,
                },
                DecodedVoteChoice {
                    id: "1".to_owned(),
                    selected: -1,
                    write_in_text: None,
                },
                DecodedVoteChoice {
                    id: "2".to_owned(),
                    selected: -1,
                    write_in_text: None,
                },
                DecodedVoteChoice {
                    id: "3".to_owned(),
                    selected: -1,
                    write_in_text: None,
                },
                DecodedVoteChoice {
                    id: "4".to_owned(),
                    selected: -1,
                    write_in_text: None,
                },
            ];

            let mut plaintext_prepare = DecodedVoteContest {
                contest_id: contest.id.clone(),
                is_explicit_invalid: false,
                invalid_errors: vec![],
                invalid_alerts: vec![],
                choices: vec![],
            };

            if i < 5 {
                choices[0].selected = 0;
            }

            plaintext_prepare.choices = choices;

            let plaintext = contest
                .encode_plaintext_contest_bigint(&plaintext_prepare)
                .unwrap();

            writeln!(file, "{}", plaintext)?;
        }

        let cli = CliRun {
            stage: "main".to_string(),
            pipe_id: "decode-ballots".to_string(),
            config: fixture.config_path.clone(),
            input_dir: fixture.root_dir.join("tests").join("input-dir"),
            output_dir: fixture.root_dir.join("tests").join("output-dir"),
        };

        let config = cli.validate()?;
        let mut state = State::new(&cli, &config)?;

        // DecodeBallots
        state.exec_next()?;

        // VoteReceipts
        state.exec_next()?;

        // DoTally
        state.exec_next()?;

        // MarkWinners
        state.exec_next()?;

        // Generate reports
        state.exec_next()?;

        // Generate database
        state.exec_next()?;

        let mut path = cli.output_dir.clone();
        path.push("velvet-generate-reports");
        path.push(format!("{}{}", PREFIX_ELECTION, &election.id));
        path.push(format!("{}{}", PREFIX_CONTEST, &contest.id));
        path.push(format!("{}{}", PREFIX_AREA, &area_config.id));
        path.push("report.json");

        let f = fs::File::open(&path)?;

        let reports: TemplateData = serde_json::from_reader(f)?;
        let report = &reports.reports[0];

        assert_eq!(report.contest_result.total_votes, 10);
        assert_eq!(report.contest_result.census, 100);
        assert_eq!(
            report
                .candidate_result
                .iter()
                .map(|cr| cr.total_count)
                .sum::<u64>(),
            5
        );

        assert_eq!(report.contest_result.total_blank_votes, 5);
        assert_eq!(report.contest_result.total_valid_votes, 10);
        assert_eq!(report.contest_result.total_invalid_votes, 0);

        Ok(())
    }

    #[test]
    fn test_blank_votes_implicit_invalid() -> Result<()> {
        let fixture = TestFixture::new()?;

        let election_event_id = Uuid::new_v4();
        let areas: Vec<Uuid> = vec![Uuid::new_v4()];

        let mut election = fixture.create_election_config(&election_event_id, areas)?;
        election.ballot_styles.clear();

        // First ballot style
        let contest = fixture.create_contest_config_with_min_max_votes(
            &election.tenant_id,
            &election_event_id,
            &election.id,
            1,
            1,
        )?;

        // first area
        let area_config = fixture.create_area_config(
            &election.tenant_id,
            &election_event_id,
            &election.id,
            &Uuid::from_str(&contest.id).unwrap(),
            100,
            0,
            None,
            election.areas.first().cloned().map(|val| val.id),
        )?;

        election.ballot_styles.push(generate_ballot_style(
            &election.tenant_id,
            &election.election_event_id,
            &election.id,
            &area_config.id,
            vec![contest.clone()],
        ));

        let ballot_file = fixture
            .input_dir_ballots
            .join(format!("election__{}", &election.id))
            .join(format!("contest__{}", &contest.id))
            .join(format!("area__{}", area_config.id));

        let mut file = fs::OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(ballot_file.join("ballots.csv"))?;

        for i in 0..10 {
            let mut choices = vec![
                DecodedVoteChoice {
                    id: "0".to_owned(),
                    selected: -1,
                    write_in_text: None,
                },
                DecodedVoteChoice {
                    id: "1".to_owned(),
                    selected: -1,
                    write_in_text: None,
                },
                DecodedVoteChoice {
                    id: "2".to_owned(),
                    selected: -1,
                    write_in_text: None,
                },
                DecodedVoteChoice {
                    id: "3".to_owned(),
                    selected: -1,
                    write_in_text: None,
                },
                DecodedVoteChoice {
                    id: "4".to_owned(),
                    selected: -1,
                    write_in_text: None,
                },
            ];

            let mut plaintext_prepare = DecodedVoteContest {
                contest_id: contest.id.clone(),
                is_explicit_invalid: false,
                invalid_errors: vec![],
                invalid_alerts: vec![],
                choices: vec![],
            };

            if i < 5 {
                choices[0].selected = 0;
            } // else we don't select any => blank vote

            plaintext_prepare.choices = choices;

            let plaintext = contest
                .encode_plaintext_contest_bigint(&plaintext_prepare)
                .unwrap();

            writeln!(file, "{}", plaintext)?;
        }

        let cli = CliRun {
            stage: "main".to_string(),
            pipe_id: "decode-ballots".to_string(),
            config: fixture.config_path.clone(),
            input_dir: fixture.root_dir.join("tests").join("input-dir"),
            output_dir: fixture.root_dir.join("tests").join("output-dir"),
        };

        let config = cli.validate()?;
        let mut state = State::new(&cli, &config)?;

        // DecodeBallots
        state.exec_next()?;

        // VoteReceipts
        state.exec_next()?;

        // DoTally
        state.exec_next()?;

        // MarkWinners
        state.exec_next()?;

        // Generate reports
        state.exec_next()?;

        // Generate database
        state.exec_next()?;

        let mut path = cli.output_dir.clone();
        path.push("velvet-generate-reports");
        path.push(format!("{}{}", PREFIX_ELECTION, &election.id));
        path.push(format!("{}{}", PREFIX_CONTEST, &contest.id));
        path.push(format!("{}{}", PREFIX_AREA, &area_config.id));
        path.push("report.json");

        let f = fs::File::open(&path)?;

        let reports: TemplateData = serde_json::from_reader(f)?;
        let report = &reports.reports[0];

        assert_eq!(report.contest_result.census, 100);
        assert_eq!(
            report
                .candidate_result
                .iter()
                .map(|cr| cr.total_count)
                .sum::<u64>(),
            5
        );

        assert_eq!(report.contest_result.total_votes, 10);
        assert_eq!(report.contest_result.total_blank_votes, 0);
        assert_eq!(report.contest_result.total_valid_votes, 5);
        assert_eq!(report.contest_result.total_invalid_votes, 5);

        Ok(())
    }

    #[test]
    fn test_hierarchical_area_aggregation() -> Result<()> {
        // Step 1: Creating Election event, election, contest, 2 areas with
        // parent-child relation and connect contest to parent area
        let fixture = TestFixture::new()?;

        let election_event_id = Uuid::new_v4();
        let parent_area_id = Uuid::new_v4();
        let child_area_id = Uuid::new_v4();
        let area_ids: Vec<Uuid> = vec![parent_area_id.clone(), child_area_id.clone()];

        let mut election = fixture.create_election_config_2(
            &election_event_id,
            vec![
                (child_area_id, Some(parent_area_id)),
                (parent_area_id, None),
            ],
        )?;
        election.ballot_styles.clear();

        let contest =
            fixture.create_contest_config(&election.tenant_id, &election_event_id, &election.id)?;

        // Create hierarchical area structure and associate parent area with the
        // contest
        let parent_area_config = fixture
            .create_area_config(
                &election.tenant_id,
                &election_event_id,
                &election.id,
                &Uuid::from_str(&contest.id).unwrap(),
                150,
                0,
                None,
                Some(parent_area_id.to_string()),
            )
            .unwrap();
        let child_area_config = fixture
            .create_area_config(
                &election.tenant_id,
                &election_event_id,
                &election.id,
                &Uuid::from_str(&contest.id).unwrap(),
                100,
                0,
                Some(parent_area_id.clone()),
                Some(child_area_id.to_string()),
            )
            .unwrap();
        let areas_config = vec![parent_area_config.clone(), child_area_config.clone()];

        // TODO: what's the use of this?
        election.ballot_styles.push(generate_ballot_style(
            &election.tenant_id,
            &election.election_event_id,
            &election.id,
            &parent_area_config.id,
            vec![contest.clone()],
        ));
        election.ballot_styles.push(generate_ballot_style(
            &election.tenant_id,
            &election.election_event_id,
            &election.id,
            &child_area_config.id,
            vec![contest.clone()],
        ));

        // Step 2: Create 10 votes for each area
        for i in 0..2 {
            println!(
                " ----- i {} Area {} Contest {}",
                i, areas_config[i].id, contest.id
            );
            let ballots_path = fixture
                .input_dir_ballots
                .join(format!("election__{}", &election.id))
                .join(format!("contest__{}", contest.id))
                .join(format!("area__{}", areas_config[i].id))
                .join("ballots.csv");
            println!("ballots_path={ballots_path:?}");

            let mut ballots_csv_file = fs::OpenOptions::new()
                .write(true)
                .append(true)
                .create(true)
                .open(ballots_path)?;

            (0..10).try_for_each(|ballot_num| {
                let choices = vec![
                    DecodedVoteChoice {
                        id: "0".to_owned(),
                        selected: if ballot_num % 5 == 0 { 0 } else { -1 },
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: "1".to_owned(),
                        selected: if ballot_num % 5 == 1 { 0 } else { -1 },
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: "2".to_owned(),
                        selected: if ballot_num % 5 == 2 { 0 } else { -1 },
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: "3".to_owned(),
                        selected: if ballot_num % 5 == 3 { 0 } else { -1 },
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: "4".to_owned(),
                        selected: if ballot_num % 5 == 4 { 0 } else { -1 },
                        write_in_text: None,
                    },
                ];

                let plaintext_prepare = DecodedVoteContest {
                    contest_id: contest.id.clone(),
                    is_explicit_invalid: false,
                    invalid_errors: vec![],
                    invalid_alerts: vec![],
                    choices: choices,
                };

                let plaintext = contest
                    .encode_plaintext_contest_bigint(&plaintext_prepare)
                    .unwrap();
                writeln!(ballots_csv_file, "{}", plaintext)?;

                Ok::<(), Error>(())
            })?;
        }

        // Step 3: Generate tallies and test expected results
        // Set up CLI configuration
        let cli = CliRun {
            stage: "main".to_string(),
            pipe_id: "decode-ballots".to_string(),
            config: fixture.config_path.clone(),
            input_dir: fixture.root_dir.join("tests").join("input-dir"),
            output_dir: fixture.root_dir.join("tests").join("output-dir"),
        };

        let config = cli.validate()?;
        let mut state = State::new(&cli, &config)?;

        // Execute pipeline stages
        state.exec_next()?; // DecodeBallots
        state.exec_next()?; // VoteReceipts
        state.exec_next()?; // DoTally
        state.exec_next()?; // MarkWinners
        state.exec_next()?; // Generate reports

        // Verify results for the contest
        // Results would be just of voters that were directly assigned the area
        for (index, area_config) in areas_config.into_iter().enumerate() {
            let area_path = cli
                .output_dir
                .join("velvet-generate-reports")
                .join(format!("{}{}", PREFIX_ELECTION, &election.id))
                .join(format!("{}{}", PREFIX_CONTEST, &contest.id))
                .join(format!("{}{}", PREFIX_AREA, &area_config.id));

            // check total_votes in non-aggregated report
            let report_path = area_path.join("report.json");
            let f = fs::File::open(&report_path)?;
            let reports: TemplateData = serde_json::from_reader(f)?;
            let report = &reports.reports[0];
            assert_eq!(
                report.contest_result.total_votes, 10,
                "testing 10 votes expected in the contest for the area"
            );

            // the parent area config has no parent, but should have an
            // aggregate report
            if area_config.parent_id.is_none() {
                let aggregate_report_path = area_path.join("aggregate").join("report.json");
                println!("aggregate_report_path = {aggregate_report_path:?}");
                let f = fs::File::open(&aggregate_report_path)?;
                let reports: TemplateData = serde_json::from_reader(f)?;
                let report = &reports.reports[0];
                assert_eq!(
                    report.contest_result.total_votes,
                    // in parent, aggregate is 20: 10 from the children + 10
                    // itself
                    20,
                    "testing total_votes in aggregate result"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn test_check_voting_not_allowed_next() {
        // Case 1: InvalidVotePolicy::NOT_ALLOWED but there aren't any invalid_errors -> false
        let contest1 = get_contest_plurality(
            EOverVotePolicy::ALLOWED,
            EBlankVotePolicy::ALLOWED,
            InvalidVotePolicy::NOT_ALLOWED,
            None,
        );
        let mut decoded_contests1: HashMap<String, DecodedVoteContest> = HashMap::new();
        let decoded_contest = get_decoded_contest_plurality(&contest1);
        decoded_contests1.insert(contest1.id.clone(), decoded_contest);

        let result = check_voting_not_allowed_next_util(vec![contest1], decoded_contests1);
        assert_eq!(result, false);

        // Case 2: EBlankVotePolicy::NOT_ALLOWED and there aren't any votes cast -> true
        let contest2: Contest = get_contest_plurality(
            EOverVotePolicy::ALLOWED,
            EBlankVotePolicy::NOT_ALLOWED,
            InvalidVotePolicy::ALLOWED,
            None,
        );
        let mut decoded_contests2: HashMap<String, DecodedVoteContest> = HashMap::new();
        let decoded_contest = get_decoded_contest_plurality(&contest2);
        decoded_contests2.insert(contest2.id.clone(), decoded_contest);

        let result = check_voting_not_allowed_next_util(vec![contest2], decoded_contests2);
        assert_eq!(result, true);

        // Case 3: EBlankVotePolicy::NOT_ALLOWED but minVotes = 0 and InvalidVotePolicy::NOT_ALLOWED but there aren't any invalid_errors -> false
        let contest3 = get_contest_plurality(
            EOverVotePolicy::ALLOWED,
            EBlankVotePolicy::NOT_ALLOWED,
            InvalidVotePolicy::NOT_ALLOWED,
            Some(0),
        );
        let mut decoded_contests3: HashMap<String, DecodedVoteContest> = HashMap::new();
        let decoded_contest = get_decoded_contest_plurality(&contest3);
        decoded_contests3.insert(contest3.id.clone(), decoded_contest);

        let result = check_voting_not_allowed_next_util(vec![contest3], decoded_contests3);
        assert_eq!(result, true);

        // Case 4: EBlankVotePolicy::NOT_ALLOWED and InvalidVotePolicy::NOT_ALLOWED with invalid errors -> true
        let contest4 = get_contest_plurality(
            EOverVotePolicy::ALLOWED,
            EBlankVotePolicy::NOT_ALLOWED,
            InvalidVotePolicy::NOT_ALLOWED,
            None,
        );
        let mut decoded_contests4: HashMap<String, DecodedVoteContest> = HashMap::new();
        let decoded_contest = get_decoded_contest_plurality(&contest4);
        decoded_contests4.insert(contest4.id.clone(), decoded_contest);

        let result = check_voting_not_allowed_next_util(vec![contest4], decoded_contests4);
        assert_eq!(result, true);
    }

    #[test]
    fn test_check_voting_error_dialog() {
        // Case 1: InvalidVotePolicy::WARN_INVALID_IMPLICIT_AND_EXPLICIT and is_explicit_invalid = true -> true
        let contest1 = get_contest_plurality(
            EOverVotePolicy::ALLOWED,
            EBlankVotePolicy::ALLOWED,
            InvalidVotePolicy::WARN_INVALID_IMPLICIT_AND_EXPLICIT,
            None,
        );
        let mut decoded_contests1: HashMap<String, DecodedVoteContest> = HashMap::new();
        let decoded_contest = get_decoded_contest_plurality(&contest1);
        decoded_contests1.insert(contest1.id.clone(), decoded_contest);

        let result = check_voting_error_dialog_util(vec![contest1], decoded_contests1);
        assert_eq!(result, true);

        // Case 2: EBlankVotePolicy::WARN and choices_selected = 0 -> true
        let contest2 = get_contest_plurality(
            EOverVotePolicy::ALLOWED,
            EBlankVotePolicy::WARN,
            InvalidVotePolicy::ALLOWED,
            None,
        );
        let mut decoded_contests2: HashMap<String, DecodedVoteContest> = HashMap::new();
        let decoded_contest = get_decoded_contest_plurality(&contest2);
        decoded_contests2.insert(contest2.id.clone(), decoded_contest);

        let result = check_voting_error_dialog_util(vec![contest2], decoded_contests2);
        assert_eq!(result, true);

        // Case 3: EBlankVotePolicy::ALLOWED and minVotes = 0 and InvalidVotePolicy::NOT_ALLOWED -> false
        let contest3 = get_contest_plurality(
            EOverVotePolicy::ALLOWED,
            EBlankVotePolicy::ALLOWED,
            InvalidVotePolicy::NOT_ALLOWED,
            Some(0),
        );
        let mut decoded_contests3: HashMap<String, DecodedVoteContest> = HashMap::new();
        let decoded_contest = get_decoded_contest_plurality(&contest3);
        decoded_contests3.insert(contest3.id.clone(), decoded_contest);

        let result = check_voting_error_dialog_util(vec![contest3], decoded_contests3);
        assert_eq!(result, false);
    }
}
