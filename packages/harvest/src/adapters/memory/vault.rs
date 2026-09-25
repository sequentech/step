// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::vault::SecretVault;
use anyhow::anyhow;
use deadpool_postgres::Transaction;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

/// Seals a value as `sealed:<user id>:<value>`, so tests can see which
/// voter it was bound to. A refusing vault fails every call.
#[derive(Default)]
pub struct MemoryVault {
    refuses: bool,
    /// Document id and password of each saved document password.
    pub document_passwords: Mutex<Vec<(String, String)>>,
}

impl MemoryVault {
    pub fn refusing() -> Self {
        Self {
            refuses: true,
            ..Default::default()
        }
    }

    fn check(&self) -> anyhow::Result<()> {
        match self.refuses {
            true => Err(anyhow!("the vault is sealed")),
            false => Ok(()),
        }
    }
}

pub fn sealed(user_id: &str, value: &str) -> String {
    format!("sealed:{user_id}:{value}")
}

#[rocket::async_trait]
impl SecretVault for MemoryVault {
    async fn encrypt_secret_attributes(
        &self,
        _tenant_id: &str,
        _election_event_id: &str,
        user_id: &str,
        secret_names: &HashSet<String>,
        values: HashMap<String, Option<Vec<String>>>,
    ) -> anyhow::Result<HashMap<String, Vec<String>>> {
        self.check()?;
        values
            .into_iter()
            .map(|(name, values)| {
                if !secret_names.contains(&name) {
                    return Err(anyhow!("`{name}` is not encrypted"));
                }
                let values = values.unwrap_or_default();
                Ok((
                    name,
                    values.iter().map(|value| sealed(user_id, value)).collect(),
                ))
            })
            .collect()
    }

    async fn decrypt_secret_attribute(
        &self,
        _tenant_id: &str,
        _election_event_id: &str,
        user_id: &str,
        _attribute_name: &str,
        values: &[String],
    ) -> anyhow::Result<Vec<String>> {
        self.check()?;
        let prefix = sealed(user_id, "");
        values
            .iter()
            .map(|value| {
                value
                    .strip_prefix(&prefix)
                    .map(str::to_string)
                    .ok_or_else(|| anyhow!("sealed for another voter"))
            })
            .collect()
    }

    async fn save_document_password(
        &self,
        _transaction: &Transaction<'_>,
        _tenant_id: &str,
        _election_event_id: Option<&str>,
        document_id: &str,
        password: &str,
    ) -> anyhow::Result<String> {
        self.check()?;
        let mut saved = self.document_passwords.lock().unwrap();
        saved.push((document_id.to_string(), password.to_string()));
        Ok(format!("secret-{}", saved.len()))
    }

    async fn check_report_password(
        &self,
        _transaction: &Transaction<'_>,
        _tenant_id: String,
        _election_event_id: String,
        _report_id: Option<String>,
        _password: String,
    ) -> anyhow::Result<()> {
        self.check()
    }
}
