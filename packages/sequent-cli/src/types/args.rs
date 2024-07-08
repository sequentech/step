use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: ArgsCommand,
}

#[derive(Subcommand, Debug)]
pub enum ArgsCommand {
    /// Configure the application
    Configure {
        #[arg(short, long)]
        field1: String,
        
        #[arg(short, long)]
        field2: String,
    },
}