// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::{env, fs};

use crate::utils::read_config::read_config;

pub fn download_private_key(
    election_event_id: &str,
    private_key: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let config = read_config()?;

    let exe_path = env::current_exe().map_err(|_| "Failed to get current executable path")?;
    let parent_dir = exe_path
        .parent()
        .ok_or("Failed to get executable directory")?;

    let dir_path = parent_dir.join("keys");
    if !Path::new(&dir_path).exists() {
        fs::create_dir_all(&dir_path)?;
    }

    // Generate the file name
    let file_name = format!(
        "encrypted_private_key_trustee_{}_{}.txt",
        config.username, election_event_id
    );
    let file_path = dir_path.join(file_name);

    // Write the private key to the file
    let mut file = File::create(file_path.clone())?;
    file.write_all(private_key.as_bytes())?;

    Ok(file_path)
}

pub fn get_private_key_content(
    election_event_id: &str,
    client_username: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let exe_path = std::env::current_exe().map_err(|_| "Failed to get current executable path")?;
    let parent_dir = exe_path
        .parent()
        .ok_or("Failed to get executable directory")?;

    let dir_path = parent_dir.join("keys");
    let file_path = format!(
        "encrypted_private_key_trustee_{}_{}.txt",
        client_username, election_event_id
    );

    let path = dir_path.join(file_path);

    let mut file = File::open(path)?;

    // Read the file contents into a string
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    Ok(content)
}
