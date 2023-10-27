// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use sequent_core::ballot::{
    BallotStyle, Candidate, CandidatePresentation, CandidateUrl, Contest, ContestPresentation,
    ElectionStatus, PublicKeyConfig, VotingStatus,
};
use sequent_core::ballot_codec::BigUIntCodec;
use sequent_core::plaintext::DecodedVoteContest;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::config::{self, Config};
use crate::pipes::pipe_name::PipeName;

pub struct TestFixture {
    pub config_path: PathBuf,
    pub root_dir: PathBuf,
    pub input_dir_configs: PathBuf,
    pub input_dir_ballots: PathBuf,
}

impl TestFixture {
    pub fn new() -> Result<Self> {
        let config_path = PathBuf::from(format!("test-velvet-config-{}.json", Uuid::new_v4()));
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(&config_path)?;

        writeln!(file, "{}", serde_json::to_string(&get_config())?)?;

        let root_dir = PathBuf::from(format!("./tests-input__{}", Uuid::new_v4()));
        let input_dir = root_dir.join("tests").join("input-dir").join("default");
        let input_dir_configs = input_dir.join("configs");
        let input_dir_ballots = input_dir.join("ballots");

        fs::create_dir_all(&input_dir_configs)?;
        fs::create_dir_all(&input_dir_ballots)?;

        Ok(Self {
            config_path,
            root_dir,
            input_dir_configs,
            input_dir_ballots,
        })
    }

    pub fn create_election_config(&self) -> Result<Uuid> {
        let uuid = Uuid::new_v4();

        let mut path = self.input_dir_configs.join(format!("election__{uuid}"));
        fs::create_dir_all(path.as_path())?;

        path.push("election-config.json");
        let mut file = fs::File::create(path)?;
        writeln!(file, "{}", serde_json::to_string(&get_election_config())?)?;

        Ok(uuid)
    }

    pub fn create_contest_config(&self, election_uuid: &Uuid) -> Result<Uuid> {
        let uuid = Uuid::new_v4();

        let dir = self
            .input_dir_configs
            .join(format!("election__{election_uuid}"))
            .join(format!("contest__{uuid}"));
        fs::create_dir_all(&dir)?;

        let mut file = fs::File::create(dir.join("contest-config.json"))?;
        writeln!(file, "{}", serde_json::to_string(&get_contest_config())?)?;

        Ok(uuid)
    }

    pub fn create_region_dir(&self, election_uuid: &Uuid, contest_uuid: &Uuid) -> Result<Uuid> {
        let uuid = Uuid::new_v4();

        let dir_configs = self
            .input_dir_configs
            .join(format!("election__{election_uuid}"))
            .join(format!("contest__{contest_uuid}"))
            .join(format!("region__{uuid}"));
        let dir_ballots = self
            .input_dir_ballots
            .join(format!("election__{election_uuid}"))
            .join(format!("contest__{contest_uuid}"))
            .join(format!("region__{uuid}"));
        fs::create_dir_all(dir_configs)?;
        fs::create_dir_all(dir_ballots)?;

        Ok(uuid)
    }
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        fs::remove_file(&self.config_path).unwrap();
        fs::remove_dir_all(&self.root_dir).unwrap();
    }
}

pub fn get_config() -> Config {
    let stages_def = {
        let mut map = HashMap::new();
        map.insert(
            "main".to_string(),
            config::Stage {
                pipeline: vec![
                    config::PipeConfig {
                        id: "decode-ballots".to_string(),
                        pipe: PipeName::DecodeBallots,
                        config: Some(serde_json::Value::Null),
                    },
                    config::PipeConfig {
                        id: "do-tally".to_string(),
                        pipe: PipeName::DoTally,
                        config: Some(serde_json::Value::Null),
                    },
                    config::PipeConfig {
                        id: "mark-winners".to_string(),
                        pipe: PipeName::MarkWinners,
                        config: Some(serde_json::Value::Null),
                    },
                    config::PipeConfig {
                        id: "gen-report".to_string(),
                        pipe: PipeName::GenerateReport,
                        config: Some(serde_json::Value::Null),
                    },
                ],
            },
        );
        map
    };

    let stages = config::Stages {
        order: vec!["main".to_string()],
        stages_def,
    };

    let config = Config {
        version: "0.0.0".to_string(),
        stages,
    };

    config
}

pub fn get_election_config() -> BallotStyle {
    let ballot_style = BallotStyle {
        id: "9570d82a-d92a-44d7-b483-d5a6c8c398a8".into(),
        tenant_id: "9570d82a-d92a-44d7-b483-d5a6c8c398a8".into(),
        election_event_id: "9570d82a-d92a-44d7-b483-d5a6c8c398a8".into(),
        election_id: "9570d82a-d92a-44d7-b483-d5a6c8c398a8".into(),
        description: Some("Write-ins simple".into()),
        public_key: Some(PublicKeyConfig {
            public_key: "ajR/I9RqyOwbpsVRucSNOgXVLCvLpfQxCgPoXGQ2RF4".into(),
            is_demo: false,
        }),
        area_id: "9570d82a-d92a-44d7-b483-d5a6c8c398a8".into(),
        status: Some(ElectionStatus {
            voting_status: VotingStatus::OPEN,
        }),
        contests: vec![get_contest_config()],
    };

    ballot_style
}

pub fn get_contest_config() -> Contest {
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

    contest
}
