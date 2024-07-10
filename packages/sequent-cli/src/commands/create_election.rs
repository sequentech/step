// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use clap::Args;

#[derive(Args)]
#[command(about = "Create a new election", long_about = None)]
pub struct CreateElection;

impl CreateElection {
    pub fn run(&self) {

        // match create_election(&event) {
        //     Ok(_) => println!("Election event created successfully!"),
        //     Err(err) => eprintln!("Failed to create election event: {}", err),
        // }
    }
}

// fn create_election(event: &Election) -> Result<(), Box<dyn std::error::Error>> {
//     let config = read_config()?;
//     let client = reqwest::blocking::Client::new();

//     let endpoint_url = format!("{}/election", config.endpoint_url);
//     let response = client
//         .post(&endpoint_url)
//         .bearer_auth(config.auth_token)
//         .json(&event)
//         .send()?;

//         if response.status().is_success() {
//             let response_data: CreateElectionEventResponse = response.json()?;
//             Ok(response_data.id)
//         } else {
//             let status = response.status();
//             let error_message = response.text()?;
//             let error = format!("HTTP Status: {}\nError Message: {}", status, error_message);
//             Err(Box::from(error))
//         }
// }
