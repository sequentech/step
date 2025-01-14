// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

mod services;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "ElectionApp", about = "An application to manage elections")]
struct Args {
    #[arg(short = 'e', long)]
    election_event_id: String,

    #[arg(short = 'p', long, default_value_t = 1)]
    participants: u64,

    #[arg(short = 'r', long, default_value_t = 10)]
    enrollments_per_participant: u64, // how many enrollments single participant will run

    #[arg(short = 'o', long)]
    otp_code: String,

    #[arg(long, default_value_t = false)]
    update: bool,
}

fn main() {
    let args = Args::parse();

    let enrollment_election_test_name =
        services::loadero_service::get_enrollment_test_name_str(&args.election_event_id);
    let test = match services::loadero_service::get_test_by_name(enrollment_election_test_name) {
        Ok(test) => test,
        Err(e) => {
            eprintln!("Error get test by name: {}", e);
            None
        }
    };
    if args.update && test.is_some() {
        match services::loadero_service::update_script(
            &test.unwrap(),
            &args.election_event_id,
            args.enrollments_per_participant,
            &args.otp_code,
        ) {
            Ok(_) => println!("Test updated successfully."),
            Err(e) => eprintln!("Error update script test: {}", e),
        }
    } else {
        match services::loadero_service::run_enrollment_test(
            &args.election_event_id,
            args.participants,
            args.enrollments_per_participant,
            &args.otp_code,
            test,
        ) {
            Ok(_) => println!("Test ran successfully."),
            Err(e) => eprintln!("Error running test: {}", e),
        }
    }
}
