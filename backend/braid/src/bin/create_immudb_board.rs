cfg_if::cfg_if! {
    if #[cfg(feature = "bb-test")] {

use anyhow::{anyhow, Result};
use braid::util::init_log;
use clap::Parser;
use std::path::PathBuf;
use tracing::{info, instrument};

#[derive(Parser)]
struct Cli {
    #[arg(long)]
    cache_dir: PathBuf,


    #[arg(long)]
    immudb_url: String,

    // TODO: Missing: authentication options:
    // a) Username & password
    // b) PEM certificate path

    #[arg(value_enum)]
    action: Action,
}

#[derive(clap::ValueEnum, Clone)]
enum Action {
    Init,
    Ballots,
    List,
}

#[tokio::main]
#[instrument]
async fn main() -> Result<()> {
    init_log(true);
    let args = Cli::parse();
    Ok(())
}

    } else {

fn main() {
    println!("Requires the 'bb-test' feature");
}

    }
}
