// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use serde::Serialize;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::state::State;
    use crate::cli::CliRun;
    use crate::fixtures;
    use crate::fixtures::TestFixture;
    use crate::pipes::decode_ballots::OUTPUT_DECODED_BALLOTS_FILE;
    use crate::pipes::do_tally::OUTPUT_CONTEST_RESULT_FILE;
    use crate::pipes::mark_winners::{WinnerCandidate, OUTPUT_WINNERS};
    use crate::pipes::pipe_name::PipeNameOutputDir;
    use anyhow::{Error, Result};
    use rand::Rng;
    use sequent_core::ballot::{Candidate, CandidatePresentation, Contest, ContestPresentation};
    use sequent_core::ballot_codec::BigUIntCodec;
    use sequent_core::plaintext::{DecodedVoteChoice, DecodedVoteContest};
    use sequent_core::services::{pdf, reports};
    use serde_json::{Map, Value};
    use std::fs::{self, OpenOptions};
    use std::io::Write;
    use std::path::PathBuf;
    use uuid::Uuid;
    use walkdir::WalkDir;

    fn generate_ballots(
        fixture: &TestFixture,
        election_num: u32,
        contest_num: u32,
        region_num: u32,
        ballots_num: u32,
    ) -> Result<()> {
        let rng = rand::thread_rng();

        (0..election_num).try_for_each(|_| {
            let uuid_election = fixture.create_election_config()?;
            (0..contest_num).try_for_each(|_| {
                let uuid_contest = fixture.create_contest_config(&uuid_election)?;
                (0..region_num).try_for_each(|_| {
                    let uuid_region = fixture.create_region_dir(&uuid_election, &uuid_contest)?;

                    let file = fixture
                        .input_dir_ballots
                        .join(format!("election__{uuid_election}"))
                        .join(format!("contest__{uuid_contest}"))
                        .join(format!("region__{uuid_region}"));

                    let mut file = fs::OpenOptions::new()
                        .write(true)
                        .append(true)
                        .create(true)
                        .open(file.join("ballots.csv"))?;

                    (0..ballots_num).try_for_each(|i| {
                        let contest = fixtures::get_contest_config();

                        let mut choices = vec![];
                        let mut plaintext_prepare = DecodedVoteContest {
                            contest_id: contest.id.clone(),
                            is_explicit_invalid: false,
                            invalid_errors: vec![],
                            choices: vec![],
                        };
                        if i == 4 {
                            choices = vec![
                                DecodedVoteChoice {
                                    id: "0".to_owned(),
                                    selected: -1,
                                    write_in_text: None,
                                },
                                DecodedVoteChoice {
                                    id: "1".to_owned(),
                                    selected: 0,
                                    write_in_text: None,
                                },
                                DecodedVoteChoice {
                                    id: "2".to_owned(),
                                    selected: -1,
                                    write_in_text: None,
                                },
                                DecodedVoteChoice {
                                    id: "3".to_owned(),
                                    selected: 110,
                                    write_in_text: None,
                                },
                                DecodedVoteChoice {
                                    id: "4".to_owned(),
                                    selected: -1,
                                    write_in_text: None,
                                },
                            ]
                        } else {
                            choices = vec![
                                DecodedVoteChoice {
                                    id: "0".to_owned(),
                                    selected: -1,
                                    write_in_text: None,
                                },
                                DecodedVoteChoice {
                                    id: "1".to_owned(),
                                    selected: 0,
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
                            ]
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

    #[test]
    fn test_create_configs() -> Result<()> {
        let fixture = TestFixture::new()?;

        let uuid_election = fixture.create_election_config()?;
        let uuid_contest = fixture.create_contest_config(&uuid_election)?;

        let entries = fs::read_dir(&fixture.input_dir_configs)?;
        let count = entries.count();
        assert_eq!(count, 1);

        let entries = fs::read_dir(
            &fixture
                .input_dir_configs
                .join(format!("election__{uuid_election}")),
        )?;
        let count = entries.count();
        assert_eq!(count, 2);

        let entries = fs::read_dir(
            &fixture
                .input_dir_configs
                .join(format!("election__{uuid_election}"))
                .join(format!("contest__{uuid_contest}")),
        )?;
        let count = entries.count();
        assert_eq!(count, 1);

        Ok(())
    }

    #[test]
    fn test_create_ballots() -> Result<()> {
        let fixture = TestFixture::new()?;

        generate_ballots(&fixture, 5, 10, 3, 5)?;

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

        // count count regions
        let mut entries = fs::read_dir(&fixture.input_dir_ballots.join(&election_uuid))?;
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
        let region_num = 3;
        let ballot_num = 20;

        let fixture = TestFixture::new()?;

        generate_ballots(&fixture, election_num, contest_num, region_num, ballot_num)?;

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
            election_num * contest_num * region_num
        );

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
            election_num * contest_num * region_num + election_num * contest_num
        );

        // MarkWinners
        state.exec_next()?;

        assert_eq!(
            WalkDir::new(cli.output_dir.as_path())
                .into_iter()
                .filter_map(Result::ok)
                .filter(|e| { e.path().file_name().map_or(false, |f| f == OUTPUT_WINNERS) })
                .count() as u32,
            election_num * contest_num * region_num + election_num * contest_num
        );

        // Report
        state.exec_next()?;

        Ok(())
    }

    #[test]
    fn test_generate_reports() -> Result<()> {
        let winner = WinnerCandidate {
            candidate: Candidate {
                id: "1".to_string(),
                tenant_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".to_string(),
                election_event_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".to_string(),
                election_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".to_string(),
                contest_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".to_string(),
                name: Some("Miguel Pimentel Inventado".to_string()),
                description: None,
                candidate_type: None,
                presentation: Some(CandidatePresentation {
                    is_explicit_invalid: false,
                    is_category_list: false,
                    invalid_vote_position: None,
                    is_write_in: false,
                    sort_order: Some(1),
                    urls: None,
                }),
            },
            total_points: 19,
        };

        let contest = Contest {
            id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
            tenant_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
            election_event_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
            election_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
            name: Some("Secretario General".into()),
            description: Some(
                "Elige quien quieres que sea tu Secretario General en tu municipio".into(),
            ),
            max_votes: 1,
            min_votes: 0,
            voting_type: Some("first-past-the-post".into()),
            counting_algorithm: Some("plurality-at-large".into()), /* plurality-at-large|borda-nauru|borda|borda-mas-madrid|desborda3|desborda2|desborda|cumulative */
            is_encrypted: true,
            candidates: vec![
                Candidate {
                    id: "0".into(),
                    tenant_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
                    election_event_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
                    election_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
                    contest_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
                    name: Some("José Rabano Pimiento".into()),
                    description: None,
                    candidate_type: None,
                    presentation: Some(CandidatePresentation {
                        is_explicit_invalid: false,
                        is_write_in: false,
                        sort_order: Some(0),
                        urls: None,
                        invalid_vote_position: None,
                        is_category_list: false,
                    }),
                },
                Candidate {
                    id: "1".into(),
                    tenant_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
                    election_event_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
                    election_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
                    contest_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
                    name: Some("Miguel Pimentel Inventado".into()),
                    description: None,
                    candidate_type: None,
                    presentation: Some(CandidatePresentation {
                        is_explicit_invalid: false,
                        is_write_in: false,
                        sort_order: Some(1),
                        urls: None,
                        invalid_vote_position: None,
                        is_category_list: false,
                    }),
                },
                Candidate {
                    id: "2".into(),
                    tenant_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
                    election_event_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
                    election_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
                    contest_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
                    name: Some("Juan Iglesias Torquemada".into()),
                    description: None,
                    candidate_type: None,
                    presentation: Some(CandidatePresentation {
                        is_explicit_invalid: false,
                        is_write_in: false,
                        sort_order: Some(2),
                        urls: None,
                        invalid_vote_position: None,
                        is_category_list: false,
                    }),
                },
                Candidate {
                    id: "3".into(),
                    tenant_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
                    election_event_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
                    election_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
                    contest_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
                    name: Some("Mari Pili Hernández Ordoñez".into()),
                    description: None,
                    candidate_type: None,
                    presentation: Some(CandidatePresentation {
                        is_explicit_invalid: false,
                        is_write_in: false,
                        sort_order: Some(3),
                        urls: None,
                        invalid_vote_position: None,
                        is_category_list: false,
                    }),
                },
                Candidate {
                    id: "4".into(),
                    tenant_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
                    election_event_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
                    election_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
                    contest_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
                    name: Some("Juan Y Medio".into()),
                    description: None,
                    candidate_type: None,
                    presentation: Some(CandidatePresentation {
                        is_explicit_invalid: false,
                        is_write_in: false,
                        sort_order: Some(4),
                        urls: None,
                        invalid_vote_position: None,
                        is_category_list: false,
                    }),
                },
            ],
            presentation: Some(ContestPresentation {
                allow_writeins: false,
                base32_writeins: true,
                invalid_vote_policy: "allowed".into(),
                cumulative_number_of_checkboxes: None,
                shuffle_categories: true,
                shuffle_all_options: true,
                shuffle_category_list: None,
                show_points: false,
                enable_checkable_lists: None,
            }),
        };

        let mut map = Map::new();
        map.insert("contest".to_owned(), serde_json::to_value(&contest)?);
        map.insert("winner".to_owned(), serde_json::to_value(&winner)?);

        let html = include_str!("../resources/report.html");
        let render = reports::render_template_text(html, map)?;

        let bytes = pdf::html_to_pdf(render)?;

        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open("test-report.pdf")?;

        file.write_all(&bytes)?;

        assert!(PathBuf::from("test-report.pdf").exists());

        Ok(())
    }
}

// define some data
#[derive(Serialize)]
pub struct Team {
    name: String,
    pts: u16,
}
