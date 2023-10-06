mod error;

use crate::config::Config;

use self::error::{Error, Result};
use clap::{Parser, Subcommand};
use std::{fs::File, path::PathBuf};

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

#[derive(Parser)]
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
    pub fn validate(&self) -> Result<()> {
        self.parse_config()?;
        Ok(())
    }

    fn parse_config(&self) -> Result<()> {
        if !self.config.exists() {
            return Err(Error::ConfigNotFound);
        }

        let file = File::open(&self.config).map_err(|_| Error::CannotOpenConfig)?;
        let cfg: Config = serde_json::from_reader(file)?;

        dbg!(cfg);

        Ok(())
    }
}
