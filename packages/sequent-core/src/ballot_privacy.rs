// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Browser and native checks of the exact signed electoral-log bytes. Trusted
//! keys and scope come from published configuration, never from these entries.
use crate::ballot::{
    verify_ballot_signature, BallotBoxKey, HashableBallot, SignedHashableBallot,
};
use crate::ballot_receipt::{
    ballot_id, verify_cast_receipt, verify_cast_signature,
    verify_received_ballot, CastReceipt, ReceivedBallot,
};
use crate::encrypt::{hash_ballot, hash_multi_ballot};
use crate::multi_ballot::{
    verify_multi_ballot_signature, HashableMultiBallot,
    SignedHashableMultiBallot,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use borsh::BorshDeserialize;
use electoral_log::cast_vote_seal::{
    CastVoteSealKeyPin, SealedBallot, SEAL_FORMAT,
};
use electoral_log::messages::message::Message;
pub use electoral_log::messages::privacy::pseudonym_hash;
use electoral_log::messages::privacy::{
    BallotCastEvidence, BallotReceivedEvidence,
};
use electoral_log::messages::statement::{StatementBody, StatementType};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use voting_crypto::signing::VerificationKey;

const MAX_ENTRY_BASE64_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BallotPrivacyTrust {
    pub tenant_id: String,
    pub election_event_id: String,
    pub election_id: String,
    pub system_public_key: String,
    pub ballot_box_key: BallotBoxKey,
    /// Whether the event's cast votes are sealed to its HSM key. Absent in a
    /// context from before sealing existed, which reads as not sealed.
    #[serde(default)]
    pub cast_vote_seal: CastVoteSealTrust,
}

/// Whether an election event seals its cast votes, as its electoral log
/// shows. A ballot logged sealed (`hsm-seal-v1`) is accepted only for an
/// event that seals, and only sealed to the key its log pins.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "policy", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CastVoteSealTrust {
    /// The event's log pins no cast-vote seal key: a sealed ballot is
    /// refused.
    #[default]
    NotSealed,
    /// The event's cast votes are sealed to the key its log pins.
    Sealed {
        /// Standard base64 of the event's whole canonical Borsh
        /// `CastVoteSealKeyPublished` message, which the event's system key
        /// signs; checked here, never taken on trust.
        key_entry: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BallotPrivacyProof {
    pub ballot_id: String,
    pub pseudonym: String,
    pub voting_channel: String,
    /// Standard base64 of the entire canonical Borsh Message, not JSON metadata.
    pub received_entry: Option<String>,
    pub cast_entry: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    Passed,
    Failed,
    Pending,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BallotPrivacyChecks {
    pub binding: CheckStatus,
    pub received: CheckStatus,
    #[serde(rename = "ballotId")]
    pub ballot_id: CheckStatus,
    pub cast: CheckStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BallotPrivacyReport {
    pub checks: BallotPrivacyChecks,
}

fn status(passed: bool) -> CheckStatus {
    if passed {
        CheckStatus::Passed
    } else {
        CheckStatus::Failed
    }
}

fn decode(encoded: &str) -> Option<Message> {
    if encoded.len() > MAX_ENTRY_BASE64_BYTES {
        return None;
    }
    Message::try_from_slice(&STANDARD.decode(encoded).ok()?).ok()
}

fn context(
    trust: &BallotPrivacyTrust,
    tenant: &str,
    event: &str,
    election: &str,
) -> bool {
    trust.tenant_id == tenant
        && trust.election_event_id == event
        && trust.election_id == election
}

fn authority(trust: &BallotPrivacyTrust, message: &Message) -> bool {
    VerificationKey::from_b64_string(&trust.system_public_key)
        .is_ok_and(|key| message.verify(&key).is_ok())
        && message.statement.head.event.0 == trust.election_event_id
}

/// The key the event's log pins for its cast votes: `None` for an event that
/// doesn't seal them, or whose pin entry isn't this event's, signed by its
/// system key, with a consistent fingerprint.
fn pinned_seal_key(trust: &BallotPrivacyTrust) -> Option<CastVoteSealKeyPin> {
    let CastVoteSealTrust::Sealed { key_entry } = &trust.cast_vote_seal else {
        return None;
    };
    let message = decode(key_entry)?;
    if !authority(trust, &message)
        || !matches!(
            message.statement.head.kind,
            StatementType::CastVoteSealKeyPublished
        )
    {
        return None;
    }
    let StatementBody::CastVoteSealKeyPublished(pin) = message.statement.body
    else {
        return None;
    };
    let fingerprint = hex::decode(&pin.public_key)
        .ok()
        .map(|point| hex::encode(Sha256::digest(point)));
    (pin.tenant_id == trust.tenant_id
        && pin.election_event_id == trust.election_event_id
        && pin.format == SEAL_FORMAT
        && fingerprint.as_deref() == Some(pin.fingerprint.as_str()))
    .then_some(pin)
}

fn received_context(
    trust: &BallotPrivacyTrust,
    proof: &BallotPrivacyProof,
    evidence: &BallotReceivedEvidence,
    receipt: &ReceivedBallot,
) -> bool {
    context(
        trust,
        &evidence.tenant_id,
        &evidence.election_event_id,
        &evidence.election_id,
    ) && context(
        trust,
        &receipt.statement.tenant_id,
        &receipt.statement.election_event_id,
        &receipt.statement.election_id,
    ) && receipt.ballot_id == proof.ballot_id
        && evidence.voter_signing_pk == receipt.statement.voter_signing_pk
        && pseudonym_hash(&proof.pseudonym).as_deref()
            == Some(evidence.pseudonym_hash.as_str())
}

/// Verify the actual encrypted payload, including its hash and voter signature.
/// An attestation over a hash alone cannot demonstrate this check.
///
/// An election event whose cast votes are sealed to its HSM key logs the
/// sealed ballot (`hsm-seal-v1`) instead: nobody outside the tally can open
/// it, so neither its content hash nor its voter signature can be checked
/// here. See [`sealed_voter_ballot`] for what is.
fn voter_ballot(
    trust: &BallotPrivacyTrust,
    evidence: &BallotReceivedEvidence,
    received: &ReceivedBallot,
) -> bool {
    let statement = &received.statement;
    if let Ok(sealed) = SealedBallot::from_content(&evidence.encrypted_ballot) {
        return pinned_seal_key(trust).is_some_and(|pin| {
            sealed_voter_ballot(evidence, statement, &sealed, &pin)
        });
    }
    if let Ok(ballot) =
        serde_json::from_str::<SignedHashableBallot>(&evidence.encrypted_ballot)
    {
        return ballot.voter_signing_pk.as_deref()
            == Some(statement.voter_signing_pk.as_str())
            && ballot.voter_ballot_signature.as_deref()
                == Some(statement.voter_ballot_signature.as_str())
            && HashableBallot::try_from(&ballot)
                .ok()
                .and_then(|b| hash_ballot(&b).ok())
                .as_deref()
                == Some(statement.ballot_hash.as_str())
            && matches!(
                verify_ballot_signature(
                    &statement.ballot_hash,
                    &statement.election_id,
                    &ballot
                ),
                Ok(Some(_))
            );
    }
    if let Ok(ballot) = serde_json::from_str::<SignedHashableMultiBallot>(
        &evidence.encrypted_ballot,
    ) {
        return ballot.voter_signing_pk.as_deref()
            == Some(statement.voter_signing_pk.as_str())
            && ballot.voter_ballot_signature.as_deref()
                == Some(statement.voter_ballot_signature.as_str())
            && HashableMultiBallot::try_from(&ballot)
                .ok()
                .and_then(|b| hash_multi_ballot(&b).ok())
                .as_deref()
                == Some(statement.ballot_hash.as_str())
            && matches!(
                verify_multi_ballot_signature(
                    &statement.ballot_hash,
                    &statement.election_id,
                    &ballot
                ),
                Ok(Some(_))
            );
    }
    false
}

/// What a sealed ballot shows without its key: a well-formed `hsm-seal-v1`
/// envelope (known inner format, cast-vote hash starting with the Ballot ID)
/// sealed to the key and version the event's log pins, whose Ballot ID is
/// the receipt's ballot hash. The receipt's signature, checked separately,
/// binds that hash to the voter's key and ballot signature, which it can
/// only carry well-formed; the tally checks the opened ballot against the
/// envelope's hashes.
fn sealed_voter_ballot(
    evidence: &BallotReceivedEvidence,
    statement: &crate::ballot_receipt::ReceivedStatement,
    sealed: &SealedBallot,
    pin: &CastVoteSealKeyPin,
) -> bool {
    sealed.is_sealed_to_event(&evidence.tenant_id, &evidence.election_event_id)
        && sealed.key_id == pin.key_id
        && sealed.key_version == pin.key_version
        && sealed.ballot_id == statement.ballot_hash
}

fn cast_context(
    trust: &BallotPrivacyTrust,
    proof: &BallotPrivacyProof,
    evidence: &BallotCastEvidence,
    receipt: &CastReceipt,
) -> bool {
    context(
        trust,
        &evidence.tenant_id,
        &evidence.election_event_id,
        &evidence.election_id,
    ) && evidence.ballot_id == proof.ballot_id
        && evidence.voting_channel == proof.voting_channel
        && pseudonym_hash(&proof.pseudonym).as_deref()
            == Some(evidence.pseudonym_hash.as_str())
        && receipt.statement.election_event_id == trust.election_event_id
        && receipt.statement.election_id == trust.election_id
        && receipt.statement.ballot_id == proof.ballot_id
}

pub fn verify_ballot_log_proof(
    trust: &BallotPrivacyTrust,
    proof: &BallotPrivacyProof,
) -> BallotPrivacyReport {
    use CheckStatus::{Failed, Pending};
    let mut checks = BallotPrivacyChecks {
        binding: Pending,
        received: Pending,
        ballot_id: Pending,
        cast: Pending,
    };
    let mut received: Option<ReceivedBallot> = None;
    if let Some(encoded) = &proof.received_entry {
        checks.binding = Failed;
        checks.received = Failed;
        checks.ballot_id = Failed;
        if let Some(message) = decode(encoded) {
            if let StatementBody::BallotReceived(evidence) =
                &message.statement.body
            {
                if let Ok(receipt) = serde_json::from_str::<ReceivedBallot>(
                    &evidence.received_ballot_json,
                ) {
                    let scoped =
                        received_context(trust, proof, evidence, &receipt);
                    checks.binding = status(
                        scoped
                            && authority(trust, &message)
                            && matches!(
                                message.statement.head.kind,
                                StatementType::BallotReceived
                            )
                            && message.user_id.is_none()
                            && message.sender.name == proof.pseudonym
                            && message.username.as_deref()
                                == Some(proof.pseudonym.as_str()),
                    );
                    checks.received = status(
                        scoped
                            && voter_ballot(trust, evidence, &receipt)
                            && verify_received_ballot(
                                &trust.ballot_box_key,
                                &receipt,
                            )
                            .is_ok(),
                    );
                    checks.ballot_id = status(
                        scoped
                            && ballot_id(
                                &receipt.statement,
                                &receipt.received_signature,
                            )
                            .ok()
                            .as_deref()
                                == Some(proof.ballot_id.as_str()),
                    );
                    received = Some(receipt);
                }
            }
        }
    }
    if let Some(encoded) = &proof.cast_entry {
        checks.cast = Failed;
        if let Some(message) = decode(encoded) {
            if let StatementBody::PrivacyCastVote(evidence) =
                &message.statement.body
            {
                if let Ok(receipt) = serde_json::from_str::<CastReceipt>(
                    &evidence.cast_receipt_json,
                ) {
                    let valid = authority(trust, &message)
                        && cast_context(trust, proof, evidence, &receipt)
                        && matches!(
                            message.statement.head.kind,
                            StatementType::CastVote
                        )
                        && message.user_id.is_none()
                        && message.sender.name == proof.pseudonym
                        && message.username.as_deref()
                            == Some(proof.pseudonym.as_str())
                        && verify_cast_receipt(&trust.ballot_box_key, &receipt)
                            .is_ok();
                    checks.cast = match &received {
                        Some(received) => status(
                            valid
                                && checks.binding == CheckStatus::Passed
                                && received.statement.received_at
                                    == receipt.statement.received_at
                                && received.statement.key_id
                                    == receipt.statement.key_id
                                && verify_cast_signature(
                                    &received.statement.voter_signing_pk,
                                    &trust.election_id,
                                    &proof.ballot_id,
                                    &receipt.statement.cast_signature,
                                )
                                .is_ok(),
                        ),
                        None if valid && proof.received_entry.is_none() => {
                            Pending
                        }
                        None => Failed,
                    };
                }
            }
        }
    }
    BallotPrivacyReport { checks }
}
