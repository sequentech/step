// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The result statement of each contest area of a completed results tally:
//! the stored result with the counts it is made of, signed for the election
//! verifier (`protocol_board::export_signature`) and kept in the tally session
//! contest's annotations, from where "Download result statement" exports it.

use crate::services::protocol_board::{load_dkg_board_row, load_manager};
use crate::services::weight_batches::contest_weight_batches;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use protocol_board::export_signature::{ExportKind, SignedExport};
use protocol_board::{BoardName, ElementPayload, TallyView};
use sequent_core::ballot::{BallotStyle, ContestEncryptionPolicy};
use sequent_core::result_count::count_contest_votes;
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use sequent_core::types::ceremonies::CountingAlgType;
use sequent_core::types::hasura::core::{
    BallotStyle as BallotStyleHasura, TallySession, TallySessionContest, TallySheet,
};
use sequent_core::types::result_statement::{
    BatchCounts, ContestStatement, ResultCounts, ResultStatement, TallySheetCounts,
    RESULT_STATEMENT_VERSION,
};
use std::collections::{HashMap, HashSet};
use tracing::{info, instrument};
use uuid::Uuid;

/// Every payload of each decrypted board by batch, before any is dropped,
/// which are the votes the election verifier decodes.
pub fn decrypted_payloads(
    views: &[(i64, BoardName, TallyView)],
) -> HashMap<i64, Vec<ElementPayload>> {
    views
        .iter()
        .filter_map(|(batch, _, view)| match view {
            TallyView::Decrypted { payloads } => Some((*batch, payloads.clone())),
            TallyView::BallotsPending
            | TallyView::BallotsPosted
            | TallyView::Mixing
            | TallyView::Decrypting
            | TallyView::Unusable { .. } => None,
        })
        .collect()
}

fn sheet_counts(sheet: &TallySheet, explicit_blank: &HashSet<&str>) -> ResultCounts {
    let mut counts = ResultCounts::default();
    let Some(content) = sheet.content.as_ref() else {
        return counts;
    };
    counts.total_valid_votes = content.total_valid_votes.unwrap_or(0);
    if let Some(invalid) = content.invalid_votes.as_ref() {
        counts.explicit_invalid_votes = invalid.explicit_invalid.unwrap_or(0);
        counts.implicit_invalid_votes = invalid.implicit_invalid.unwrap_or(0);
    }
    counts.blank_votes = content.total_blank_votes.unwrap_or(0);
    for candidate in content.candidate_results.values() {
        if !explicit_blank.contains(candidate.candidate_id.as_str()) {
            counts
                .candidates
                .insert(candidate.candidate_id.clone(), candidate.total_votes.unwrap_or(0));
        }
    }
    counts
}

async fn stored_result(
    tx: &Transaction<'_>,
    session_contest: &TallySessionContest,
    contest_id: &str,
    results_event_id: &str,
    explicit_blank: &HashSet<&str>,
) -> Result<Option<ResultCounts>> {
    let params: [&(dyn tokio_postgres::types::ToSql + Sync); 5] = [
        &session_contest.tenant_id,
        &session_contest.election_event_id,
        &results_event_id,
        &session_contest.area_id,
        &contest_id,
    ];
    let filter = "tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid
         AND results_event_id=$3::text::uuid AND area_id=$4::text::uuid
         AND contest_id=$5::text::uuid";
    let Some(row) = tx
        .query_opt(
            &format!(
                "SELECT total_valid_votes, explicit_invalid_votes, implicit_invalid_votes,
                        total_blank_votes
                 FROM sequent_backend.results_area_contest WHERE {filter} LIMIT 1"
            ),
            &params,
        )
        .await?
    else {
        return Ok(None);
    };
    let count = |index: usize| -> u64 {
        row.get::<_, Option<i64>>(index)
            .and_then(|value| u64::try_from(value).ok())
            .unwrap_or(0)
    };
    let mut counts = ResultCounts {
        total_valid_votes: count(0),
        explicit_invalid_votes: count(1),
        implicit_invalid_votes: count(2),
        blank_votes: count(3),
        ..Default::default()
    };
    for row in tx
        .query(
            &format!(
                "SELECT candidate_id::text, cast_votes
                 FROM sequent_backend.results_area_contest_candidate WHERE {filter}"
            ),
            &params,
        )
        .await?
    {
        let candidate: String = row.get(0);
        if !explicit_blank.contains(candidate.as_str()) {
            let votes = row
                .get::<_, Option<i64>>(1)
                .and_then(|value| u64::try_from(value).ok())
                .unwrap_or(0);
            counts.candidates.insert(candidate, votes);
        }
    }
    Ok(Some(counts))
}

/// Writes the signed result statement of every single-contest
/// `plurality-at-large` contest area of `tally_session`, replacing an earlier
/// one (a recount). Other contests get none: the election verifier checks
/// only these.
#[instrument(skip_all, err)]
pub async fn write_result_statements(
    tx: &Transaction<'_>,
    tally_session: &TallySession,
    tally_session_contests: &[TallySessionContest],
    ballot_styles: &[BallotStyleHasura],
    batch_payloads: &HashMap<i64, Vec<ElementPayload>>,
    tally_sheets: &[TallySheet],
    results_event_id: &str,
) -> Result<()> {
    if tally_session
        .configuration
        .clone()
        .unwrap_or_default()
        .get_contest_encryption_policy()
        != ContestEncryptionPolicy::SINGLE_CONTEST
    {
        return Ok(());
    }
    let tenant_id = &tally_session.tenant_id;
    let election_event_id = &tally_session.election_event_id;
    let (_, dkg) = load_dkg_board_row(
        tx,
        tenant_id,
        election_event_id,
        &tally_session.keys_ceremony_id,
    )
    .await?;
    let manager = load_manager(tx, tenant_id, election_event_id, &dkg.name).await?;
    let boards: HashMap<i64, Uuid> = tx
        .query(
            "SELECT batch, id FROM sequent_backend.protocol_board
             WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid
               AND tally_session_id=$3::text::uuid AND batch IS NOT NULL",
            &[tenant_id, election_event_id, &tally_session.id],
        )
        .await?
        .into_iter()
        .map(|row| (i64::from(row.get::<_, i32>(0)), row.get::<_, Uuid>(1)))
        .collect();

    for session_contest in tally_session_contests {
        let Some(contest_id) = session_contest.contest_id.as_deref() else {
            continue;
        };
        let Some((style_value, style)) = ballot_styles
            .iter()
            .filter(|row| {
                row.election_id == session_contest.election_id
                    && row.area_id.as_deref() == Some(session_contest.area_id.as_str())
            })
            .filter_map(|row| {
                let eml = row.ballot_eml.as_deref()?;
                let value: serde_json::Value = serde_json::from_str(eml).ok()?;
                let style: BallotStyle = deserialize_str(eml).ok()?;
                Some((value, style))
            })
            .find(|(_, style)| style.contests.iter().any(|c| c.id == contest_id))
        else {
            continue;
        };
        let Some(contest) = style.contests.iter().find(|c| c.id == contest_id) else {
            continue;
        };
        if contest.get_counting_algorithm() != CountingAlgType::PluralityAtLarge {
            continue;
        }
        let explicit_blank: HashSet<&str> = contest
            .candidates
            .iter()
            .filter(|candidate| candidate.is_explicit_blank())
            .map(|candidate| candidate.id.as_str())
            .collect();
        let mut batches = vec![];
        for (batch, multiplier) in contest_weight_batches(session_contest)? {
            let payloads = batch_payloads
                .get(&batch)
                .ok_or_else(|| anyhow!("batch {batch} has no decrypted votes"))?;
            let board = boards
                .get(&batch)
                .ok_or_else(|| anyhow!("batch {batch} has no tally board"))?;
            batches.push(BatchCounts {
                tally_id: board.as_u128().to_string(),
                multiplier,
                counts: count_contest_votes(contest, payloads),
            });
        }
        let Some(result) = stored_result(
            tx,
            session_contest,
            contest_id,
            results_event_id,
            &explicit_blank,
        )
        .await?
        else {
            continue;
        };
        let statement = ResultStatement {
            version: RESULT_STATEMENT_VERSION,
            configuration_hash: dkg.configuration.hash().as_str().to_string(),
            election_id: session_contest.election_id.clone(),
            area_id: session_contest.area_id.clone(),
            ballot_style: style_value,
            contests: vec![ContestStatement {
                contest_id: contest_id.to_string(),
                counting_algorithm: CountingAlgType::PluralityAtLarge.to_string(),
                result,
                batches,
                tally_sheets: tally_sheets
                    .iter()
                    .filter(|sheet| {
                        sheet.area_id == session_contest.area_id && sheet.contest_id == contest_id
                    })
                    .map(|sheet| TallySheetCounts {
                        tally_sheet_id: sheet.id.clone(),
                        counts: sheet_counts(sheet, &explicit_blank),
                    })
                    .collect(),
            }],
        };
        let envelope = SignedExport::sign(
            ExportKind::ResultStatement,
            &serde_json::to_vec(&statement)?,
            &manager,
        )?;
        // Kept as text so that the export returns the signed bytes unchanged.
        tx.execute(
            "UPDATE sequent_backend.tally_session_contest
             SET annotations = jsonb_set(COALESCE(annotations, '{}'::jsonb),
                                         '{result_statement}', to_jsonb($1::text)),
                 last_updated_at = NOW()
             WHERE id=$2::text::uuid AND tenant_id=$3::text::uuid
               AND election_event_id=$4::text::uuid",
            &[
                &String::from_utf8(envelope).context("result statement is not UTF-8")?,
                &session_contest.id,
                tenant_id,
                election_event_id,
            ],
        )
        .await?;
        info!(
            "wrote the result statement of tally session contest {}",
            session_contest.id
        );
    }
    Ok(())
}
