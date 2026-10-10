// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::utils::trustees::{
    check_private_key::CheckPrivateKey,
    get_trustee_private_key::GetTrusteePrivateKey,
    store_private_key::{download_private_key, read_stored_private_key, store_event_private_key},
};
use clap::Args;
use colored::Colorize;
use windmill::services::ceremonies::keys_ceremony::PrivateKeyDownloadUnavailable;

#[derive(Args)]
#[command(about = "Complete Key Ceremony", long_about = None)]
pub struct Complete {
    /// Election event id - the election event to complete the key ceremony for
    #[arg(long)]
    election_event_id: String,

    /// Key ceremony id - the key ceremony to complete
    #[arg(long)]
    key_ceremony_id: String,

    /// Native trustee keys file, kept on this machine.
    #[arg(long)]
    trustee_config: std::path::PathBuf,
}

impl Complete {
    pub fn run(&self) {
        match complete_ceremony_with_keys(
            &self.election_event_id,
            &self.key_ceremony_id,
            &self.trustee_config,
        ) {
            Ok(path) => {
                println!(
                    "{}",
                    format!(
                        "Success! Completed key custody. Public request stored at: {}",
                        path
                    )
                    .green(),
                );
            }
            Err(err) => {
                eprintln!("Error! Failed to complete key ceremony: {}", err)
            }
        }
    }
}

pub fn complete_ceremony(
    election_event_id: &str,
    key_ceremony_id: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let keys_file = std::env::var_os("TRUSTEE_CONFIG_PATH")
        .ok_or("TRUSTEE_CONFIG_PATH must name your native trustee keys file")?;
    complete_ceremony_with_keys(
        election_event_id,
        key_ceremony_id,
        std::path::Path::new(&keys_file),
    )
}

fn complete_ceremony_with_keys(
    election_event_id: &str,
    key_ceremony_id: &str,
    keys_file: &std::path::Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let (request, path) =
        match GetTrusteePrivateKey::get_trustee_private_key(election_event_id, key_ceremony_id) {
            Ok(request) => {
                let path = download_private_key(election_event_id, key_ceremony_id, &request)?;
                (request, path)
            }
            Err(error) if error.is::<PrivateKeyDownloadUnavailable>() => {
                read_stored_private_key(election_event_id, key_ceremony_id)?
            }
            Err(error) => return Err(error),
        };
    let custody: protocol_board::CustodyRequest = serde_json::from_str(&request)?;
    let secrets = protocol_board::TrusteeSecrets::parse(&std::fs::read_to_string(keys_file)?)?;
    let proof = custody.attest(&secrets)?;
    if !CheckPrivateKey::check(election_event_id, key_ceremony_id, &proof)? {
        return Err("Failed to confirm native key custody".into());
    }
    store_event_private_key(election_event_id, &request)?;
    Ok(path.to_string_lossy().into_owned())
}
