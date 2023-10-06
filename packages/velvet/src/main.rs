use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "Velvet")]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
    // #[arg(short, long)]
    // config: PathBuf,
    //
    // #[arg(short, long)]
    // input_dir: PathBuf,
    //
    // #[arg(short, long)]
    // output_dir: PathBuf,
}

#[derive(Subcommand)]
enum Commands {
    Run { stage: String, pipe_id: String },
}

// $> velvet run {stage} {optional-pipe} \
//   --config ./path/to/velvet-config.json \
//   --input-dir ./path/to/input-dir \
//   --output-dir ./path/to/output-dir

fn main() -> std::result::Result<(), String> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { stage, pipe_id } => {
            dbg!(&stage);
            dbg!(&pipe_id);
        }
    }

    Ok(())
}
