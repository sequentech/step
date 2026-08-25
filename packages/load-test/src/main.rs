// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

mod config;

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(
    name = "load-test",
    version,
    about = "Provisions election events across tenants and casts votes against them directly over the network"
)]
struct Cli {
    /// Path to the layers.yaml file describing tenants, election events, and
    /// vote load per event
    #[arg(long)]
    layers_file: PathBuf,

    /// Path to the election-event.json template imported into every
    /// synthetic election event
    #[arg(long)]
    election_event_template: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let layers = config::load_layers(&cli.layers_file)?;
    let _template = config::load_election_event_template(&cli.election_event_template)?;

    println!(
        "Loaded {} tenant(s) from {}",
        layers.tenants.len(),
        cli.layers_file.display()
    );
    println!(
        "Loaded election event template from {}",
        cli.election_event_template.display()
    );

    Ok(())
}
