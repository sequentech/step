use super::error::{Error, Result};
use crate::cli::CliRun;
use std::{fs, path::PathBuf};
use uuid::Uuid;

pub trait PipeInputsRead {
    // read input_dir into PipeInput
    fn read_input_dir_config(&self) -> Result<()>;
}

pub struct PipeInputs {
    pub cli: CliRun,
    // pub election_list: Vec<ElectionConfig>,
}

impl PipeInputsRead for PipeInputs {
    fn read_input_dir_config(&self) -> Result<()> {
        let entries = fs::read_dir(format!(
            "{}/default/configs",
            &self.cli.input_dir.to_str().ok_or(Error::Toto)?
        ))?;

        // entries.map(|e| e.path()).collect::<Result<Vec<_>>>();
        for entry in entries {
            self.read_election_list_config(&entry?.path());
        }

        Ok(())
    }
}

impl PipeInputs {
    fn read_election_list_config(&self, path: &PathBuf) {
        dbg!(path);
    }
}

struct ElectionConfig {
    id: Uuid,
    config: PathBuf,
    contest_list: Vec<ContestForElectionConfig>,
}

struct ContestForElectionConfig {
    id: Uuid,
    election_id: Uuid,
    config: PathBuf,
}
