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

impl PipeInputs {
    pub fn new(cli: &CliRun) -> Result<Self> {
        // input_dir has already been validated
        let input = &cli.input_dir.to_str().unwrap();

        Self::read_input_dir_config(input)?;

        Ok(Self { cli: cli.clone() })
    }

    fn read_input_dir_config(input_dir: &str) -> Result<()> {
        let entries = fs::read_dir(format!("{}/default/configs", input_dir))?;

        // entries.map(|e| e.path()).collect::<Result<Vec<_>>>();
        for entry in entries {
            Self::read_election_list_config(&entry?.path());
        }

        Ok(())
    }

    fn read_election_list_config(path: &PathBuf) {
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
