// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::{env, fs};

use crate::utils::read_config::read_config;

fn private_key_path(file_name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let exe_path = env::current_exe().map_err(|_| "Failed to get current executable path")?;
    let parent_dir = exe_path
        .parent()
        .ok_or("Failed to get executable directory")?;

    Ok(parent_dir.join("keys").join(file_name))
}

// The key of an election event, which confirm-key-tally reads
fn event_private_key_path(
    election_event_id: &str,
    username: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    private_key_path(&format!(
        "encrypted_private_key_trustee_{}_{}.txt",
        username, election_event_id
    ))
}

// The key of a single key ceremony, as complete-key-ceremony stores it
fn ceremony_private_key_path(
    election_event_id: &str,
    key_ceremony_id: &str,
    username: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    private_key_path(&format!(
        "encrypted_private_key_trustee_{}_{}_{}.txt",
        username, election_event_id, key_ceremony_id
    ))
}

fn write_private_key(
    file_path: PathBuf,
    private_key: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Some(dir_path) = file_path.parent() {
        if !dir_path.exists() {
            fs::create_dir_all(dir_path)?;
        }
    }

    // Write the private key to the file
    let mut file = File::create(&file_path)?;
    file.write_all(private_key.as_bytes())?;

    Ok(file_path)
}

pub fn download_private_key(
    election_event_id: &str,
    key_ceremony_id: &str,
    private_key: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let config = read_config()?;
    write_private_key(
        ceremony_private_key_path(election_event_id, key_ceremony_id, &config.username)?,
        private_key,
    )
}

/// Stores a checked key as the election event's key.
pub fn store_event_private_key(
    election_event_id: &str,
    private_key: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let config = read_config()?;
    write_private_key(
        event_private_key_path(election_event_id, &config.username)?,
        private_key,
    )
}

/// The private key a previous run stored for the configured trustee in a key
/// ceremony, and where it is.
pub fn read_stored_private_key(
    election_event_id: &str,
    key_ceremony_id: &str,
) -> Result<(String, PathBuf), Box<dyn std::error::Error>> {
    let config = read_config()?;
    let file_path =
        ceremony_private_key_path(election_event_id, key_ceremony_id, &config.username)?;
    let private_key = fs::read_to_string(&file_path)
        .map_err(|error| format!("{}: {}", file_path.display(), error))?;

    Ok((private_key, file_path))
}

pub fn get_private_key_content(
    election_event_id: &str,
    client_username: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let path = event_private_key_path(election_event_id, client_username)?;

    let mut file = File::open(path)?;

    // Read the file contents into a string
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    Ok(content)
}
