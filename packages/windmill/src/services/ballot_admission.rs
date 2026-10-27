// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Authenticate acceptance before any mutable database row reaches the sequencer.
//! The event signing key stays in the application. A database row alone cannot
//! cause the sequencer to create a trusted cast-vote record.
use anyhow::{ensure, Context, Result};
use chrono::{DateTime, SecondsFormat, TimeZone, Utc};
use electoral_log::adapters::ballot_box::{AcceptBallot, PendingBallot};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use voting_crypto::signing::{Signature, SigningKey, VerificationKey};

const DOMAIN: &[u8] = b"sequent.ballot-admission.v1\0";
const MAX_RECORD_BYTES: usize = 64 * 1024;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Payload {
    event: String,
    election: String,
    area: String,
    voter: String,
    ballot_id: String,
    format: String,
    content_sha256: String,
    voter_signature: Option<String>,
    pseudonym_hash: String,
    ballot_hash: String,
    voting_channel: String,
    voter_ip: Option<String>,
    voter_country: Option<String>,
    username: Option<String>,
    accepted_at: String,
    // The latest revote is selected by sequence, so order is authenticated too.
    sequence: i64,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Admission {
    payload: Payload,
    signature: Signature,
}

fn payload(
    ballot: &AcceptBallot<'_>,
    accepted_at: &DateTime<Utc>,
    sequence: i64,
) -> Result<Payload> {
    ensure!(sequence > 0, "Invalid ballot acceptance sequence");
    ensure!(
        accepted_at.timestamp_subsec_nanos() % 1_000_000 == 0,
        "Acceptance time must have millisecond precision"
    );
    Ok(Payload {
        event: ballot.election_event_id.into(),
        election: ballot.election_id.into(),
        area: ballot.area_id.into(),
        voter: ballot.voter_id.into(),
        ballot_id: ballot.ballot_id.into(),
        format: ballot.format.into(),
        content_sha256: hex::encode(Sha256::digest(ballot.content.as_bytes())),
        voter_signature: ballot.voter_signature.map(hex::encode),
        pseudonym_hash: hex::encode(ballot.pseudonym_hash),
        ballot_hash: hex::encode(ballot.ballot_hash),
        voting_channel: ballot.voting_channel.into(),
        voter_ip: ballot.voter_ip.map(str::to_owned),
        voter_country: ballot.voter_country.map(str::to_owned),
        username: ballot.username.map(str::to_owned),
        accepted_at: accepted_at.to_rfc3339_opts(SecondsFormat::Millis, true),
        sequence,
    })
}

fn signed_bytes(payload: &Payload) -> Result<Vec<u8>> {
    let mut bytes = DOMAIN.to_vec();
    bytes.extend(serde_json::to_vec(payload)?);
    Ok(bytes)
}

/// The caller holds the voter's acceptance lock before reserving its order.
/// Gaps from refused requests are harmless; an issued order is never reused.
pub async fn reserve_sequence(event: &str) -> Result<i64> {
    let store = super::electoral_log_board::get_event_store(event).await?;
    let db = store.client().await?;
    Ok(db
        .query_one("SELECT nextval('ballot_box_seq')", &[])
        .await?
        .try_get(0)?)
}

/// Build the log record from the accepted input in memory, before storing it.
/// Never re-read unauthenticated database fields to issue a new trusted signature.
pub fn pending(
    ballot: &AcceptBallot<'_>,
    accepted_at: &DateTime<Utc>,
    sequence: i64,
) -> PendingBallot {
    PendingBallot {
        seq: sequence,
        ballot_id: ballot.ballot_id.into(),
        election_id: ballot.election_id.into(),
        area_id: ballot.area_id.into(),
        voter_id: ballot.voter_id.into(),
        pseudonym_hash: ballot.pseudonym_hash.to_vec(),
        ballot_hash: ballot.ballot_hash.to_vec(),
        voting_channel: ballot.voting_channel.into(),
        status: ballot.status.to_string(),
        voter_ip: ballot.voter_ip.map(str::to_owned),
        voter_country: ballot.voter_country.map(str::to_owned),
        username: ballot.username.map(str::to_owned),
        accepted_at: accepted_at.timestamp(),
        accepted_at_millis: accepted_at.timestamp_millis(),
        format: ballot.format.into(),
        content: ballot.content.into(),
        voter_signature: ballot.voter_signature.map(<[u8]>::to_vec),
        admission_record: ballot.admission_record.to_vec(),
    }
}

pub fn sign(
    ballot: &AcceptBallot<'_>,
    key: &SigningKey,
    accepted_at: &DateTime<Utc>,
    sequence: i64,
) -> Result<Vec<u8>> {
    let payload = payload(ballot, accepted_at, sequence)?;
    let signature = key.sign(&signed_bytes(&payload)?)?;
    let record = serde_json::to_vec(&Admission { payload, signature })?;
    ensure!(
        record.len() <= MAX_RECORD_BYTES,
        "Ballot admission metadata exceeds the limit"
    );
    Ok(record)
}

pub fn verify(
    ballot: &AcceptBallot<'_>,
    key: &VerificationKey,
    accepted_at: &DateTime<Utc>,
    sequence: i64,
) -> Result<()> {
    ensure!(
        ballot.admission_record.len() <= MAX_RECORD_BYTES,
        "Ballot admission record exceeds the limit"
    );
    let admission: Admission = serde_json::from_slice(ballot.admission_record)
        .context("Missing or malformed ballot admission record")?;
    key.verify(&admission.signature, &signed_bytes(&admission.payload)?)
        .context("Invalid ballot admission signature")?;
    ensure!(
        admission.payload == payload(ballot, accepted_at, sequence)?,
        "Stored ballot differs from its authenticated acceptance"
    );
    Ok(())
}

pub fn verify_pending(ballot: &PendingBallot, event: &str, key: &VerificationKey) -> Result<()> {
    let at = Utc
        .timestamp_millis_opt(ballot.accepted_at_millis)
        .single()
        .context("Invalid ballot acceptance time")?;
    let accepted = AcceptBallot {
        election_event_id: event,
        election_id: &ballot.election_id,
        area_id: &ballot.area_id,
        voter_id: &ballot.voter_id,
        ballot_id: &ballot.ballot_id,
        format: &ballot.format,
        content: &ballot.content,
        voter_signature: ballot.voter_signature.as_deref(),
        admission_record: &ballot.admission_record,
        pseudonym_hash: &ballot.pseudonym_hash,
        ballot_hash: &ballot.ballot_hash,
        voting_channel: &ballot.voting_channel,
        // Admission authenticates immutable acceptance attributes. Datafix's
        // separately audited status transitions do not rewrite that record.
        status: electoral_log::adapters::ballot_box::BallotStatus::Valid,
        voter_ip: ballot.voter_ip.as_deref(),
        voter_country: ballot.voter_country.as_deref(),
        username: ballot.username.as_deref(),
        allowed_votes: 0,
    };
    verify(&accepted, key, &at, ballot.seq)
}

#[cfg(test)]
mod tests {
    use super::*;
    use electoral_log::adapters::ballot_box::BallotStatus;
    fn ballot() -> AcceptBallot<'static> {
        AcceptBallot {
            election_event_id: "event",
            election_id: "election",
            area_id: "area",
            voter_id: "voter",
            ballot_id: "ballot",
            format: "hashable-ballot",
            content: "encrypted-vote",
            voter_signature: Some(b"signature"),
            admission_record: &[],
            pseudonym_hash: &[1; 64],
            ballot_hash: &[2; 64],
            voting_channel: "ONLINE",
            status: BallotStatus::Valid,
            voter_ip: Some("192.0.2.1"),
            voter_country: Some("ZZ"),
            username: Some("fixture"),
            allowed_votes: 1,
        }
    }
    #[test]
    fn authenticated_acceptance_rejects_changes_before_sequencing_including_revote_order() {
        let key = SigningKey::generate().unwrap();
        let public = VerificationKey::from_sk(&key).unwrap();
        let at = Utc
            .timestamp_millis_opt(1_793_102_400_000)
            .single()
            .unwrap();
        let record = sign(&ballot(), &key, &at, 3).unwrap();
        let original = AcceptBallot {
            admission_record: &record,
            ..ballot()
        };
        verify(&original, &public, &at, 3).unwrap();
        let tampered = [
            AcceptBallot {
                voter_id: "other",
                ..original.clone()
            },
            AcceptBallot {
                area_id: "elsewhere",
                ..original.clone()
            },
            AcceptBallot {
                election_event_id: "other-event",
                ..original.clone()
            },
            AcceptBallot {
                election_id: "other-election",
                ..original.clone()
            },
            AcceptBallot {
                ballot_id: "another",
                ..original.clone()
            },
            AcceptBallot {
                format: "other",
                ..original.clone()
            },
            AcceptBallot {
                content: "different-encrypted-vote",
                ..original.clone()
            },
            AcceptBallot {
                ballot_hash: &[3; 64],
                ..original.clone()
            },
            AcceptBallot {
                pseudonym_hash: &[4; 64],
                ..original.clone()
            },
            AcceptBallot {
                voting_channel: "KIOSK",
                ..original.clone()
            },
            AcceptBallot {
                voter_signature: None,
                ..original.clone()
            },
            AcceptBallot {
                voter_ip: None,
                ..original.clone()
            },
            AcceptBallot {
                voter_country: None,
                ..original.clone()
            },
            AcceptBallot {
                username: None,
                ..original.clone()
            },
        ];
        for changed in tampered {
            assert!(verify(&changed, &public, &at, 3).is_err());
        }
        assert!(verify(&original, &public, &at, 4).is_err());
        assert!(verify(
            &original,
            &public,
            &(at + chrono::Duration::milliseconds(1)),
            3
        )
        .is_err());
        assert!(verify(
            &original,
            &VerificationKey::from_sk(&SigningKey::generate().unwrap()).unwrap(),
            &at,
            3
        )
        .is_err());
        assert!(verify(&ballot(), &public, &at, 3).is_err());
    }
}
