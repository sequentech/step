// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use sequent_core::ballot::Contest;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use tracing::instrument;
use uuid::Uuid;

use crate::config::vote_receipt::PipeConfigVoteReceipts;
use crate::config::{self, Config};
use crate::pipes::pipe_inputs::{AreaConfig, ElectionConfig};
use crate::pipes::pipe_name::PipeName;

#[derive(Debug)]
pub struct TestFixture {
    pub config_path: PathBuf,
    pub root_dir: PathBuf,
    pub input_dir_configs: PathBuf,
    pub input_dir_ballots: PathBuf,
}

impl TestFixture {
    #[instrument]
    pub fn new() -> Result<Self> {
        let config_path = PathBuf::from(format!("test-velvet-config-{}.json", Uuid::new_v4()));
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(&config_path)?;

        writeln!(file, "{}", serde_json::to_string(&get_config()?)?)?;

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

    #[instrument]
    pub fn new_mc() -> Result<Self> {
        let config_path = PathBuf::from(format!("test-velvet-config-{}.json", Uuid::new_v4()));
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(&config_path)?;

        writeln!(file, "{}", serde_json::to_string(&get_config_mcballots()?)?)?;

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

    #[instrument]
    pub fn create_election_config(
        &self,
        election_event_id: &Uuid,
        areas: Vec<Uuid>,
    ) -> Result<ElectionConfig> {
        let election = super::elections::get_election_config_1(election_event_id, areas);

        let mut path = self
            .input_dir_configs
            .join(format!("election__{}", election.id));
        fs::create_dir_all(path.as_path())?;

        path.push("election-config.json");
        let mut file = fs::File::create(path)?;
        writeln!(file, "{}", serde_json::to_string(&election)?)?;

        Ok(election)
    }

    #[instrument]
    pub fn create_election_config_2(
        &self,
        election_event_id: &Uuid,
        areas: Vec<(Uuid, Option<Uuid>)>,
    ) -> Result<ElectionConfig> {
        let election = super::elections::get_election_config_3(election_event_id, areas);

        let mut path = self
            .input_dir_configs
            .join(format!("election__{}", election.id));
        fs::create_dir_all(path.as_path())?;

        path.push("election-config.json");
        let mut file = fs::File::create(path)?;
        writeln!(file, "{}", serde_json::to_string(&election)?)?;

        Ok(election)
    }

    #[instrument]
    pub fn create_contest_config(
        &self,
        tenant_id: &Uuid,
        election_event_id: &Uuid,
        election_id: &Uuid,
    ) -> Result<Contest> {
        let contest = super::contests::get_contest_1(tenant_id, election_event_id, election_id);

        let dir = self
            .input_dir_configs
            .join(format!("election__{}", &election_id))
            .join(format!("contest__{}", &contest.id));
        fs::create_dir_all(&dir)?;

        let mut file = fs::File::create(dir.join("contest-config.json"))?;
        writeln!(file, "{}", serde_json::to_string(&contest)?)?;

        Ok(contest)
    }

    #[instrument]
    pub fn create_contest_config_with_min_max_votes(
        &self,
        tenant_id: &Uuid,
        election_event_id: &Uuid,
        election_id: &Uuid,
        min_votes: u64,
        max_votes: u64,
    ) -> Result<Contest> {
        let contest = super::contests::get_contest_min_max_votes(
            tenant_id,
            election_event_id,
            election_id,
            min_votes,
            max_votes,
        );

        let dir = self
            .input_dir_configs
            .join(format!("election__{}", &election_id))
            .join(format!("contest__{}", &contest.id));
        fs::create_dir_all(&dir)?;

        let mut file = fs::File::create(dir.join("contest-config.json"))?;
        writeln!(file, "{}", serde_json::to_string(&contest)?)?;

        Ok(contest)
    }

    #[instrument]
    pub fn create_area_config(
        &self,
        tenant_id: &Uuid,
        election_event_id: &Uuid,
        election_id: &Uuid,
        contest_id: &Uuid,
        census: u64,
        auditable_votes: u64,
        parent_id: Option<Uuid>,
        area_id: Option<String>,
    ) -> Result<AreaConfig> {
        let area_config = super::areas::get_area_config(
            tenant_id,
            election_event_id,
            election_id,
            census,
            auditable_votes,
            parent_id,
            area_id,
        );

        let dir_configs = self
            .input_dir_configs
            .join(format!("election__{election_id}"))
            .join(format!("contest__{contest_id}"))
            .join(format!("area__{}", area_config.id));
        let dir_ballots = self
            .input_dir_ballots
            .join(format!("election__{election_id}"))
            .join(format!("contest__{contest_id}"))
            .join(format!("area__{}", area_config.id));
        fs::create_dir_all(&dir_configs)?;
        fs::create_dir_all(dir_ballots)?;

        let mut file = fs::File::create(dir_configs.join("area-config.json"))?;
        writeln!(file, "{}", serde_json::to_string(&area_config)?)?;

        Ok(area_config)
    }
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        if env::var("CLEANUP_FILES").unwrap_or("true".to_string()) == "true" {
            fs::remove_file(&self.config_path).unwrap();
            fs::remove_dir_all(&self.root_dir).unwrap();
        }
    }
}

#[instrument]
pub fn get_config() -> Result<Config> {
    let vote_receipt_pipe_config = PipeConfigVoteReceipts::new();

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
                        id: "vote-receipts".to_string(),
                        pipe: PipeName::VoteReceipts,
                        config: Some(serde_json::to_value(vote_receipt_pipe_config)?),
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
                        pipe: PipeName::GenerateReports,
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

    Ok(Config {
        version: "0.0.0".to_string(),
        stages,
    })
}

#[instrument]
pub fn get_config_mcballots() -> Result<Config> {
    let vote_receipt_pipe_config = PipeConfigVoteReceipts::new();
    let mcballot_receipt_pipe_config = PipeConfigVoteReceipts::mcballot(None);

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
                        id: "decode-multi-ballots".to_string(),
                        pipe: PipeName::DecodeMCBallots,
                        config: Some(serde_json::Value::Null),
                    },
                    config::PipeConfig {
                        id: "vote-receipts".to_string(),
                        pipe: PipeName::VoteReceipts,
                        config: Some(serde_json::to_value(vote_receipt_pipe_config)?),
                    },
                    config::PipeConfig {
                        id: "multi-ballot-receipts".to_string(),
                        pipe: PipeName::MCBallotReceipts,
                        config: Some(serde_json::to_value(mcballot_receipt_pipe_config)?),
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
                        pipe: PipeName::GenerateReports,
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

    Ok(Config {
        version: "0.0.0".to_string(),
        stages,
    })
}
