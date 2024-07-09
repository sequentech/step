mod commands;
mod types;
mod utils;

use clap::{Parser, Subcommand};
use types::args::Args;


#[derive(Parser)]
#[command(name = "sequent", version = "1.0", about = "CLI tool for managing Sequent tasks")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Config(commands::configure::Config),
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Config(cmd) => cmd.run(),
    }
}