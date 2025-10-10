// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

mod services;

use anyhow::Result;
use clap::Parser;
use services::{
    enrollment::run_enrollment_test, login::run_login_test, reports::run_reports_test,
    voting::run_voting_test,
};

#[derive(Parser, Debug)]
#[command(
    name = "E2ELoadTestRunner",
    about = "A generic load-test runner for Loadero"
)]
pub struct Args {
    #[arg(long, default_value = "enrollment")]
    test_type: String,

    #[arg(short = 'p', long, default_value_t = 1)]
    participants: u64,

    #[arg(long, default_value_t = false)]
    update: bool,

    #[arg(long, long, default_value_t = 60)]
    test_duration: u64,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let result = match args.test_type.as_str() {
        "enrollment" => run_enrollment_test(&args)?,
        "voting" => run_voting_test(&args)?,
        "login" => run_login_test(&args)?,
        "reports" => run_reports_test(&args)?,
        _ => {
            eprintln!("Unknown test type: {}", args.test_type);
            std::process::exit(1);
        }
    };

    // if let Err(e) = result {
    //     eprintln!("Error: {}", e);
    //     std::process::exit(1);
    // }

    Ok(())
}
