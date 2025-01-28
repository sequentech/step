// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

mod services;

use clap::Parser;
use services::enrollment::{run_enrollment_test, EnrollmentScenarioData};

#[derive(Parser, Debug)]
#[command(
    name = "E2ELoadTestRunner",
    about = "A generic load-test runner for Loadero"
)]
pub struct Args {
    #[arg(long, default_value = "enrollment")]
    test_type: String,

    #[arg(long, default_value = "Generic Load Test")]
    test_name: String,

    #[arg(short = 'p', long, default_value_t = 1)]
    participants: u64,

    #[arg(long)]
    scenario_data_json: Option<String>,

    #[arg(short = 'o', long)]
    otp_code: String,

    #[arg(long, default_value_t = false)]
    update: bool,

    #[arg(long, long, default_value_t = 60)]
    test_duration: u64,
}

fn main() {
    let args = Args::parse();

    match args.test_type.as_str() {
        "enrollment" => {
            run_enrollment_test(&args);
        }
        _ => {
            eprintln!("Unknown test type: {}", args.test_type);
        }
    }
}
