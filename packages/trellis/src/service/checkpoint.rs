// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Durable monitor checkpoints, bound to a service and log identity.

use super::RootInfo;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::Path};

/// A saved trusted checkpoint and a persistent record of verification failures.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Service this checkpoint belongs to.
    pub server_url: String,
    /// Log this checkpoint belongs to.
    pub log_name: String,
    /// Last successfully verified root and size.
    pub state: RootInfo,
    /// Sticky alert; successful later polls do not erase a previous failure.
    pub verification_failed: bool,
}

impl Checkpoint {
    /// Loads a checkpoint, refusing malformed files and identity mismatches.
    ///
    /// A missing file is the only condition that returns `None`.
    /// # Errors
    /// Returns an error on I/O failure, malformed data, or a different service/log.
    pub fn load(path: &Path, server_url: &str, log_name: &str) -> Result<Option<Self>> {
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error).context("Cannot read checkpoint"),
        };
        let checkpoint: Self = serde_json::from_slice(&bytes).context("Invalid checkpoint file")?;
        anyhow::ensure!(
            checkpoint.server_url == server_url && checkpoint.log_name == log_name,
            "Checkpoint belongs to a different service or log"
        );
        checkpoint.validate()?;
        Ok(Some(checkpoint))
    }

    /// Writes and syncs a replacement before atomically installing it.
    ///
    /// Use one monitor writer per checkpoint file. The parent directory must exist.
    /// # Errors
    /// Returns an error on invalid state or any failure to durably save the checkpoint.
    pub fn save(&self, path: &Path) -> Result<()> {
        self.validate()?;
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .context("Cannot create checkpoint temporary file")?;
        let result = (|| -> Result<()> {
            file.write_all(&serde_json::to_vec_pretty(self)?)?;
            file.sync_all()?;
            fs::rename(&temporary, path)?;
            fs::File::open(parent)?.sync_all()?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result.context("Cannot persist checkpoint")
    }

    /// Checks the metadata domain accepted by the underlying Merkle implementation.
    fn validate(&self) -> Result<()> {
        anyhow::ensure!(
            self.state.root.len() == 32 && self.state.tree_size <= (1_u64 << 63),
            "Invalid checkpoint root or size"
        );
        if self.state.tree_size == 0 {
            anyhow::ensure!(
                self.state.root == crate::tree::CtMerkleTree::new().root(),
                "Invalid empty checkpoint root"
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkpoint_survives_restart_and_retains_failure() -> Result<()> {
        let path = std::env::temp_dir().join(format!(
            "trellis-checkpoint-test-{}.json",
            std::process::id()
        ));
        assert!(Checkpoint::load(&path, "service", "log")?.is_none());
        let mut checkpoint = Checkpoint {
            server_url: "service".into(),
            log_name: "log".into(),
            state: RootInfo {
                root: crate::tree::CtMerkleTree::new().root(),
                tree_size: 0,
            },
            verification_failed: false,
        };
        checkpoint.save(&path)?;
        checkpoint.state = RootInfo {
            root: vec![3; 32],
            tree_size: 2,
        };
        checkpoint.verification_failed = true;
        checkpoint.save(&path)?;
        assert_eq!(Checkpoint::load(&path, "service", "log")?, Some(checkpoint));
        assert!(Checkpoint::load(&path, "other", "log").is_err());
        assert!(Checkpoint::load(&path, "service", "other").is_err());
        fs::write(&path, b"corrupt")?;
        assert!(Checkpoint::load(&path, "service", "log").is_err());
        fs::remove_file(path)?;
        Ok(())
    }
}
