use super::error::{Error, Result};
use super::CliRun;
use crate::pipes::match_run;
use crate::{config::Config, pipes::pipe_name::PipeName};

#[derive(Debug)]
pub struct State {
    pub cli: CliRun,
    pub stages: Vec<Stage>,
}

#[derive(Debug)]
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

    pub fn exec_next(&mut self, stage: &str) -> Result<()> {
        let stage = self
            .stages
            .iter_mut()
            .find(|s| s.name == stage)
            .ok_or(Error::StageNotFound)?;

        let curr_index = stage
            .pipeline
            .iter()
            .position(|p| *p == stage.current_pipe)
            .ok_or(Error::PipeNotFound)?;

        match_run(&self.cli, stage.current_pipe)?;

        if curr_index + 1 < stage.pipeline.len() {
            stage.current_pipe = stage.pipeline[curr_index + 1];
        }

        Ok(())
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
                    pipeline: vec![config::PipeConfig {
                        id: "do-tally".to_string(),
                        pipe: PipeName::DoTally,
                        config: Some(serde_json::Value::Null),
                    }],
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
        assert_eq!(state.stages.len(), 1);
        assert_eq!(state.stages[0].name, "main");
        assert_eq!(state.stages[0].current_pipe, PipeName::DoTally);

        Ok(())
    }

    #[test]
    fn test_state_exec_next() -> Result<()> {
        let fixture = TestFixture::new()?;

        let cli = CliRun {
            stage: "main".to_string(),
            pipe_id: "do-tally".to_string(),
            config: PathBuf::new(),
            input_dir: PathBuf::from(format!("{}/tests/input-dir", &fixture.root_dir)),
            output_dir: PathBuf::from(format!("{}/tests/output-dir", &fixture.root_dir)),
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
                            id: "consolidation".to_string(),
                            pipe: PipeName::Consolidation,
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

        state.exec_next("main")?;
        assert_eq!(state.stages[0].current_pipe, PipeName::Consolidation);

        Ok(())
    }
}
