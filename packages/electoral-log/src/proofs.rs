// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Versioned electoral-record commitments and portable verification bundles.
use crate::domain::LogEntry;
use crate::messages::newtypes::ElectoralLogCheckpointReason;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use strand::signature::{StrandSignature, StrandSignaturePk};
pub use trellis::journal::{
    Checkpoint, Consistency, Evidence, Inclusion, Journal, JournalError, TreeAudit,
};

/// Domain tag of the first record-commitment format. Changing the encoding requires a new tag.
pub const LEAF_FORMAT_V1: &str = "sequent-electoral-log-v1";

/// Hash the exact stored record, including its database ID and delivery identity.
///
/// The commitment is SHA-256 over the compact JSON array
/// `[LEAF_FORMAT_V1, board, entry]`, with `entry` fields in declaration order.
pub fn leaf_hash(board: &str, entry: &LogEntry) -> Result<Vec<u8>> {
    Ok(Sha256::digest(serde_json::to_vec(&(LEAF_FORMAT_V1, board, entry))?).to_vec())
}

/// Domain tag of signed checkpoint publications.
pub const CHECKPOINT_FORMAT_V1: &str = "sequent-electoral-log-checkpoint-v1";

/// Bytes signed when a checkpoint is published: the compact JSON array
/// `[CHECKPOINT_FORMAT_V1, log_name, log_id, tree_size, hex(root), reason]`.
pub fn checkpoint_signing_bytes(
    checkpoint: &Checkpoint,
    reason: ElectoralLogCheckpointReason,
) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(&(
        CHECKPOINT_FORMAT_V1,
        &checkpoint.log_name,
        checkpoint.log_id,
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
        let hash = leaf_hash(&trusted.log_name, &self.entry)?;
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

    /// Historical proofs depend on these exact bytes; never update this vector in place.
    #[test]
    fn v1_leaf_encoding_is_stable() {
        let entry = LogEntry {
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
        };
        let encoded = serde_json::to_string(&(LEAF_FORMAT_V1, "tenantboard", &entry)).unwrap();
        assert_eq!(
            encoded,
            r#"["sequent-electoral-log-v1","tenantboard",{"delivery_id":"delivery-1","message":{"id":42,"created":1700000000,"sender_pk":"sender-pk","statement_timestamp":1700000001,"statement_kind":"CastVote","message":[0,255,128,39,0],"version":"2","user_id":"user-1","username":"O'Brien","election_id":"election-a","area_id":null,"ballot_id":"00ff"}}]"#
        );
        assert_eq!(
            hex::encode(leaf_hash("tenantboard", &entry).unwrap()),
            "da558ad92d0d8f516b7b3102cc472b8a9b66a53d30cba9c9d497819c1eb282d1"
        );
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
            log_id: 3,
            tree_size: 9,
            root: vec![5; 32],
        };
        let tally = ElectoralLogCheckpointReason::TallyCompleted;
        let bytes = checkpoint_signing_bytes(&checkpoint, tally).unwrap();
        assert_eq!(
            String::from_utf8(bytes.clone()).unwrap(),
            format!(
                r#"["{CHECKPOINT_FORMAT_V1}","board",3,9,"{}","TALLY_COMPLETED"]"#,
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
                log_id: 4,
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
