// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::documents::DocumentStorage;
use anyhow::anyhow;
use sequent_core::types::hasura::core::Document;
use std::collections::HashMap;
use std::io::Write;
use std::sync::Mutex;
use tempfile::NamedTempFile;

/// Object contents by document id. Downloading any other document fails
/// the way a missing S3 object does.
#[derive(Default)]
pub struct MemoryDocumentStorage(Mutex<HashMap<String, Vec<u8>>>);

impl MemoryDocumentStorage {
    pub fn store(&self, document_id: &str, contents: &[u8]) {
        self.0
            .lock()
            .unwrap()
            .insert(document_id.to_string(), contents.to_vec());
    }
}

#[rocket::async_trait]
impl DocumentStorage for MemoryDocumentStorage {
    async fn download(
        &self,
        _tenant_id: &str,
        document: &Document,
    ) -> anyhow::Result<NamedTempFile> {
        let contents = self
            .0
            .lock()
            .unwrap()
            .get(&document.id)
            .cloned()
            .ok_or_else(|| anyhow!("No stored object for {}", document.id))?;
        let mut file = NamedTempFile::new()?;
        file.write_all(&contents)?;
        Ok(file)
    }
}
