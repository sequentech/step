// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::vault::SecretVault;
use anyhow::anyhow;
use deadpool_postgres::Transaction;
use std::sync::Mutex;

/// Stores document passwords. A refusing vault fails every call.
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

#[rocket::async_trait]
impl SecretVault for MemoryVault {
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
