// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::types::config::ConfigData;
use crate::utils::cast_vote::get_ballot_styles::BallotStyle;
use crate::utils::cast_vote::get_first_available_election::get_first_available_election;
use crate::utils::keycloak::generate_keycloak_token;
use crate::utils::read_config::read_config;
use clap::Args;
use serde::Deserialize;
use serde_json::Value;
use std::fs::File;
use std::io::BufReader;

#[derive(Args, Debug)]
#[command(about = "Insert multiple votes from a CSV file", long_about = None)]
pub struct InsertMultipleVotes {
    /// Path to the CSV file containing voter credentials
    #[arg(long)]
    csv_path: String,
    /// Number of votes to process (defaults to all votes in the CSV)
    #[arg(long, default_value = "0")]
    num_of_votes: usize,
    /// Password to use for all voters
    #[arg(long)]
    password: String,
}

#[derive(Debug, Deserialize, Clone)]
struct VoterCredential {
    username: String,
    password: String,
}

impl InsertMultipleVotes {
    pub fn run(&self) {
        match insert_multiple_votes(&self.csv_path, self.num_of_votes, &self.password) {
            Ok((success_count, error_count)) => {
                println!("\nSummary:");
                println!("Successfully processed votes: {}", success_count);
                println!("Failed votes: {}", error_count);
            }
            Err(err) => {
                eprintln!("Error! Failed to process votes: {}", err);
            }
        }
    }
}

pub fn insert_multiple_votes(
    csv_path: &str,
    num_of_votes: usize,
    password: &str,
) -> Result<(usize, usize), Box<dyn std::error::Error>> {
    let base_config = read_config()?;

    let file = File::open(csv_path)?;
    let reader = BufReader::new(file);
    let mut csv_reader = csv::ReaderBuilder::new().from_reader(reader);

    let headers = csv_reader.headers()?.clone();

    let username_index = headers
        .iter()
        .position(|h| h == "username")
        .ok_or_else(|| "CSV does not contain a 'username' column")?;

    let mut success_count = 0;
    let mut error_count = 0;
    let mut processed_count = 0;

    for result in csv_reader.records() {
        if num_of_votes > 0 && processed_count >= num_of_votes {
            break;
        }

        match result {
            Ok(record) => {
                let username = record.get(username_index).unwrap_or("");
 
                let voter = VoterCredential {
                    username: username.to_string(),
                    password: password.to_string(),
                };

                match process_voter(&base_config, &voter) {
                    Ok(_) => {
                        success_count += 1;
                    }
                    Err(err) => {
                        error_count += 1;
                        eprintln!("Error processing voter {}: {}", voter.username, err);
                    }
                }
            }
            Err(err) => {
                error_count += 1;
                eprintln!("Error reading CSV row: {}", err);
            }
        }
        processed_count += 1;
    }

    if num_of_votes > 0 && processed_count < num_of_votes {
        println!("CSV has only {} votes", processed_count);
    }

    Ok((success_count, error_count))
}

fn process_voter(
    base_config: &ConfigData,
    voter: &VoterCredential,
) -> Result<(), Box<dyn std::error::Error>> {
    // Generate new Keycloak token for this voter
    let auth_details = generate_keycloak_token(
        &base_config.keycloak_url,
        &voter.username,
        &voter.password,
        &base_config.client_id,
        &base_config.client_secret,
        &base_config.tenant_id,
        &base_config.election_event_id,
    )?;

    // Create new config with voter's token
    let config = ConfigData {
        auth_token: auth_details.access_token,
        refresh_token: auth_details.refresh_token,
        username: voter.username.clone(),
        endpoint_url: base_config.endpoint_url.clone(),
        tenant_id: base_config.tenant_id.clone(),
        keycloak_url: base_config.keycloak_url.clone(),
        client_id: base_config.client_id.clone(),
        client_secret: base_config.client_secret.clone(),
        election_event_id: base_config.election_event_id.clone(),
    };

    println!("username: {}", config.username);

    // Get first available election and its ballot style
    let (ballot_style, election) = get_first_available_election(&config)
        .map_err(|e| format!("Failed to get first available election: {}", e))?;


    // TODO: need to finish the random selection and hash it so it can be inserted as cast vote in the content.
    // Create random ballot selection
    // let ballot_selection = create_random_ballot_selection(&ballot_style)
    //     .map_err(|e| format!("Failed to create random ballot selection: {}", e))?;

    // Cast the vote
    // let _cast_vote = insert_cast_vote(
    //     &config,
    //     &election.id,
    //     &ballot_style.id,
    //     &ballot_selection,
    // ).map_err(|e| format!("Failed to insert cast vote: {}", e))?;

    Ok(())
}

fn parse_eml(ballot_eml: &serde_json::Value) -> Result<Value, Box<dyn std::error::Error>> {
    let eml: Value = serde_json::from_value(ballot_eml.clone())?;
    Ok(eml)
}

fn create_random_ballot_selection(ballot_style: &BallotStyle) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let mut rng = rand::thread_rng();
    let ballot_eml = &ballot_style.ballot_eml;
    let ballot_json = parse_eml(ballot_eml)?;
    let contests = ballot_eml["contests"]
        .as_array()
        .ok_or("No contests found in ballot EML")?;
    println!("contest: {:?}", contests[0]);
    
    for contest in contests {
        let contest_id = contest["id"]
            .as_str()
            .ok_or("Contest ID not found")?;
        let choices = contest["Selection"]
            .as_array()
            .ok_or("No choices found in contest")?;

        // Create a contest selection structure matching the voting portal
    //     let mut contest_selection = serde_json::json!({
    //         "contest_id": contest_id,
    //         "is_explicit_invalid": false,
    //         "invalid_errors": [],
    //         "invalid_alerts": [],
    //         "choices": choices.iter().map(|choice| {
    //             serde_json::json!({
    //                 "id": choice["@id"].as_str().unwrap_or(""),
    //                 "selected": -1  // -1 means not selected
    //             })
    //         }).collect::<Vec<_>>()
    //     });

    //     // Randomly select one choice
    //     if let Some(choice) = choices.choose(&mut rng) {
    //         let choice_id = choice["@id"].as_str().unwrap_or("");
    //         // Find and update the selected choice
    //         if let Some(choices) = contest_selection["choices"].as_array_mut() {
    //             if let Some(selected_choice) = choices.iter_mut().find(|c| c["id"] == choice_id) {
    //                 selected_choice["selected"] = serde_json::json!(1);
    //             }
    //         }
    //     }

    //     selections.push(contest_selection);
    // }
    }
    Ok(serde_json::json!({
        "selections": []
    }))
} 