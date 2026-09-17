// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::{env, fs};

use crate::utils::read_config::read_config;

fn private_key_path(
    election_event_id: &str,
    username: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let exe_path = env::current_exe().map_err(|_| "Failed to get current executable path")?;
    let parent_dir = exe_path
        .parent()
        .ok_or("Failed to get executable directory")?;

    // Generate the file name
    let file_name = format!(
        "encrypted_private_key_trustee_{}_{}.txt",
        username, election_event_id
    );
    Ok(parent_dir.join("keys").join(file_name))
}

pub fn download_private_key(
    election_event_id: &str,
    private_key: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let config = read_config()?;
    let file_path = private_key_path(election_event_id, &config.username)?;

    if let Some(dir_path) = file_path.parent() {
        if !dir_path.exists() {
            fs::create_dir_all(dir_path)?;
        }
    }

    // Write the private key to the file
    let mut file = File::create(file_path.clone())?;
    file.write_all(private_key.as_bytes())?;

    Ok(file_path)
}

/// The private key a previous run stored for the configured trustee, and
/// where it is.
pub fn read_stored_private_key(
    election_event_id: &str,
) -> Result<(String, PathBuf), Box<dyn std::error::Error>> {
    let config = read_config()?;
    let file_path = private_key_path(election_event_id, &config.username)?;
    let private_key = fs::read_to_string(&file_path)
        .map_err(|error| format!("{}: {}", file_path.display(), error))?;

    Ok((private_key, file_path))
}

pub fn get_private_key_content(
    election_event_id: &str,
    client_username: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let path = private_key_path(election_event_id, client_username)?;

    let mut file = File::open(path)?;

    // Read the file contents into a string
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    Ok(content)
}
