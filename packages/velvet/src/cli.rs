use clap::{Parser, Subcommand};
use std::path::PathBuf;

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

// $> velvet run {stage} {optional-pipe} \
//   --config ./path/to/velvet-config.json \
//   --input-dir ./path/to/input-dir \
//   --output-dir ./path/to/output-dir
