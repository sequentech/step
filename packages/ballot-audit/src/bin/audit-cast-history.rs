// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{ensure, Context, Result};
use ballot_audit::{decode_message, digest, Scope};
use clap::Parser;
use electoral_log::messages::{
    message::Message,
    statement::{StatementBody, StatementType},
};
use sequent_core::{
    ballot::{HashableBallot, TYPES_VERSION},
    encrypt::{hash_ballot_sha512, hash_multi_ballot_sha512},
    multi_ballot::HashableMultiBallot,
};
use serde::{Deserialize, Deserializer, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};
use strand::{
    hash::Hash,
    serialization::{StrandDeserialize, StrandSerialize},
    signature::StrandSignaturePk,
};
use uuid::Uuid;

/// Compare signed CastVote history with every Hasura row for one event/election.
/// All areas, statuses and revotes must be supplied. Never connects to a service.
#[derive(Parser)]
struct Args {
    #[arg(long)]
    scope: PathBuf,
    #[arg(long)]
    scope_sha512: String,
    /// Complete tenant/event/election history as a raw cast_vote JSON array.
    #[arg(long)]
    cast_vote_history: PathBuf,
    #[arg(long)]
    cast_vote_history_sha512: String,
    /// JSON array of unpadded-base64 full native electoral-log Message bytes.
    #[arg(long)]
    electoral_log: PathBuf,
    #[arg(long)]
    electoral_log_sha512: String,
    /// Filter reported receipt groups only; the entire supplied history is checked.
    #[arg(long)]
    ballot_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedScope {
    format_version: u32,
    scope: Scope,
    batches: Vec<u32>,
}

fn required_nullable<'de, D, T>(deserializer: D) -> std::result::Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(deserializer)
}

// Extra columns, including area/status/timestamp, are deliberately not used to
// filter historical coverage. No latest-vote or eligibility selection occurs.
#[derive(Deserialize)]
struct HistoryRow {
    id: String,
    tenant_id: String,
    election_event_id: String,
    #[serde(deserialize_with = "required_nullable")]
    election_id: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    voter_id_string: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    content: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    ballot_id: Option<String>,
}

#[derive(Default)]
struct Coverage {
    row_ids: Vec<String>,
    log_records: usize,
    message_digests: BTreeSet<String>,
    statement_digests: BTreeSet<String>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Classification {
    Matched,
    MatchedRepeatedLogs,
    MatchedAmbiguousRowAttribution,
    MatchedAmbiguousLogOccurrences,
    SignedLogWithoutHasuraRow,
    HasuraRowWithoutSignedLog,
}

#[derive(Debug, Serialize)]
struct ReceiptCoverage {
    ballot_id: String,
    cast_vote_ids: Vec<String>,
    signed_log_records: usize,
    distinct_signed_log_message_bytes: usize,
    distinct_signed_statements: usize,
    duplicate_log_message_records: usize,
    classification: Classification,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Outcome {
    Consistent,
    Incomplete,
    Ambiguous,
}

#[derive(Debug, Serialize)]
struct HistoryReport {
    format_version: u32,
    outcome: Outcome,
    checked_hasura_rows: usize,
    checked_signed_cast_vote_log_records: usize,
    ignored_other_log_records: usize,
    distinct_signed_log_message_bytes: usize,
    duplicate_log_message_records: usize,
    signed_log_records_without_hasura_row: usize,
    hasura_rows_without_signed_log: usize,
    ambiguous_row_attribution_groups: usize,
    ambiguous_log_occurrence_groups: usize,
    unfiltered_receipt_groups: usize,
    report_filtered: bool,
    requested_receipt_found: Option<bool>,
    receipts: Vec<ReceiptCoverage>,
    source_sha512: BTreeMap<&'static str, String>,
    assurance: &'static str,
}

fn is_receipt(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn read_pinned(path: &Path, pin: &str, label: &str) -> Result<Vec<u8>> {
    ensure!(
        pin.len() == 128
            && pin
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "{label} SHA-512 pin must be 128 lowercase hexadecimal characters"
    );
    let bytes = fs::read(path).with_context(|| format!("cannot read {label} source"))?;
    ensure!(digest(&bytes)? == pin, "{label} source digest mismatch");
    Ok(bytes)
}

fn ballot_hash(content: &str, single_contest: bool) -> Result<Hash> {
    if single_contest {
        let ballot: HashableBallot =
            serde_json::from_str(content).context("unhashable historical single-contest ballot")?;
        ensure!(
            ballot.version == TYPES_VERSION,
            "unsupported historical ballot version"
        );
        hash_ballot_sha512(&ballot).context("unhashable historical single-contest ballot")
    } else {
        let ballot: HashableMultiBallot =
            serde_json::from_str(content).context("unhashable historical multi-contest ballot")?;
        ensure!(
            ballot.version == TYPES_VERSION,
            "unsupported historical ballot version"
        );
        hash_multi_ballot_sha512(&ballot).context("unhashable historical multi-contest ballot")
    }
}

fn audit_history(
    scope: &Scope,
    rows: &[HistoryRow],
    logs: &[Vec<u8>],
    requested_receipt: Option<&str>,
) -> Result<HistoryReport> {
    for id in [
        &scope.tenant_id,
        &scope.election_event_id,
        &scope.election_id,
    ] {
        ensure!(!id.trim().is_empty(), "missing historical scope identifier");
    }
    ensure!(
        requested_receipt.is_none_or(is_receipt),
        "requested receipt must be 64 lowercase hexadecimal characters"
    );
    let system_key = StrandSignaturePk::from_der_b64_string(&scope.electoral_log_system_public_key)
        .context("invalid pinned electoral-log system key")?;
    let mut groups: BTreeMap<(Hash, Hash), Coverage> = BTreeMap::new();
    let mut row_ids = BTreeSet::new();
    for row in rows {
        let id = Uuid::parse_str(&row.id).context("invalid historical cast_vote UUID")?;
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
        let full_hash = ballot_hash(content, scope.contest_id.is_some())?;
        ensure!(
            row.ballot_id.as_deref() == Some(hex::encode(&full_hash[..32]).as_str()),
            "historical stored ballot receipt does not match native full-ballot hash"
        );
        let pseudonym = strand::hash::hash_to_array(&voter.strand_serialize()?)?;
        groups
            .entry((pseudonym, full_hash))
            .or_default()
            .row_ids
            .push(id.to_string());
    }
    let mut ignored = 0;
    let mut checked_logs = 0;
    let mut message_digests = BTreeSet::new();
    for bytes in logs {
        let message =
            Message::strand_deserialize(bytes).context("invalid full electoral-log message")?;
        // Verify before filtering: a tampered signed scope must not hide a log
        // when its corresponding database row has also been removed.
        message
            .verify(&system_key)
            .context("historical electoral-log sender/system signature verification failed")?;
        let cast_vote_kind = matches!(message.statement.head.kind, StatementType::CastVote);
        let cast_vote_body = matches!(
            message.statement.body,
            StatementBody::CastVote(..) | StatementBody::CastVoteWithChannel(..)
        );
        ensure!(
            cast_vote_kind == cast_vote_body,
            "electoral-log CastVote kind/body mismatch"
        );
        if message.statement.head.event.0 != scope.election_event_id || !cast_vote_kind {
            ignored += 1;
            continue;
        }
        let (election, pseudonym, full_hash) = match &message.statement.body {
            StatementBody::CastVote(e, p, h, _, _)
            | StatementBody::CastVoteWithChannel(e, p, h, _, _, _) => (e, p, h),
            _ => anyhow::bail!("electoral-log CastVote kind/body mismatch"),
        };
        if election.0.as_deref() != Some(scope.election_id.as_str()) {
            ignored += 1;
            continue;
        }

        let message_digest = digest(bytes)?;
        message_digests.insert(message_digest.clone());
        let coverage = groups
            .entry((
                pseudonym.0.clone().into_inner(),
                full_hash.0.clone().into_inner(),
            ))
            .or_default();
        coverage.log_records += 1;
        coverage.message_digests.insert(message_digest);
        coverage
            .statement_digests
            .insert(digest(&message.statement.strand_serialize()?)?);
        checked_logs += 1;
    }
    let mut missing_rows = 0;
    let mut missing_logs = 0;
    let mut ambiguous = 0;
    let mut ambiguous_logs = 0;
    let mut receipts = Vec::new();
    for ((_, full_hash), coverage) in groups {
        let classification = if coverage.row_ids.is_empty() {
            missing_rows += coverage.log_records;
            Classification::SignedLogWithoutHasuraRow
        } else if coverage.log_records == 0 {
            missing_logs += coverage.row_ids.len();
            Classification::HasuraRowWithoutSignedLog
        } else if coverage.row_ids.len() > 1 {
            ambiguous += 1;
            Classification::MatchedAmbiguousRowAttribution
        } else if coverage.message_digests.len() > 1 {
            ambiguous_logs += 1;
            Classification::MatchedAmbiguousLogOccurrences
        } else if coverage.log_records > 1 {
            Classification::MatchedRepeatedLogs
        } else {
            Classification::Matched
        };
        receipts.push(ReceiptCoverage {
            ballot_id: hex::encode(&full_hash[..32]),
            cast_vote_ids: coverage.row_ids,
            signed_log_records: coverage.log_records,
            distinct_signed_log_message_bytes: coverage.message_digests.len(),
            distinct_signed_statements: coverage.statement_digests.len(),
            duplicate_log_message_records: coverage.log_records - coverage.message_digests.len(),
            classification,
        });
    }
    receipts
        .sort_by(|a, b| (&a.ballot_id, &a.cast_vote_ids).cmp(&(&b.ballot_id, &b.cast_vote_ids)));
    let requested_receipt_found =
        requested_receipt.map(|requested| receipts.iter().any(|r| r.ballot_id == requested));
    let unfiltered_receipt_groups = receipts.len();
    let outcome =
        if missing_rows != 0 || missing_logs != 0 || requested_receipt_found == Some(false) {
            Outcome::Incomplete
        } else if ambiguous != 0 || ambiguous_logs != 0 {
            Outcome::Ambiguous
        } else {
            Outcome::Consistent
        };
    if let Some(requested) = requested_receipt {
        receipts.retain(|receipt| receipt.ballot_id == requested);
    }
    Ok(HistoryReport {
        format_version: 1, outcome,
        checked_hasura_rows: rows.len(), checked_signed_cast_vote_log_records: checked_logs,
        ignored_other_log_records: ignored,
        distinct_signed_log_message_bytes: message_digests.len(),
        duplicate_log_message_records: checked_logs - message_digests.len(),
        signed_log_records_without_hasura_row: missing_rows,
        hasura_rows_without_signed_log: missing_logs,
        ambiguous_row_attribution_groups: ambiguous,
        ambiguous_log_occurrence_groups: ambiguous_logs,
        unfiltered_receipt_groups, report_filtered: requested_receipt.is_some(), requested_receipt_found,
        receipts, source_sha512: BTreeMap::new(),
        assurance: "Coverage comparison of the supplied pinned tenant/event/election Hasura history against signed successful CastVote statements, across all areas/statuses/revotes. Only signed event/election/pseudonym/full ballot hashes are matched; unsigned log area/user/receipt metadata is not authority. Repeated logs and repeated identical ballots do not establish a one-to-one row/event attribution. Multiple rows or distinct complete log messages sharing a tuple are ambiguous and return exit 2; byte-identical log duplicates are reported without assuming independent occurrences. Distinct signed statement counts are also reported: differing unsigned metadata does not prove another signing event. All input records are signature-verified before scope filtering; foreign or rotated system keys are unsupported without an independently trusted key set. Exported messages lack database row IDs; exact byte duplicates are reported, not treated as unique events. A signed log without a row is unexplained/incomplete, not a lawful selection exclusion. A row without a log is incomplete evidence, not proof of tampering: native success logging is best effort. This does not establish historical completeness, contemporaneous source authenticity, ImmuDB inclusion/consistency proofs, eligibility, tally inclusion, or integrity of metadata omitted from the native receipt hash.",
    })
}

fn run(args: Args) -> Result<HistoryReport> {
    let scope_bytes = read_pinned(&args.scope, &args.scope_sha512, "scope")?;
    let row_bytes = read_pinned(
        &args.cast_vote_history,
        &args.cast_vote_history_sha512,
        "cast-vote history",
    )?;
    let log_bytes = read_pinned(
        &args.electoral_log,
        &args.electoral_log_sha512,
        "electoral log",
    )?;
    let saved: SavedScope = serde_json::from_slice(&scope_bytes).context("invalid saved scope")?;
    ensure!(
        saved.format_version == 1 && !saved.batches.is_empty(),
        "unsupported or incomplete saved scope"
    );
    let rows: Vec<HistoryRow> =
        serde_json::from_slice(&row_bytes).context("invalid historical cast_vote export")?;
    let encoded: Vec<String> =
        serde_json::from_slice(&log_bytes).context("invalid electoral-log JSON export")?;
    let logs = encoded
        .iter()
        .map(|value| decode_message(value).context("invalid historical electoral-log encoding"))
        .collect::<Result<Vec<_>>>()?;
    let mut report = audit_history(&saved.scope, &rows, &logs, args.ballot_id.as_deref())?;
    report.source_sha512 = BTreeMap::from([
        ("scope", args.scope_sha512),
        ("cast_vote_history", args.cast_vote_history_sha512),
        ("electoral_log", args.electoral_log_sha512),
    ]);
    Ok(report)
}

fn main() {
    match run(Args::parse()) {
        Ok(report) => {
            if serde_json::to_writer_pretty(std::io::stdout().lock(), &report).is_err() {
                eprintln!("Historical audit failed: cannot write report");
                std::process::exit(1);
            }
            println!();
            if report.outcome != Outcome::Consistent {
                std::process::exit(2);
            }
        }
        Err(error) => {
            // Never print nested parser/crypto errors containing private raw data.
            eprintln!("Historical audit failed: {error}");
            std::process::exit(1);
        }
    }
}
