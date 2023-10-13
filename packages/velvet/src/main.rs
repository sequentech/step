mod cli;
mod config;
mod pipes;

#[cfg(test)]
mod fixtures;

use clap::Parser;
use cli::{Cli, Commands};

fn main() -> std::result::Result<(), Box<dyn std::error::Error + 'static>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run(run) => {
            run.validate()?;
        }
    }

    Ok(())
}
