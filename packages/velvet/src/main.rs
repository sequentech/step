mod cli;
mod config;
mod pipes;

#[cfg(test)]
mod fixtures;

use clap::Parser;
use cli::{state::State, Cli, Commands};

fn main() -> std::result::Result<(), Box<dyn std::error::Error + 'static>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run(run) => {
            let config = run.validate()?;

            let mut state = State::new(&run, &config)?;
            state.exec_next()?;
        }
    }

    Ok(())
}
