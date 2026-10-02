// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Transactional leaves with bounded, restartable in-memory proof processing.
use crate::{ConsistencyProof, InclusionProof, LeafHash, tree::CtMerkleTree};
use anyhow::{Context, Result, anyhow, ensure};
use deadpool_postgres::Pool;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use tokio_postgres::Transaction;

/// Database schema, installed by the application's provisioning operation.
pub const SCHEMA: &str = include_str!("../schema.sql");
/// Maximum leaves loaded per log in one processor pass.
pub const BATCH_SIZE: i64 = 1000;

/// A named log generation and its processed Merkle state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Checkpoint {
    /// Application board name.
    pub log_name: String,
    /// Persisted generation; deleting and recreating a board starts a new log.
    pub log_id: i64,
    /// Number of committed leaves incorporated in this root.
    pub tree_size: u64,
    /// SHA-256 tree root bytes.
    pub root: Vec<u8>,
}

/// Inclusion proof at a specific processed checkpoint.
#[derive(Debug, Serialize, Deserialize)]
pub struct Inclusion {
    /// Checkpoint used to generate the proof.
    pub checkpoint: Checkpoint,
    /// Proof for the requested persisted leaf.
    pub proof: InclusionProof,
}
impl Inclusion {
    /// Verify membership against an independently supplied checkpoint.
    /// # Errors
    /// Returns an error for mismatched metadata or an invalid proof.
    pub fn verify(&self, hash: &[u8], checkpoint: &Checkpoint) -> Result<()> {
        ensure!(&self.checkpoint == checkpoint, "Checkpoint mismatch");
        ensure!(
            self.proof.root == checkpoint.root && self.proof.tree_size == checkpoint.tree_size,
            "Proof checkpoint mismatch"
        );
        ensure!(
            self.proof.index < checkpoint.tree_size && self.proof.index < (1u64 << 63),
            "Invalid leaf index"
        );
        self.proof
            .verify(hash)
            .map_err(|e| anyhow!("Invalid inclusion proof: {e:?}"))
    }
}

/// Append-only proof linking two complete checkpoints.
#[derive(Debug, Serialize, Deserialize)]
pub struct Consistency {
    /// Earlier checkpoint.
    pub old: Checkpoint,
    /// Current processed checkpoint.
    pub new: Checkpoint,
    /// Merkle path; an empty initial checkpoint needs no path.
    pub proof: Option<ConsistencyProof>,
}
impl Consistency {
    /// Verify extension of the supplied earlier checkpoint.
    /// # Errors
    /// Returns an error when log identity, sizes, roots or proof do not agree.
    pub fn verify(&self, old: &Checkpoint) -> Result<()> {
        ensure!(
            &self.old == old && old.log_id == self.new.log_id && old.log_name == self.new.log_name,
            "Checkpoint identity mismatch"
        );
        ensure!(self.new.tree_size >= old.tree_size, "Tree shrank");
        if old.tree_size == 0 {
            ensure!(
                old.root == CtMerkleTree::new().root() && self.proof.is_none(),
                "Invalid empty checkpoint"
            );
            return Ok(());
        }
        let proof = self.proof.as_ref().context("Missing consistency proof")?;
        ensure!(
            proof.old_tree_size == old.tree_size
                && proof.new_tree_size == self.new.tree_size
                && proof.new_root == self.new.root,
            "Proof checkpoint mismatch"
        );
        proof
            .verify(&old.root)
            .map_err(|e| anyhow!("Invalid consistency proof: {e:?}"))
    }
}

/// Reusable proof processor sharing the application's PostgreSQL pool.
#[derive(Clone)]
pub struct Journal {
    pool: Pool,
    trees: Arc<Mutex<HashMap<i64, CtMerkleTree>>>,
}
impl Journal {
    /// Construct an empty cache; durable leaves remain authoritative.
    #[must_use]
    pub fn new(pool: Pool) -> Self {
        Self {
            pool,
            trees: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Add a leaf inside the same transaction as the application record.
    /// # Errors
    /// Returns an error if the log is absent or persistence fails.
    pub async fn append(
        tx: &Transaction<'_>,
        name: &str,
        source_id: i64,
        hash: &[u8],
    ) -> Result<()> {
        ensure!(hash.len() == 32, "Expected SHA-256 leaf hash");
        let row = tx
            .query_one(
                "UPDATE trellis_logs SET size = size + 1 WHERE name = $1 RETURNING id, size - 1",
                &[&name],
            )
            .await?;
        let log_id: i64 = row.get(0);
        let index: i64 = row.get(1);
        tx.execute(
            "INSERT INTO trellis_leaves (log_id, leaf_index, source_id, hash) VALUES ($1,$2,$3,$4)",
            &[&log_id, &index, &source_id, &hash],
        )
        .await?;
        Ok(())
    }

    /// Load at most one batch per log, including logs with no new source writes.
    /// # Errors
    /// Returns database or persisted ordering errors; the next pass can retry.
    pub async fn process_once(&self) -> Result<()> {
        let conn = self.pool.get().await?;
        let logs = conn
            .query("SELECT id FROM trellis_logs ORDER BY id", &[])
            .await?;
        let mut trees = self.trees.lock().await;
        let ids: Vec<i64> = logs.iter().map(|r| r.get(0)).collect();
        trees.retain(|id, _| ids.contains(id));
        for id in ids {
            let tree = trees.entry(id).or_insert_with(CtMerkleTree::new);
            let cursor = i64::try_from(tree.len())?;
            let rows = conn.query("SELECT leaf_index, hash FROM trellis_leaves WHERE log_id = $1 AND leaf_index >= $2 ORDER BY leaf_index LIMIT $3", &[&id, &cursor, &BATCH_SIZE]).await?;
            for row in rows {
                let index: i64 = row.get(0);
                ensure!(
                    u64::try_from(index)? == tree.len(),
                    "Non-contiguous Trellis journal"
                );
                tree.push(LeafHash::new(row.get(1)));
            }
            tokio::task::yield_now().await;
        }
        Ok(())
    }

    /// Return the processed checkpoint and number of leaves durably committed.
    /// # Errors
    /// Returns an error when the log does not exist or PostgreSQL is unavailable.
    pub async fn checkpoint(&self, name: &str) -> Result<(Checkpoint, u64)> {
        let conn = self.pool.get().await?;
        let mut trees = self.trees.lock().await;
        let row = conn
            .query_opt(
                "SELECT id, size FROM trellis_logs WHERE name = $1",
                &[&name],
            )
            .await?
            .context("Log does not exist")?;
        let id: i64 = row.get(0);
        let committed: i64 = row.get(1);
        let tree = trees.entry(id).or_insert_with(CtMerkleTree::new);
        Ok((
            Checkpoint {
                log_name: name.to_owned(),
                log_id: id,
                tree_size: tree.len(),
                root: tree.root(),
            },
            u64::try_from(committed)?,
        ))
    }

    /// Generate membership evidence, or `None` while the leaf is pending processing.
    /// # Errors
    /// Returns an error for unknown records, missing logs or database failures.
    pub async fn inclusion(&self, name: &str, source_id: i64) -> Result<Option<Inclusion>> {
        let conn = self.pool.get().await?;
        let row = conn.query_opt("SELECT l.id, e.leaf_index, e.hash FROM trellis_logs l JOIN trellis_leaves e ON e.log_id=l.id WHERE l.name=$1 AND e.source_id=$2", &[&name, &source_id]).await?.context("Log entry does not exist")?;
        let id: i64 = row.get(0);
        let index: i64 = row.get(1);
        let hash: Vec<u8> = row.get(2);
        let trees = self.trees.lock().await;
        let Some(tree) = trees.get(&id) else {
            return Ok(None);
        };
        if u64::try_from(index)? >= tree.len() {
            return Ok(None);
        }
        let proof = tree
            .prove_inclusion(&hash)
            .map_err(|e| anyhow!("Cannot generate inclusion proof: {e:?}"))?;
        let checkpoint = Checkpoint {
            log_name: name.to_owned(),
            log_id: id,
            tree_size: tree.len(),
            root: tree.root(),
        };
        Ok(Some(Inclusion {
            proof: InclusionProof {
                index: u64::try_from(index)?,
                root: checkpoint.root.clone(),
                proof_bytes: proof.as_bytes().to_vec(),
                tree_size: checkpoint.tree_size,
            },
            checkpoint,
        }))
    }

    /// Generate an extension proof from a previously saved checkpoint.
    /// # Errors
    /// Returns an error for another generation or an unknown/unprocessed checkpoint.
    pub async fn consistency(&self, old: &Checkpoint) -> Result<Consistency> {
        let (new, _) = self.checkpoint(&old.log_name).await?;
        ensure!(
            old.log_id == new.log_id && old.tree_size <= new.tree_size,
            "Checkpoint is not in this log generation"
        );
        let trees = self.trees.lock().await;
        let tree = trees.get(&new.log_id).context("Tree not processed")?;
        let proof = if old.tree_size == 0 {
            ensure!(
                old.root == CtMerkleTree::new().root(),
                "Invalid empty checkpoint"
            );
            None
        } else {
            ensure!(
                tree.get_size_for_root(&old.root) == Some(old.tree_size),
                "Unknown checkpoint"
            );
            Some(ConsistencyProof {
                old_tree_size: old.tree_size,
                new_tree_size: new.tree_size,
                new_root: new.root.clone(),
                proof_bytes: tree
                    .prove_consistency_between(&old.root, &new.root)
                    .map_err(|e| anyhow!("Cannot generate consistency proof: {e:?}"))?
                    .as_bytes()
                    .to_vec(),
            })
        };
        Ok(Consistency {
            old: old.clone(),
            new,
            proof,
        })
    }
}
