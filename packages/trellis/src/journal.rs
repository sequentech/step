// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Transactional Merkle log persisted in PostgreSQL.
//!
//! An append writes its leaves, the perfect subtrees they complete, and the updated
//! size and root in the same transaction as the application records. Proofs for any
//! committed size are assembled from `O(log n)` stored hashes, so readers keep no
//! state and every reader returns the same answer.
use crate::rfc6962::{self, Frontier, Hash, Node, Nodes, Recorder, TreeError};
use crate::{ConsistencyProof, InclusionProof};
use anyhow::{Context, Result, anyhow, ensure};
use deadpool_postgres::{Object, Pool};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt,
    fmt::Write as _,
    time::SystemTime,
};
use tokio_postgres::{GenericClient, IsolationLevel, Transaction};
use uuid::Uuid;

/// Database schema, installed by the application's provisioning operation.
pub const SCHEMA: &str = include_str!("../schema.sql");
/// Leaves read per database round trip when rebuilding or auditing a log.
pub const BATCH_SIZE: i64 = 1000;
/// Rows per multi-row insert during appends.
pub const INSERT_CHUNK: usize = 5000;
/// Mismatching positions listed per audit finding.
const AUDIT_SAMPLE: usize = 100;

/// A log and its Merkle state at some size.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Checkpoint {
    /// Application board name.
    pub log_name: String,
    /// Identity of the log in every database that stores it; deleting and recreating
    /// a board starts a new log.
    pub log_uid: Uuid,
    /// Number of committed leaves incorporated in this root.
    pub tree_size: u64,
    /// SHA-256 tree root bytes.
    pub root: Vec<u8>,
}

/// Inclusion proof at a specific checkpoint.
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

/// Append-only proof linking two checkpoints.
#[derive(Debug, Serialize, Deserialize)]
pub struct Consistency {
    /// Earlier checkpoint.
    pub old: Checkpoint,
    /// Later checkpoint.
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
            &self.old == old
                && old.log_uid == self.new.log_uid
                && old.log_name == self.new.log_name,
            "Checkpoint identity mismatch"
        );
        ensure!(self.new.tree_size >= old.tree_size, "Tree shrank");
        if old.tree_size == 0 {
            ensure!(
                old.root == rfc6962::empty_root() && self.proof.is_none(),
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
            .verify(&old.root, old.tree_size)
            .map_err(|e| anyhow!("Invalid consistency proof: {e:?}"))
    }
}

/// Inclusion evidence anchored to a caller's trusted checkpoint.
#[derive(Debug, Serialize, Deserialize)]
pub struct Evidence {
    /// Membership proof, at the trusted checkpoint when it already covers the leaf,
    /// otherwise at the current checkpoint.
    pub inclusion: Inclusion,
    /// Present when the inclusion checkpoint is newer than the trusted one: proves that
    /// it extends the trusted checkpoint.
    pub consistency: Option<Consistency>,
}
impl Evidence {
    /// Verify membership of `hash`, anchored to the caller's trusted checkpoint.
    ///
    /// Without a consistency proof the trusted checkpoint must equal the inclusion
    /// checkpoint. With one, it must be the consistency proof's earlier checkpoint.
    /// # Errors
    /// Returns an error when any link between the trusted checkpoint and the leaf fails.
    pub fn verify(&self, hash: &[u8], trusted: &Checkpoint) -> Result<()> {
        match &self.consistency {
            Some(consistency) => {
                consistency.verify(trusted)?;
                self.inclusion.verify(hash, &consistency.new)
            }
            None => self.inclusion.verify(hash, trusted),
        }
    }
}

/// Why the journal cannot answer a proof request.
#[derive(Debug)]
pub enum JournalError {
    /// The named log or record does not exist.
    NotFound(String),
    /// The supplied checkpoint is not part of this log's history: another log, a root
    /// this log never had at that size, or more leaves than were ever committed.
    Diverged(String),
    /// Stored Merkle data is internally inconsistent, or the log was created before
    /// subtrees were stored and must be rebuilt.
    Corrupt(String),
    /// Database or other operational failure.
    Failed(anyhow::Error),
}
impl fmt::Display for JournalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(what) => write!(f, "{what} does not exist"),
            Self::Diverged(reason) => {
                write!(f, "Checkpoint is not in this log's history: {reason}")
            }
            Self::Corrupt(reason) => write!(f, "Stored Merkle data is inconsistent: {reason}"),
            Self::Failed(error) => write!(f, "{error:#}"),
        }
    }
}
impl std::error::Error for JournalError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Failed(error) => Some(&**error),
            _ => None,
        }
    }
}
impl From<anyhow::Error> for JournalError {
    fn from(error: anyhow::Error) -> Self {
        Self::Failed(error)
    }
}
impl From<tokio_postgres::Error> for JournalError {
    fn from(error: tokio_postgres::Error) -> Self {
        Self::Failed(error.into())
    }
}
impl From<deadpool_postgres::PoolError> for JournalError {
    fn from(error: deadpool_postgres::PoolError) -> Self {
        Self::Failed(error.into())
    }
}
impl From<std::num::TryFromIntError> for JournalError {
    fn from(error: std::num::TryFromIntError) -> Self {
        Self::Failed(error.into())
    }
}
impl From<TreeError> for JournalError {
    fn from(error: TreeError) -> Self {
        Self::Corrupt(error.to_string())
    }
}

/// Comparison of a log's stored tree with one recomputed from its leaves.
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TreeAudit {
    /// Committed size recorded for the log.
    pub committed_size: u64,
    /// Leaves read, in position order, before the first gap.
    pub leaves: u64,
    /// Whether leaf positions run from 0 without gaps.
    pub dense: bool,
    /// Stored perfect subtrees (level 1 and up).
    pub stored_nodes: u64,
    /// Stored perfect subtrees whose hash differs from the recomputed one, or are missing.
    pub node_mismatches: u64,
    /// First mismatching subtrees, as `(level, index)`.
    pub first_node_mismatches: Vec<(u8, u64)>,
    /// Whether the stored root matches the recomputed one.
    pub root_matches: bool,
    /// The log was created before subtrees were stored and must be rebuilt.
    pub unbuilt: bool,
    /// Roots recomputed from the leaves at the requested sizes the leaves cover.
    pub roots: BTreeMap<u64, Vec<u8>>,
}
impl TreeAudit {
    /// True when the stored tree is exactly the tree of the stored leaves.
    #[must_use]
    pub const fn is_clean(&self) -> bool {
        self.dense
            && !self.unbuilt
            && self.leaves == self.committed_size
            && self.node_mismatches == 0
            && self.stored_nodes == self.expected_nodes()
            && self.root_matches
    }

    /// Number of perfect subtrees (level 1 and up) in a tree of the leaves read.
    #[must_use]
    pub const fn expected_nodes(&self) -> u64 {
        self.leaves.saturating_sub(self.leaves.count_ones() as u64)
    }
}

/// Whether a log takes new leaves.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LogState {
    /// Appends extend the log.
    Open,
    /// The log is closed: appends are refused, proofs are still answered.
    Sealed,
}
impl LogState {
    /// State of a log whose `sealed_at` column is `sealed_at`.
    const fn from_sealed_at(sealed_at: Option<SystemTime>) -> Self {
        if sealed_at.is_some() {
            Self::Sealed
        } else {
            Self::Open
        }
    }
}

/// A log of the database, with its stored size and root.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LogSummary {
    /// Stored size and root, not checked against the stored subtrees.
    pub checkpoint: Checkpoint,
    /// Whether the log takes new leaves.
    pub state: LogState,
}

/// Columns of `trellis_logs` that `head` and `Journal::logs` read, in this order.
const LOG_COLUMNS: &str = "id, uid, size, root, sealed_at";

/// Current state of a log, checked against its stored subtrees.
struct Head {
    /// Key of the log's rows in this database.
    id: i64,
    /// Identity of the log.
    uid: Uuid,
    /// Whether the log takes new leaves.
    state: LogState,
    /// Committed leaves.
    size: u64,
    /// Root over the committed leaves.
    root: Hash,
    /// Right edge of the tree, read from the stored subtrees.
    frontier: Frontier,
}

/// Whether reading a log's head locks its row.
#[derive(Clone, Copy)]
enum RowLock {
    /// Plain read.
    Read,
    /// Lock the row until the transaction ends, serializing appends.
    ForUpdate,
}
impl Head {
    /// Checkpoint of the current state.
    fn checkpoint(&self, name: &str) -> Checkpoint {
        Checkpoint {
            log_name: name.to_owned(),
            log_uid: self.uid,
            tree_size: self.size,
            root: self.root.to_vec(),
        }
    }
}

/// Convert stored bytes to a hash.
fn hash_of(bytes: &[u8]) -> Result<Hash> {
    Hash::try_from(bytes).map_err(|_| anyhow!("Stored hash is not 32 bytes"))
}

/// Read a log's current state and check that its stored right edge produces its root.
///
/// A log created before subtrees were stored still has the empty root with a nonzero
/// size; it is reported as needing a rebuild.
async fn head<C: GenericClient + Sync>(
    client: &C,
    name: &str,
    lock: RowLock,
) -> Result<Head, JournalError> {
    let lock = match lock {
        RowLock::Read => "",
        RowLock::ForUpdate => " FOR UPDATE",
    };
    let row = client
        .query_opt(
            &format!("SELECT {LOG_COLUMNS} FROM trellis_logs WHERE name = $1{lock}"),
            &[&name],
        )
        .await?
        .ok_or_else(|| JournalError::NotFound(format!("Log '{name}'")))?;
    let id: i64 = row.try_get(0)?;
    let uid: Uuid = row.try_get(1)?;
    let size = u64::try_from(row.try_get::<_, i64>(2)?)?;
    let root = hash_of(&row.try_get::<_, Vec<u8>>(3)?)?;
    let state = LogState::from_sealed_at(row.try_get(4)?);
    let unbuilt = is_unbuilt(size, &root);
    let not_built = || JournalError::Corrupt(format!("log '{name}' must be rebuilt"));
    let edge = rfc6962::decomposition(size);
    let stored = fetch(client, id, &edge).await?;
    let mut hashes = Vec::with_capacity(edge.len());
    for node in edge {
        match stored.get(&node) {
            Some(hash) => hashes.push(*hash),
            None if unbuilt => return Err(not_built()),
            None => return Err(TreeError::MissingNode(node).into()),
        }
    }
    let frontier = Frontier::from_hashes(size, hashes)?;
    if frontier.root() != root {
        return Err(if unbuilt {
            not_built()
        } else {
            JournalError::Corrupt(format!(
                "the stored subtrees of log '{name}' do not produce its stored root"
            ))
        });
    }
    Ok(Head {
        id,
        uid,
        state,
        size,
        root,
        frontier,
    })
}

/// Whether a log was created before subtrees were stored: those logs gained the empty
/// root with a nonzero size, which no tree with leaves can have.
fn is_unbuilt(size: u64, root: &[u8]) -> bool {
    size > 0 && root == rfc6962::empty_root()
}

/// Start a read-only transaction that sees one snapshot of the database.
async fn snapshot(client: &mut Object) -> Result<deadpool_postgres::Transaction<'_>, JournalError> {
    Ok(client
        .build_transaction()
        .isolation_level(IsolationLevel::RepeatableRead)
        .read_only(true)
        .start()
        .await?)
}

/// Lowercase hexadecimal encoding of a hash, for messages.
fn hex(hash: &[u8]) -> String {
    hash.iter().fold(String::new(), |mut out, byte| {
        // Writing to a `String` cannot fail.
        let _ = write!(out, "{byte:02x}");
        out
    })
}

/// Fetch the stored hashes of the given perfect subtrees; level 0 reads leaves.
async fn fetch<C: GenericClient + Sync>(
    client: &C,
    log_id: i64,
    needed: &[Node],
) -> Result<HashMap<Node, Hash>, JournalError> {
    let mut leaves = Vec::new();
    let mut levels = Vec::new();
    let mut indexes = Vec::new();
    for node in needed {
        if node.level == 0 {
            leaves.push(i64::try_from(node.index)?);
        } else {
            levels.push(i16::from(node.level));
            indexes.push(i64::try_from(node.index)?);
        }
    }
    let mut found = HashMap::with_capacity(needed.len());
    if !leaves.is_empty() {
        for row in client
            .query(
                "SELECT leaf_index, hash FROM trellis_leaves \
                 WHERE log_id = $1 AND leaf_index = ANY($2)",
                &[&log_id, &leaves],
            )
            .await?
        {
            let index = u64::try_from(row.try_get::<_, i64>(0)?)?;
            let data: Vec<u8> = row.try_get(1)?;
            found.insert(Node { level: 0, index }, rfc6962::leaf_hash(&data));
        }
    }
    if !levels.is_empty() {
        for row in client
            .query(
                "SELECT n.level, n.idx, n.hash FROM trellis_nodes n \
                 JOIN UNNEST($2::SMALLINT[], $3::BIGINT[]) AS w(level, idx) \
                 ON n.level = w.level AND n.idx = w.idx WHERE n.log_id = $1",
                &[&log_id, &levels, &indexes],
            )
            .await?
        {
            let level = u8::try_from(row.try_get::<_, i16>(0)?)?;
            let index = u64::try_from(row.try_get::<_, i64>(1)?)?;
            found.insert(
                Node { level, index },
                hash_of(&row.try_get::<_, Vec<u8>>(2)?)?,
            );
        }
    }
    Ok(found)
}

/// Run a tree computation against stored hashes, fetching exactly the nodes it reads.
async fn with_nodes<C, T, F>(client: &C, log_id: i64, compute: F) -> Result<T, JournalError>
where
    C: GenericClient + Sync,
    F: Fn(&dyn Nodes) -> Result<T, TreeError> + Send,
{
    let recorder = Recorder::default();
    compute(&recorder)?;
    let nodes = fetch(client, log_id, &recorder.into_nodes()).await?;
    Ok(compute(&nodes)?)
}

/// Check that `old` is a checkpoint of this log's history and prove `head` extends it.
async fn extension<C: GenericClient + Sync>(
    client: &C,
    name: &str,
    head: &Head,
    old: &Checkpoint,
) -> Result<Consistency, JournalError> {
    if old.log_name != name || old.log_uid != head.uid {
        return Err(JournalError::Diverged(
            "checkpoint belongs to another log".into(),
        ));
    }
    if old.tree_size > head.size {
        return Err(JournalError::Diverged(format!(
            "checkpoint has {} leaves but only {} were committed",
            old.tree_size, head.size
        )));
    }
    let (root, path) = with_nodes(client, head.id, |nodes| {
        let root = rfc6962::root_at(nodes, old.tree_size)?;
        let path = if old.tree_size == 0 {
            None
        } else {
            Some(rfc6962::consistency_path(nodes, old.tree_size, head.size)?)
        };
        Ok((root, path))
    })
    .await?;
    if old.root != root {
        // Damaged stored subtrees also yield a different root; they are a fault of this
        // log, not evidence that the checkpoint is outside its history.
        let stored_tree_is_consistent = path.as_ref().is_none_or(|path| {
            ConsistencyProof {
                old_tree_size: old.tree_size,
                new_tree_size: head.size,
                new_root: head.root.to_vec(),
                proof_bytes: path.concat(),
            }
            .verify(&root, old.tree_size)
            .is_ok()
        });
        if !stored_tree_is_consistent {
            return Err(JournalError::Corrupt(format!(
                "the stored subtrees of log '{name}' do not produce its root"
            )));
        }
        return Err(JournalError::Diverged(format!(
            "this log had a different root at size {}",
            old.tree_size
        )));
    }
    let consistency = Consistency {
        old: old.clone(),
        new: head.checkpoint(name),
        proof: path.map(|path| ConsistencyProof {
            old_tree_size: old.tree_size,
            new_tree_size: head.size,
            new_root: head.root.to_vec(),
            proof_bytes: path.concat(),
        }),
    };
    // The old root matched, so a failure here means the stored nodes disagree with the root.
    consistency
        .verify(old)
        .map_err(|error| JournalError::Corrupt(format!("{error:#}")))?;
    Ok(consistency)
}

/// Prove leaf `index` (with stored data `data`) of the log stored under `log_id` at
/// `checkpoint`, whose root is trusted.
async fn inclusion_at<C: GenericClient + Sync>(
    client: &C,
    log_id: i64,
    checkpoint: &Checkpoint,
    index: u64,
    data: &[u8],
) -> Result<Inclusion, JournalError> {
    let path = with_nodes(client, log_id, |nodes| {
        rfc6962::inclusion_path(nodes, index, checkpoint.tree_size)
    })
    .await?;
    let inclusion = Inclusion {
        proof: InclusionProof {
            index,
            root: checkpoint.root.clone(),
            proof_bytes: path.concat(),
            tree_size: checkpoint.tree_size,
        },
        checkpoint: checkpoint.clone(),
    };
    inclusion
        .verify(data, checkpoint)
        .map_err(|error| JournalError::Corrupt(format!("{error:#}")))?;
    Ok(inclusion)
}

/// Recompute a log's tree from its leaves and compare it with the stored tree, also
/// recording the recomputed roots at the given sizes.
///
/// Reads the whole log in batches; run it inside a repeatable-read transaction so the
/// leaves, nodes and head come from one snapshot.
/// # Errors
/// Returns `NotFound` for an unknown log, or `Failed` on database errors.
pub async fn audit_tree<C: GenericClient + Sync>(
    client: &C,
    name: &str,
    sizes: &[u64],
) -> Result<TreeAudit, JournalError> {
    let sizes: HashSet<u64> = sizes.iter().copied().collect();
    let log = client
        .query_opt(
            "SELECT id, size, root FROM trellis_logs WHERE name = $1",
            &[&name],
        )
        .await?
        .ok_or_else(|| JournalError::NotFound(format!("Log '{name}'")))?;
    let log_id: i64 = log.try_get(0)?;
    let mut audit = TreeAudit {
        committed_size: u64::try_from(log.try_get::<_, i64>(1)?)?,
        dense: true,
        ..TreeAudit::default()
    };
    let stored_root: Vec<u8> = log.try_get(2)?;
    audit.unbuilt = is_unbuilt(audit.committed_size, &stored_root);
    let mut frontier = Frontier::new();
    if sizes.contains(&0) {
        audit.roots.insert(0, frontier.root().to_vec());
    }
    while audit.dense {
        let rows = client
            .query(
                "SELECT leaf_index, hash FROM trellis_leaves \
                 WHERE log_id = $1 AND leaf_index >= $2 ORDER BY leaf_index LIMIT $3",
                &[&log_id, &i64::try_from(frontier.size())?, &BATCH_SIZE],
            )
            .await?;
        if rows.is_empty() {
            break;
        }
        let mut completed = Vec::new();
        for row in rows {
            if u64::try_from(row.try_get::<_, i64>(0)?)? != frontier.size() {
                audit.dense = false;
                break;
            }
            let data: Vec<u8> = row.try_get(1)?;
            completed.extend(frontier.push(&data));
            if sizes.contains(&frontier.size()) {
                audit
                    .roots
                    .insert(frontier.size(), frontier.root().to_vec());
            }
        }
        let nodes: Vec<Node> = completed.iter().map(|(node, _)| *node).collect();
        let stored = fetch(client, log_id, &nodes).await?;
        for (node, hash) in completed {
            if stored.get(&node) != Some(&hash) {
                audit.node_mismatches = audit.node_mismatches.saturating_add(1);
                if audit.first_node_mismatches.len() < AUDIT_SAMPLE {
                    audit.first_node_mismatches.push((node.level, node.index));
                }
            }
        }
    }
    audit.leaves = frontier.size();
    audit.stored_nodes = u64::try_from(
        client
            .query_one(
                "SELECT count(*) FROM trellis_nodes WHERE log_id = $1",
                &[&log_id],
            )
            .await?
            .try_get::<_, i64>(0)?,
    )?;
    audit.root_matches = audit.dense && frontier.root().as_slice() == stored_root.as_slice();
    Ok(audit)
}

/// Create an empty log named `name`, with identity `uid` or a new one, and return its
/// identity. Creating a log that exists returns its identity.
/// # Errors
/// Returns an error when `uid` is given and the log, or another log, already has a
/// different identity or name, or when persistence fails.
pub async fn create_log<C: GenericClient + Sync>(
    client: &C,
    name: &str,
    uid: Option<Uuid>,
) -> Result<Uuid> {
    client
        .execute(
            "INSERT INTO trellis_logs (name, uid) VALUES ($1, coalesce($2, gen_random_uuid())) \
             ON CONFLICT DO NOTHING",
            &[&name, &uid],
        )
        .await?;
    let stored: Uuid = client
        .query_opt("SELECT uid FROM trellis_logs WHERE name = $1", &[&name])
        .await?
        .with_context(|| format!("Log identity {uid:?} belongs to another log than '{name}'"))?
        .try_get(0)?;
    if let Some(uid) = uid {
        ensure!(
            stored == uid,
            "Log '{name}' exists with identity {stored}, not {uid}"
        );
    }
    Ok(stored)
}

/// Close a log to appends and return its final checkpoint. Sealing a sealed log
/// returns the same checkpoint.
/// # Errors
/// Returns `NotFound` for an unknown log, `Corrupt` when its stored subtrees do not
/// produce its root, or `Failed`.
pub async fn seal<C: GenericClient + Sync>(
    client: &C,
    name: &str,
) -> Result<Checkpoint, JournalError> {
    let head = head(client, name, RowLock::ForUpdate).await?;
    client
        .execute(
            "UPDATE trellis_logs SET sealed_at = coalesce(sealed_at, now()) WHERE id = $1",
            &[&head.id],
        )
        .await?;
    Ok(head.checkpoint(name))
}

/// Stateless proof service over the application's database pool.
#[derive(Clone)]
pub struct Journal {
    /// Connections for reading the persisted tree.
    pool: Pool,
}
impl Journal {
    /// Construct a reader over the pool; the database is the only state.
    #[must_use]
    pub const fn new(pool: Pool) -> Self {
        Self { pool }
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
        Self::append_batch(tx, name, &[(source_id, hash.to_vec())]).await
    }

    /// Add leaves, in order, inside the same transaction as the application records.
    ///
    /// Locks the log row, writes the leaves and the perfect subtrees they complete, and
    /// updates the size and root, so the tree is committed with the records.
    /// # Errors
    /// Returns an error if the log is absent, a hash is not 32 bytes, the stored
    /// subtrees do not produce the stored root (or the log must be rebuilt), or
    /// persistence fails.
    pub async fn append_batch(
        tx: &Transaction<'_>,
        name: &str,
        leaves: &[(i64, Vec<u8>)],
    ) -> Result<()> {
        if leaves.is_empty() {
            return Ok(());
        }
        ensure!(
            leaves.iter().all(|(_, hash)| hash.len() == 32),
            "Expected SHA-256 leaf hashes"
        );
        let Head {
            id: log_id,
            state,
            size,
            mut frontier,
            ..
        } = head(tx, name, RowLock::ForUpdate)
            .await
            .map_err(|error| anyhow!("Cannot append to Trellis log '{name}': {error}"))?;
        ensure!(
            state == LogState::Open,
            "Cannot append to Trellis log '{name}': it is sealed"
        );
        let mut nodes = Vec::new();
        for (_, hash) in leaves {
            nodes.extend(frontier.push(hash));
        }
        let mut next = size;
        for chunk in leaves.chunks(INSERT_CHUNK) {
            let mut indexes = Vec::with_capacity(chunk.len());
            for _ in chunk {
                indexes.push(i64::try_from(next)?);
                next = next.checked_add(1).context("Trellis log is full")?;
            }
            let sources: Vec<i64> = chunk.iter().map(|(source, _)| *source).collect();
            let hashes: Vec<&[u8]> = chunk.iter().map(|(_, hash)| hash.as_slice()).collect();
            tx.execute(
                "INSERT INTO trellis_leaves (log_id, leaf_index, source_id, hash) \
                 SELECT $1, * FROM UNNEST($2::BIGINT[], $3::BIGINT[], $4::BYTEA[])",
                &[&log_id, &indexes, &sources, &hashes],
            )
            .await?;
        }
        for chunk in nodes.chunks(INSERT_CHUNK) {
            let levels: Vec<i16> = chunk
                .iter()
                .map(|(node, _)| i16::from(node.level))
                .collect();
            let mut indexes = Vec::with_capacity(chunk.len());
            for (node, _) in chunk {
                indexes.push(i64::try_from(node.index)?);
            }
            let hashes: Vec<&[u8]> = chunk.iter().map(|(_, hash)| hash.as_slice()).collect();
            tx.execute(
                "INSERT INTO trellis_nodes (log_id, level, idx, hash) \
                 SELECT $1, * FROM UNNEST($2::SMALLINT[], $3::BIGINT[], $4::BYTEA[])",
                &[&log_id, &levels, &indexes, &hashes],
            )
            .await?;
        }
        tx.execute(
            "UPDATE trellis_logs SET size = $2, root = $3 WHERE id = $1",
            &[
                &log_id,
                &i64::try_from(frontier.size())?,
                &frontier.root().as_slice(),
            ],
        )
        .await?;
        Ok(())
    }

    /// Return the current checkpoint of a log.
    /// # Errors
    /// Returns `NotFound` for an unknown log, `Corrupt` when its stored subtrees do not
    /// produce its root, or `Failed` if the database is unavailable.
    pub async fn checkpoint(&self, name: &str) -> Result<Checkpoint, JournalError> {
        let mut client = self.pool.get().await?;
        let tx = snapshot(&mut client).await?;
        let checkpoint = head(&*tx, name, RowLock::Read).await?.checkpoint(name);
        tx.commit().await?;
        Ok(checkpoint)
    }

    /// Generate membership evidence at the current checkpoint.
    /// # Errors
    /// Returns `NotFound` for unknown logs or records, or `Failed`.
    pub async fn inclusion(&self, name: &str, source_id: i64) -> Result<Inclusion, JournalError> {
        Ok(self.evidence(name, source_id, None).await?.inclusion)
    }

    /// Generate membership evidence anchored to an optional trusted checkpoint.
    ///
    /// Without a trusted checkpoint the proof is at the current checkpoint. With one,
    /// the proof is at the trusted checkpoint when it already contains the leaf, and
    /// otherwise at the current checkpoint together with a consistency proof from the
    /// trusted one.
    /// # Errors
    /// Returns `NotFound`, `Diverged` for a trusted checkpoint outside this log's
    /// history, `Corrupt`, or `Failed`.
    pub async fn evidence(
        &self,
        name: &str,
        source_id: i64,
        trusted: Option<&Checkpoint>,
    ) -> Result<Evidence, JournalError> {
        let mut client = self.pool.get().await?;
        let tx = snapshot(&mut client).await?;
        let reader: &Transaction<'_> = &tx;
        let head = head(reader, name, RowLock::Read).await?;
        let row = reader
            .query_opt(
                "SELECT leaf_index, hash FROM trellis_leaves WHERE log_id = $1 AND source_id = $2",
                &[&head.id, &source_id],
            )
            .await?
            .ok_or_else(|| JournalError::NotFound(format!("Log entry {source_id}")))?;
        let index = u64::try_from(row.try_get::<_, i64>(0)?)?;
        let data: Vec<u8> = row.try_get(1)?;
        let current = head.checkpoint(name);
        let evidence = match trusted {
            None => Evidence {
                inclusion: inclusion_at(reader, head.id, &current, index, &data).await?,
                consistency: None,
            },
            Some(trusted) => {
                let link = extension(reader, name, &head, trusted).await?;
                if index < trusted.tree_size {
                    Evidence {
                        inclusion: inclusion_at(reader, head.id, trusted, index, &data).await?,
                        consistency: None,
                    }
                } else {
                    Evidence {
                        inclusion: inclusion_at(reader, head.id, &current, index, &data).await?,
                        consistency: Some(link),
                    }
                }
            }
        };
        tx.commit().await?;
        Ok(evidence)
    }

    /// Prove that the current checkpoint extends a previously saved one.
    /// # Errors
    /// Returns `NotFound`, `Diverged` when the saved checkpoint is not in this log's
    /// history, `Corrupt`, or `Failed`.
    pub async fn consistency(&self, old: &Checkpoint) -> Result<Consistency, JournalError> {
        let mut client = self.pool.get().await?;
        let tx = snapshot(&mut client).await?;
        let reader: &Transaction<'_> = &tx;
        let head = head(reader, &old.log_name, RowLock::Read).await?;
        let consistency = extension(reader, &old.log_name, &head, old).await?;
        tx.commit().await?;
        Ok(consistency)
    }

    /// Every log of the database, in creation order, with its stored size and root.
    /// # Errors
    /// Returns `Failed` if the database is unavailable.
    pub async fn logs(&self) -> Result<Vec<LogSummary>, JournalError> {
        let rows = self
            .pool
            .get()
            .await?
            .query(
                &format!("SELECT {LOG_COLUMNS}, name FROM trellis_logs ORDER BY id"),
                &[],
            )
            .await?;
        let mut logs = Vec::with_capacity(rows.len());
        for row in rows {
            logs.push(LogSummary {
                checkpoint: Checkpoint {
                    log_name: row.try_get(5)?,
                    log_uid: row.try_get(1)?,
                    tree_size: u64::try_from(row.try_get::<_, i64>(2)?)?,
                    root: row.try_get(3)?,
                },
                state: LogState::from_sealed_at(row.try_get(4)?),
            });
        }
        Ok(logs)
    }

    /// Names of the logs created before subtrees were stored, which must be rebuilt.
    /// # Errors
    /// Returns `Failed` if the database is unavailable.
    pub async fn unbuilt_logs(&self) -> Result<Vec<String>, JournalError> {
        let rows = self
            .pool
            .get()
            .await?
            .query(
                "SELECT name FROM trellis_logs WHERE size > 0 AND root = $1 ORDER BY name",
                &[&rfc6962::empty_root().as_slice()],
            )
            .await?;
        Ok(rows
            .iter()
            .map(|row| row.try_get(0))
            .collect::<Result<_, _>>()?)
    }

    /// Recompute a log's stored subtrees and root from its leaves.
    ///
    /// For logs created before subtrees were stored, and for repairing damaged or
    /// missing subtrees. The root is a pure function of the ordered leaves, so earlier
    /// checkpoints remain valid. Unless the log was created before subtrees were
    /// stored, its stored root must be the root of some prefix of its leaves: changed
    /// leaves are not turned into a consistent new history. Appends to the log wait
    /// until the rebuild commits.
    /// # Errors
    /// Returns `NotFound` for an unknown log, `Corrupt` when leaves are missing or the
    /// rebuilt root differs from the stored one, or `Failed` when persistence fails.
    pub async fn rebuild(&self, name: &str) -> Result<Checkpoint, JournalError> {
        let mut client = self.pool.get().await?;
        let tx = client.transaction().await?;
        let log = tx
            .query_opt(
                &format!("SELECT {LOG_COLUMNS} FROM trellis_logs WHERE name = $1 FOR UPDATE"),
                &[&name],
            )
            .await?
            .ok_or_else(|| JournalError::NotFound(format!("Log '{name}'")))?;
        let log_id: i64 = log.try_get(0)?;
        let identity: Uuid = log.try_get(1)?;
        let size = u64::try_from(log.try_get::<_, i64>(2)?)?;
        let stored_root = hash_of(&log.try_get::<_, Vec<u8>>(3)?)?;
        let unbuilt = is_unbuilt(size, &stored_root);
        tx.execute("DELETE FROM trellis_nodes WHERE log_id = $1", &[&log_id])
            .await?;
        let mut frontier = Frontier::new();
        let mut matches_prefix = frontier.root() == stored_root;
        loop {
            let rows = tx
                .query(
                    "SELECT leaf_index, hash FROM trellis_leaves \
                     WHERE log_id = $1 AND leaf_index >= $2 ORDER BY leaf_index LIMIT $3",
                    &[&log_id, &i64::try_from(frontier.size())?, &BATCH_SIZE],
                )
                .await?;
            if rows.is_empty() {
                break;
            }
            let mut nodes = Vec::new();
            for row in rows {
                let index = u64::try_from(row.try_get::<_, i64>(0)?)?;
                if index != frontier.size() {
                    return Err(JournalError::Corrupt(format!(
                        "log '{name}' has no leaf at position {}",
                        frontier.size()
                    )));
                }
                let data: Vec<u8> = row.try_get(1)?;
                nodes.extend(frontier.push(&data));
                if !unbuilt && !matches_prefix {
                    matches_prefix = frontier.root() == stored_root;
                }
            }
            let levels: Vec<i16> = nodes
                .iter()
                .map(|(node, _)| i16::from(node.level))
                .collect();
            let mut indexes = Vec::with_capacity(nodes.len());
            for (node, _) in &nodes {
                indexes.push(i64::try_from(node.index)?);
            }
            let hashes: Vec<&[u8]> = nodes.iter().map(|(_, hash)| hash.as_slice()).collect();
            tx.execute(
                "INSERT INTO trellis_nodes (log_id, level, idx, hash) \
                 SELECT $1, * FROM UNNEST($2::SMALLINT[], $3::BIGINT[], $4::BYTEA[])",
                &[&log_id, &levels, &indexes, &hashes],
            )
            .await?;
        }
        if frontier.size() != size {
            return Err(JournalError::Corrupt(format!(
                "log '{name}' has {} leaves but records {size} committed",
                frontier.size()
            )));
        }
        let root = frontier.root();
        if !unbuilt && !matches_prefix {
            return Err(JournalError::Corrupt(format!(
                "the stored root {} of log '{name}' is not a root of its leaves; rebuilding \
                 would replace it with {}",
                hex(&stored_root),
                hex(&root)
            )));
        }
        tx.execute(
            "UPDATE trellis_logs SET root = $2 WHERE id = $1",
            &[&log_id, &root.as_slice()],
        )
        .await?;
        tx.commit().await?;
        Ok(Checkpoint {
            log_name: name.to_owned(),
            log_uid: identity,
            tree_size: size,
            root: root.to_vec(),
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn audit_requires_every_property() {
        let clean = TreeAudit {
            committed_size: 5,
            leaves: 5,
            dense: true,
            stored_nodes: 3,
            node_mismatches: 0,
            first_node_mismatches: vec![],
            root_matches: true,
            unbuilt: false,
            roots: BTreeMap::new(),
        };
        assert!(clean.is_clean());
        for broken in [
            TreeAudit {
                dense: false,
                ..clean.clone()
            },
            TreeAudit {
                leaves: 4,
                ..clean.clone()
            },
            TreeAudit {
                stored_nodes: 4,
                ..clean.clone()
            },
            TreeAudit {
                node_mismatches: 1,
                ..clean.clone()
            },
            TreeAudit {
                root_matches: false,
                ..clean.clone()
            },
            TreeAudit {
                unbuilt: true,
                ..clean.clone()
            },
        ] {
            assert!(!broken.is_clean(), "{broken:?}");
        }
    }

    #[test]
    fn evidence_requires_its_trusted_anchor() {
        // Build the proofs a server would return from in-memory subtrees.
        let leaves: Vec<Vec<u8>> = (0..11_u8).map(|i| vec![i; 32]).collect();
        let mut frontier = Frontier::new();
        let mut nodes: HashMap<Node, Hash> = HashMap::new();
        let mut checkpoints = vec![];
        for (i, leaf) in leaves.iter().enumerate() {
            nodes.insert(
                Node {
                    level: 0,
                    index: i as u64,
                },
                rfc6962::leaf_hash(leaf),
            );
            nodes.extend(frontier.push(leaf));
            checkpoints.push(Checkpoint {
                log_name: "board".into(),
                log_uid: Uuid::from_u128(7),
                tree_size: frontier.size(),
                root: frontier.root().to_vec(),
            });
        }
        let trusted = &checkpoints[5];
        let current = &checkpoints[10];
        let link = Consistency {
            old: trusted.clone(),
            new: current.clone(),
            proof: Some(ConsistencyProof {
                old_tree_size: 6,
                new_tree_size: 11,
                new_root: current.root.clone(),
                proof_bytes: rfc6962::consistency_path(&nodes, 6, 11).unwrap().concat(),
            }),
        };
        let inclusion = Inclusion {
            proof: InclusionProof {
                index: 8,
                root: current.root.clone(),
                proof_bytes: rfc6962::inclusion_path(&nodes, 8, 11).unwrap().concat(),
                tree_size: 11,
            },
            checkpoint: current.clone(),
        };
        let evidence = Evidence {
            inclusion,
            consistency: Some(link),
        };
        evidence.verify(&leaves[8], trusted).expect("anchored");
        assert!(evidence.verify(&leaves[9], trusted).is_err());
        assert!(evidence.verify(&leaves[8], current).is_err());
        let unlinked = Evidence {
            consistency: None,
            ..evidence
        };
        assert!(unlinked.verify(&leaves[8], trusted).is_err());
        unlinked
            .verify(&leaves[8], current)
            .expect("exact checkpoint");
    }
}
