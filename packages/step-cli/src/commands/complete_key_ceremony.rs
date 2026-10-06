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
}

impl Complete {
    pub fn run(&self) {
        match complete_ceremony(&self.election_event_id, &self.key_ceremony_id) {
            Ok(path) => {
                println!(
                    "{}",
                    format!(
                        "Success! Successfully completed key ceremony. Path to key: {}",
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
    let (private_key, path) =
        match GetTrusteePrivateKey::get_trustee_private_key(&election_event_id, &key_ceremony_id) {
            // Store the key before checking it, as a checked key can't be
            // downloaded again
            Ok(private_key) => {
                let path =
                    download_private_key(&election_event_id, &key_ceremony_id, &private_key)?;
                (private_key, path)
            }
            // A previous run already checked the key, so check the stored copy
            Err(error) if error.is::<PrivateKeyDownloadUnavailable>() => read_stored_private_key(
                &election_event_id,
                &key_ceremony_id,
            )
            .map_err(|read_error| {
                format!("{error}, and the stored private key could not be read: {read_error}")
            })?,
            Err(error) => return Err(error),
        };
    let checked = CheckPrivateKey::check(&election_event_id, &key_ceremony_id, &private_key)?;
    if checked {
        // confirm-key-tally reads the checked key of the election event
        store_event_private_key(&election_event_id, &private_key)?;
        let path_str = path.to_str().unwrap_or_default();
        Ok(path_str.to_string())
    } else {
        Err(Box::from("Failed to check key"))
    }
}
