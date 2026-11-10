// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::{Vault, VaultManagerType};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::env;
use tracing::instrument;

/// Why a process without its master secret does not start.
const MISSING: &str =
    "MASTER_SECRET is not set: it must hold the master secret, 64 hexadecimal characters";

#[derive(Debug)]
pub struct EnvVarMasterSecret;

#[async_trait]
impl Vault for EnvVarMasterSecret {
    /// The master secret comes only from the environment: nothing is stored here,
    /// and the value is neither logged nor recorded in the span.
    #[instrument(skip(value), err)]
    async fn save_secret(&self, _key: String, value: String) -> Result<()> {
        drop(value);
        Err(anyhow!(MISSING))
    }

    /// A missing or empty variable is an error, never a reason to generate a secret.
    #[instrument(err)]
    async fn read_secret(&self, _key: String) -> Result<Option<String>> {
        from_variable(env::var("MASTER_SECRET"))
    }

    #[instrument]
    fn vault_type(&self) -> VaultManagerType {
        VaultManagerType::EnvVarMasterSecret
    }
}

/// The master secret that `MASTER_SECRET`, as read, holds. Its errors never carry
/// the value.
fn from_variable(variable: Result<String, env::VarError>) -> Result<Option<String>> {
    match variable {
        Ok(master_secret) if !master_secret.is_empty() => Ok(Some(master_secret)),
        _ => Err(anyhow!(MISSING)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_or_empty_variable_is_refused() {
        for variable in [Err(env::VarError::NotPresent), Ok(String::new())] {
            assert_eq!(from_variable(variable).unwrap_err().to_string(), MISSING);
        }
        let synthetic = "5a".repeat(32);
        assert_eq!(
            from_variable(Ok(synthetic.clone())).unwrap(),
            Some(synthetic)
        );
    }

    #[tokio::test]
    async fn nothing_is_saved_and_the_refusal_omits_the_value() {
        let synthetic = "5a".repeat(32);
        let error = EnvVarMasterSecret
            .save_secret("master_secret".into(), synthetic.clone())
            .await
            .unwrap_err()
            .to_string();
        assert_eq!(error, MISSING);
        assert!(!error.contains(&synthetic));
    }
}
