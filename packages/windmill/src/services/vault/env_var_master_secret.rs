// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::{Vault, VaultManagerType};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::env;
use tracing::{info, instrument};

#[derive(Debug)]
pub struct EnvVarMasterSecret;

#[async_trait]
impl Vault for EnvVarMasterSecret {
    #[instrument(err)]
    async fn save_secret(&self, _key: String, _value: String) -> Result<()> {
        // it is not needed
        Ok(())
    }

    #[instrument(err)]
    async fn read_secret(&self, _key: String) -> Result<Option<String>> {
        let master_secret = env::var("MASTER_SECRET").context("MASTER_SECRET must be set")?;
        Ok(Some(master_secret))
    }

    #[instrument]
    fn vault_type(&self) -> VaultManagerType {
        VaultManagerType::EnvVarMasterSecret
    }
}
