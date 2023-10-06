use clap::Parser;
use cli::{Cli, Commands};

mod cli;

fn main() -> std::result::Result<(), String> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run(run) => {
            dbg!(&run.stage);
            dbg!(&run.pipe_id);
        }
    }

    Ok(())
}
