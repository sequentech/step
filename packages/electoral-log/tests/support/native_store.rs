// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use electoral_log::BoardClient;
pub struct DatabaseServer {
    url: String,
    namespace: String,
}
impl DatabaseServer {
    pub async fn start() -> Result<Self> {
        let url = std::env::var("WBRAID_AUDIT_TEST_DATABASE_URL").context(
            "Set WBRAID_AUDIT_TEST_DATABASE_URL to an isolated disposable PostgreSQL database",
        )?;
        BoardClient::new(&url).await?;
        Ok(Self {
            url,
            namespace: uuid::Uuid::new_v4().simple().to_string(),
        })
    }
    pub fn board(&self, suffix: &str) -> String {
        format!("native-delivery-{}-{suffix}", self.namespace)
    }
    pub async fn client(&self) -> Result<BoardClient> {
        BoardClient::new(&self.url).await
    }
}
