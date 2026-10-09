// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Versioned electoral-record commitments and portable verification bundles.
use crate::domain::LogEntry;
use crate::messages::newtypes::ElectoralLogCheckpointReason;
use anyhow::Result;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use strand::signature::{StrandSignature, StrandSignaturePk};
pub use trellis::journal::{
    Checkpoint, Consistency, Evidence, Inclusion, Journal, JournalError, LogState, LogSummary,
    TreeAudit,
};
pub use uuid::Uuid;

/// Domain tag of the record-commitment format. Changing the encoding requires a new tag.
pub const LEAF_FORMAT_V2: &str = "sequent-electoral-log-v2";

/// The log a record is committed to: its board name and its identity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct LogIdentity {
    pub name: String,
    pub uid: Uuid,
}
impl LogIdentity {
    /// The log a checkpoint belongs to.
    pub fn of(checkpoint: &Checkpoint) -> Self {
        Self {
            name: checkpoint.log_name.clone(),
            uid: checkpoint.log_uid,
        }
    }
}

/// Commit to a record of a log: the log's identity, the record's delivery ID and its
/// fields, but not where a database stores it. A log copied to another database, in
/// the same order, keeps its roots.
///
/// The commitment is SHA-256 over the compact JSON array `[LEAF_FORMAT_V2, log_name,
/// log_uid, delivery_id, created, sender_pk, statement_timestamp, statement_kind,
/// base64(message), version, user_id, username, election_id, area_id, ballot_id]`,
/// with the UUID hyphenated in lowercase, base64 with padding and absent values as
/// `null`.
pub fn leaf_hash(log: &LogIdentity, entry: &LogEntry) -> Result<Vec<u8>> {
    let record = &entry.message;
    let preimage = serde_json::to_vec(&(
        LEAF_FORMAT_V2,
        &log.name,
        log.uid.hyphenated().to_string(),
        &entry.delivery_id,
        record.created,
        &record.sender_pk,
        record.statement_timestamp,
        &record.statement_kind,
        STANDARD.encode(&record.message),
        &record.version,
        &record.user_id,
        &record.username,
        &record.election_id,
        &record.area_id,
        &record.ballot_id,
    ))?;
    Ok(Sha256::digest(preimage).to_vec())
}

/// Domain tag of signed checkpoint publications.
pub const CHECKPOINT_FORMAT_V2: &str = "sequent-electoral-log-checkpoint-v2";

/// Bytes signed when a checkpoint is published: the compact JSON array
/// `[CHECKPOINT_FORMAT_V2, log_name, log_uid, tree_size, hex(root), reason]`.
pub fn checkpoint_signing_bytes(
    checkpoint: &Checkpoint,
    reason: ElectoralLogCheckpointReason,
) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(&(
        CHECKPOINT_FORMAT_V2,
        &checkpoint.log_name,
        checkpoint.log_uid.hyphenated().to_string(),
        checkpoint.tree_size,
        hex::encode(&checkpoint.root),
        reason.to_string(),
    ))?)
}

/// Verify the signature of a published checkpoint.
///
/// `signer_pk` is the DER/base64 public key and `signature` the base64 signature, as
/// stored with the publication. Callers must also check that the signer is the
/// expected key for the election event.
pub fn verify_checkpoint_signature(
    checkpoint: &Checkpoint,
    reason: ElectoralLogCheckpointReason,
    signer_pk: &str,
    signature: &str,
) -> Result<()> {
    let pk = StrandSignaturePk::from_der_b64_string(signer_pk)?;
    let signature = StrandSignature::from_b64_string(signature)?;
    pk.verify(&signature, &checkpoint_signing_bytes(checkpoint, reason)?)?;
    Ok(())
}

/// A stored record with evidence of its membership in the board's Trellis log.
#[derive(Debug, Serialize, Deserialize)]
pub struct RecordProof {
    pub entry: LogEntry,
    pub inclusion: Inclusion,
    /// Present when the proof was requested relative to a trusted checkpoint; proves
    /// that the inclusion checkpoint extends it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consistency: Option<Consistency>,
}
impl RecordProof {
    /// Combine a stored record with journal evidence for it.
    pub fn new(entry: LogEntry, evidence: Evidence) -> Self {
        Self {
            entry,
            inclusion: evidence.inclusion,
            consistency: evidence.consistency,
        }
    }

    /// Verify the record against a trusted checkpoint.
    ///
    /// Without a consistency proof the trusted checkpoint must be exactly the inclusion
    /// checkpoint; with one, it must be the checkpoint the consistency proof extends.
    pub fn verify(&self, trusted: &Checkpoint) -> Result<()> {
        let hash = leaf_hash(&LogIdentity::of(trusted), &self.entry)?;
        match &self.consistency {
            Some(consistency) => {
                consistency.verify(trusted)?;
                self.inclusion.verify(&hash, &consistency.new)
            }
            None => self.inclusion.verify(&hash, trusted),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ElectoralLogMessage;

    fn identity() -> LogIdentity {
        LogIdentity {
            name: "tenantboard".into(),
            uid: Uuid::parse_str("0b6f1c7e-2a3d-4e5f-8a9b-0c1d2e3f4a5b").unwrap(),
        }
    }

    fn entry() -> LogEntry {
        LogEntry {
            delivery_id: "delivery-1".into(),
            message: ElectoralLogMessage {
                id: 42,
                created: 1_700_000_000,
                sender_pk: "sender-pk".into(),
                statement_timestamp: 1_700_000_001,
                statement_kind: "CastVote".into(),
                message: vec![0, 255, 128, 39, 0],
                version: "2".into(),
                user_id: Some("user-1".into()),
                username: Some("O'Brien".into()),
                election_id: Some("election-a".into()),
                area_id: None,
                ballot_id: Some("00ff".into()),
            },
        }
    }

    /// Historical proofs depend on these exact bytes; never update this vector in place.
    #[test]
    fn v2_leaf_encoding_is_stable() {
        let preimage = r#"["sequent-electoral-log-v2","tenantboard","0b6f1c7e-2a3d-4e5f-8a9b-0c1d2e3f4a5b","delivery-1",1700000000,"sender-pk",1700000001,"CastVote","AP+AJwA=","2","user-1","O'Brien","election-a",null,"00ff"]"#;
        assert_eq!(
            leaf_hash(&identity(), &entry()).unwrap(),
            Sha256::digest(preimage.as_bytes()).to_vec()
        );
        assert_eq!(
            hex::encode(leaf_hash(&identity(), &entry()).unwrap()),
            "28e9ae58c65e0ed83b06779d38e55ac966d37c3e40bdef19d4d00f080e25018e"
        );
    }

    /// The commitment does not depend on where a database stores the record, and
    /// changes with the log and with every field.
    #[test]
    fn leaves_commit_to_the_log_and_the_record_but_not_its_storage() {
        let hash = |log: &LogIdentity, entry: &LogEntry| leaf_hash(log, entry).unwrap();
        let original = hash(&identity(), &entry());
        let mut stored_elsewhere = entry();
        stored_elsewhere.message.id = 7;
        assert_eq!(hash(&identity(), &stored_elsewhere), original);
        let other_name = LogIdentity {
            name: "other".into(),
            ..identity()
        };
        let other_uid = LogIdentity {
            uid: Uuid::from_u128(1),
            ..identity()
        };
        assert_ne!(hash(&other_name, &entry()), original);
        assert_ne!(hash(&other_uid, &entry()), original);
        let changes: [fn(&mut LogEntry); 13] = [
            |e| e.delivery_id.push('x'),
            |e| e.message.created += 1,
            |e| e.message.sender_pk.push('x'),
            |e| e.message.statement_timestamp += 1,
            |e| e.message.statement_kind.push('x'),
            |e| e.message.message.push(0),
            |e| e.message.version.push('x'),
            |e| e.message.user_id = None,
            |e| e.message.username = None,
            |e| e.message.election_id = None,
            |e| e.message.area_id = Some(String::new()),
            |e| e.message.ballot_id = None,
            |e| e.message.username = Some("O'Brien ".into()),
        ];
        for change in changes {
            let mut changed = entry();
            change(&mut changed);
            assert_ne!(hash(&identity(), &changed), original, "{changed:?}");
        }
    }

    #[test]
    fn checkpoint_signatures_bind_every_field() {
        let sk = strand::signature::StrandSignatureSk::generate().unwrap();
        let pk = StrandSignaturePk::from_sk(&sk)
            .unwrap()
            .to_der_b64_string()
            .unwrap();
        let checkpoint = Checkpoint {
            log_name: "board".into(),
            log_uid: Uuid::from_u128(3),
            tree_size: 9,
            root: vec![5; 32],
        };
        let tally = ElectoralLogCheckpointReason::TallyCompleted;
        let bytes = checkpoint_signing_bytes(&checkpoint, tally).unwrap();
        assert_eq!(
            String::from_utf8(bytes.clone()).unwrap(),
            format!(
                r#"["{CHECKPOINT_FORMAT_V2}","board","00000000-0000-0000-0000-000000000003",9,"{}","TALLY_COMPLETED"]"#,
                "05".repeat(32)
            )
        );
        let signature = sk.sign(&bytes).unwrap().to_b64_string().unwrap();
        verify_checkpoint_signature(&checkpoint, tally, &pk, &signature).unwrap();
        let closed = ElectoralLogCheckpointReason::VotingClosed;
        assert!(verify_checkpoint_signature(&checkpoint, closed, &pk, &signature).is_err());
        for changed in [
            Checkpoint {
                log_name: "other".into(),
                ..checkpoint.clone()
            },
            Checkpoint {
                log_uid: Uuid::from_u128(4),
                ..checkpoint.clone()
            },
            Checkpoint {
                tree_size: 10,
                ..checkpoint.clone()
            },
            Checkpoint {
                root: vec![6; 32],
                ..checkpoint.clone()
            },
        ] {
            assert!(verify_checkpoint_signature(&changed, tally, &pk, &signature).is_err());
        }
    }

    #[test]
    fn checkpoint_reasons_have_stable_names() {
        for (reason, name) in [
            (ElectoralLogCheckpointReason::VotingClosed, "VOTING_CLOSED"),
            (
                ElectoralLogCheckpointReason::TallyCompleted,
                "TALLY_COMPLETED",
            ),
            (ElectoralLogCheckpointReason::VotingOpened, "VOTING_OPENED"),
            (ElectoralLogCheckpointReason::Periodic, "PERIODIC"),
        ] {
            assert_eq!(reason.to_string(), name);
            assert_eq!(
                name.parse::<ElectoralLogCheckpointReason>().unwrap(),
                reason
            );
            assert_eq!(
                serde_json::to_string(&reason).unwrap(),
                format!("\"{name}\"")
            );
        }
        assert!("voting_closed"
            .parse::<ElectoralLogCheckpointReason>()
            .is_err());
    }

    /// Signed checkpoint messages store the reason by its index, so the indices of
    /// existing reasons never change.
    #[test]
    fn checkpoint_reasons_keep_their_encoded_indices() {
        for (reason, index) in [
            (ElectoralLogCheckpointReason::VotingClosed, 0u8),
            (ElectoralLogCheckpointReason::TallyCompleted, 1),
            (ElectoralLogCheckpointReason::VotingOpened, 2),
            (ElectoralLogCheckpointReason::Periodic, 3),
        ] {
            assert_eq!(borsh::to_vec(&reason).unwrap(), vec![index]);
        }
    }

    /// `schema.sql` is applied directly by provisioning scripts, so it carries a copy of
    /// the Trellis schema. Keep the copy identical to the one the journal expects.
    #[test]
    fn electoral_log_schema_embeds_the_trellis_schema() {
        assert!(include_str!("../schema.sql").contains(trellis::journal::SCHEMA));
    }
}
