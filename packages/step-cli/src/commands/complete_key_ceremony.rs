// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::utils::trustees::{
    confirm_key_custody::ConfirmKeyCustody,
    get_trustee_custody_request::GetTrusteeCustodyRequest,
    store_custody_request::{
        download_custody_request, read_stored_custody_request, store_event_custody_request,
    },
};
use clap::Args;
use colored::Colorize;
use windmill::services::ceremonies::keys_ceremony::CustodyRequestDownloadUnavailable;

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
    let (request, path) = match GetTrusteeCustodyRequest::get_trustee_custody_request(
        election_event_id,
        key_ceremony_id,
    ) {
        Ok(request) => {
            let path = download_custody_request(election_event_id, key_ceremony_id, &request)?;
            (request, path)
        }
        Err(error) if error.is::<CustodyRequestDownloadUnavailable>() => {
            read_stored_custody_request(election_event_id, key_ceremony_id)?
        }
        Err(error) => return Err(error),
    };
    let custody: protocol_board::CustodyRequest = serde_json::from_str(&request)?;
    let config = crate::utils::read_config::read_config()?;
    if custody.tenant_id != config.tenant_id
        || custody.election_event_id != election_event_id
        || custody.keys_ceremony_id != key_ceremony_id
        || custody.trustee_name != config.username
        || custody.tally_session_id.is_some()
    {
        return Err("custody request does not match this trustee and key ceremony".into());
    }
    let secrets = protocol_board::TrusteeSecrets::parse(&std::fs::read_to_string(keys_file)?)?;
    let proof = custody.attest(&secrets)?;
    if !ConfirmKeyCustody::check(election_event_id, key_ceremony_id, &proof)? {
        return Err("Failed to confirm native key custody".into());
    }
    store_event_custody_request(election_event_id, &request)?;
    Ok(path.to_string_lossy().into_owned())
}
