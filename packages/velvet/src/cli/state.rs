// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::error::{Error, Result};
use super::CliRun;
use crate::pipes::PipeManager;
use crate::{config::Config, pipes::pipe_name::PipeName};

#[derive(Debug)]
pub struct State {
    pub cli: CliRun,
    pub stages: Vec<Stage>,
}

#[derive(Debug, Clone)]
pub struct Stage {
    pub name: String,
    pub pipeline: Vec<PipeName>,
    pub current_pipe: PipeName,
}

impl State {
    pub fn new(cli: &CliRun, config: &Config) -> Result<Self> {
        let stages = config
            .stages
            .order
            .iter()
            .map(|stage_name| {
                let pipeline = &config
                    .stages
                    .stages_def
                    .get(stage_name)
                    .ok_or(Error::StageDefinition(format!(
                        "Pipeline is not defined for stage '{stage_name}'"
                    )))?
                    .pipeline;

                Ok(Stage {
                    name: stage_name.to_string(),
                    pipeline: pipeline.iter().map(|p| p.pipe).collect(),
                    current_pipe: pipeline
                        .iter()
                        .find(|p| p.id == cli.pipe_id)
                        .ok_or(Error::StageDefinition(format!(
                            "Pipe '{}' is not found",
                            cli.pipe_id
                        )))?
                        .pipe,
                })
            })
            .collect::<Result<Vec<Stage>>>()?;

        Ok(Self {
            cli: cli.clone(),
            stages,
        })
    }

    pub fn exec_next(&mut self) -> Result<()> {
        let stage_name = self.cli.stage.clone();
        let stage = self.get_stage(&stage_name).ok_or(Error::PipeNotFound)?;

        let cli = self.cli.clone();
        let pm = PipeManager::get_pipe(cli, stage.clone())?.ok_or(Error::PipeNotFound)?;
        pm.exec().map_err(|e| Error::PipeExec(e.to_string()))?;

        if let Some(pipe) = stage.next_pipe() {
            self.set_current_pipe(&stage_name, pipe)?;
        }

        Ok(())
    }

    fn get_stage(&self, stage_name: &str) -> Option<&Stage> {
        self.stages.iter().find(|s| s.name == stage_name)
    }

    fn set_current_pipe(&mut self, stage_name: &str, next_pipe: PipeName) -> Result<()> {
        let stage = self
            .stages
            .iter_mut()
            .find(|s| s.name == stage_name)
            .ok_or(Error::PipeNotFound)?;

        stage.current_pipe = next_pipe;

        Ok(())
    }
}

impl Stage {
    pub fn previous_pipe(&self) -> Option<PipeName> {
        let curr_index = self.pipeline.iter().position(|p| *p == self.current_pipe);
        if let Some(curr_index) = curr_index {
            if curr_index > 0 {
                return Some(self.pipeline[curr_index - 1]);
            }
        }
        None
    }
    pub fn next_pipe(&self) -> Option<PipeName> {
        let curr_index = self.pipeline.iter().position(|p| *p == self.current_pipe);
        if let Some(curr_index) = curr_index {
            if curr_index + 1 < self.pipeline.len() {
                return Some(self.pipeline[curr_index + 1]);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config, fixtures::TestFixture};
    use anyhow::Result;
    use std::{collections::HashMap, path::PathBuf};

    #[test]
    fn test_state_new() -> Result<()> {
        let cli = CliRun {
            stage: "main".to_string(),
            pipe_id: "do-tally".to_string(),
            config: PathBuf::new(),
            input_dir: PathBuf::new(),
            output_dir: PathBuf::new(),
        };

        let stages_def = {
            let mut map = HashMap::new();
            map.insert(
                "main".to_string(),
                config::Stage {
                    pipeline: vec![
                        config::PipeConfig {
                            id: "decode".to_string(),
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
            version: "1.0".to_string(),
            stages,
        };

        let state = State::new(&cli, &config)?;
        let stage = &state.stages[0];
        assert_eq!(state.stages.len(), 1);
        assert_eq!(stage.name, "main");
        assert_eq!(stage.previous_pipe(), Some(PipeName::DecodeBallots));
        assert_eq!(stage.current_pipe, PipeName::DoTally);
        assert_eq!(stage.next_pipe(), Some(PipeName::MarkWinners));

        Ok(())
    }

    #[test]
    fn test_state_exec_next() -> Result<()> {
        let fixture = TestFixture::new()?;

        let cli = CliRun {
            stage: "main".to_string(),
            pipe_id: "do-tally".to_string(),
            config: PathBuf::new(),
            input_dir: fixture.root_dir.join("tests").join("input-dir"),
            output_dir: fixture.root_dir.join("tests").join("output-dir"),
        };

        let stages_def = {
            let mut map = HashMap::new();
            map.insert(
                "main".to_string(),
                config::Stage {
                    pipeline: vec![
                        config::PipeConfig {
                            id: "decode".to_string(),
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
            version: "1.0.0".to_string(),
            stages,
        };

        let mut state = State::new(&cli, &config)?;
        assert_eq!(state.stages.len(), 1);
        assert_eq!(state.stages[0].name, "main");
        assert_eq!(state.stages[0].current_pipe, PipeName::DoTally);

        state.exec_next()?;
        assert_eq!(state.stages[0].current_pipe, PipeName::MarkWinners);

        Ok(())
    }
}
