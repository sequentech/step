// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::error::{Error, Result};
use super::CliRun;
use crate::config::PipeConfig;
use crate::pipes::error::Error as PipesError;
use crate::pipes::generate_reports::{ElectionReportDataComputed, GenerateReports};
use crate::pipes::pipe_inputs::PipeInputs;
use crate::pipes::PipeManager;
use crate::{config::Config, pipes::pipe_name::PipeName};
use tracing::instrument;

#[derive(Debug, Clone)]
pub struct State {
    pub cli: CliRun,
    pub stages: Vec<Stage>,
}

#[derive(Debug, Clone)]
pub struct Stage {
    pub name: String,
    pub pipeline: Vec<PipeConfig>,
    pub current_pipe: Option<PipeName>,
    pub previous_pipe: Option<PipeName>,
}

impl State {
    #[instrument(err, skip(config), name = "State::new")]
    pub fn new(cli: &CliRun, config: &Config) -> Result<Self> {
        let stages =
            config
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

                    let current_pipe = pipeline.iter().find(|p| p.id == cli.pipe_id).ok_or(
                        Error::StageDefinition(format!("Pipe '{}' is not found", cli.pipe_id)),
                    )?;

                    Ok(Stage {
                        name: stage_name.to_string(),
                        pipeline: pipeline.to_vec(),
                        previous_pipe: None,
                        current_pipe: Some(current_pipe.pipe),
                    })
                })
                .collect::<Result<Vec<Stage>>>()?;

        Ok(Self {
            cli: cli.clone(),
            stages,
        })
    }

    #[instrument(skip_all)]
    pub fn get_next(&self) -> Option<PipeName> {
        let stage_name = self.cli.stage.clone();
        self.get_stage(&stage_name)
            .map(|stage| {
                if stage.current_pipe == stage.previous_pipe {
                    None
                } else {
                    stage.current_pipe
                }
            })
            .flatten()
    }

    #[instrument(skip_all)]
    pub fn exec_next(&mut self) -> Result<()> {
        let stage_name = self.cli.stage.clone();
        let stage = self.get_stage(&stage_name).ok_or(Error::PipeNotFound)?;

        let cli = self.cli.clone();
        let pm = PipeManager::get_pipe(cli, stage.clone())?.ok_or(Error::PipeNotFound)?;

        let res = pm.exec();

        if let Err(e) = res {
            if let PipesError::FileAccess(file, _) = e {
                println!("File not found: {} -- Not processed", file.display())
            } else {
                return Err(Error::FromPipe(e));
            }
        }

        self.set_current_pipe(&stage_name, stage.next_pipe())?;

        Ok(())
    }

    #[instrument(skip(self))]
    fn get_stage(&self, stage_name: &str) -> Option<&Stage> {
        self.stages.iter().find(|s| s.name == stage_name)
    }

    #[instrument(skip(self))]
    fn set_current_pipe(&mut self, stage_name: &str, next_pipe: Option<PipeName>) -> Result<()> {
        let stage = self
            .stages
            .iter_mut()
            .find(|s| s.name == stage_name)
            .ok_or(Error::PipeNotFound)?;

        stage.previous_pipe = stage.current_pipe.clone();
        stage.current_pipe = next_pipe;
        stage.current_pipe = next_pipe;

        Ok(())
    }

    #[instrument(skip_all, err)]
    pub fn get_results(&self, force: bool) -> Result<Vec<ElectionReportDataComputed>> {
        let next_pipename = self.get_next();

        // not all pipelines have been executed, bail out
        if next_pipename.is_some() && !force {
            return Err(Error::PipeNotFound);
        }

        let stage_name = self.cli.stage.clone();
        let stage_ref = self.get_stage(&stage_name).ok_or(Error::PipeNotFound)?;
        let mut stage = stage_ref.clone();
        stage.current_pipe = Some(PipeName::GenerateReports);
        let cli = self.cli.clone();
        let pipe_inputs = PipeInputs::new(cli, stage.clone())?;

        let gen_reports = GenerateReports::new(pipe_inputs);
        let reports = gen_reports.read_reports()?;
        Ok(reports)
    }
}

impl Stage {
    #[instrument(skip_all)]
    pub fn previous_pipe(&self) -> Option<PipeName> {
        if let Some(current_pipe) = self.current_pipe {
            let curr_index = self.pipeline.iter().position(|p| p.pipe == current_pipe);
            if let Some(curr_index) = curr_index {
                if curr_index > 0 {
                    return Some(self.pipeline[curr_index - 1].pipe);
                }
            }
            None
        } else {
            Some(self.pipeline[self.pipeline.len() - 1].pipe)
        }
    }

    #[instrument(skip_all)]
    pub fn next_pipe(&self) -> Option<PipeName> {
        if let Some(current_pipe) = self.current_pipe {
            let curr_index = self.pipeline.iter().position(|p| p.pipe == current_pipe);
            if let Some(curr_index) = curr_index {
                if curr_index + 1 < self.pipeline.len() {
                    return Some(self.pipeline[curr_index + 1].pipe);
                }
            }
            None
        } else {
            None
        }
    }

    pub fn pipe_config(&self, pipe: Option<PipeName>) -> Option<PipeConfig> {
        if let Some(pipe) = pipe {
            self.pipeline.iter().find(|pc| pc.pipe == pipe).cloned()
        } else {
            None
        }
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
        assert_eq!(stage.current_pipe, Some(PipeName::DoTally));
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
        assert_eq!(state.cli.stage, "main");
        assert_eq!(state.cli.pipe_id, "do-tally");

        assert_eq!(state.stages.len(), 1);
        assert_eq!(state.stages[0].name, "main");
        assert_eq!(state.stages[0].current_pipe, Some(PipeName::DoTally));

        state.exec_next()?;
        assert_eq!(state.cli.stage, "main");
        assert_eq!(state.cli.pipe_id, "do-tally");
        assert_eq!(state.stages[0].current_pipe, Some(PipeName::MarkWinners));

        state.exec_next()?;
        assert_eq!(state.cli.pipe_id, "do-tally");
        assert_eq!(state.stages[0].current_pipe, None);

        Ok(())
    }
}
