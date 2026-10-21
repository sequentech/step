// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{bail, ensure, Context, Result};
use b3::messages::{
    artifact::{Ballots, Configuration},
    message::Message,
    statement::Statement,
};
use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine};
use electoral_log::messages::{
    message::Message as ElectoralMessage,
    statement::{StatementBody, StatementType},
};
use sequent_core::{
    ballot::{HashableBallot, TYPES_VERSION},
    encrypt::{hash_ballot_sha512, hash_multi_ballot_sha512},
    multi_ballot::HashableMultiBallot,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use strand::signature::StrandSignaturePk;
use strand::{
    backend::ristretto::RistrettoCtx,
    elgamal::Ciphertext,
    serialization::{StrandDeserialize, StrandSerialize},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Scope {
    pub tenant_id: String,
    pub election_event_id: String,
    pub election_id: String,
    pub area_id: String,
    pub tally_session_id: String,
    pub tally_session_contest_id: String,
    pub board_name: String,
    pub public_key_sha512: String,
    pub electoral_log_system_public_key: String,
    pub contest_id: Option<String>,
    pub base_batch: u32,
    pub policy: MultiplicityPolicy,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MultiplicityPolicy {
    Unweighted,
    Delegated,
    Weighted,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SelectedVote {
    pub cast_vote_id: String,
    pub ballot_id: String,
    /// SHA-512 of original content UTF-8 bytes, anchored by the manifest digest.
    pub content_sha512: String,
    pub electoral_log_message_sha512: String,
    pub multiplicity: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub format_version: u32,
    pub scope: Scope,
    /// Exact expected batch set from the saved tally annotations/mapping.
    pub batches: Vec<u32>,
    /// Complete selection independently reproduced from consistent snapshots.
    pub selected: Vec<SelectedVote>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CastVote {
    pub voter_id: String,
    pub voting_channel: String,
    pub cast_vote_id: String,
    pub ballot_id: String,
    /// Preserve the database text exactly; do not JSON-reserialize it.
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct Location {
    pub batch: u32,
    pub index: usize,
}
#[derive(Debug, Serialize)]
pub struct ReceiptMatch {
    pub cast_vote_id: String,
    pub ballot_id: String,
    /// All matching input positions; identical ciphertexts are not uniquely attributable.
    pub locations: Vec<Location>,
}
#[derive(Debug, Serialize)]
pub struct AuditReport {
    pub scope: Scope,
    pub checked_cast_votes: usize,
    pub checked_batches: usize,
    pub checked_electoral_log_messages: usize,
    pub legacy_online_channel_inferences: usize,
    pub receipts: Vec<ReceiptMatch>,
    pub assurance: &'static str,
}

pub fn digest(bytes: &[u8]) -> Result<String> {
    Ok(hex::encode(strand::hash::hash_to_array(bytes)?))
}

fn projection(content: &str, contest_id: Option<&str>) -> Result<(strand::hash::Hash, Vec<u8>)> {
    let (receipt, ciphertext): (strand::hash::Hash, Ciphertext<RistrettoCtx>) =
        if let Some(id) = contest_id {
            let ballot: HashableBallot =
                serde_json::from_str(content).context("invalid single-contest ballot")?;
            ensure!(
                ballot.version == TYPES_VERSION,
                "unsupported ballot version"
            );
            let contests = ballot.deserialize_contests::<RistrettoCtx>()?;
            let unique: BTreeSet<_> = contests.iter().map(|c| &c.contest_id).collect();
            ensure!(unique.len() == contests.len(), "duplicate contest IDs");
            let ciphertext = contests
                .iter()
                .find(|c| c.contest_id == id)
                .context("selected contest missing")?
                .ciphertext
                .clone();
            (hash_ballot_sha512(&ballot)?, ciphertext)
        } else {
            let ballot: HashableMultiBallot =
                serde_json::from_str(content).context("invalid multi-contest ballot")?;
            ensure!(
                ballot.version == TYPES_VERSION,
                "unsupported ballot version"
            );
            let contests = ballot.deserialize_contests::<RistrettoCtx>()?;
            let unique: BTreeSet<_> = contests.contest_ids.iter().collect();
            ensure!(
                !unique.is_empty() && unique.len() == contests.contest_ids.len(),
                "empty or duplicate contest IDs"
            );
            (hash_multi_ballot_sha512(&ballot)?, contests.ciphertext)
        };
    Ok((receipt, ciphertext.strand_serialize()?))
}

fn contributions(scope: &Scope, multiplicity: u64) -> Result<Vec<(u32, u64)>> {
    ensure!(multiplicity > 0, "zero multiplicity");
    match scope.policy {
        MultiplicityPolicy::Unweighted => {
            ensure!(multiplicity == 1, "unweighted multiplicity must be one");
            Ok(vec![(scope.base_batch, 1)])
        }
        MultiplicityPolicy::Delegated => Ok(vec![(scope.base_batch, multiplicity)]),
        MultiplicityPolicy::Weighted => {
            ensure!(
                multiplicity <= sequent_core::types::keycloak::MAX_VOTE_WEIGHT,
                "weight exceeds supported limit"
            );
            (0..sequent_core::types::keycloak::VOTE_WEIGHT_BATCHES)
                .filter(|bit| multiplicity & (1u64 << bit) != 0)
                .map(|bit| {
                    Ok((
                        scope
                            .base_batch
                            .checked_add(bit)
                            .context("batch overflow")?,
                        1,
                    ))
                })
                .collect()
        }
    }
}

/// Audits one complete, independently pinned area/contest scope. This does not
/// establish snapshot provenance or verify shuffle/decryption/counting proofs.
#[allow(clippy::too_many_arguments)]
pub fn audit(
    manifest_bytes: &[u8],
    expected_manifest_sha512: &str,
    rows: &[CastVote],
    configuration_bytes: &[u8],
    expected_configuration_sha512: &str,
    messages: &[Vec<u8>],
    ballot_id: Option<&str>,
    electoral_log_messages: &[Vec<u8>],
) -> Result<AuditReport> {
    ensure!(
        digest(manifest_bytes)? == expected_manifest_sha512,
        "manifest digest mismatch"
    );
    ensure!(
        digest(configuration_bytes)? == expected_configuration_sha512,
        "configuration digest mismatch"
    );
    let manifest: Manifest = serde_json::from_slice(manifest_bytes).context("invalid manifest")?;
    ensure!(manifest.format_version == 1, "unsupported manifest version");
    let scope = &manifest.scope;
    for value in [
        &scope.tenant_id,
        &scope.election_event_id,
        &scope.election_id,
        &scope.area_id,
        &scope.tally_session_id,
        &scope.tally_session_contest_id,
        &scope.board_name,
    ] {
        ensure!(!value.trim().is_empty(), "missing scope identifier");
    }
    ensure!(
        scope
            .contest_id
            .as_ref()
            .is_none_or(|id| !id.trim().is_empty()),
        "empty contest ID"
    );
    let batch_set: BTreeSet<_> = manifest.batches.iter().copied().collect();
    ensure!(
        !batch_set.is_empty() && batch_set.len() == manifest.batches.len(),
        "empty or duplicate batch scope"
    );
    let configuration = Configuration::<RistrettoCtx>::strand_deserialize(configuration_bytes)?;
    ensure!(configuration.is_valid(), "invalid configuration");
    let system_pk = StrandSignaturePk::from_der_b64_string(&scope.electoral_log_system_public_key)
        .context("invalid pinned electoral-log system key")?;
    let mut logs_by_hash = BTreeMap::new();
    for bytes in electoral_log_messages {
        ensure!(
            logs_by_hash.insert(digest(bytes)?, bytes).is_none(),
            "duplicate electoral-log message"
        );
    }
    let mut used_logs = BTreeSet::new();
    let mut legacy_online_channel_inferences = 0;
    let mut rows_by_id = BTreeMap::new();
    for row in rows {
        ensure!(
            rows_by_id.insert(&row.cast_vote_id, row).is_none(),
            "duplicate cast vote row"
        );
    }
    let mut selected_ids = BTreeSet::new();
    let mut receipts = BTreeSet::new();
    let mut expected: BTreeMap<(u32, Vec<u8>), u64> = BTreeMap::new();
    let mut projected = Vec::new();
    let mut computed_batches = BTreeSet::new();
    for selected in &manifest.selected {
        ensure!(
            !selected.cast_vote_id.trim().is_empty(),
            "empty selected cast vote ID"
        );
        ensure!(
            selected_ids.insert(&selected.cast_vote_id),
            "duplicate selected cast vote ID"
        );
        ensure!(
            receipts.insert(&selected.ballot_id),
            "ambiguous duplicate receipt"
        );
        let row = rows_by_id
            .get(&selected.cast_vote_id)
            .context("selected cast vote missing from export")?;
        ensure!(
            row.ballot_id == selected.ballot_id,
            "stored receipt differs from manifest"
        );
        ensure!(
            digest(row.content.as_bytes())? == selected.content_sha512,
            "full content digest mismatch for {}",
            row.cast_vote_id
        );
        let (full_receipt_hash, ciphertext) = projection(&row.content, scope.contest_id.as_deref())
            .with_context(|| format!("invalid cast vote {}", row.cast_vote_id))?;
        ensure!(
            hex::encode(&full_receipt_hash[..32]) == row.ballot_id,
            "receipt hash mismatch for {}",
            row.cast_vote_id
        );
        ensure!(
            used_logs.insert(&selected.electoral_log_message_sha512),
            "reused electoral-log evidence"
        );
        let log_bytes = logs_by_hash
            .get(&selected.electoral_log_message_sha512)
            .context("selected electoral-log message missing")?;
        let log = ElectoralMessage::strand_deserialize(log_bytes)
            .context("invalid electoral-log message")?;
        log.verify(&system_pk)
            .context("electoral-log signature verification failed")?;
        ensure!(
            log.statement.head.event.0 == scope.election_event_id
                && matches!(log.statement.head.kind, StatementType::CastVote),
            "electoral-log event or statement kind mismatch"
        );
        let (election, pseudonym, cast_hash, channel) = match &log.statement.body {
            StatementBody::CastVote(e, p, h, _, _) => {
                legacy_online_channel_inferences += 1;
                (e, p, h, "ONLINE")
            }
            StatementBody::CastVoteWithChannel(e, p, h, _, _, c) => (e, p, h, c.0.as_str()),
            _ => bail!("electoral-log evidence is not a cast vote"),
        };
        ensure!(
            election.0.as_deref() == Some(scope.election_id.as_str()),
            "electoral-log election mismatch"
        );
        ensure!(!row.voter_id.is_empty(), "missing cast vote voter ID");
        let voter_hash = strand::hash::hash_to_array(&row.voter_id.strand_serialize()?)?;
        ensure!(
            pseudonym.0.clone().into_inner() == voter_hash,
            "electoral-log voter pseudonym mismatch"
        );
        ensure!(
            cast_hash.0.clone().into_inner() == full_receipt_hash,
            "electoral-log full ballot hash mismatch"
        );
        ensure!(
            channel == row.voting_channel,
            "electoral-log voting channel mismatch"
        );
        for (batch, count) in contributions(scope, selected.multiplicity)? {
            computed_batches.insert(batch);
            let total = expected.entry((batch, ciphertext.clone())).or_default();
            *total = total.checked_add(count).context("multiplicity overflow")?;
        }
        projected.push((row, ciphertext));
    }
    ensure!(
        used_logs.len() == logs_by_hash.len(),
        "unselected electoral-log evidence"
    );
    ensure!(
        selected_ids.len() == rows_by_id.len(),
        "export contains unselected cast vote rows"
    );
    if computed_batches.is_empty() {
        computed_batches.insert(scope.base_batch);
    }
    ensure!(
        computed_batches == batch_set,
        "selected set does not produce declared batch scope"
    );
    let mut seen_batches = BTreeSet::new();
    let mut actual: BTreeMap<(u32, Vec<u8>), u64> = BTreeMap::new();
    let mut locations: BTreeMap<Vec<u8>, Vec<Location>> = BTreeMap::new();
    for bytes in messages {
        let message = Message::strand_deserialize(bytes).context("invalid B3 message")?;
        let Statement::Ballots(_, _, batch, ciphertexts_hash, public_key_hash, _) =
            &message.statement
        else {
            bail!("input contains a non-Ballots message");
        };
        let batch = u32::try_from(*batch).context("batch out of range")?;
        ensure!(
            batch_set.contains(&batch),
            "B3 batch outside declared scope"
        );
        ensure!(seen_batches.insert(batch), "duplicate B3 batch");
        let artifact = message
            .artifact
            .as_ref()
            .context("missing Ballots artifact")?;
        ensure!(
            strand::hash::hash_to_array(artifact)? == ciphertexts_hash.0,
            "Ballots artifact hash mismatch"
        );
        ensure!(
            hex::encode(public_key_hash.0) == scope.public_key_sha512,
            "public key hash outside pinned scope"
        );
        ensure!(
            digest(artifact)? != expected_configuration_sha512,
            "Ballots artifact cannot be the configuration"
        );
        let ballots = Ballots::<RistrettoCtx>::strand_deserialize(artifact)?;
        message
            .verify(&configuration)
            .context("B3 signature/configuration verification failed")?;
        for (index, ciphertext) in ballots.ciphertexts.0.iter().enumerate() {
            let ciphertext = ciphertext.strand_serialize()?;
            *actual.entry((batch, ciphertext.clone())).or_default() += 1;
            locations
                .entry(ciphertext)
                .or_default()
                .push(Location { batch, index });
        }
    }
    ensure!(seen_batches == batch_set, "missing B3 batch");
    ensure!(
        expected == actual,
        "tally input multiset mismatch: omitted, additional, duplicated or substituted ciphertexts"
    );
    if let Some(id) = ballot_id {
        ensure!(
            receipts.iter().any(|receipt| receipt.as_str() == id),
            "requested ballot receipt not in selected scope"
        );
    }
    let mut matches = Vec::new();
    for (row, ciphertext) in projected {
        if ballot_id.is_some_and(|id| id != row.ballot_id) {
            continue;
        }
        matches.push(ReceiptMatch {
            cast_vote_id: row.cast_vote_id.clone(),
            ballot_id: row.ballot_id.clone(),
            locations: locations
                .get(&ciphertext)
                .context("missing ciphertext locations")?
                .iter()
                .map(|p| Location {
                    batch: p.batch,
                    index: p.index,
                })
                .collect(),
        });
    }
    Ok(AuditReport { scope: manifest.scope, checked_cast_votes: rows.len(), checked_batches: seen_batches.len(),
        checked_electoral_log_messages: used_logs.len(),
        legacy_online_channel_inferences, receipts: matches,
        assurance: "Signed electoral-log voter/event/election/full ballot hashes match every selected cast vote. Channels match signed values except reported legacy ONLINE inferences. Exact pre-shuffle input multiset and receipt hashes match the pinned selected-set manifest. Full content bytes match its separate digest anchors. This selected-scope result does not reconcile every historical CastVote log to Hasura or detect accepted casts missing from both supplied history and tally inputs. ImmuDB append-only history, snapshot/selection provenance and mixnet/counting proofs must be verified separately." })
}

pub fn decode_message(value: &str) -> Result<Vec<u8>> {
    STANDARD_NO_PAD
        .decode(value)
        .context("invalid unpadded base64 message")
}
