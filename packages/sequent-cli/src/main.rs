mod commands;
mod types;
mod utils;

use clap::{Parser, Subcommand};


#[derive(Parser)]
#[command(name = "sequent-cli", version = "1.0", about = "CLI tool for managing Sequent tasks")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Config(commands::configure::Config),
    CreateElectionEvent(commands::create_election_event::CreateElectionEvent)
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Config(cmd) => cmd.run(),
        Commands::CreateElectionEvent(create_event) => create_event.run(),
    }
}