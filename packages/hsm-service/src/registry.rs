// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Each key's versions and their states. A key id names a role ("the tenant
//! log key"); each version is a separate device object, labelled
//! `<key id>#v<n>`. Rotation adds a version and retires the active one:
//! retired versions no longer sign, but their public keys stay listed so
//! everything they signed still verifies. Destroying a version removes the
//! device object and keeps its record.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

use crate::provider::Algorithm;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VersionState {
    Active,
    Retired,
    Destroyed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyVersion {
    pub version: u32,
    pub label: String,
    pub state: VersionState,
    pub created_at: String,
    pub retired_at: Option<String>,
    pub destroyed_at: Option<String>,
    /// Hex of the raw public key (Ed25519) or uncompressed point (P-256);
    /// empty for secret keys.
    pub public_key: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyRecord {
    pub key_id: String,
    pub algorithm: Algorithm,
    pub versions: Vec<KeyVersion>,
}

impl KeyRecord {
    pub fn active(&self) -> Option<&KeyVersion> {
        self.versions
            .iter()
            .find(|v| v.state == VersionState::Active)
    }

    pub fn version(&self, version: u32) -> Option<&KeyVersion> {
        self.versions.iter().find(|v| v.version == version)
    }

    pub fn next_version(&self) -> u32 {
        self.versions.iter().map(|v| v.version).max().unwrap_or(0) + 1
    }
}

pub fn label(key_id: &str, version: u32) -> String {
    format!("{key_id}#v{version}")
}

pub fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("registry I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("registry file is invalid: {0}")]
    Invalid(String),
}

/// The key records, saved atomically (write, sync, rename) after each change.
pub struct Registry {
    path: PathBuf,
    keys: BTreeMap<String, KeyRecord>,
}

impl Registry {
    pub fn open(path: &Path) -> Result<Self, RegistryError> {
        let keys = if path.exists() {
            serde_json::from_slice(&std::fs::read(path)?)
                .map_err(|e| RegistryError::Invalid(e.to_string()))?
        } else {
            BTreeMap::new()
        };
        Ok(Registry {
            path: path.to_path_buf(),
            keys,
        })
    }

    pub fn get(&self, key_id: &str) -> Option<&KeyRecord> {
        self.keys.get(key_id)
    }

    pub fn all(&self) -> impl Iterator<Item = &KeyRecord> {
        self.keys.values()
    }

    pub fn put(&mut self, record: KeyRecord) -> Result<(), RegistryError> {
        let previous = self.keys.insert(record.key_id.clone(), record.clone());
        if let Err(error) = self.save() {
            match previous {
                Some(previous) => self.keys.insert(record.key_id, previous),
                None => self.keys.remove(&record.key_id),
            };
            return Err(error);
        }
        Ok(())
    }

    fn save(&self) -> Result<(), RegistryError> {
        let bytes = serde_json::to_vec_pretty(&self.keys)
            .map_err(|e| RegistryError::Invalid(e.to_string()))?;
        let temporary = self.path.with_extension("tmp");
        {
            use std::io::Write;
            let mut file = std::fs::File::create(&temporary)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
        }
        std::fs::rename(&temporary, &self.path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_persist_and_versions_count_up() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("keys.json");
        let mut registry = Registry::open(&path).unwrap();
        let record = KeyRecord {
            key_id: "log/system".to_string(),
            algorithm: Algorithm::Ed25519,
            versions: vec![KeyVersion {
                version: 1,
                label: label("log/system", 1),
                state: VersionState::Active,
                created_at: now(),
                retired_at: None,
                destroyed_at: None,
                public_key: "00".to_string(),
            }],
        };
        registry.put(record.clone()).unwrap();
        let reopened = Registry::open(&path).unwrap();
        assert_eq!(reopened.get("log/system"), Some(&record));
        assert_eq!(record.next_version(), 2);
        assert_eq!(record.active().unwrap().label, "log/system#v1");
    }
}
