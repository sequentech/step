// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::{Vault, VaultManagerType};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::env;
use tracing::{error, instrument};

#[derive(Debug)]
pub struct EnvVarMasterSecret;

#[async_trait]
impl Vault for EnvVarMasterSecret {
    #[instrument(skip(_value), err)]
    async fn save_secret(&self, _key: String, _value: String) -> Result<()> {
        // If initialize_master_secret failed to read, it creates the master secret value
        // and tries to save it calling to this function.
        // We want it to fail becasue the admin must be aware that the set up was wrong.
        error!("MASTER_SECRET must be set to a 32-byte key encoded as 64 hexadecimal characters.");
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
