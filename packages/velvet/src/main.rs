mod cli;
mod config;

use clap::Parser;
use cli::{Cli, Commands};


fn main() -> std::result::Result<(), Box<dyn std::error::Error + 'static>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run(run) => {
            run.validate()?;
            dbg!(&run.stage);
            dbg!(&run.pipe_id);
        }
    }

    Ok(())
}
