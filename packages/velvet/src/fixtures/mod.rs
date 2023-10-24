use anyhow::Result;
use sequent_core::ballot::*;
use sequent_core::ballot_codec::*;
use sequent_core::plaintext::*;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use uuid::Uuid;

pub struct TestFixture {
    pub config_path: PathBuf,
    // TODO: refact into PathBuf instead of String
    pub root_dir: String,
    // TODO: refact into PathBuf instead of String
    pub input_dir_configs: String,
    // TODO: refact into PathBuf instead of String
    pub input_dir_ballots: String,
}

impl TestFixture {
    pub fn new() -> Result<Self> {
        let config_path = PathBuf::from(format!("test-velvet-config-{}.json", Uuid::new_v4()));
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(&config_path)?;

        writeln!(file, "{}", get_config())?;

        let root_dir = format!("./tests-input__{}", Uuid::new_v4());
        let input_dir_configs = format!("{}/tests/input-dir/default/configs", &root_dir);
        let input_dir_ballots = format!("{}/tests/input-dir/default/ballots", &root_dir);

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

        let dir = format!("{}/election__{}", self.input_dir_configs, uuid);
        fs::create_dir_all(&dir)?;
        let mut file = fs::File::create(format!("{}/election-config.json", dir))?;

        let dir = format!("{}/election__{}", self.input_dir_ballots, uuid);
        fs::create_dir_all(dir)?;

        writeln!(file, "{}", get_election_config())?;

        Ok(uuid)
    }

    pub fn create_contest_config(&self, election_uuid: &Uuid) -> Result<Uuid> {
        let uuid = Uuid::new_v4();

        let dir = format!(
            "{}/election__{}/contest__{}",
            self.input_dir_configs, election_uuid, uuid
        );

        fs::create_dir_all(&dir)?;
        let mut file = fs::File::create(format!("{}/contest-config.json", dir))?;

        let dir = format!(
            "{}/election__{}/contest__{}",
            self.input_dir_ballots, election_uuid, uuid
        );
        fs::create_dir_all(dir)?;

        writeln!(file, "{}", get_contest_config())?;

        Ok(uuid)
    }
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        fs::remove_file(&self.config_path).unwrap();
        fs::remove_dir_all(&self.root_dir).unwrap();
    }
}

fn get_config() -> String {
    let config_content = r#"
        {
            "version": "0.0.0",
            "stages": {
                "order": ["main"],
                "main": {
                    "pipeline": [
                        {
                            "id": "decode-ballots",
                            "pipe": "VelvetDecodeBallots",
                            "config": {}
                        },
                        {
                            "id": "do-tally",
                            "pipe": "VelvetDoTally",
                            "config": {
                                "invalidateVotes": "Fail"
                            }
                        },
                        {
                            "id": "consolidation",
                            "pipe": "VelvetConsolidation",
                            "config": {}
                        },
                        {
                            "id": "ties-resolution",
                            "pipe": "VelvetTiesResolution",
                            "config": {}
                        },
                        {
                            "id": "compute-result",
                            "pipe": "VelvetComputeResult",
                            "config": {}
                        },
                        {
                            "id": "gen-report",
                            "pipe": "VelvetGenerateReport",
                            "config": {
                                "formats": ["pdf", "csv"]
                            }
                        }
                    ]
                }
            }
        }
    "#;

    config_content.to_string()
}

fn get_election_config() -> String {
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
        contests: vec![Contest {
            id: "1c1500ac-173e-4e78-a59d-91bfa3678c5a".into(),
            tenant_id: "9570d82a-d92a-44d7-b483-d5a6c8c398a8".into(),
            election_event_id: "9570d82a-d92a-44d7-b483-d5a6c8c398a8".into(),
            election_id: "9570d82a-d92a-44d7-b483-d5a6c8c398a8".into(),
            name: Some("Test contest title".into()),
            description: None,
            max_votes: 2,
            min_votes: 1,
            voting_type: Some("first-past-the-post".into()),
            counting_algorithm: Some("plurality-at-large".into()),
            is_encrypted: true,
            candidates: vec![
                Candidate {
                    id: "f257cd3a-d1cf-4b97-91f8-2dfe156b015c".into(),
                    tenant_id: "9570d82a-d92a-44d7-b483-d5a6c8c398a8".into(),
                    election_event_id: "9570d82a-d92a-44d7-b483-d5a6c8c398a8".into(),
                    election_id: "9570d82a-d92a-44d7-b483-d5a6c8c398a8".into(),
                    contest_id: "1c1500ac-173e-4e78-a59d-91bfa3678c5a".into(),
                    name: Some("Example option 1".into()),
                    description: Some(
                        "This is an option with an simple example description.".into(),
                    ),
                    candidate_type: None,
                    presentation: Some(CandidatePresentation {
                        is_explicit_invalid: false,
                        is_write_in: false,
                        sort_order: Some(0),
                        urls: None,
                    }),
                },
                Candidate {
                    id: "17325099-f5ab-4c48-a142-6d7ed721e9bb".into(),
                    tenant_id: "9570d82a-d92a-44d7-b483-d5a6c8c398a8".into(),
                    election_event_id: "9570d82a-d92a-44d7-b483-d5a6c8c398a8".into(),
                    election_id: "9570d82a-d92a-44d7-b483-d5a6c8c398a8".into(),
                    contest_id: "1c1500ac-173e-4e78-a59d-91bfa3678c5a".into(),
                    name: Some("Example option 1".into()),
                    description: Some(
                        "This is an option with an simple example description.".into(),
                    ),
                    candidate_type: None,
                    presentation: Some(CandidatePresentation {
                        is_explicit_invalid: false,
                        is_write_in: false,
                        sort_order: Some(1),
                        urls: Some(vec![
                            CandidateUrl {
                                url: "https://sequentech.io".into(),
                                kind: None,
                                title: None,
                                is_image: false,
                            },
                            CandidateUrl {
                                url: "/XFQwVFL.jpg".into(),
                                kind: None,
                                title: None,
                                is_image: true,
                            },
                        ]),
                    }),
                },
                Candidate {
                    id: "61320aac-0d78-4001-845e-a2f2bd8e800b".into(),
                    tenant_id: "9570d82a-d92a-44d7-b483-d5a6c8c398a8".into(),
                    election_event_id: "9570d82a-d92a-44d7-b483-d5a6c8c398a8".into(),
                    election_id: "9570d82a-d92a-44d7-b483-d5a6c8c398a8".into(),
                    contest_id: "1c1500ac-173e-4e78-a59d-91bfa3678c5a".into(),
                    name: None,
                    description: None,
                    candidate_type: None,
                    presentation: Some(CandidatePresentation {
                        is_explicit_invalid: false,
                        is_write_in: true,
                        sort_order: Some(2),
                        urls: None,
                    }),
                },
                Candidate {
                    id: "e9ad3ed1-4fd5-4498-a0e7-3a3c22ef57d5".into(),
                    tenant_id: "9570d82a-d92a-44d7-b483-d5a6c8c398a8".into(),
                    election_event_id: "9570d82a-d92a-44d7-b483-d5a6c8c398a8".into(),
                    election_id: "9570d82a-d92a-44d7-b483-d5a6c8c398a8".into(),
                    contest_id: "1c1500ac-173e-4e78-a59d-91bfa3678c5a".into(),
                    name: None,
                    description: None,
                    candidate_type: None,
                    presentation: Some(CandidatePresentation {
                        is_explicit_invalid: false,
                        is_write_in: true,
                        sort_order: Some(3),
                        urls: None,
                    }),
                },
            ],
            presentation: Some(ContestPresentation {
                allow_writeins: true,
                base32_writeins: true,
                invalid_vote_policy: "allowed".into(),
                cumulative_number_of_checkboxes: None,
                shuffle_categories: true,
                shuffle_all_options: true,
                shuffle_category_list: None,
                show_points: false,
            }),
        }],
    };

    serde_json::to_string(&ballot_style).unwrap()
}

pub fn get_contest_config() -> String {
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
        }),
    };

    serde_json::to_string(&contest).unwrap()
}

