// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::temp_path::generate_temp_file;
use crate::util::aws::get_max_upload_size;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use tempfile::TempPath;
use tracing::instrument;

pub const KIND: &str = "KIND";
pub const INDEXES: &str = "INDEXES";
pub const SECRET_VALUE: &str = "SECRET_VALUE";

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, EnumString, Display)]
pub enum SecretKind {
    PROTOCOL_MANAGER_KEYS,
    PROTOCOL_MANAGER,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, EnumString, Display)]
pub enum SecretKind {
    KEYS,
    PROTOCOL_MANAGER,
}
pub struct SecretExportData {
    kind: SecretKind,
    indexes: Vec<String>,
    secret_value: String,
}

impl SecretExportData {
    pub fn get_record(&self) -> Vec<String> {
        vec![
            self.kind.to_string(),
            self.indexes.join("|"),
            self.secret_value.clone(),
        ]
    }

    pub fn get_secret_key(&self) -> String {
        match self.kind {
            SecretKind::KEYS => "".to_string(),
            SecretKind::PROTOCOL_MANAGER => "".to_string(),
        }
    }
}

#[instrument(err, skip(secrets))]
pub fn create_election_event_secrets_csv(secrets: &Vec<SecretExportData>) -> Result<TempPath> {
    let mut writer = csv::WriterBuilder::new().delimiter(b',').from_writer(
        generate_temp_file("export-secrets-", ".csv")
            .with_context(|| "Error creating temporary file")?,
    );
    let headers: Vec<String> = vec![
        KIND.to_string(),
        INDEXES.to_string(),
        SECRET_VALUE.to_string(),
    ];
    writer.write_record(&headers)?;
    for secret in secrets {
        let record = secret.get_record();
        writer
            .write_record(&record)
            .with_context(|| "Error writing record")?;
    }
    writer
        .flush()
        .with_context(|| "Error flushing CSV writer")?;

    let temp_path = writer
        .into_inner()
        .with_context(|| "Error getting inner writer")?
        .into_temp_path();

    let size = temp_path.metadata()?.len();
    if size > get_max_upload_size()? as u64 {
        return Err(anyhow!("File too large: {} > {}", size, get_max_upload_size()?).into());
    }

    Ok(temp_path)
}
