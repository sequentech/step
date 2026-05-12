// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Environment-variable based secret backend.

use super::{Vault, VaultManagerType};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::env;
use tracing::{error, info, instrument};

#[derive(Debug)]
/// Secret backend that reads the master secret from `MASTER_SECRET`.
pub struct EnvVarMasterSecret;

#[async_trait]
impl Vault for EnvVarMasterSecret {
    #[instrument(err)]
    /// Rejects storing secrets and prints the generated value for manual setup.
    ///
    /// # Errors
    ///
    /// Always returns an error to force operators to set `MASTER_SECRET` explicitly.
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
    /// Reads `MASTER_SECRET` from the process environment.
    ///
    /// # Errors
    ///
    /// Returns an error only if reading the environment variable fails unexpectedly.
    async fn read_secret(&self, _key: String) -> Result<Option<String>> {
        if let Ok(master_secret) = env::var("MASTER_SECRET") {
            Ok(Some(master_secret))
        } else {
            error!("MASTER_SECRET must be set.");
            Ok(None)
        }
    }

    #[instrument]
    /// Identifies this backend as environment-variable based.
    fn vault_type(&self) -> VaultManagerType {
        VaultManagerType::EnvVarMasterSecret
    }
}
