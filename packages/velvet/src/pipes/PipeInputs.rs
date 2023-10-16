use super::error::{Error, Result};
use crate::cli::CliRun;
use std::{
    fs,
    path::{Path, PathBuf},
};
use uuid::Uuid;

const PREFIX_ELECTION: &str = "election__";
const PREFIX_CONTEST: &str = "contest__";

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
        let input = &cli.input_dir.to_str().ok_or(Error::IncorrectPath)?;
        Self::read_input_dir_config(input)?;

        Ok(Self { cli: cli.clone() })
    }

    fn read_input_dir_config(input_dir: &str) -> Result<()> {
        let entries = fs::read_dir(format!("{}/default/configs", input_dir))?;

        for entry in entries {
            Self::read_election_list_config(&entry?.path())?;
        }

        Ok(())
    }

    fn read_election_list_config(path: &Path) -> Result<()> {
        let entries = fs::read_dir(path.to_str().ok_or(Error::IncorrectPath)?)?;
        let election_uuid = Self::parse_path_components(path, PREFIX_ELECTION);
        dbg!(election_uuid);

        for entry in entries {
            let path = entry?.path();
            if path.is_dir() {
                let contest_uuid = Self::parse_path_components(&path, PREFIX_CONTEST);
                dbg!(contest_uuid);
                dbg!(path);
            }
        }

        Ok(())
    }

    fn parse_path_components(path: &Path, prefix: &str) -> Option<Uuid> {
        for component in path.components() {
            let part = component.as_os_str().to_string_lossy();

            if let Some(res) = part.strip_prefix(prefix) {
                return Uuid::parse_str(res).ok();
            }
        }

        None
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
