// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::fixtures::ballot_styles::generate_ballot_style;
use crate::fixtures::TestFixture;
use crate::pipes::pipe_inputs::BALLOTS_FILE;
use anyhow::{Error, Result};
use sequent_core::ballot_codec::BigUIntCodec;
use sequent_core::plaintext::{DecodedVoteChoice, DecodedVoteContest};
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::state::State;
    use crate::cli::CliRun;
    use crate::fixtures::ballot_styles::generate_ballot_style;
    use crate::fixtures::TestFixture;
    use crate::pipes::decode_ballots::OUTPUT_DECODED_BALLOTS_FILE;
    use crate::pipes::do_tally::OUTPUT_CONTEST_RESULT_FILE;
    use crate::pipes::generate_reports::ReportDataComputed;
    use crate::pipes::mark_winners::OUTPUT_WINNERS;
    use crate::pipes::pipe_inputs::{PREFIX_AREA, PREFIX_CONTEST, PREFIX_ELECTION};
    use crate::pipes::pipe_name::PipeNameOutputDir;
    use anyhow::{Error, Result};
    use sequent_core::ballot_codec::BigUIntCodec;
    use sequent_core::plaintext::{DecodedVoteChoice, DecodedVoteContest};
    use std::fs;
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
    fn test_pipes_exec() -> Result<()> {
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

        // test first contest
        let mut path = cli.output_dir.clone();
        path.push("velvet-generate-reports");
        path.push(format!("{}{}", PREFIX_ELECTION, &election.id));
        path.push(format!("{}{}", PREFIX_CONTEST, &first_contest_id));
        path.push("report.json");

        let f = fs::File::open(&path)?;

        let reports: Vec<ReportDataComputed> = serde_json::from_reader(f)?;
        let report = &reports[0];

        assert_eq!(report.contest_result.total_votes, 142);
        assert_eq!(report.contest_result.census, 200);
        assert_eq!(
            report
                .candidate_result
                .iter()
                .map(|cr| cr.total_count)
                .sum::<u64>(),
            134
        );

        let mut path = cli.output_dir.clone();
        path.push("velvet-generate-reports");
        path.push(format!("{}{}", PREFIX_ELECTION, &election.id));
        path.push(format!("{}{}", PREFIX_CONTEST, &first_contest_id));
        path.push(format!("{}{}", PREFIX_AREA, &first_area_id));
        path.push("report.json");

        let f = fs::File::open(&path)?;

        let reports: Vec<ReportDataComputed> = serde_json::from_reader(f)?;
        let report = &reports[0];

        assert_eq!(report.contest_result.total_votes, 100);
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

        let reports: Vec<ReportDataComputed> = serde_json::from_reader(f)?;
        let report = &reports[0];

        assert_eq!(report.contest_result.total_votes, 20);
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

        let mut path = cli.output_dir.clone();
        path.push("velvet-generate-reports");
        path.push(format!("{}{}", PREFIX_ELECTION, &election.id));
        path.push(format!("{}{}", PREFIX_CONTEST, &contest.id));
        path.push(format!("{}{}", PREFIX_AREA, &area_config.id));
        path.push("report.json");

        let f = fs::File::open(&path)?;

        let reports: Vec<ReportDataComputed> = serde_json::from_reader(f)?;
        let report = &reports[0];

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

        let mut path = cli.output_dir.clone();
        path.push("velvet-generate-reports");
        path.push(format!("{}{}", PREFIX_ELECTION, &election.id));
        path.push(format!("{}{}", PREFIX_CONTEST, &contest.id));
        path.push(format!("{}{}", PREFIX_AREA, &area_config.id));
        path.push("report.json");

        let f = fs::File::open(&path)?;

        let reports: Vec<ReportDataComputed> = serde_json::from_reader(f)?;
        let report = &reports[0];

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

        let mut path = cli.output_dir.clone();
        path.push("velvet-generate-reports");
        path.push(format!("{}{}", PREFIX_ELECTION, &election.id));
        path.push(format!("{}{}", PREFIX_CONTEST, &contest.id));
        path.push(format!("{}{}", PREFIX_AREA, &area_config.id));
        path.push("report.json");

        let f = fs::File::open(&path)?;

        let reports: Vec<ReportDataComputed> = serde_json::from_reader(f)?;
        let report = &reports[0];

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

    // #[test]
    // fn test_hierarchical_area_aggregation() -> Result<()> {        
    //     let fixture = TestFixture::new()?;

    //     let election_event_id = Uuid::new_v4();
    //     let areas: Vec<Uuid> = (0..3).map(|_| Uuid::new_v4()).collect();

    //     let mut election = fixture.create_election_config(&election_event_id, areas.clone())?;
    //     election.ballot_styles.clear(); 

    //     // Create three contests: Grandfather, parent, child hierarchy
    //     let contests: Vec<_> = (0..3).map(|_| {
    //         fixture.create_contest_config(&election.tenant_id, &election_event_id, &election.id).unwrap()
    //     }).collect();

    //     // Create hierarchical area structure and associate each area with a unique contest
    //     let areas_config = areas.iter().enumerate().map(|(i, area_id)| {
    //         let parent_id = if i == 0 { None } else { Some(areas[i - 1]) };
    //         fixture.create_area_config(
    //             &election.tenant_id,
    //             &election_event_id,
    //             &election.id,
    //             &Uuid::from_str(&contests[i].id).unwrap(),
    //             100,
    //             0,
    //             parent_id,
    //             Some((*area_id).to_string()),
    //         ).unwrap()
    //     }).collect::<Vec<_>>();

    //     // Assign each contest to the corresponding area
    //     // TODO: understand if this affects the connection between parent areas-contests
    //     for (i, area) in areas_config.iter().enumerate() {
    //         election.ballot_styles.push(generate_ballot_style(
    //             &election.tenant_id,
    //             &election.election_event_id,
    //             &election.id,
    //             &area.id,
    //             vec![contests[i].clone()],
    //         ));
    //     }

    //     // Generate ballots for the voter associated with area 3 for all contests
    //     for i in 0..3 {
    //         println!(" ----- i {} Area {} Contest {}", i, areas_config[i].id, contests[i].id);
    //         let ballot_file = fixture
    //             .input_dir_ballots
    //             .join(format!("election__{}", &election.id))
    //             .join(format!("contest__{}", contests[i].id))
    //             .join(format!("area__{}", areas_config[i].id));

    //         let mut file = fs::OpenOptions::new()
    //             .write(true)
    //             .append(true)
    //             .create(true)
    //             .open(ballot_file.join("ballots.csv"))?;

    //         (0..10).try_for_each(|j| {
    //             let mut choices = vec![
    //                 DecodedVoteChoice { id: "0".to_owned(), selected: -1, write_in_text: None },
    //                 DecodedVoteChoice { id: "1".to_owned(), selected: -1, write_in_text: None },
    //                 DecodedVoteChoice { id: "2".to_owned(), selected: -1, write_in_text: None },
    //                 DecodedVoteChoice { id: "3".to_owned(), selected: -1, write_in_text: None },
    //                 DecodedVoteChoice { id: "4".to_owned(), selected: -1, write_in_text: None },
    //             ];

    //             let mut plaintext_prepare = DecodedVoteContest {
    //                 contest_id: contests[i].id.clone(),
    //                 is_explicit_invalid: false,
    //                 invalid_errors: vec![],
    //                 invalid_alerts: vec![],
    //                 choices: vec![],
    //             };

    //             // For each contest: assigns selections to the choices
    //             // TODO: make the votes different for each contest 
    //             // TODO: decide if i need to vote for every contest or just the grandpa one
    //             match j {
    //                 1 => choices[0].selected = 0,
    //                 2 => choices[1].selected = 0,
    //                 3 => choices[2].selected = 0,
    //                 4 => choices[3].selected = 0,
    //                 5 => choices[4].selected = 0,
    //                 6 => (),
    //                 _ => choices[1].selected = 0,
    //             }

    //             plaintext_prepare.choices = choices;

    //             let plaintext = contests[i].encode_plaintext_contest_bigint(&plaintext_prepare).unwrap();
    //             writeln!(file, "{}", plaintext)?;

    //             Ok::<(), Error>(())
    //         })?;
    //     }

    //     // Set up CLI configuration
    //     let cli = CliRun {
    //         stage: "main".to_string(),
    //         pipe_id: "decode-ballots".to_string(),
    //         config: fixture.config_path.clone(),
    //         input_dir: fixture.root_dir.join("tests").join("input-dir"),
    //         output_dir: fixture.root_dir.join("tests").join("output-dir"),
    //     };

    //     let config = cli.validate()?;
    //     let mut state = State::new(&cli, &config)?;

    //     // Execute pipeline stages
    //     state.exec_next()?; // DecodeBallots
    //     state.exec_next()?; // VoteReceipts
    //     state.exec_next()?; // DoTally
    //     state.exec_next()?; // MarkWinners
    //     state.exec_next()?; // Generate reports

    //     // Verify results for the contests themselves
    //     // Results would be just of voters that were directly assigned the area
    //     for i in 0..3 {
    //         let mut path = cli.output_dir.clone();
    //         path.push("velvet-generate-reports");
    //         path.push(format!("{}{}", PREFIX_ELECTION, &election.id));
    //         path.push(format!("{}{}", PREFIX_CONTEST, &contests[i].id));
    //         path.push(format!("{}{}", PREFIX_AREA, &areas_config[i].id));
    //         path.push("report.json");

    //         let f = fs::File::open(&path)?;
    //         let reports: Vec<ReportDataComputed> = serde_json::from_reader(f)?;
    //         let report = &reports[0];

    //         assert_eq!(report.contest_result.total_votes, 10);
    //     }

    //     // Verify aggregated results
    //     // TODO: find "aggregate" folder
    //     for contest in &contests {
    //         let mut path = cli.output_dir.clone();
    //         path.push("velvet-generate-reports");
    //         path.push(format!("{}{}", PREFIX_ELECTION, &election.id));
    //         path.push(format!("{}{}", PREFIX_CONTEST, &contest.id));
    //         path.push("aggregate");
    //         path.push("report.json");

    //         let f = fs::File::open(&path)?;
    //         let reports: Vec<ReportDataComputed> = serde_json::from_reader(f)?;
    //         let report = &reports[0];

    //         assert_eq!(report.contest_result.total_votes, 10);
    //     }

    //     Ok(())
    // }


    #[test]
    fn test_hierarchical_area_aggregation() -> Result<()> {      
        // Step 1: Creating Election event, election, contest, 2 areas wi parent-child relation
        // Conect contest to parent area
        let fixture = TestFixture::new()?;

        let election_event_id = Uuid::new_v4();
        let area_ids: Vec<Uuid> = (0..2).map(|_| Uuid::new_v4()).collect();

        let mut election = fixture.create_election_config(&election_event_id, area_ids.clone())?;
        election.ballot_styles.clear(); 

        let contest = fixture.create_contest_config(&election.tenant_id, &election_event_id, &election.id)?;

        // Create hierarchical area structure and associate parent area with the contest
        let areas_config = area_ids.iter().enumerate().map(|(i, area_id)| {
            let parent_id = if i == 0 { None } else { Some(area_ids[i - 1]) }; // TODO: 
            fixture.create_area_config(
                &election.tenant_id,
                &election_event_id,
                &election.id,
                &Uuid::from_str(&contest.id).unwrap(), // TODO: to do it only for the first area
                100,
                0,
                parent_id,
                Some((*area_id).to_string()),
            ).unwrap()
        }).collect::<Vec<_>>();

        // Assign each contest to the corresponding area
        // TODO: understand if this affects the connection between parent areas-contests
        // for (i, area) in areas_config.iter().enumerate() {
            election.ballot_styles.push(generate_ballot_style(
                &election.tenant_id,
                &election.election_event_id,
                &election.id,
                &areas_config[0].id,
                vec![contest.clone()],
            ));
        // }

        // Step 2: Create ballot to vote from parent area and another one for child area
        // Generate ballots for the voter associated with area 3 for all contests
        for i in 0..2 {
            println!(" ----- i {} Area {} Contest {}", i, areas_config[i].id, contest.id);
            let ballot_file = fixture
                .input_dir_ballots
                .join(format!("election__{}", &election.id))
                .join(format!("contest__{}", contest.id))
                .join(format!("area__{}", areas_config[i].id));

            let mut file = fs::OpenOptions::new()
                .write(true)
                .append(true)
                .create(true)
                .open(ballot_file.join("ballots.csv"))?;

            (0..10).try_for_each(|j| {
                let mut choices = vec![
                    DecodedVoteChoice { id: "0".to_owned(), selected: -1, write_in_text: None },
                    DecodedVoteChoice { id: "1".to_owned(), selected: -1, write_in_text: None },
                    DecodedVoteChoice { id: "2".to_owned(), selected: -1, write_in_text: None },
                    DecodedVoteChoice { id: "3".to_owned(), selected: -1, write_in_text: None },
                    DecodedVoteChoice { id: "4".to_owned(), selected: -1, write_in_text: None },
                ];

                let mut plaintext_prepare = DecodedVoteContest {
                    contest_id: contests[i].id.clone(),
                    is_explicit_invalid: false,
                    invalid_errors: vec![],
                    invalid_alerts: vec![],
                    choices: vec![],
                };

                // For each contest: assigns selections to the choices
                // TODO: make the votes different for each contest 
                // TODO: decide if i need to vote for every contest or just the grandpa one
                match j {
                    1 => choices[0].selected = 0,
                    2 => choices[1].selected = 0,
                    3 => choices[2].selected = 0,
                    4 => choices[3].selected = 0,
                    5 => choices[4].selected = 0,
                    6 => (),
                    _ => choices[1].selected = 0,
                }

                plaintext_prepare.choices = choices;

                let plaintext = contest.encode_plaintext_contest_bigint(&plaintext_prepare).unwrap();
                writeln!(file, "{}", plaintext)?;

                Ok::<(), Error>(())
            })?;
        }

        // Step 3: Generate tallys and test expected results
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

        // Verify results for the contests themselves
        // Results would be just of voters that were directly assigned the area
        for i in 0..3 {
            let mut path = cli.output_dir.clone();
            path.push("velvet-generate-reports");
            path.push(format!("{}{}", PREFIX_ELECTION, &election.id));
            path.push(format!("{}{}", PREFIX_CONTEST, &contests[i].id));
            path.push(format!("{}{}", PREFIX_AREA, &areas_config[i].id));
            path.push("report.json");

            let f = fs::File::open(&path)?;
            let reports: Vec<ReportDataComputed> = serde_json::from_reader(f)?;
            let report = &reports[0];

            assert_eq!(report.contest_result.total_votes, 10);
        }

        // Verify aggregated results
        // TODO: find "aggregate" folder
        for contest in &contests {
            let mut path = cli.output_dir.clone();
            path.push("velvet-generate-reports");
            path.push(format!("{}{}", PREFIX_ELECTION, &election.id));
            path.push(format!("{}{}", PREFIX_CONTEST, &contest.id));
            path.push("aggregate");
            path.push("report.json");

            let f = fs::File::open(&path)?;
            let reports: Vec<ReportDataComputed> = serde_json::from_reader(f)?;
            let report = &reports[0];

            assert_eq!(report.contest_result.total_votes, 10);
        }

        Ok(())
    }
}
