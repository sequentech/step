// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The service's own audit log: one JSON line per operation, each naming the
//! hash of the line before it, so a removed or changed line breaks the chain.
//!
//! It is independent of the electoral log on purpose. The electoral log's
//! entries are signed through this service; if each of those signatures
//! were in turn written to the electoral log, every entry would cause
//! another. Here, signing a log entry adds one audit line and nothing more.
//! Messages never appear in the log, only their SHA-256.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const GENESIS: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    pub seq: u64,
    pub at: String,
    pub caller: String,
    pub operation: String,
    pub key_id: Option<String>,
    pub key_version: Option<u32>,
    pub purpose: Option<String>,
    /// SHA-256 of each message, in order (several for a batch).
    pub message_sha256: Vec<String>,
    pub request_id: Option<String>,
    pub official: Option<String>,
    pub outcome: String,
    pub prev: String,
    pub hash: String,
}

#[derive(Clone, Debug, Default)]
pub struct Event {
    pub caller: String,
    pub operation: String,
    pub key_id: Option<String>,
    pub key_version: Option<u32>,
    pub purpose: Option<String>,
    pub message_sha256: Vec<String>,
    pub request_id: Option<String>,
    pub official: Option<String>,
    pub outcome: String,
}

fn record_hash(record: &Record) -> String {
    let mut unhashed = record.clone();
    unhashed.hash = String::new();
    // serde_json writes struct fields in declaration order: stable bytes.
    let bytes = serde_json::to_vec(&unhashed).unwrap_or_default();
    hex::encode(Sha256::digest(bytes))
}

struct Tail {
    file: File,
    seq: u64,
    prev: String,
}

pub struct AuditLog {
    path: PathBuf,
    tail: Mutex<Tail>,
}

#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    #[error("audit log I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("audit log line {line} is broken: {reason}")]
    Broken { line: u64, reason: String },
}

impl AuditLog {
    /// Opens the log, checking the whole chain first: the service refuses to
    /// start on a broken log rather than append to it.
    pub fn open(path: &Path) -> Result<Self, AuditError> {
        let (seq, prev) = if path.exists() {
            verify(path)?
        } else {
            (0, GENESIS.to_string())
        };
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(AuditLog {
            path: path.to_path_buf(),
            tail: Mutex::new(Tail { file, seq, prev }),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Appends and syncs one record; an operation whose record can't be
    /// written is reported as failed by the caller.
    pub fn append(&self, event: Event) -> Result<Record, AuditError> {
        let mut tail = self.tail.lock().map_err(|_| AuditError::Broken {
            line: 0,
            reason: "lock poisoned".to_string(),
        })?;
        let mut record = Record {
            seq: tail.seq + 1,
            at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            caller: event.caller,
            operation: event.operation,
            key_id: event.key_id,
            key_version: event.key_version,
            purpose: event.purpose,
            message_sha256: event.message_sha256,
            request_id: event.request_id,
            official: event.official,
            outcome: event.outcome,
            prev: tail.prev.clone(),
            hash: String::new(),
        };
        record.hash = record_hash(&record);
        let mut line = serde_json::to_vec(&record).map_err(|e| AuditError::Broken {
            line: record.seq,
            reason: e.to_string(),
        })?;
        line.push(b'\n');
        tail.file.write_all(&line)?;
        tail.file.sync_data()?;
        tail.seq = record.seq;
        tail.prev = record.hash.clone();
        Ok(record)
    }
}

/// Checks every line's sequence, link and hash; returns the last sequence
/// number and hash.
pub fn verify(path: &Path) -> Result<(u64, String), AuditError> {
    let reader = BufReader::new(File::open(path)?);
    let mut seq = 0u64;
    let mut prev = GENESIS.to_string();
    for line in reader.lines() {
        let line = line?;
        let line_number = seq + 1;
        let record: Record = serde_json::from_str(&line).map_err(|e| AuditError::Broken {
            line: line_number,
            reason: e.to_string(),
        })?;
        if record.seq != line_number {
            return Err(AuditError::Broken {
                line: line_number,
                reason: "sequence gap".to_string(),
            });
        }
        if record.prev != prev {
            return Err(AuditError::Broken {
                line: line_number,
                reason: "chain link".to_string(),
            });
        }
        if record_hash(&record) != record.hash {
            return Err(AuditError::Broken {
                line: line_number,
                reason: "hash".to_string(),
            });
        }
        seq = record.seq;
        prev = record.hash;
    }
    Ok((seq, prev))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(n: u8) -> Event {
        Event {
            caller: "windmill".to_string(),
            operation: "sign".to_string(),
            key_id: Some("log/system".to_string()),
            key_version: Some(1),
            purpose: Some("electoral-log-entry".to_string()),
            message_sha256: vec![hex::encode([n; 32])],
            outcome: "ok".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn the_chain_survives_a_restart_and_detects_edits() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.jsonl");
        {
            let log = AuditLog::open(&path).unwrap();
            log.append(event(1)).unwrap();
            log.append(event(2)).unwrap();
        }
        let log = AuditLog::open(&path).unwrap();
        let third = log.append(event(3)).unwrap();
        assert_eq!(third.seq, 3);
        assert_eq!(verify(&path).unwrap().0, 3);

        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::write(&path, text.replacen("windmill", "intruder", 1)).unwrap();
        assert!(matches!(
            verify(&path),
            Err(AuditError::Broken { line: 1, .. })
        ));
        assert!(AuditLog::open(&path).is_err());
    }

    #[test]
    fn a_removed_line_breaks_the_chain() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.jsonl");
        let log = AuditLog::open(&path).unwrap();
        for n in 1..=3 {
            log.append(event(n)).unwrap();
        }
        let lines: Vec<String> = std::fs::read_to_string(&path)
            .unwrap()
            .lines()
            .map(str::to_string)
            .collect();
        std::fs::write(&path, format!("{}\n{}\n", lines[0], lines[2])).unwrap();
        assert!(verify(&path).is_err());
    }
}
