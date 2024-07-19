// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

mod cli;
mod config;
mod fixtures;
mod pipes;
mod utils;

use cli::console::ciccp_consolidation;
use pipes::mark_winners::MarkWinners;

use crate::pipes::error::{Error, Result};
use std::env;
/* 
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
}*/
fn main() -> Result<()> {

    // Ensure correct number of arguments
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <base_path> <folder_common>", args[0]);
        std::process::exit(1);
    }

    let base_path = &args[1];
    let folder_common = &args[2];

    let contest_result = ciccp_consolidation(base_path.as_str(), folder_common.as_str())?;
    let winners = MarkWinners::get_winners(&contest_result);
    let aggregate_str = serde_json::to_string(&contest_result)?;
    println!("{}", aggregate_str);

    Ok(())
}