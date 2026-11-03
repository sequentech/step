// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::{env, fs};

use crate::utils::read_config::read_config;

fn custody_request_path(file_name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let exe_path = env::current_exe().map_err(|_| "Failed to get current executable path")?;
    let parent_dir = exe_path
        .parent()
        .ok_or("Failed to get executable directory")?;

    Ok(parent_dir.join("keys").join(file_name))
}

// The custody request for an election event, which confirm-key-tally reads
fn event_custody_request_path(
    election_event_id: &str,
    username: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    custody_request_path(&format!(
        "custody_request_trustee_{}_{}.json",
        username, election_event_id
    ))
}

// The custody request for a single key ceremony, as complete-key-ceremony stores it
fn ceremony_custody_request_path(
    election_event_id: &str,
    key_ceremony_id: &str,
    username: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    custody_request_path(&format!(
        "custody_request_trustee_{}_{}_{}.json",
        username, election_event_id, key_ceremony_id
    ))
}

fn write_custody_request(
    file_path: PathBuf,
    custody_request: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Some(dir_path) = file_path.parent() {
        if !dir_path.exists() {
            fs::create_dir_all(dir_path)?;
        }
    }

    // Write the public custody request to the file
    let mut file = File::create(&file_path)?;
    file.write_all(custody_request.as_bytes())?;

    Ok(file_path)
}

pub fn download_custody_request(
    election_event_id: &str,
    key_ceremony_id: &str,
    custody_request: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let config = read_config()?;
    write_custody_request(
        ceremony_custody_request_path(election_event_id, key_ceremony_id, &config.username)?,
        custody_request,
    )
}

/// Stores a confirmed custody request for the election event.
pub fn store_event_custody_request(
    election_event_id: &str,
    custody_request: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let config = read_config()?;
    write_custody_request(
        event_custody_request_path(election_event_id, &config.username)?,
        custody_request,
    )
}

/// The public custody request a previous run stored for the configured trustee in a key
/// ceremony, and where it is.
pub fn read_stored_custody_request(
    election_event_id: &str,
    key_ceremony_id: &str,
) -> Result<(String, PathBuf), Box<dyn std::error::Error>> {
    let config = read_config()?;
    let file_path =
        ceremony_custody_request_path(election_event_id, key_ceremony_id, &config.username)?;
    let custody_request = fs::read_to_string(&file_path)
        .map_err(|error| format!("{}: {}", file_path.display(), error))?;

    Ok((custody_request, file_path))
}

pub fn get_custody_request_content(
    election_event_id: &str,
    client_username: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let path = event_custody_request_path(election_event_id, client_username)?;

    let mut file = File::open(path)?;

    // Read the file contents into a string
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    Ok(content)
}
