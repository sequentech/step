// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use clap::Args;

use crate::utils::trustees::{
    check_private_key::CheckPrivateKey, get_private_key::GetPrivateKey,
    store_private_key::download_private_key,
};

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
                    "Success! Successfully completed key ceremony. Path to key: {}",
                    path
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
    let private_key = GetPrivateKey::get_trustee_private_key(&election_event_id, &key_ceremony_id)?;
    let checked = CheckPrivateKey::check(&election_event_id, &key_ceremony_id, &private_key)?;
    if checked {
        let path = download_private_key(&election_event_id, &private_key)?;
        let path_str = path.to_str().unwrap_or_default();
        Ok(path_str.to_string())
    } else {
        Err(Box::from("Failed to check key"))
    }
}
