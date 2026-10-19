// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Append-only privacy statements. JSON strings retain the existing receipt
//! and ballot formats verbatim inside the authority-signed Borsh statement.
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Debug, Clone)]
pub struct BallotReceivedEvidence {
    pub tenant_id: String,
    pub election_event_id: String,
    pub election_id: String,
    /// Lowercase hexadecimal SHA-512 of the canonical XXXX-XXXX pseudonym.
    pub pseudonym_hash: String,
    pub voter_signing_pk: String,
    pub received_ballot_json: String,
    pub encrypted_ballot: String,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Debug, Clone)]
pub struct BallotCastEvidence {
    pub tenant_id: String,
    pub election_event_id: String,
    pub election_id: String,
    pub pseudonym_hash: String,
    pub ballot_id: String,
    pub cast_receipt_json: String,
    pub voting_channel: String,
}

/// No answer or real voter identity is part of the resolution audit entry.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Debug, Clone)]
pub struct PseudonymResolution {
    pub tenant_id: String,
    pub pseudonym: String,
    pub actor_id: String,
    pub reason: String,
}

/// Scope is an opaque reference, never an IP address or raw search filter.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Debug, Clone)]
pub struct ForensicRead {
    pub tenant_id: String,
    pub actor_id: String,
    pub scope_id: String,
    pub operation: ForensicReadOperation,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum ForensicReadOperation {
    List,
    Detail,
    Export,
}

/// A fixed rejection category; never an exception message containing identity.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Debug, Clone)]
pub struct PrivateBallotEvent {
    pub election_id: String,
    pub pseudonym_hash: String,
    pub ballot_id: Option<String>,
    pub outcome: PrivateBallotOutcome,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum PrivateBallotOutcome {
    Rejected,
    Audited,
}

/// The signed hash uses the display code exactly, including its separator.
/// Normalizing typed aliases belongs at the search boundary, never here.
pub fn pseudonym_hash(pseudonym: &str) -> Option<String> {
    use sha2::{Digest, Sha512};
    const ALPHABET: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    let bytes = pseudonym.as_bytes();
    if bytes.len() != 9
        || bytes.get(4) != Some(&b'-')
        || !bytes
            .iter()
            .enumerate()
            .all(|(i, b)| i == 4 || ALPHABET.contains(b))
    {
        return None;
    }
    Some(hex::encode(Sha512::digest(bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::message::Message;
    use crate::messages::statement::{StatementBody, StatementType};
    use voting_crypto::signing::SigningKey;

    fn evidence() -> BallotReceivedEvidence {
        BallotReceivedEvidence {
            tenant_id: "tenant".into(),
            election_event_id: "event".into(),
            election_id: "election".into(),
            pseudonym_hash: pseudonym_hash("7QKM-2HXD").unwrap(),
            voter_signing_pk: "key".into(),
            received_ballot_json:
                r#"{"ballot_id":"ABCD-EFGH","received_at":"2028-05-08T03:00:00.000Z"}"#.into(),
            encrypted_ballot: "{}".into(),
        }
    }

    #[test]
    fn privacy_types_append_after_every_existing_wire_variant() {
        assert_eq!(
            borsh::to_vec(&StatementBody::BallotReceived(evidence())).unwrap()[0],
            46
        );
        assert_eq!(
            borsh::to_vec(&StatementType::BallotReceived).unwrap(),
            vec![72]
        );
        assert_eq!(
            borsh::to_vec(&StatementType::BallotAudited).unwrap(),
            vec![73]
        );
        assert_eq!(
            borsh::to_vec(&StatementType::PseudonymResolved).unwrap(),
            vec![74]
        );
        assert_eq!(
            borsh::to_vec(&StatementType::ForensicRead).unwrap(),
            vec![75]
        );
    }

    #[test]
    fn private_message_has_no_identity_and_refuses_inconsistent_pseudonyms() {
        let key = SigningKey::from_b64_string(
            "MC4CAQAwBQYDK2VwBCIEIAEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEB",
        )
        .unwrap();
        let message = Message::ballot_received_message("7QKM-2HXD", evidence(), &key).unwrap();
        assert_eq!(message.statement.head.timestamp, 1841367600);
        assert_eq!(
            borsh::to_vec(&message).unwrap(),
            borsh::to_vec(
                &Message::ballot_received_message("7QKM-2HXD", evidence(), &key).unwrap()
            )
            .unwrap()
        );
        let mut invalid = evidence();
        invalid.received_ballot_json =
            r#"{"ballot_id":"ABCD-EFGH","received_at":"invalid"}"#.into();
        assert!(Message::ballot_received_message("7QKM-2HXD", invalid, &key).is_err());
        assert!(message.user_id.is_none());
        assert_eq!(message.username.as_deref(), Some("7QKM-2HXD"));
        assert_eq!(message.sender.name, "7QKM-2HXD");
        assert!(Message::ballot_received_message("ABCD-EFGH", evidence(), &key).is_err());
        assert!(Message::ballot_received_message("7qkm-2hxd", evidence(), &key).is_err());
    }
}
