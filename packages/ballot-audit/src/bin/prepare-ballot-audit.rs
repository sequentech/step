// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{bail, ensure, Context, Result};
use b3::messages::{artifact::Configuration, message::Message, statement::Statement};
use ballot_audit::{
    audit, decode_message, digest, incomplete, invalid_input, validate_sha512_pin, AuditReport,
    CastVote, ExitStatus, Manifest, MultiplicityPolicy, Scope, SelectedVote, SelectionReason,
};
use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine};
use chrono::{DateTime, FixedOffset};
use clap::Parser;
use electoral_log::messages::{
    message::Message as ElectoralMessage,
    statement::{StatementBody, StatementType},
};
use sequent_core::{
    ballot::{HashableBallot, VotingStatusChannel},
    encrypt::{hash_ballot_sha512, hash_multi_ballot_sha512},
    multi_ballot::HashableMultiBallot,
    types::{
        hasura::core::{TallySessionContest, TallySessionContestAnnotations},
        keycloak::VOTE_WEIGHT_BATCHES,
        participation::{ParticipationChannel, VotesByChannel},
    },
};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use strand::{
    backend::ristretto::RistrettoCtx,
    serialization::{StrandDeserialize, StrandSerialize},
    signature::StrandSignaturePk,
};
use uuid::Uuid;

/// Build a verified local bundle from complete, independently pinned exports.
/// This command never connects to a database, network service or live election.
#[derive(Parser)]
struct Args {
    /// JSON {format_version:1, scope:<Scope>, batches:[...]} from saved tally mapping.
    #[arg(long)]
    scope: PathBuf,
    /// Independently authenticated SHA-512 of the exact saved scope file.
    #[arg(long)]
    scope_sha512: String,
    /// Complete scoped Hasura cast_vote history (or full table), including all statuses/revotes.
    #[arg(long)]
    cast_votes: PathBuf,
    #[arg(long)]
    cast_votes_sha512: String,
    /// Frozen JSON {format_version:1, scope_sha512, voters:[{voter_id,multiplicity}]}.
    #[arg(long)]
    eligible_voters: PathBuf,
    #[arg(long)]
    eligible_voters_sha512: String,
    /// Full board export: JSON array of unpadded-base64 B3 Message bytes.
    #[arg(long)]
    board_messages: PathBuf,
    #[arg(long)]
    board_messages_sha512: String,
    /// Full electoral-log export: JSON array of unpadded-base64 signed Message bytes.
    #[arg(long)]
    electoral_log: PathBuf,
    #[arg(long)]
    electoral_log_sha512: String,
    /// Exporter's saved-mapping.json, to cross-check the tally annotations Windmill saved.
    #[arg(long, requires = "saved_mapping_sha512")]
    saved_mapping: Option<PathBuf>,
    #[arg(long, requires = "saved_mapping")]
    saved_mapping_sha512: Option<String>,
    /// New local directory. Existing paths are never overwritten.
    #[arg(long)]
    output: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SavedScope {
    format_version: u32,
    scope: Scope,
    batches: Vec<u32>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum CastVoteStatus {
    Valid,
    Discarded,
    InProgress,
}

fn required_nullable<'de, D, T>(deserializer: D) -> std::result::Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(deserializer)
}

// Full table exports can contain unrelated columns. Required nullable columns
// must nevertheless be present so an incomplete projection is not accepted.
#[derive(Clone, Debug, Deserialize)]
struct RawCastVote {
    id: String,
    tenant_id: String,
    election_event_id: String,
    #[serde(deserialize_with = "required_nullable")]
    election_id: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    area_id: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    created_at: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    voter_id_string: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    content: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    ballot_id: Option<String>,
    status: CastVoteStatus,
    annotations: Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EligibleVoter {
    voter_id: String,
    multiplicity: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FrozenEligibility {
    format_version: u32,
    scope_sha512: String,
    voters: Vec<EligibleVoter>,
}

#[derive(Debug)]
struct SelectedRaw<'a> {
    row: &'a RawCastVote,
    multiplicity: u64,
}

fn select_rows<'a>(
    scope: &Scope,
    scope_sha512: &str,
    rows: &'a [RawCastVote],
    eligibility: &FrozenEligibility,
) -> Result<Vec<SelectedRaw<'a>>> {
    if eligibility.format_version != 1 {
        return Err(invalid_input("unsupported frozen eligibility version"));
    }
    ensure!(
        eligibility.scope_sha512 == scope_sha512,
        "frozen eligibility scope digest mismatch"
    );
    let mut eligible = BTreeMap::new();
    for voter in &eligibility.voters {
        ensure!(
            !voter.voter_id.is_empty(),
            "empty frozen eligibility voter ID"
        );
        ensure!(
            voter.multiplicity > 0,
            "zero frozen eligibility multiplicity"
        );
        match scope.policy {
            MultiplicityPolicy::Unweighted => ensure!(
                voter.multiplicity == 1,
                "unweighted frozen multiplicity must be one"
            ),
            MultiplicityPolicy::Weighted => ensure!(
                voter.multiplicity <= sequent_core::types::keycloak::MAX_VOTE_WEIGHT,
                "frozen weight exceeds supported limit"
            ),
            MultiplicityPolicy::Delegated => {}
        }
        ensure!(
            eligible
                .insert(voter.voter_id.as_str(), voter.multiplicity)
                .is_none(),
            "duplicate frozen eligibility voter ID"
        );
    }
    latest_scoped_rows(scope, rows)?
        .into_iter()
        .filter_map(|(voter, row)| {
            if row.status != CastVoteStatus::Valid {
                return None;
            }
            eligible.get(voter).map(|multiplicity| {
                ensure!(
                    row.content
                        .as_ref()
                        .is_some_and(|content| !content.is_empty()),
                    "selected cast vote lacks content"
                );
                ensure!(
                    row.ballot_id.as_ref().is_some_and(|id| !id.is_empty()),
                    "selected cast vote lacks ballot receipt"
                );
                Ok(SelectedRaw {
                    row,
                    multiplicity: *multiplicity,
                })
            })
        })
        .collect()
}

fn in_scope(scope: &Scope, row: &RawCastVote) -> bool {
    row.tenant_id == scope.tenant_id
        && row.election_event_id == scope.election_event_id
        && row.election_id.as_deref() == Some(scope.election_id.as_str())
        && row.area_id.as_deref() == Some(scope.area_id.as_str())
}

// Mirror the two DISTINCT ON streams: prefer any valid row, otherwise keep
// the latest discarded row for the downstream exclusion audit.
fn latest_scoped_rows<'a>(
    scope: &Scope,
    rows: &'a [RawCastVote],
) -> Result<BTreeMap<&'a str, &'a RawCastVote>> {
    type OrderKey = (bool, Option<DateTime<FixedOffset>>, Uuid);
    let mut latest: BTreeMap<&str, (OrderKey, &RawCastVote)> = BTreeMap::new();
    let mut row_ids = BTreeSet::new();
    for row in rows {
        let id = Uuid::parse_str(&row.id).map_err(|_| invalid_input("invalid cast_vote UUID"))?;
        ensure!(row_ids.insert(id), "duplicate cast_vote row ID");
        if !in_scope(scope, row) {
            continue;
        }
        if row.status == CastVoteStatus::InProgress {
            return Err(incomplete(
                "scoped in-progress cast vote blocks preparation",
            ));
        }
        let voter = row
            .voter_id_string
            .as_deref()
            .filter(|id| !id.is_empty())
            .context("scoped cast vote lacks voter ID")?;
        let created_at = row
            .created_at
            .as_deref()
            .map(DateTime::parse_from_rfc3339)
            .transpose()
            .map_err(|_| invalid_input("invalid scoped cast vote timestamp; expected RFC3339"))?;
        let key = (row.status == CastVoteStatus::Valid, created_at, id);
        if latest.get(voter).is_none_or(|(old_key, _)| key > *old_key) {
            latest.insert(voter, (key, row));
        }
    }
    Ok(latest
        .into_iter()
        .map(|(voter, (_, row))| (voter, row))
        .collect())
}

#[derive(Default, Debug, Serialize)]
struct ReasonCount {
    rows: usize,
    resolved_multiplicity_sum: u64,
    rows_without_resolved_multiplicity: usize,
    stage_cast_count: u64,
    stage_channel_participation_count: u64,
}

#[derive(Debug, Serialize)]
struct StageExclusions {
    discarded: u64,
    no_eligible_voter: u64,
    total: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum ReceiptVerification {
    SelectedAuditVerified,
    NotChecked,
}

#[derive(Debug, Serialize)]
struct RowDecision {
    cast_vote_id: String,
    ballot_id: Option<String>,
    reason: SelectionReason,
    latest_scoped_cast_vote_id: Option<String>,
    resolved_multiplicity: Option<u64>,
    stage_cast_count: u64,
    stage_channel_participation_count: u64,
    receipt_verification: ReceiptVerification,
}

#[derive(Debug, Serialize)]
struct SelectionAccounting {
    exported_rows: usize,
    rows_by_reason: BTreeMap<SelectionReason, ReasonCount>,
    frozen_eligible_voters: usize,
    frozen_eligible_voters_without_valid_vote: usize,
    frozen_eligible_multiplicity_without_valid_vote: u64,
    selected_input_positions: u64,
    stage_exclusions: StageExclusions,
    stage_cast_count: u64,
    /// Absent when a counted row stores a non-string voting channel, whose
    /// PostgreSQL text form is not reproduced here.
    stage_votes_by_channel: Option<VotesByChannel>,
}

// Mirrors COALESCE(annotations->>'voting_channel', 'ONLINE') in Windmill's
// area_ballots_query.
fn stage_channel(row: &RawCastVote) -> Option<String> {
    match row.annotations.get("voting_channel") {
        None | Some(Value::Null) => Some(VotingStatusChannel::ONLINE.to_string()),
        Some(Value::String(channel)) => Some(channel.clone()),
        Some(_) => None,
    }
}

fn account_selection(
    scope: &Scope,
    rows: &[RawCastVote],
    eligibility: &FrozenEligibility,
    selected: &[SelectedRaw<'_>],
) -> Result<(SelectionAccounting, Vec<RowDecision>)> {
    let eligible: BTreeMap<_, _> = eligibility
        .voters
        .iter()
        .map(|v| (v.voter_id.as_str(), v.multiplicity))
        .collect();
    let latest = latest_scoped_rows(scope, rows)?;
    let latest_ids: BTreeSet<_> = latest.values().map(|row| row.id.as_str()).collect();
    let selected_ids: BTreeSet<_> = selected.iter().map(|v| v.row.id.as_str()).collect();
    let selected_voters: BTreeSet<_> = selected
        .iter()
        .filter_map(|v| v.row.voter_id_string.as_deref())
        .collect();
    let mut rows_by_reason: BTreeMap<_, ReasonCount> = [
        SelectionReason::OutsideScope,
        SelectionReason::Discarded,
        SelectionReason::NoEligibleVoter,
        SelectionReason::SupersededRevote,
        SelectionReason::Selected,
    ]
    .into_iter()
    .map(|reason| (reason, ReasonCount::default()))
    .collect();
    let mut decisions = Vec::with_capacity(rows.len());
    let mut stage_votes_by_channel = Some(VotesByChannel::new());
    for row in rows {
        let outside = !in_scope(scope, row);
        let multiplicity = if outside {
            None
        } else {
            row.voter_id_string
                .as_deref()
                .and_then(|id| eligible.get(id).copied())
        };
        let reason = if outside {
            SelectionReason::OutsideScope
        } else if !latest_ids.contains(row.id.as_str()) {
            SelectionReason::SupersededRevote
        } else if row.status == CastVoteStatus::Discarded {
            SelectionReason::Discarded
        } else if multiplicity.is_none() {
            SelectionReason::NoEligibleVoter
        } else if selected_ids.contains(row.id.as_str()) {
            SelectionReason::Selected
        } else {
            bail!("selection accounting does not match selected scope");
        };
        let count = rows_by_reason.entry(reason).or_default();
        count.rows += 1;
        let (cast_count, channel_count) = match reason {
            SelectionReason::Discarded | SelectionReason::NoEligibleVoter => (1, 1),
            SelectionReason::Selected => {
                let multiplicity =
                    multiplicity.context("selected row lacks resolved multiplicity")?;
                (
                    if matches!(scope.policy, MultiplicityPolicy::Delegated) {
                        multiplicity
                    } else {
                        1
                    },
                    multiplicity,
                )
            }
            SelectionReason::OutsideScope | SelectionReason::SupersededRevote => (0, 0),
        };
        if channel_count > 0 {
            stage_votes_by_channel = match (stage_votes_by_channel, stage_channel(row)) {
                (Some(mut counts), Some(channel)) => {
                    let total = counts
                        .entry(ParticipationChannel::from(channel))
                        .or_default();
                    *total = total
                        .checked_add(channel_count)
                        .context("stage channel count overflow")?;
                    Some(counts)
                }
                _ => None,
            };
        }
        decisions.push(RowDecision {
            cast_vote_id: row.id.clone(),
            ballot_id: row.ballot_id.clone(),
            reason,
            latest_scoped_cast_vote_id: if outside {
                None
            } else {
                row.voter_id_string
                    .as_deref()
                    .and_then(|voter| latest.get(voter))
                    .map(|latest| latest.id.clone())
            },
            resolved_multiplicity: multiplicity,
            stage_cast_count: cast_count,
            stage_channel_participation_count: channel_count,
            receipt_verification: if reason == SelectionReason::Selected {
                ReceiptVerification::SelectedAuditVerified
            } else {
                ReceiptVerification::NotChecked
            },
        });
        count.stage_cast_count = count
            .stage_cast_count
            .checked_add(cast_count)
            .context("stage cast count overflow")?;
        count.stage_channel_participation_count = count
            .stage_channel_participation_count
            .checked_add(channel_count)
            .context("stage channel count overflow")?;
        if let Some(multiplicity) = multiplicity {
            count.resolved_multiplicity_sum = count
                .resolved_multiplicity_sum
                .checked_add(multiplicity)
                .context("selection accounting multiplicity overflow")?;
        } else {
            count.rows_without_resolved_multiplicity += 1;
        }
    }
    let mut without_valid = 0;
    let mut without_valid_multiplicity = 0u64;
    for voter in &eligibility.voters {
        if !selected_voters.contains(voter.voter_id.as_str()) {
            without_valid += 1;
            without_valid_multiplicity = without_valid_multiplicity
                .checked_add(voter.multiplicity)
                .context("unselected eligibility multiplicity overflow")?;
        }
    }
    let selected_input_positions = selected.iter().try_fold(0u64, |total, selected| {
        let count = match scope.policy {
            MultiplicityPolicy::Unweighted => 1,
            MultiplicityPolicy::Delegated => selected.multiplicity,
            MultiplicityPolicy::Weighted => u64::from(selected.multiplicity.count_ones()),
        };
        total
            .checked_add(count)
            .context("selected input position count overflow")
    })?;
    let stage_cast_count = rows_by_reason.values().try_fold(0u64, |total, count| {
        total
            .checked_add(count.stage_cast_count)
            .context("stage cast count overflow")
    })?;
    let discarded = rows_by_reason[&SelectionReason::Discarded].stage_cast_count;
    let no_eligible_voter = rows_by_reason[&SelectionReason::NoEligibleVoter].stage_cast_count;
    let stage_exclusions = StageExclusions {
        discarded,
        no_eligible_voter,
        total: discarded
            .checked_add(no_eligible_voter)
            .context("stage exclusion count overflow")?,
    };
    Ok((
        SelectionAccounting {
            exported_rows: rows.len(),
            rows_by_reason,
            frozen_eligible_voters: eligible.len(),
            frozen_eligible_voters_without_valid_vote: without_valid,
            frozen_eligible_multiplicity_without_valid_vote: without_valid_multiplicity,
            selected_input_positions,
            stage_exclusions,
            stage_cast_count,
            stage_votes_by_channel,
        },
        decisions,
    ))
}

fn read_pinned(path: &Path, pin: &str, label: &str) -> Result<Vec<u8>> {
    validate_sha512_pin(pin, label)?;
    let bytes = fs::read(path).map_err(|_| invalid_input(format!("cannot read {label} source")))?;
    ensure!(digest(&bytes)? == pin, "{label} source digest mismatch");
    Ok(bytes)
}

fn decode_export(bytes: &[u8], label: &str) -> Result<Vec<Vec<u8>>> {
    let encoded: Vec<String> = serde_json::from_slice(bytes)
        .map_err(|_| invalid_input(format!("invalid {label} JSON export")))?;
    encoded
        .iter()
        .map(|value| {
            decode_message(value)
                .map_err(|_| invalid_input(format!("invalid {label} message encoding")))
        })
        .collect()
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum PublicKeyCheck {
    /// The board export carries no PublicKey statement to compare with.
    NotOnBoard,
    Matched {
        signed_statements: usize,
    },
}

struct BoardInputs {
    configuration: Vec<u8>,
    ballots: Vec<Vec<u8>>,
    public_key: PublicKeyCheck,
}

fn board_inputs(saved: &SavedScope, messages: &[Vec<u8>]) -> Result<BoardInputs> {
    if saved.format_version != 1 {
        return Err(invalid_input("unsupported saved scope version"));
    }
    let batches: BTreeSet<_> = saved.batches.iter().copied().collect();
    if batches.is_empty() || batches.len() != saved.batches.len() {
        return Err(invalid_input("empty or duplicate saved batch scope"));
    }
    let mut configuration = None;
    let mut public_keys = Vec::new();
    let mut selected = Vec::new();
    for bytes in messages {
        let message = Message::strand_deserialize(bytes)
            .map_err(|_| invalid_input("invalid full board message"))?;
        match &message.statement {
            Statement::Configuration(_, configuration_hash) => {
                ensure!(configuration.is_none(), "ambiguous board configuration");
                let artifact = message
                    .artifact
                    .as_ref()
                    .context("configuration artifact missing")?;
                ensure!(
                    strand::hash::hash_to_array(artifact)? == configuration_hash.0,
                    "configuration artifact hash mismatch"
                );
                let parsed = Configuration::<RistrettoCtx>::strand_deserialize(artifact)
                    .context("invalid board configuration")?;
                ensure!(parsed.is_valid(), "invalid board configuration");
                message
                    .verify(&parsed)
                    .context("configuration signature verification failed")?;
                configuration = Some((artifact.clone(), parsed));
            }
            Statement::PublicKey(_, _, public_key_hash, _, _)
            | Statement::PublicKeySigned(_, _, public_key_hash, _, _) => {
                if let Some(artifact) = &message.artifact {
                    ensure!(
                        strand::hash::hash_to_array(artifact)? == public_key_hash.0,
                        "board public key artifact hash mismatch"
                    );
                }
                public_keys.push((hex::encode(public_key_hash.0), message));
            }
            Statement::Ballots(_, _, batch, _, _, _)
                if u32::try_from(*batch)
                    .ok()
                    .is_some_and(|batch| batches.contains(&batch)) =>
            {
                selected.push(bytes.clone());
            }
            _ => {}
        }
    }
    let (configuration, parsed) =
        configuration.ok_or_else(|| incomplete("full board export lacks configuration"))?;
    for (public_key_hash, message) in &public_keys {
        message
            .verify(&parsed)
            .context("board public key signature verification failed")?;
        ensure!(
            *public_key_hash == saved.scope.public_key_sha512,
            "pinned public key hash differs from the board PublicKey statement"
        );
    }
    Ok(BoardInputs {
        configuration,
        ballots: selected,
        public_key: if public_keys.is_empty() {
            PublicKeyCheck::NotOnBoard
        } else {
            PublicKeyCheck::Matched {
                signed_statements: public_keys.len(),
            }
        },
    })
}

struct LogEvidence<'a> {
    bytes: &'a [u8],
    channel: String,
}

type LogKey = (strand::hash::Hash, strand::hash::Hash);

fn index_logs<'a>(
    scope: &Scope,
    messages: &'a [Vec<u8>],
) -> Result<BTreeMap<LogKey, Vec<LogEvidence<'a>>>> {
    let system_pk = StrandSignaturePk::from_der_b64_string(&scope.electoral_log_system_public_key)
        .map_err(|_| invalid_input("invalid pinned electoral-log system key"))?;
    let mut indexed: BTreeMap<LogKey, Vec<LogEvidence<'a>>> = BTreeMap::new();
    for bytes in messages {
        let message = ElectoralMessage::strand_deserialize(bytes)
            .map_err(|_| invalid_input("invalid full electoral-log message"))?;
        if message.statement.head.event.0 != scope.election_event_id
            || !matches!(message.statement.head.kind, StatementType::CastVote)
        {
            continue;
        }
        let (election, pseudonym, ballot_hash, channel) = match &message.statement.body {
            StatementBody::CastVote(e, p, h, _, _) => {
                (e, p, h, VotingStatusChannel::ONLINE.to_string())
            }
            StatementBody::CastVoteWithChannel(e, p, h, _, _, c) => (e, p, h, c.0.clone()),
            _ => bail!("electoral-log cast-vote kind/body mismatch"),
        };
        if election.0.as_deref() != Some(scope.election_id.as_str()) {
            continue;
        }
        message
            .verify(&system_pk)
            .context("scoped electoral-log signature verification failed")?;
        indexed
            .entry((
                pseudonym.0.clone().into_inner(),
                ballot_hash.0.clone().into_inner(),
            ))
            .or_default()
            .push(LogEvidence { bytes, channel });
    }
    Ok(indexed)
}

fn voting_channel(row: &RawCastVote) -> Result<String> {
    if !(row.annotations.is_null() || row.annotations.is_object()) {
        return Err(invalid_input("invalid cast vote annotations object"));
    }
    match row
        .annotations
        .get("voting_channel")
        .filter(|value| !value.is_null())
    {
        Some(value) => Ok(serde_json::from_value::<VotingStatusChannel>(value.clone())
            .map_err(|_| invalid_input("invalid cast vote voting channel"))?
            .to_string()),
        None => Ok(VotingStatusChannel::ONLINE.to_string()),
    }
}

#[derive(Debug, Deserialize)]
struct SavedMapping {
    tally_session_contest: TallySessionContest,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum AnnotationField {
    Matched,
    NotRecorded,
    NotReproducible,
}

/// Saved eligible voter, ballots-without-voter and cast counts are always
/// compared once annotations are present. The auditable ballots document is
/// not: its archived JSONL is not authenticated by this reconstruction.
#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum TallyAnnotationsCheck {
    NotSupplied,
    AnnotationsAbsent,
    Checked {
        votes_by_channel: AnnotationField,
        weight_bit_mask: AnnotationField,
    },
}

fn check_tally_annotations(
    saved: &SavedScope,
    mapping: &SavedMapping,
    eligibility: &FrozenEligibility,
    accounting: &SelectionAccounting,
) -> Result<TallyAnnotationsCheck> {
    let scope = &saved.scope;
    let contest = &mapping.tally_session_contest;
    ensure!(
        contest.id == scope.tally_session_contest_id
            && contest.tenant_id == scope.tenant_id
            && contest.election_event_id == scope.election_event_id
            && contest.election_id == scope.election_id
            && contest.area_id == scope.area_id
            && contest.tally_session_id == scope.tally_session_id
            && contest.contest_id == scope.contest_id,
        "saved tally session contest differs from pinned scope"
    );
    ensure!(
        u32::try_from(contest.session_id).ok() == Some(scope.base_batch),
        "saved tally session batch differs from pinned base batch"
    );
    let Some(annotations) = contest.annotations.clone().filter(|value| !value.is_null()) else {
        return Ok(TallyAnnotationsCheck::AnnotationsAbsent);
    };
    let annotations: TallySessionContestAnnotations = serde_json::from_value(annotations)
        .map_err(|_| invalid_input("invalid saved tally session contest annotations"))?;
    ensure!(
        u64::try_from(eligibility.voters.len()).ok() == Some(annotations.elegible_voters),
        "saved eligible voter count differs from frozen eligibility"
    );
    ensure!(
        annotations.ballots_without_voter == accounting.stage_exclusions.total,
        "saved ballots without voter differ from reconstructed stage exclusions"
    );
    ensure!(
        annotations.casted_ballots == accounting.stage_cast_count,
        "saved cast ballot count differs from reconstructed selection"
    );
    let votes_by_channel = match (
        &annotations.votes_by_channel,
        &accounting.stage_votes_by_channel,
    ) {
        (None, _) => AnnotationField::NotRecorded,
        (Some(_), None) => AnnotationField::NotReproducible,
        (Some(saved_counts), Some(counts)) => {
            ensure!(
                saved_counts == counts,
                "saved votes by channel differ from reconstructed selection"
            );
            AnnotationField::Matched
        }
    };
    let offsets = saved
        .batches
        .iter()
        .map(|batch| {
            batch
                .checked_sub(scope.base_batch)
                .context("saved batch below base batch")
        })
        .collect::<Result<BTreeSet<_>>>()?;
    let weight_bit_mask = match (annotations.weight_bit_mask, scope.policy) {
        (Some(mask), _) => {
            let masked: BTreeSet<_> = (0..VOTE_WEIGHT_BATCHES)
                .filter(|bit| mask & (1u32 << bit) != 0)
                .collect();
            ensure!(
                mask >> VOTE_WEIGHT_BATCHES == 0 && masked == offsets,
                "saved weight bit mask differs from the batch scope"
            );
            AnnotationField::Matched
        }
        (None, MultiplicityPolicy::Weighted) => AnnotationField::NotRecorded,
        (None, MultiplicityPolicy::Unweighted | MultiplicityPolicy::Delegated) => {
            ensure!(
                offsets == BTreeSet::from([0]),
                "unweighted saved tally contest spans several batches"
            );
            AnnotationField::Matched
        }
    };
    Ok(TallyAnnotationsCheck::Checked {
        votes_by_channel,
        weight_bit_mask,
    })
}

struct PreparedBundle {
    manifest: Vec<u8>,
    configuration: Vec<u8>,
    rows: Vec<CastVote>,
    messages: Vec<Vec<u8>>,
    logs: Vec<Vec<u8>>,
    report: AuditReport,
    selection_accounting: SelectionAccounting,
    row_decisions: Vec<RowDecision>,
    public_key: PublicKeyCheck,
    collapsed_identical_electoral_log_records: usize,
    tally_annotations: TallyAnnotationsCheck,
}

fn prepare_bundle(
    saved: SavedScope,
    scope_sha512: &str,
    raw_rows: &[RawCastVote],
    eligibility: &FrozenEligibility,
    board_messages: &[Vec<u8>],
    log_messages: &[Vec<u8>],
    mapping: Option<&SavedMapping>,
) -> Result<PreparedBundle> {
    let selected = select_rows(&saved.scope, scope_sha512, raw_rows, eligibility)?;
    let (selection_accounting, row_decisions) =
        account_selection(&saved.scope, raw_rows, eligibility, &selected)?;
    let tally_annotations = match mapping {
        Some(mapping) => {
            check_tally_annotations(&saved, mapping, eligibility, &selection_accounting)?
        }
        None => TallyAnnotationsCheck::NotSupplied,
    };
    let board = board_inputs(&saved, board_messages)?;
    let indexed_logs = index_logs(&saved.scope, log_messages)?;
    let mut manifest = Manifest {
        format_version: 1,
        scope: saved.scope,
        batches: saved.batches,
        selected: Vec::new(),
    };
    let mut rows = Vec::new();
    let mut logs = Vec::new();
    let mut collapsed_identical_electoral_log_records = 0;
    for selected in selected {
        let raw = selected.row;
        let voter_id = raw
            .voter_id_string
            .as_ref()
            .context("selected cast vote lacks voter ID")?;
        let content = raw
            .content
            .as_ref()
            .context("selected cast vote lacks content")?;
        let ballot_id = raw
            .ballot_id
            .as_ref()
            .context("selected cast vote lacks ballot receipt")?;
        let full_ballot_hash = if manifest.scope.contest_id.is_some() {
            let ballot: HashableBallot =
                serde_json::from_str(content).context("invalid selected single-contest ballot")?;
            hash_ballot_sha512(&ballot).context("cannot hash selected single-contest ballot")?
        } else {
            let ballot: HashableMultiBallot =
                serde_json::from_str(content).context("invalid selected multi-contest ballot")?;
            hash_multi_ballot_sha512(&ballot)
                .context("cannot hash selected multi-contest ballot")?
        };
        let voter_hash = strand::hash::hash_to_array(&voter_id.strand_serialize()?)?;
        // A lost log write (meta#13796) or Windmill's best-effort post-cast
        // logging leaves an accepted vote without evidence, not a contradiction.
        let matches = indexed_logs
            .get(&(voter_hash, full_ballot_hash))
            .ok_or_else(|| {
                incomplete("selected cast vote has no matching signed electoral-log evidence")
            })?;
        let distinct: BTreeSet<_> = matches.iter().map(|evidence| evidence.bytes).collect();
        if distinct.len() != 1 {
            return Err(incomplete(
                "selected cast vote has ambiguous signed electoral-log evidence",
            ));
        }
        collapsed_identical_electoral_log_records += matches.len() - 1;
        let log = &matches[0];
        let channel = voting_channel(raw)?;
        ensure!(
            channel == log.channel,
            "selected cast vote channel differs from signed electoral log"
        );
        manifest.selected.push(SelectedVote {
            cast_vote_id: raw.id.clone(),
            ballot_id: ballot_id.clone(),
            content_sha512: digest(content.as_bytes())?,
            electoral_log_message_sha512: digest(log.bytes)?,
            multiplicity: selected.multiplicity,
        });
        rows.push(CastVote {
            voter_id: voter_id.clone(),
            voting_channel: channel,
            cast_vote_id: raw.id.clone(),
            ballot_id: ballot_id.clone(),
            content: content.clone(),
        });
        logs.push(log.bytes.to_vec());
    }
    let manifest = serde_json::to_vec_pretty(&manifest)?;
    let report = audit(
        &manifest,
        &digest(&manifest)?,
        &rows,
        &board.configuration,
        &digest(&board.configuration)?,
        &board.ballots,
        None,
        &logs,
    )
    .context("generated bundle failed complete input audit")?;
    ensure!(
        report.duplicate_selected_receipts.is_empty(),
        "{} ballot receipt(s) are shared by several selected cast votes; one encrypted ballot was accepted for more than one voter",
        report.duplicate_selected_receipts.len()
    );
    Ok(PreparedBundle {
        manifest,
        configuration: board.configuration,
        rows,
        messages: board.ballots,
        logs,
        report,
        selection_accounting,
        row_decisions,
        public_key: board.public_key,
        collapsed_identical_electoral_log_records,
        tally_annotations,
    })
}

fn encode_messages(messages: &[Vec<u8>]) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec_pretty(
        &messages
            .iter()
            .map(|bytes| STANDARD_NO_PAD.encode(bytes))
            .collect::<Vec<_>>(),
    )?)
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .context("cannot create bundle file without overwriting")?;
    file.write_all(bytes).context("cannot write bundle file")?;
    file.sync_all().context("cannot flush bundle file")?;
    Ok(())
}

fn write_bundle(args: &Args, bundle: &PreparedBundle) -> Result<()> {
    let anchors = serde_json::json!({
        "manifest_sha512": digest(&bundle.manifest)?,
        "configuration_sha512": digest(&bundle.configuration)?,
    });
    let provenance = serde_json::json!({
        "format_version": 1,
        "source_sha512": {
            "scope": args.scope_sha512,
            "cast_votes": args.cast_votes_sha512,
            "eligible_voters": args.eligible_voters_sha512,
            "board_messages": args.board_messages_sha512,
            "electoral_log": args.electoral_log_sha512,
            "saved_mapping": args.saved_mapping_sha512,
        },
        "anchors": anchors,
        "selection_accounting": bundle.selection_accounting,
        "tally_annotations": bundle.tally_annotations,
        "board_public_key": bundle.public_key,
        "collapsed_identical_electoral_log_records": bundle.collapsed_identical_electoral_log_records,
        "checked_cast_votes": bundle.report.checked_cast_votes,
        "checked_batches": bundle.report.checked_batches,
        "checked_electoral_log_messages": bundle.report.checked_electoral_log_messages,
        "legacy_online_channel_inferences": bundle.report.legacy_online_channel_inferences,
        "limitations": [
            "Source digests were supplied by the operator. This program does not independently authenticate those pins or establish export completeness and snapshot consistency.",
            "Frozen eligibility is an externally authenticated upstream decision, not independently replayed here. It must preserve the complete eligibility projection for this exact scope, including enabled status, area/election authorization, and resolved weights or delegation multiplicities. No live voter state is consulted.",
            "Selection accounting partitions complete exported row history. Superseded rows are not stage exclusions. Only the latest discarded row when no valid exists, or the latest valid row without frozen eligibility, contributes one exclusion and one channel participation count. Included rows contribute resolved multiplicity to channel participation; weighted cast counts remain headcounts. This reconstruction does not authenticate the archived encrypted-ballots.jsonl exclusion artifact.",
            "A missing frozen eligibility entry establishes only no_eligible_voter at this stage, not which upstream enabled, area, authorization, or other eligibility clause failed. Keep the source eligibility evidence to establish that reason.",
            "This newly generated manifest cannot prove historical integrity of ballot metadata omitted from the native receipt hash. Historical full-content integrity requires an independently authenticated contemporaneous source anchor.",
            "anchors.json restates this bundle's own digests. Pass ballot-audit manifest and configuration pins obtained outside the bundle; pins read from it only show the bundle is self-consistent.",
            "Saved tally annotations are compared only when --saved-mapping is supplied: eligible voters with the frozen eligibility size, ballots without voter with stage exclusions, cast ballots and votes by channel with the reconstructed stage counts, and the weight bit mask with the batch scope. Their auditable ballots document is not compared. Rows saved before these annotations existed are reported as not checked.",
            "The saved board name and tally/area/contest/batch mapping require external authentication. B3 signatures alone do not establish this application-level mapping.",
            "ImmuDB append-only history, mixnet shuffle/decryption proofs and final counting are not verified here.",
            "This local bundle contains raw voter identifiers and electoral-log metadata. Keep it private; stdout contains only aggregate counts."
        ]
    });
    let files = [
        ("manifest.json", bundle.manifest.clone()),
        ("configuration.borsh", bundle.configuration.clone()),
        ("cast-votes.json", serde_json::to_vec_pretty(&bundle.rows)?),
        ("messages.json", encode_messages(&bundle.messages)?),
        ("electoral-log.json", encode_messages(&bundle.logs)?),
        ("anchors.json", serde_json::to_vec_pretty(&anchors)?),
        ("provenance.json", serde_json::to_vec_pretty(&provenance)?),
        (
            "selection.json",
            serde_json::to_vec_pretty(&serde_json::json!({
                "format_version": 1,
                "scope_sha512": args.scope_sha512,
                "accounting": bundle.selection_accounting,
                "rows": bundle.row_decisions,
                "limitation": "Reconstructed selection and exclusion references from the pinned cast_vote source and externally authenticated frozen eligibility. Only selected receipts have their hashes and signed electoral-log links verified. Other ballot_id values are stored receipt labels, not verified receipt hashes. The archived stage exclusion JSONL is not authenticated by this reconstruction."
            }))?,
        ),
    ];
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(&args.output).context(
        "cannot create output directory; it must not already exist and its parent must exist",
    )?;
    for (name, bytes) in files {
        write_new(&args.output.join(name), &bytes)?;
    }
    Ok(())
}

fn run(args: Args) -> Result<()> {
    let scope_bytes = read_pinned(&args.scope, &args.scope_sha512, "scope")?;
    let raw_bytes = read_pinned(&args.cast_votes, &args.cast_votes_sha512, "cast votes")?;
    let eligibility_bytes = read_pinned(
        &args.eligible_voters,
        &args.eligible_voters_sha512,
        "frozen eligible voters",
    )?;
    let board_bytes = read_pinned(
        &args.board_messages,
        &args.board_messages_sha512,
        "board messages",
    )?;
    let log_bytes = read_pinned(
        &args.electoral_log,
        &args.electoral_log_sha512,
        "electoral log",
    )?;
    let saved: SavedScope = serde_json::from_slice(&scope_bytes).map_err(|_| {
        invalid_input("invalid saved scope; expected format_version, scope and batches")
    })?;
    let rows: Vec<RawCastVote> = serde_json::from_slice(&raw_bytes)
        .map_err(|_| invalid_input("invalid full cast_vote table export"))?;
    let eligibility: FrozenEligibility =
        serde_json::from_slice(&eligibility_bytes).map_err(|_| {
            invalid_input("invalid frozen eligibility; a scope-bound envelope is required")
        })?;
    let board_messages = decode_export(&board_bytes, "board")?;
    let log_messages = decode_export(&log_bytes, "electoral log")?;
    let mapping: Option<SavedMapping> = match (&args.saved_mapping, &args.saved_mapping_sha512) {
        (Some(path), Some(pin)) => Some(
            serde_json::from_slice(&read_pinned(path, pin, "saved mapping")?)
                .map_err(|_| invalid_input("invalid saved mapping"))?,
        ),
        _ => None,
    };
    let bundle = prepare_bundle(
        saved,
        &args.scope_sha512,
        &rows,
        &eligibility,
        &board_messages,
        &log_messages,
        mapping.as_ref(),
    )?;
    write_bundle(&args, &bundle)?;
    println!("Prepared verified local bundle: {} selected cast votes, {} batches, {} signed electoral-log messages. Independently authenticate the source pins and read provenance.json before relying on this result.", bundle.report.checked_cast_votes, bundle.report.checked_batches, bundle.report.checked_electoral_log_messages);
    Ok(())
}

fn main() {
    if let Err(error) = run(Args::parse()) {
        // Parser/library source errors can contain ballot or voter data.
        eprintln!("prepare-ballot-audit: {error}");
        std::process::exit(ExitStatus::of(&error).code());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope() -> Scope {
        Scope {
            tenant_id: "tenant".into(),
            election_event_id: "event".into(),
            election_id: "election".into(),
            area_id: "area".into(),
            tally_session_id: "tally".into(),
            tally_session_contest_id: "contest-mapping".into(),
            board_name: "board".into(),
            public_key_sha512: "00".repeat(64),
            electoral_log_system_public_key: String::new(),
            contest_id: Some("contest".into()),
            base_batch: 10,
            policy: MultiplicityPolicy::Unweighted,
        }
    }

    fn row(id: u64, voter: &str, status: CastVoteStatus, time: Option<&str>) -> RawCastVote {
        RawCastVote {
            id: format!("00000000-0000-4000-8000-{id:012x}"),
            tenant_id: "tenant".into(),
            election_event_id: "event".into(),
            election_id: Some("election".into()),
            area_id: Some("area".into()),
            created_at: time.map(str::to_owned),
            voter_id_string: Some(voter.into()),
            content: Some(" {\"unchanged\": true} \n".into()),
            ballot_id: Some("receipt".into()),
            status,
            annotations: serde_json::json!({"voting_channel":"ONLINE"}),
        }
    }

    fn eligible() -> FrozenEligibility {
        FrozenEligibility {
            format_version: 1,
            scope_sha512: "scope-pin".into(),
            voters: vec![EligibleVoter {
                voter_id: "voter".into(),
                multiplicity: 1,
            }],
        }
    }

    fn select(rows: &[RawCastVote]) -> Result<Vec<SelectedRaw<'_>>> {
        select_rows(&scope(), "scope-pin", rows, &eligible())
    }

    #[test]
    fn latest_valid_precedes_newer_discarded() {
        let rows = vec![
            row(
                1,
                "voter",
                CastVoteStatus::Valid,
                Some("2026-01-01T00:00:00Z"),
            ),
            row(
                2,
                "voter",
                CastVoteStatus::Valid,
                Some("2026-01-02T00:00:00Z"),
            ),
            row(
                3,
                "voter",
                CastVoteStatus::Discarded,
                Some("2026-01-03T00:00:00Z"),
            ),
        ];
        let selected = select(&rows).unwrap();
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].row.id, rows[1].id);
        assert_eq!(selected[0].row.content, rows[1].content);
    }

    #[test]
    fn timestamp_nulls_last_then_uuid_desc() {
        let rows = vec![
            row(3, "voter", CastVoteStatus::Valid, None),
            row(
                1,
                "voter",
                CastVoteStatus::Valid,
                Some("2026-01-01T00:00:00Z"),
            ),
            row(
                2,
                "voter",
                CastVoteStatus::Valid,
                Some("2026-01-01T01:00:00+01:00"),
            ),
        ];
        assert_eq!(select(&rows).unwrap()[0].row.id, rows[2].id);
        assert_eq!(select(&rows[..1]).unwrap()[0].row.id, rows[0].id);
    }

    #[test]
    fn discarded_only_and_absent_eligibility_are_excluded() {
        let rows = vec![
            row(1, "voter", CastVoteStatus::Discarded, None),
            row(2, "ineligible", CastVoteStatus::Valid, None),
        ];
        assert!(select(&rows).unwrap().is_empty());
    }

    #[test]
    fn each_scope_dimension_filters_before_selection() {
        let mut rows = vec![row(1, "voter", CastVoteStatus::Valid, None)];
        for dimension in 0..4 {
            let mut other = row(dimension + 2, "voter", CastVoteStatus::InProgress, None);
            match dimension {
                0 => other.tenant_id = "other".into(),
                1 => other.election_event_id = "other".into(),
                2 => other.election_id = Some("other".into()),
                _ => other.area_id = Some("other".into()),
            }
            rows.push(other);
        }
        assert_eq!(select(&rows).unwrap().len(), 1);
    }

    #[test]
    fn in_progress_blocks_even_ineligible_voter() {
        let rows = vec![row(1, "ineligible", CastVoteStatus::InProgress, None)];
        assert!(select(&rows)
            .unwrap_err()
            .to_string()
            .contains("in-progress"));
    }

    #[test]
    fn duplicate_rows_and_eligibility_fail() {
        let first = row(1, "voter", CastVoteStatus::Valid, None);
        assert!(select(&[first.clone(), first]).is_err());
        let mut eligibility = eligible();
        eligibility.voters.push(EligibleVoter {
            voter_id: "voter".into(),
            multiplicity: 1,
        });
        assert!(select_rows(&scope(), "scope-pin", &[], &eligibility).is_err());
    }

    #[test]
    fn frozen_eligibility_must_match_scope() {
        assert!(select_rows(&scope(), "other-pin", &[], &eligible()).is_err());
        assert!(serde_json::from_str::<FrozenEligibility>(
            r#"[{"voter_id":"voter","multiplicity":1}]"#
        )
        .is_err());
        assert!(
            serde_json::from_str::<FrozenEligibility>(r#"{"format_version":1,"voters":[]}"#)
                .is_err()
        );
    }

    #[test]
    fn missing_selected_payload_fails() {
        let mut selected = row(1, "voter", CastVoteStatus::Valid, None);
        selected.content = None;
        assert!(select(&[selected]).is_err());
    }

    #[test]
    fn missing_nullable_export_column_is_not_null() {
        let source = serde_json::json!({
            "id":"00000000-0000-4000-8000-000000000001", "tenant_id":"tenant",
            "election_event_id":"event", "election_id":"election", "area_id":"area",
            "voter_id_string":"voter", "content":"{}", "ballot_id":"receipt",
            "status":"valid", "annotations":null
        });
        assert!(serde_json::from_value::<RawCastVote>(source).is_err());
    }
    #[test]
    fn stage_exclusions_do_not_count_old_revotes_or_apply_eligible_multiplicity() {
        let mut scoped = scope();
        scoped.policy = MultiplicityPolicy::Delegated;
        let rows = vec![
            row(1, "voter", CastVoteStatus::Valid, None),
            row(2, "voter", CastVoteStatus::Valid, None),
            row(3, "voter", CastVoteStatus::Discarded, None),
            row(4, "discarded-voter", CastVoteStatus::Discarded, None),
            row(5, "discarded-voter", CastVoteStatus::Discarded, None),
            row(6, "ineligible", CastVoteStatus::Valid, None),
            row(7, "ineligible", CastVoteStatus::Valid, None),
        ];
        let eligibility = FrozenEligibility {
            format_version: 1,
            scope_sha512: "scope-pin".into(),
            voters: vec![
                EligibleVoter {
                    voter_id: "voter".into(),
                    multiplicity: 3,
                },
                EligibleVoter {
                    voter_id: "discarded-voter".into(),
                    multiplicity: 5,
                },
                EligibleVoter {
                    voter_id: "not-voted".into(),
                    multiplicity: 2,
                },
            ],
        };
        let selected = select_rows(&scoped, "scope-pin", &rows, &eligibility).unwrap();
        let (accounting, decisions) =
            account_selection(&scoped, &rows, &eligibility, &selected).unwrap();
        assert_eq!(accounting.stage_exclusions.total, 2);
        assert_eq!(accounting.stage_exclusions.discarded, 1);
        assert_eq!(accounting.stage_exclusions.no_eligible_voter, 1);
        assert_eq!(
            accounting.rows_by_reason[&SelectionReason::SupersededRevote].rows,
            4
        );
        assert_eq!(
            accounting.rows_by_reason[&SelectionReason::Selected].stage_cast_count,
            3
        );
        assert_eq!(
            accounting.rows_by_reason[&SelectionReason::Discarded].resolved_multiplicity_sum,
            5
        );
        assert_eq!(
            accounting.rows_by_reason[&SelectionReason::Discarded].stage_cast_count,
            1
        );
        assert_eq!(accounting.frozen_eligible_voters_without_valid_vote, 2);
        assert_eq!(
            accounting.frozen_eligible_multiplicity_without_valid_vote,
            7
        );
        let discarded = decisions
            .iter()
            .find(|decision| decision.cast_vote_id == rows[4].id)
            .unwrap();
        assert_eq!(discarded.reason, SelectionReason::Discarded);
        let superseded = decisions
            .iter()
            .find(|decision| decision.cast_vote_id == rows[2].id)
            .unwrap();
        assert_eq!(superseded.reason, SelectionReason::SupersededRevote);
        assert_eq!(
            superseded.latest_scoped_cast_vote_id.as_ref(),
            Some(&rows[1].id)
        );
    }

    #[test]
    fn weighted_input_positions_and_participation_are_distinct() {
        let mut scoped = scope();
        scoped.policy = MultiplicityPolicy::Weighted;
        let mut eligibility = eligible();
        eligibility.voters[0].multiplicity = 5;
        let rows = vec![row(1, "voter", CastVoteStatus::Valid, None)];
        let selected = select_rows(&scoped, "scope-pin", &rows, &eligibility).unwrap();
        let (accounting, _) = account_selection(&scoped, &rows, &eligibility, &selected).unwrap();
        assert_eq!(accounting.selected_input_positions, 2);
        assert_eq!(
            accounting.rows_by_reason[&SelectionReason::Selected].stage_cast_count,
            1
        );
        assert_eq!(
            accounting.rows_by_reason[&SelectionReason::Selected].stage_channel_participation_count,
            5
        );
    }

    #[test]
    fn uuid_order_and_duplicates_use_uuid_values_not_text_case() {
        let mut a = row(10, "voter", CastVoteStatus::Valid, None);
        a.id = a.id.to_uppercase();
        let b = row(11, "voter", CastVoteStatus::Valid, None);
        assert_eq!(select(&[a.clone(), b.clone()]).unwrap()[0].row.id, b.id);
        let mut duplicate = a.clone();
        duplicate.id = duplicate.id.to_lowercase();
        assert!(select(&[a, duplicate]).is_err());
    }
}
