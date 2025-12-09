// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

mod cli;
mod config;
mod fixtures;
mod pipes;
mod utils;

use clap::Parser;
use cli::{state::State, Cli, Commands};
use sequent_core::util::init_log::init_log;
use tracing::{event, Level};

fn main() -> std::result::Result<(), Box<dyn std::error::Error + 'static>> {
    let cli = Cli::parse();
    init_log(true);

    match cli.command {
        Commands::Run(run) => {
            let config = run.validate()?;
            let mut state = State::new(&run, &config)?;

            while let Some(next_stage) = state.get_next() {
                let stage_name = next_stage.to_string();
                event!(Level::INFO, "Exec {}", stage_name);
                state.exec_next()?;
            }
        }
    }

    Ok(())
}
