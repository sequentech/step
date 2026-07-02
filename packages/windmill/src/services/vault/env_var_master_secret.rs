// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::{Vault, VaultManagerType};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::env;
use tracing::{error, info, instrument};

#[derive(Debug)]
pub struct EnvVarMasterSecret;

#[async_trait]
impl Vault for EnvVarMasterSecret {
    #[instrument(err)]
    async fn save_secret(&self, _key: String, value: String) -> Result<()> {
        // If initialize_master_secret failed to read, it creates the master secret value
        // and tries to save it calling to this function.
        // We want it to fail becasue the admin must be aware that the set up was wrong.
        // We will then print the generated value to the console and return an error, so the admin can add it manually.
        info!("Generated master secret automatically.");
        info!("Please set manually MASTER_SECRET = {value} ");
        Err(anyhow::anyhow!("MASTER_SECRET env var missing."))
    }

    #[instrument(err)]
    async fn read_secret(&self, _key: String) -> Result<Option<String>> {
        match env::var("MASTER_SECRET") {
            Ok(master_secret) => Ok(Some(master_secret)),
            Err(_) => {
                error!("MASTER_SECRET must be set.");
                Ok(None)
            }
        }
    }

    #[instrument]
    fn vault_type(&self) -> VaultManagerType {
        VaultManagerType::EnvVarMasterSecret
    }
}
