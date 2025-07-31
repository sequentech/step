// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{config::Config, utils::parse_file};

use super::error::{Error, Result};
use clap::{Parser, Subcommand};
use std::{collections::HashSet, fs::File, path::PathBuf};

#[derive(Parser)]
#[command(name = "Velvet")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Run(CliRun),
}

#[derive(Parser, Debug, Clone)]
pub struct CliRun {
    pub stage: String,
    pub pipe_id: String,

    #[arg(short, long)]
    pub config: PathBuf,

    #[arg(short, long)]
    pub input_dir: PathBuf,

    #[arg(short, long)]
    pub output_dir: PathBuf,
}

impl CliRun {
    pub fn validate(&self) -> Result<Config> {
        let config = self.parse_config()?;

        Ok(config)
    }

    fn parse_config(&self) -> Result<Config> {
        if !self.config.exists() {
            return Err(Error::ConfigNotFound);
        }

        let file = File::open(&self.config).map_err(|_| Error::CannotOpenConfig)?;
        let config: Config = parse_file(file)?;

        for stage in &config.stages.order {
            if !config.stages.stages_def.contains_key(stage) {
                return Err(Error::StageDefinition(format!(
                    "Stage '{stage}', defined in stages.order, is not defined in stages."
                )));
            } else {
                let stage_def = config.stages.stages_def.get(stage).unwrap();
                let pipeline = &stage_def.pipeline;
                let hash_set: HashSet<_> = pipeline.iter().map(|p| p.pipe.as_ref()).collect();
                if hash_set.len() != pipeline.len() {
                    return Err(Error::StageDefinition(format!("Pipeline, defined in stages[{stage}].pipeline, should have unique pipe definition")));
                }
            }
        }

        Ok(config)
    }
}
#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::{
        config,
        fixtures::{self, TestFixture},
        pipes::pipe_name::PipeName,
    };
    use anyhow::Result;

    #[test]
    fn test_clirun_validate() -> Result<()> {
        let fixture = TestFixture::new()?;
        let cli = CliRun {
            stage: "main".to_string(),
            pipe_id: "do-tally".to_string(),
            config: fixture.config_path.clone(),
            input_dir: PathBuf::new(),
            output_dir: PathBuf::new(),
        };

        let config = cli.validate()?;

        assert!(config.stages.stages_def.contains_key("main"));

        let main_stage = config.stages.stages_def.get("main").unwrap();
        assert_eq!(main_stage.pipeline.len(), 6);

        Ok(())
    }

    #[test]
    fn test_clirun_validate_pipeline_definition() -> Result<()> {
        let fixture = TestFixture::new()?;
        let mut config = fixtures::get_config()?;

        config
            .stages
            .stages_def
            .get_mut("main")
            .unwrap()
            .pipeline
            .push(config::PipeConfig {
                id: "gen-report".to_string(),
                pipe: PipeName::GenerateReports,
                config: Some(serde_json::Value::Null),
            });

        fs::write(
            fixture.config_path.clone(),
            &serde_json::to_string(&config)?,
        )?;

        let cli = CliRun {
            stage: "main".to_string(),
            pipe_id: "do-tally".to_string(),
            config: fixture.config_path.clone(),
            input_dir: PathBuf::new(),
            output_dir: PathBuf::new(),
        };

        let res = cli.validate();

        assert!(res.is_err());
        Ok(())
    }

    #[test]
    #[should_panic]
    fn test_clirun_validate_not_found() {
        let _fixture = TestFixture::new();
        let cli = CliRun {
            stage: "main".to_string(),
            pipe_id: "do-tally".to_string(),
            config: PathBuf::from("do-not-exist.json"),
            input_dir: PathBuf::new(),
            output_dir: PathBuf::new(),
        };

        let _ = cli.validate().unwrap();
    }
}
