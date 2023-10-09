use super::error::{Error, Result};
use super::CliRun;
use crate::{config::Config, pipes::pipe_name::PipeName};

#[derive(Debug)]
pub struct State {
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

        Ok(Self { stages })
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

        if curr_index + 1 < stage.pipeline.len() {
            stage.current_pipe = stage.pipeline[curr_index + 1];
        }

        Ok(())
    }
}
