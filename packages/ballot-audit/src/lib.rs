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
    ballot::{HashableBallot, VotingStatusChannel, TYPES_VERSION},
    encrypt::{hash_ballot_sha512, hash_multi_ballot_sha512},
    multi_ballot::HashableMultiBallot,
    types::keycloak::MIN_WEIGHT_BATCH_ANONYMITY,
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

impl MultiplicityPolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            MultiplicityPolicy::Unweighted => "unweighted",
            MultiplicityPolicy::Delegated => "delegated",
            MultiplicityPolicy::Weighted => "weighted",
        }
    }
}

impl std::fmt::Display for MultiplicityPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
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

#[derive(Clone, Debug, Serialize)]
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
/// Why a stored cast vote row was or was not selected for the tally input.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectionReason {
    OutsideScope,
    Discarded,
    NoEligibleVoter,
    SupersededRevote,
    Selected,
}

/// The parts of `prepare-ballot-audit`'s `selection.json` used to explain a
/// receipt that is not in the selected scope.
#[derive(Debug, Deserialize)]
pub struct Selection {
    pub format_version: u32,
    pub rows: Vec<SelectionRow>,
}

#[derive(Debug, Deserialize)]
pub struct SelectionRow {
    pub cast_vote_id: String,
    pub ballot_id: Option<String>,
    pub reason: SelectionReason,
    pub latest_scoped_cast_vote_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ExcludedRow {
    pub cast_vote_id: String,
    pub reason: SelectionReason,
    pub latest_scoped_cast_vote_id: Option<String>,
}

pub const UNVERIFIED_STORED_LABELS: &str = "Non-selected ballot_id values come from selection.json and are unverified stored labels: their receipt hashes, contents and signed electoral-log links were not checked.";

#[derive(Debug, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum ReceiptLookup {
    Selected {
        ballot_id: String,
        cast_vote_id: String,
        locations: Vec<Location>,
    },
    Excluded {
        ballot_id: String,
        rows: Vec<ExcludedRow>,
        stored_labels: &'static str,
    },
    Missing {
        ballot_id: String,
        selection_searched: bool,
    },
    Ambiguous {
        ballot_id: String,
        selected_cast_vote_ids: Vec<String>,
    },
}

#[derive(Debug, Serialize)]
pub struct DuplicateReceipt {
    pub ballot_id: String,
    pub cast_vote_ids: Vec<String>,
}

pub const SMALL_WEIGHT_BATCH_WARNING: &str = "This weight batch holds fewer ciphertexts than the anonymity threshold, so its decrypted votes are close to individually attributable to the voters whose weight sets this batch's bit.";

#[derive(Debug, Serialize)]
pub struct PrivacyWarning {
    pub batch: u32,
    pub ciphertexts: usize,
    pub minimum_for_anonymity: usize,
    pub warning: &'static str,
}

#[derive(Debug, Serialize)]
pub struct AuditReport {
    pub scope: Scope,
    pub checked_cast_votes: usize,
    pub checked_batches: usize,
    pub checked_electoral_log_messages: usize,
    pub legacy_online_channel_inferences: usize,
    pub receipts: Vec<ReceiptMatch>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lookup: Option<ReceiptLookup>,
    /// Distinct selected cast votes carrying the same receipt: one encrypted
    /// ballot accepted under several voters and counted once for each.
    pub duplicate_selected_receipts: Vec<DuplicateReceipt>,
    pub privacy_warnings: Vec<PrivacyWarning>,
    pub assurance: &'static str,
}

impl AuditReport {
    pub fn status(&self) -> ExitStatus {
        if !self.duplicate_selected_receipts.is_empty() {
            ExitStatus::VerificationFailed
        } else if self
            .lookup
            .as_ref()
            .is_some_and(|lookup| !matches!(lookup, ReceiptLookup::Selected { .. }))
        {
            ExitStatus::Incomplete
        } else {
            ExitStatus::Success
        }
    }
}

/// Process exit statuses shared by every ballot-audit command. Clap keeps its
/// own status 2 for command-line usage errors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExitStatus {
    Success = 0,
    /// Evidence contradicts itself or the pinned scope. Unclassified failures
    /// also land here so that nothing fails open.
    VerificationFailed = 1,
    Usage = 2,
    /// Required evidence is absent: neither consistency nor contradiction is
    /// established.
    Incomplete = 3,
    /// Input could not be read or parsed, or lies outside the supported scope.
    InvalidInput = 4,
}

#[derive(Debug)]
struct Classified {
    status: ExitStatus,
    message: String,
}

impl std::fmt::Display for Classified {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Classified {}

pub fn incomplete(message: impl Into<String>) -> anyhow::Error {
    anyhow::Error::new(Classified {
        status: ExitStatus::Incomplete,
        message: message.into(),
    })
}

pub fn invalid_input(message: impl Into<String>) -> anyhow::Error {
    anyhow::Error::new(Classified {
        status: ExitStatus::InvalidInput,
        message: message.into(),
    })
}

impl ExitStatus {
    /// The outermost classification in the error chain wins.
    pub fn of(error: &anyhow::Error) -> Self {
        error
            .chain()
            .find_map(|cause| cause.downcast_ref::<Classified>())
            .map_or(ExitStatus::VerificationFailed, |c| c.status)
    }

    pub fn code(self) -> i32 {
        self as i32
    }
}

pub fn validate_sha512_pin(pin: &str, label: &str) -> Result<()> {
    if pin.len() == 128
        && pin
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        Ok(())
    } else {
        Err(invalid_input(format!(
            "{label} SHA-512 pin must be 128 lowercase hexadecimal characters"
        )))
    }
}

fn required_nullable<'de, D, T>(deserializer: D) -> std::result::Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(deserializer)
}

/// Signed CastVote identity: voter pseudonym and full native ballot hash.
pub type CastVoteKey = (strand::hash::Hash, strand::hash::Hash);

// Extra columns, including area/status/timestamp, are deliberately not used to
// filter historical coverage. No latest-vote or eligibility selection occurs.
#[derive(Deserialize)]
pub struct HistoryRow {
    pub id: String,
    pub tenant_id: String,
    pub election_event_id: String,
    #[serde(deserialize_with = "required_nullable")]
    pub election_id: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub voter_id_string: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub content: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub ballot_id: Option<String>,
}

fn history_ballot_hash(content: &str, single_contest: bool) -> Result<strand::hash::Hash> {
    if single_contest {
        let ballot: HashableBallot =
            serde_json::from_str(content).context("unhashable historical single-contest ballot")?;
        if ballot.version != TYPES_VERSION {
            return Err(invalid_input("unsupported historical ballot version"));
        }
        hash_ballot_sha512(&ballot).context("unhashable historical single-contest ballot")
    } else {
        let ballot: HashableMultiBallot =
            serde_json::from_str(content).context("unhashable historical multi-contest ballot")?;
        if ballot.version != TYPES_VERSION {
            return Err(invalid_input("unsupported historical ballot version"));
        }
        hash_multi_ballot_sha512(&ballot).context("unhashable historical multi-contest ballot")
    }
}

/// Validate a complete tenant/event/election cast_vote history and group its
/// row IDs by signed CastVote identity.
pub fn index_history(
    scope: &Scope,
    rows: &[HistoryRow],
) -> Result<BTreeMap<CastVoteKey, Vec<String>>> {
    let mut groups: BTreeMap<CastVoteKey, Vec<String>> = BTreeMap::new();
    let mut row_ids = BTreeSet::new();
    for row in rows {
        let id = uuid::Uuid::parse_str(&row.id)
            .map_err(|_| invalid_input("invalid historical cast_vote UUID"))?;
        ensure!(row_ids.insert(id), "duplicate historical cast_vote UUID");
        ensure!(
            row.tenant_id == scope.tenant_id
                && row.election_event_id == scope.election_event_id
                && row.election_id.as_deref() == Some(scope.election_id.as_str()),
            "historical Hasura row outside pinned tenant/event/election; all areas are allowed"
        );
        let voter = row
            .voter_id_string
            .as_ref()
            .filter(|id| !id.is_empty())
            .context("historical cast vote lacks voter ID")?;
        let content = row
            .content
            .as_ref()
            .context("historical cast vote lacks content")?;
        let full_hash = history_ballot_hash(content, scope.contest_id.is_some())?;
        ensure!(
            row.ballot_id.as_deref() == Some(hex::encode(&full_hash[..32]).as_str()),
            "historical stored ballot receipt does not match native full-ballot hash"
        );
        let pseudonym = strand::hash::hash_to_array(&voter.strand_serialize()?)?;
        groups
            .entry((pseudonym, full_hash))
            .or_default()
            .push(id.to_string());
    }
    Ok(groups)
}

pub fn digest(bytes: &[u8]) -> Result<String> {
    Ok(hex::encode(strand::hash::hash_to_array(bytes)?))
}

fn projection(content: &str, contest_id: Option<&str>) -> Result<(strand::hash::Hash, Vec<u8>)> {
    let (receipt, ciphertext): (strand::hash::Hash, Ciphertext<RistrettoCtx>) =
        if let Some(id) = contest_id {
            let ballot: HashableBallot =
                serde_json::from_str(content).context("invalid single-contest ballot")?;
            if ballot.version != TYPES_VERSION {
                return Err(invalid_input("unsupported ballot version"));
            }
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
            if ballot.version != TYPES_VERSION {
                return Err(invalid_input("unsupported ballot version"));
            }
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
/// Callers must also act on [`AuditReport::status`]: duplicate selected
/// receipts and unselected receipt lookups are reported, not returned as errors.
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
    audit_with_selection(
        manifest_bytes,
        expected_manifest_sha512,
        rows,
        configuration_bytes,
        expected_configuration_sha512,
        messages,
        ballot_id,
        electoral_log_messages,
        None,
    )
}

/// [`audit`], explaining a requested receipt that is not selected from the
/// `selection.json` rows written alongside the manifest.
#[allow(clippy::too_many_arguments)]
pub fn audit_with_selection(
    manifest_bytes: &[u8],
    expected_manifest_sha512: &str,
    rows: &[CastVote],
    configuration_bytes: &[u8],
    expected_configuration_sha512: &str,
    messages: &[Vec<u8>],
    ballot_id: Option<&str>,
    electoral_log_messages: &[Vec<u8>],
    selection: Option<&Selection>,
) -> Result<AuditReport> {
    ensure!(
        digest(manifest_bytes)? == expected_manifest_sha512,
        "manifest digest mismatch"
    );
    ensure!(
        digest(configuration_bytes)? == expected_configuration_sha512,
        "configuration digest mismatch"
    );
    let manifest: Manifest =
        serde_json::from_slice(manifest_bytes).map_err(|_| invalid_input("invalid manifest"))?;
    if manifest.format_version != 1 {
        return Err(invalid_input("unsupported manifest version"));
    }
    let scope = &manifest.scope;
    if [
        &scope.tenant_id,
        &scope.election_event_id,
        &scope.election_id,
        &scope.area_id,
        &scope.tally_session_id,
        &scope.tally_session_contest_id,
        &scope.board_name,
    ]
    .iter()
    .any(|value| value.trim().is_empty())
    {
        return Err(invalid_input("missing scope identifier"));
    }
    if scope
        .contest_id
        .as_ref()
        .is_some_and(|id| id.trim().is_empty())
    {
        return Err(invalid_input("empty contest ID"));
    }
    let batch_set: BTreeSet<_> = manifest.batches.iter().copied().collect();
    if batch_set.is_empty() || batch_set.len() != manifest.batches.len() {
        return Err(invalid_input("empty or duplicate batch scope"));
    }
    let configuration = Configuration::<RistrettoCtx>::strand_deserialize(configuration_bytes)
        .map_err(|_| invalid_input("invalid configuration"))?;
    if !configuration.is_valid() {
        return Err(invalid_input("invalid configuration"));
    }
    let system_pk = StrandSignaturePk::from_der_b64_string(&scope.electoral_log_system_public_key)
        .map_err(|_| invalid_input("invalid pinned electoral-log system key"))?;
    if let Some(selection) = selection {
        if selection.format_version != 1 {
            return Err(invalid_input("unsupported selection version"));
        }
        let selected_rows: BTreeSet<_> = selection
            .rows
            .iter()
            .filter(|row| row.reason == SelectionReason::Selected)
            .map(|row| row.cast_vote_id.as_str())
            .collect();
        let manifest_rows: BTreeSet<_> = manifest
            .selected
            .iter()
            .map(|selected| selected.cast_vote_id.as_str())
            .collect();
        if selected_rows != manifest_rows {
            return Err(invalid_input(
                "selection does not describe this manifest's selected cast votes",
            ));
        }
    }
    let mut logs_by_hash = BTreeMap::new();
    for bytes in electoral_log_messages {
        ensure!(
            logs_by_hash.insert(digest(bytes)?, bytes).is_none(),
            "duplicate electoral-log message"
        );
    }
    let mut used_logs = BTreeSet::new();
    let mut legacy_online_channel_inferences = 0;
    let legacy_channel = VotingStatusChannel::ONLINE.to_string();
    let mut rows_by_id = BTreeMap::new();
    for row in rows {
        ensure!(
            rows_by_id.insert(&row.cast_vote_id, row).is_none(),
            "duplicate cast vote row"
        );
    }
    let mut selected_ids = BTreeSet::new();
    let mut votes_by_receipt: BTreeMap<&str, Vec<String>> = BTreeMap::new();
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
        votes_by_receipt
            .entry(&selected.ballot_id)
            .or_default()
            .push(selected.cast_vote_id.clone());
        let row = rows_by_id
            .get(&selected.cast_vote_id)
            .ok_or_else(|| incomplete("selected cast vote missing from export"))?;
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
            .ok_or_else(|| incomplete("selected electoral-log message missing"))?;
        let log = ElectoralMessage::strand_deserialize(log_bytes)
            .map_err(|_| invalid_input("invalid electoral-log message"))?;
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
                (e, p, h, legacy_channel.as_str())
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
    let mut batch_sizes = BTreeMap::new();
    let mut actual: BTreeMap<(u32, Vec<u8>), u64> = BTreeMap::new();
    let mut locations: BTreeMap<Vec<u8>, Vec<Location>> = BTreeMap::new();
    for bytes in messages {
        let message =
            Message::strand_deserialize(bytes).map_err(|_| invalid_input("invalid B3 message"))?;
        let Statement::Ballots(_, _, batch, ciphertexts_hash, public_key_hash, _) =
            &message.statement
        else {
            return Err(invalid_input("input contains a non-Ballots message"));
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
        let ballots = Ballots::<RistrettoCtx>::strand_deserialize(artifact)
            .map_err(|_| invalid_input("invalid Ballots artifact"))?;
        message
            .verify(&configuration)
            .context("B3 signature/configuration verification failed")?;
        batch_sizes.insert(batch, ballots.ciphertexts.0.len());
        for (index, ciphertext) in ballots.ciphertexts.0.iter().enumerate() {
            let ciphertext = ciphertext.strand_serialize()?;
            *actual.entry((batch, ciphertext.clone())).or_default() += 1;
            locations
                .entry(ciphertext)
                .or_default()
                .push(Location { batch, index });
        }
    }
    if seen_batches != batch_set {
        return Err(incomplete("missing B3 batch"));
    }
    ensure!(
        expected == actual,
        "tally input multiset mismatch: omitted, additional, duplicated or substituted ciphertexts"
    );
    let privacy_warnings = match scope.policy {
        MultiplicityPolicy::Weighted => batch_sizes
            .into_iter()
            .filter(|(_, size)| *size > 0 && *size < MIN_WEIGHT_BATCH_ANONYMITY)
            .map(|(batch, ciphertexts)| PrivacyWarning {
                batch,
                ciphertexts,
                minimum_for_anonymity: MIN_WEIGHT_BATCH_ANONYMITY,
                warning: SMALL_WEIGHT_BATCH_WARNING,
            })
            .collect(),
        MultiplicityPolicy::Unweighted | MultiplicityPolicy::Delegated => Vec::new(),
    };
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
                .clone(),
        });
    }
    let lookup = ballot_id.map(|id| match matches.as_slice() {
        [single] => ReceiptLookup::Selected {
            ballot_id: id.to_owned(),
            cast_vote_id: single.cast_vote_id.clone(),
            locations: single.locations.clone(),
        },
        [] => match selection {
            Some(selection) => {
                let rows: Vec<_> = selection
                    .rows
                    .iter()
                    .filter(|row| row.ballot_id.as_deref() == Some(id))
                    .map(|row| ExcludedRow {
                        cast_vote_id: row.cast_vote_id.clone(),
                        reason: row.reason,
                        latest_scoped_cast_vote_id: row.latest_scoped_cast_vote_id.clone(),
                    })
                    .collect();
                if rows.is_empty() {
                    ReceiptLookup::Missing {
                        ballot_id: id.to_owned(),
                        selection_searched: true,
                    }
                } else {
                    ReceiptLookup::Excluded {
                        ballot_id: id.to_owned(),
                        rows,
                        stored_labels: UNVERIFIED_STORED_LABELS,
                    }
                }
            }
            None => ReceiptLookup::Missing {
                ballot_id: id.to_owned(),
                selection_searched: false,
            },
        },
        several => ReceiptLookup::Ambiguous {
            ballot_id: id.to_owned(),
            selected_cast_vote_ids: several.iter().map(|m| m.cast_vote_id.clone()).collect(),
        },
    });
    let duplicate_selected_receipts = votes_by_receipt
        .into_iter()
        .filter(|(_, cast_vote_ids)| cast_vote_ids.len() > 1)
        .map(|(ballot_id, cast_vote_ids)| DuplicateReceipt {
            ballot_id: ballot_id.to_owned(),
            cast_vote_ids,
        })
        .collect();
    Ok(AuditReport { scope: manifest.scope, checked_cast_votes: rows.len(), checked_batches: seen_batches.len(),
        checked_electoral_log_messages: used_logs.len(),
        legacy_online_channel_inferences, receipts: matches, lookup, duplicate_selected_receipts, privacy_warnings,
        assurance: "Signed electoral-log voter/event/election/full ballot hashes match every selected cast vote. Channels match signed values except reported legacy ONLINE inferences. Exact pre-shuffle input multiset and receipt hashes match the pinned selected-set manifest. Full content bytes match its separate digest anchors. The manifest and configuration pins must come from outside the bundle: anchors.json inside it only restates the bundle's own digests, so it cannot protect content fields omitted from the receipt hash against a regenerated bundle. This selected-scope result does not reconcile every historical CastVote log to Hasura or detect accepted casts missing from both supplied history and tally inputs. ImmuDB append-only history, snapshot/selection provenance and mixnet/counting proofs must be verified separately." })
}

pub fn decode_message(value: &str) -> Result<Vec<u8>> {
    STANDARD_NO_PAD
        .decode(value)
        .context("invalid unpadded base64 message")
}
