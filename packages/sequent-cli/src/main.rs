// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

mod commands;
mod types;
mod utils;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "sequent-cli",
    version = "1.0",
    about = "CLI tool for managing Sequent tasks"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Config(commands::configure::Config),
    GenerateAuth(commands::generate_auth_token::GenerateToken),
    CreateElectionEvent(commands::create_election_event::CreateElectionEventCLI),
    CreateElection(commands::create_election::CreateElection),
    CreateContest(commands::create_contest::CreateContest),
    CreateCandidate(commands::create_candidate::CreateCandidate),
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Config(cmd) => cmd.run(),
        Commands::GenerateAuth(auth) => auth.run(),
        Commands::CreateElectionEvent(create_event) => create_event.run(),
        Commands::CreateElection(create_election) => create_election.run(),
        Commands::CreateContest(create_contest) => create_contest.run(),
        Commands::CreateCandidate(create_candidate) => create_candidate.run(),
    }
}
