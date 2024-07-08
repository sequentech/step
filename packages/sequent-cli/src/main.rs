mod commands;
mod types;
mod config;

use clap::Parser;
use types::args::Args;


fn main() {
    let args = Args::parse();
    commands::run(args);
}