use super::error::{Error, Result};
use crate::cli::CliRun;
use std::fs;

pub trait PipeInputRead {
    // read input_dir into PipeInput
    fn read_input_dir_config(&self) -> Result<()>;
}

pub struct PipeInput {
    pub cli: CliRun,
}

impl PipeInputRead for PipeInput {
    fn read_input_dir_config(&self) -> Result<()> {
        let entries = fs::read_dir(format!(
            "{}/default/configs",
            &self.cli.input_dir.to_str().ok_or(Error::Toto)?
        ))?;

        // entries.map(|e| e.path()).collect::<Result<Vec<_>>>();
        for entry in entries {
            dbg!(entry?.path());
        }

        Ok(())
    }
}
