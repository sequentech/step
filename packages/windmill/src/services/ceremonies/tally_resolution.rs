// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use rocket::http::Status;
use sequent_core::types::ceremonies::{
    TallyExecutionStatus, TallyResolution, TallySessionResolution, TallySessionResolutionData,
    TallySessionResolutionStatus, TallySessionResolutionType,
};
use std::collections::HashMap;
use thiserror::Error;
use tracing::info;
use uuid::Uuid;

use super::tally_ceremony::begin_tally_session_recount;
use super::tally_validation::TallyValidationError;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::tally_session::{
    get_tally_session_by_id, lock_tally_session_for_update, update_tally_session_status,
};
use crate::postgres::tally_session_resolution::{
    create_tally_session_resolution, get_pending_resolutions, get_resolution_by_tally_session,
    submit_resolution, update_resolution,
};
use crate::services::election_event_board::get_election_event_board;
use crate::services::electoral_log::ElectoralLog;

/// Groups resolved IRV tie-break rows into a per-contest map keyed by the
/// actual contest UUID.
pub fn build_tie_resolutions_map(
    resolutions: &[TallySessionResolution],
) -> HashMap<String, Vec<TallySessionResolutionData>> {
    let mut map: HashMap<String, Vec<TallySessionResolutionData>> = HashMap::new();
    for r in resolutions
        .iter()
        .filter(|r| r.status == TallySessionResolutionStatus::Resolved)
        .filter(|r| r.resolution_type == TallySessionResolutionType::IrvTieBreak)
        .filter(|r| r.resolution_data.is_some())
    {
        let Some(actual_contest_id) = r.contest_id.as_deref() else {
            continue;
        };
        let Some(resolution_data) = r.resolution_data.clone() else {
            continue;
        };
        map.entry(actual_contest_id.to_string())
            .or_default()
            .push(resolution_data);
    }
    map
}

/// Returns true if `existing` already contains a pending IRV tie-break resolution
/// for the given `contest_id` and the same `round_number` as in `tie_metadata`.
///
/// Using `(contest_id, round_number)` as the key — rather than `contest_id` alone —
/// allows area-level `ProcessBallotsAll` runs to produce independent resolutions for
/// different rounds of the same contest without silently dropping any of them.
pub fn pending_resolution_exists(
    existing: &[TallySessionResolution],
    contest_id: &str,
    tie_metadata: &TallySessionResolutionData,
) -> bool {
    existing.iter().any(|r| {
        r.contest_id.as_deref() == Some(contest_id)
            && r.resolution_type == TallySessionResolutionType::IrvTieBreak
            && r.resolution_data.as_ref().map(|d| d.round_number) == Some(tie_metadata.round_number)
    })
}

/// Describes pending tie-breaks found in a set of results.
pub struct TieResolutionCheck {
    /// Pending ties: `(contest_id, tie_metadata)`.
    pub pending: Vec<(String, TallySessionResolutionData)>,
}

/// Scans `results_area_contest` rows for the given `results_event_id` and
/// returns any contests whose annotations contain a `pending_tie_resolution`.
pub async fn check_for_tie_resolutions(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    results_event_id: &str,
) -> Result<TieResolutionCheck> {
    // TODO Instead of checking for the annotations of results_contest in hasura check either in sqlite or add an output file for velvet with all resolutions
    let rows = hasura_transaction
        .query(
            r#"
                SELECT contest_id, annotations
                FROM sequent_backend.results_contest
                WHERE tenant_id = $1
                  AND election_event_id = $2
                  AND results_event_id = $3
            "#,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
                &Uuid::parse_str(results_event_id)?,
            ],
        )
        .await?;

    let mut pending = Vec::new();
    for row in rows {
        let contest_id_uuid: Uuid = row.get(0);
        let annotations: Option<serde_json::Value> = row.get(1);

        let Some(annotations) = annotations else {
            continue;
        };
        let Some(process_results) = annotations
            .get("process_results")
            .and_then(|v| v.as_object())
        else {
            continue;
        };
        let Some(pending_tie) = process_results.get("pending_tie_resolution") else {
            continue;
        };
        if pending_tie.is_null() {
            continue;
        }

        let tie_metadata: TallySessionResolutionData = serde_json::from_value(pending_tie.clone())?;
        pending.push((contest_id_uuid.to_string(), tie_metadata));
    }

    Ok(TieResolutionCheck { pending })
}

/// Checks for pending IRV tie-breaks in the freshly-computed results, ensures a
/// `tally_session_resolution` record exists for each, and posts a
/// `tally_paused_pending_resolution` entry to the electoral log.
///
/// Returns the IDs of all pending resolution records (empty if no ties detected).
pub async fn handle_pending_irv_resolutions(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    results_event_id: &str,
    tally_session_id: &str,
    bulletin_board_reference: Option<serde_json::Value>,
    tally_session_election_ids: Option<Vec<String>>,
) -> Result<Vec<String>> {
    let tie_resolutions = check_for_tie_resolutions(
        hasura_transaction,
        tenant_id,
        election_event_id,
        results_event_id,
    )
    .await?;

    if tie_resolutions.pending.is_empty() {
        return Ok(vec![]);
    }

    info!(
        "Detected {} pending tie resolution(s) in results - creating resolution records",
        tie_resolutions.pending.len()
    );

    let existing_pending_resolutions = get_pending_resolutions(
        hasura_transaction,
        tenant_id,
        election_event_id,
        tally_session_id,
    )
    .await?;

    let mut pending_resolution_ids: Vec<String> = vec![];
    for (contest_id, tie_metadata) in &tie_resolutions.pending {
        if !pending_resolution_exists(&existing_pending_resolutions, contest_id, tie_metadata) {
            let resolution_id = create_tally_session_resolution(
                hasura_transaction,
                tenant_id,
                election_event_id,
                tally_session_id,
                contest_id,
                TallySessionResolutionType::IrvTieBreak,
                tie_metadata.clone(),
            )
            .await?;
            info!(
                "Created pending resolution {} for IRV tie-break in contest {}",
                resolution_id, contest_id
            );
            pending_resolution_ids.push(resolution_id);
        } else if let Some(existing) = existing_pending_resolutions
            .iter()
            .find(|r| r.contest_id.as_deref() == Some(contest_id.as_str()))
        {
            pending_resolution_ids.push(existing.id.clone());
        }
    }

    info!(
        "Tally paused - awaiting administrator tie-break decisions for {} contest(s)",
        pending_resolution_ids.len()
    );
    let board_name = get_election_event_board(bulletin_board_reference)
        .with_context(|| "missing bulletin board")?;
    let electoral_log = ElectoralLog::new(
        hasura_transaction,
        tenant_id,
        Some(election_event_id),
        board_name.as_str(),
    )
    .await?;
    electoral_log
        .post_tally_paused_pending_resolution(
            election_event_id.to_string(),
            tally_session_election_ids,
            pending_resolution_ids.clone(),
        )
        .await
        .with_context(|| "error posting tally paused to electoral log")?;

    Ok(pending_resolution_ids)
}

/// What submitting tie-break decisions does to the tally session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionSubmission {
    /// The tally is paused in `AWAITING_INPUT` and resumes with the decisions.
    ResumePausedTally,
    /// The tally has completed; changing a decision recounts it.
    RecountCompletedTally,
}

#[derive(Debug, Error)]
#[error("Tie-break submission {0:?} is not permitted for this user")]
pub struct ResolutionSubmissionNotPermitted(pub ResolutionSubmission);

/// Submit multiple tally resolutions for a paused tally (batch operation).
/// `permitted` lists the kinds of submission the caller is authorized for;
/// any other kind is rejected before anything is written.
/// Returns the number of resolutions processed.
pub async fn submit_tally_resolution(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
    resolutions: &[TallyResolution],
    user_id: &str,
    username: Option<String>,
    permitted: &[ResolutionSubmission],
) -> Result<usize> {
    // Serialize with the tally task, recounts and other submissions, and read
    // the status only once the lock is held.
    lock_tally_session_for_update(
        hasura_transaction,
        tenant_id,
        election_event_id,
        tally_session_id,
    )
    .await?;
    let tally_session = get_tally_session_by_id(
        hasura_transaction,
        tenant_id,
        election_event_id,
        tally_session_id,
    )
    .await?;

    let execution_status = tally_session
        .execution_status
        .and_then(|s| s.parse::<TallyExecutionStatus>().ok())
        .ok_or_else(|| anyhow!("Missing execution status for tally session {tally_session_id}"))?;

    // Get all resolutions for this tally session (needed before status validation)
    let all_resolutions = get_resolution_by_tally_session(
        hasura_transaction,
        tenant_id,
        election_event_id,
        tally_session_id,
    )
    .await?;

    let input_contest_ids: Vec<&str> = resolutions.iter().map(|r| r.contest_id.as_str()).collect();
    let submission = classify_resolution_submission(
        &execution_status,
        tally_session.is_execution_completed,
        &input_contest_ids,
        &all_resolutions,
    )
    .map_err(|(_, msg)| TallyValidationError::new(msg))?;
    if !permitted.contains(&submission) {
        return Err(ResolutionSubmissionNotPermitted(submission).into());
    }

    let election_event =
        get_election_event_by_id(hasura_transaction, tenant_id, election_event_id).await?;
    let board_name = get_election_event_board(election_event.bulletin_board_reference.clone())
        .with_context(|| "missing bulletin board")?;
    let electoral_log = ElectoralLog::for_admin_user(
        hasura_transaction,
        &board_name,
        tenant_id,
        election_event_id,
        user_id,
        username.clone(),
        tally_session.election_ids.clone(),
        None,
    )
    .await?;

    // Validate and submit each resolution
    let mut resolved_count = 0;

    for tie_resolution in resolutions {
        // First submission: find the single Pending record for this contest.
        // Re-submission: no Pending record exists, fall back to the most recently
        // created Resolved record (admin is changing a prior decision).
        let latest_resolution = all_resolutions
            .iter()
            .find(|r| {
                r.resolution_type == TallySessionResolutionType::IrvTieBreak
                    && r.contest_id.as_ref() == Some(&tie_resolution.contest_id)
                    && r.status == TallySessionResolutionStatus::Pending
            })
            .or_else(|| {
                all_resolutions
                    .iter()
                    .filter(|r| {
                        r.resolution_type == TallySessionResolutionType::IrvTieBreak
                            && r.contest_id.as_ref() == Some(&tie_resolution.contest_id)
                    })
                    .max_by_key(|r| r.created_at)
            })
            .ok_or_else(|| {
                anyhow!(
                    "No resolution found for contest {}",
                    tie_resolution.contest_id
                )
            })?;

        // Validate selected candidate is in tied candidates
        let tied_candidate_ids =
            extract_tied_candidate_ids(latest_resolution, &tie_resolution.contest_id)
                .map_err(|(_, msg)| anyhow!(msg))?;

        if !tied_candidate_ids.contains(&tie_resolution.selected_candidate_id) {
            return Err(anyhow!(
                "Selected candidate {} is not in tied candidates for contest {}: {:?}",
                tie_resolution.selected_candidate_id,
                tie_resolution.contest_id,
                tied_candidate_ids
            ));
        }

        let pending_data = latest_resolution.resolution_data.clone().ok_or_else(|| {
            anyhow!(
                "Missing resolution data for contest {}",
                tie_resolution.contest_id
            )
        })?;
        let resolution_value = TallySessionResolutionData {
            resolved_by_candidate_id: Some(tie_resolution.selected_candidate_id.clone()),
            ..pending_data
        };

        let resolution_id = latest_resolution.id.clone();
        let resubmission = is_resubmission(latest_resolution);

        if !resubmission {
            // First submission: resolve the existing pending record
            submit_resolution(
                hasura_transaction,
                tenant_id,
                election_event_id,
                &resolution_id,
                resolution_value,
                user_id,
            )
            .await?;
            electoral_log
                .post_tally_tie_resolved(
                    election_event_id.to_string(),
                    tally_session.election_ids.clone(),
                    tie_resolution.contest_id.clone(),
                    resolution_id,
                    Some(user_id.to_string()),
                    username.clone(),
                )
                .await
                .with_context(|| "error posting tally tie resolved to electoral log")?;
        } else {
            // Re-submission: admin changed their mind — update the existing record
            info!(
                "Re-submission detected for contest {} - updating existing resolution",
                tie_resolution.contest_id
            );
            update_resolution(
                hasura_transaction,
                tenant_id,
                election_event_id,
                &resolution_id,
                resolution_value,
                user_id,
            )
            .await?;
            electoral_log
                .post_tally_tie_resolution_updated(
                    election_event_id.to_string(),
                    tally_session.election_ids.clone(),
                    tie_resolution.contest_id.clone(),
                    resolution_id,
                    Some(user_id.to_string()),
                    username.clone(),
                )
                .await
                .with_context(|| "error posting tally tie resolution updated to electoral log")?;
        }

        resolved_count += 1;
    }

    // Resume the tally so it re-runs and produces intermediate results.
    // Windmill will pause again in AWAITING_INPUT if any new tie-breaks are
    // detected during the re-run.
    match submission {
        ResolutionSubmission::ResumePausedTally => {
            update_tally_session_status(
                hasura_transaction,
                tenant_id,
                election_event_id,
                tally_session_id,
                TallyExecutionStatus::IN_PROGRESS,
                false,
            )
            .await?;
        }
        ResolutionSubmission::RecountCompletedTally => {
            let election_ids = tally_session.election_ids.clone().unwrap_or_default();
            let recount_started = begin_tally_session_recount(
                hasura_transaction,
                tenant_id,
                election_event_id,
                tally_session_id,
                &election_ids,
            )
            .await?;
            if !recount_started {
                return Err(TallyValidationError::new(
                    "Only a completed tally session with execution history can be recounted",
                )
                .into());
            }
        }
    }

    info!(
        "Tally session set to {:?} after resolution submission ({:?})",
        TallyExecutionStatus::IN_PROGRESS,
        submission
    );

    Ok(resolved_count)
}

/// Decides what submitting decisions for `input_contest_ids` does to a tally
/// session. A paused tally accepts any decision; a completed tally accepts only
/// changes to contests that already have one. Every other status is rejected:
/// a running tally would finish with the decisions it read when it started, and
/// a cancelled one is final.
pub fn classify_resolution_submission(
    execution_status: &TallyExecutionStatus,
    is_execution_completed: bool,
    input_contest_ids: &[&str],
    all_resolutions: &[TallySessionResolution],
) -> Result<ResolutionSubmission, (Status, String)> {
    let all_are_resolved_updates = input_contest_ids.iter().all(|contest_id| {
        all_resolutions.iter().any(|r| {
            r.contest_id.as_deref() == Some(contest_id)
                && r.status == TallySessionResolutionStatus::Resolved
        })
    });
    match execution_status {
        TallyExecutionStatus::AWAITING_INPUT => Ok(ResolutionSubmission::ResumePausedTally),
        TallyExecutionStatus::SUCCESS if is_execution_completed && all_are_resolved_updates => {
            Ok(ResolutionSubmission::RecountCompletedTally)
        }
        _ => Err((
            Status::BadRequest,
            format!(
                "Tally session is not awaiting input. Current status: {}",
                execution_status
            ),
        )),
    }
}

/// Extracts the list of tied candidate IDs from a resolution's `resolution_data` field.
pub fn extract_tied_candidate_ids(
    resolution: &TallySessionResolution,
    contest_id: &str,
) -> Result<Vec<String>, (Status, String)> {
    resolution
        .resolution_data
        .as_ref()
        .map(|d| d.tied_candidate_ids.clone())
        .ok_or_else(|| {
            (
                Status::BadRequest,
                format!(
                    "Invalid resolution data for contest {}: missing tied_candidate_ids",
                    contest_id
                ),
            )
        })
}

/// Returns `true` when the resolution already has a decision recorded (i.e. this is
/// an admin changing their mind rather than the first submission).
pub fn is_resubmission(resolution: &TallySessionResolution) -> bool {
    resolution.status != TallySessionResolutionStatus::Pending
}

#[cfg(test)]
mod tally_resolution_tests {
    use super::{
        build_tie_resolutions_map, classify_resolution_submission, extract_tied_candidate_ids,
        is_resubmission, pending_resolution_exists, ResolutionSubmission,
    };
    use rocket::http::Status;
    use sequent_core::types::ceremonies::{
        TallyExecutionStatus, TallySessionResolution, TallySessionResolutionData,
        TallySessionResolutionStatus, TallySessionResolutionType, TieBreakingMethod,
    };

    fn make_resolution_tests(
        contest_id: &str,
        status: TallySessionResolutionStatus,
        tied_ids: &[&str],
    ) -> TallySessionResolution {
        TallySessionResolution {
            id: "1".to_string(),
            tenant_id: "2".to_string(),
            election_event_id: "3".to_string(),
            tally_session_id: "4".to_string(),
            contest_id: Some(contest_id.to_string()),
            created_at: None,
            last_updated_at: None,
            resolution_type: TallySessionResolutionType::IrvTieBreak,
            status,
            resolution_data: Some(TallySessionResolutionData {
                round_number: Some(2),
                tied_candidate_ids: tied_ids.iter().map(|s| s.to_string()).collect(),
                vote_count: 10,
                method_used: TieBreakingMethod::ExternalProcedure,
                resolved_by_candidate_id: None,
            }),
            resolved_by_user: None,
            resolved_at: None,
            labels: None,
            annotations: None,
        }
    }

    #[test]
    fn test_submit_resolution_fails_if_not_awaiting_input() {
        let resolution = make_resolution_tests(
            "contest-1",
            TallySessionResolutionStatus::Pending,
            &["c-1", "c-2"],
        );
        let result = classify_resolution_submission(
            &TallyExecutionStatus::IN_PROGRESS,
            false,
            &["contest-1"],
            &[resolution],
        );
        assert!(result.is_err());
        let (status, msg) = result.unwrap_err();
        assert_eq!(status, Status::BadRequest);
        assert!(
            msg.contains("not awaiting input"),
            "Expected 'not awaiting input' in: {msg}"
        );
    }

    #[test]
    fn test_submit_resolution_fails_if_candidate_not_tied() {
        let resolution = make_resolution_tests(
            "contest-1",
            TallySessionResolutionStatus::Pending,
            &["c-1", "c-2"],
        );
        let tied_ids = extract_tied_candidate_ids(&resolution, "contest-1")
            .expect("tied_candidate_ids should parse");
        assert!(!tied_ids.contains(&"c-99".to_string()));
        assert!(tied_ids.contains(&"c-1".to_string()));
        assert!(tied_ids.contains(&"c-2".to_string()));
    }

    #[test]
    fn test_submit_resolution_success_updates_status() {
        let resolution = make_resolution_tests(
            "contest-1",
            TallySessionResolutionStatus::Pending,
            &["c-1", "c-2"],
        );
        let result = classify_resolution_submission(
            &TallyExecutionStatus::AWAITING_INPUT,
            false,
            &["contest-1"],
            &[resolution.clone()],
        );
        assert_eq!(result, Ok(ResolutionSubmission::ResumePausedTally));
        assert!(
            !is_resubmission(&resolution),
            "Pending resolution should not be treated as a re-submission"
        );
    }

    #[test]
    fn test_resubmit_resolution_updates_existing_record() {
        let resolution = make_resolution_tests(
            "contest-1",
            TallySessionResolutionStatus::Resolved,
            &["c-1", "c-2"],
        );
        assert!(
            is_resubmission(&resolution),
            "Resolved resolution should be treated as a re-submission"
        );
        let result = classify_resolution_submission(
            &TallyExecutionStatus::SUCCESS,
            true,
            &["contest-1"],
            &[resolution],
        );
        assert_eq!(result, Ok(ResolutionSubmission::RecountCompletedTally));
    }

    #[test]
    fn test_resubmit_resolution_rejected_when_success_not_completed() {
        let resolution = make_resolution_tests(
            "contest-1",
            TallySessionResolutionStatus::Resolved,
            &["c-1", "c-2"],
        );
        let result = classify_resolution_submission(
            &TallyExecutionStatus::SUCCESS,
            false,
            &["contest-1"],
            &[resolution],
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_submit_pending_resolution_rejected_after_completion() {
        let resolution = make_resolution_tests(
            "contest-1",
            TallySessionResolutionStatus::Pending,
            &["c-1", "c-2"],
        );
        let result = classify_resolution_submission(
            &TallyExecutionStatus::SUCCESS,
            true,
            &["contest-1"],
            &[resolution],
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_resubmit_resolution_rejected_unless_paused_or_completed() {
        let resolution = make_resolution_tests(
            "contest-1",
            TallySessionResolutionStatus::Resolved,
            &["c-1", "c-2"],
        );
        let accepted: Vec<(TallyExecutionStatus, bool)> = [
            TallyExecutionStatus::STARTED,
            TallyExecutionStatus::CONNECTED,
            TallyExecutionStatus::IN_PROGRESS,
            TallyExecutionStatus::CANCELLED,
        ]
        .into_iter()
        .flat_map(|status| [(status.clone(), false), (status, true)])
        .filter(|(status, is_completed)| {
            classify_resolution_submission(
                status,
                *is_completed,
                &["contest-1"],
                &[resolution.clone()],
            )
            .is_ok()
        })
        .collect();
        assert!(accepted.is_empty(), "Resubmission accepted in {accepted:?}");
    }

    fn make_resolution(
        contest_id: &str,
        round_number: u64,
        candidate_id: &str,
        resolution_type: TallySessionResolutionType,
    ) -> TallySessionResolution {
        TallySessionResolution {
            id: uuid::Uuid::new_v4().to_string(),
            tenant_id: "tenant-1".to_string(),
            election_event_id: "event-1".to_string(),
            tally_session_id: "session-1".to_string(),
            contest_id: Some(contest_id.to_string()),
            created_at: None,
            last_updated_at: None,
            resolution_type,
            status: TallySessionResolutionStatus::Resolved,
            resolution_data: Some(TallySessionResolutionData {
                round_number: Some(round_number),
                tied_candidate_ids: vec![],
                vote_count: 0,
                method_used: TieBreakingMethod::ExternalProcedure,
                resolved_by_candidate_id: Some(candidate_id.to_string()),
            }),
            resolved_by_user: None,
            resolved_at: None,
            labels: None,
            annotations: None,
        }
    }

    /// Multiple resolved rows for the same contest_id (different rounds) must
    /// all be preserved under the single contest key, not overwritten.
    #[test]
    fn test_build_tie_resolutions_map_groups_by_contest() {
        let rows = vec![
            make_resolution(
                "contest-1",
                1,
                "candidate-a",
                TallySessionResolutionType::IrvTieBreak,
            ),
            make_resolution(
                "contest-1",
                3,
                "candidate-c",
                TallySessionResolutionType::IrvTieBreak,
            ),
            make_resolution(
                "contest-2",
                2,
                "candidate-b",
                TallySessionResolutionType::IrvTieBreak,
            ),
        ];
        let map = build_tie_resolutions_map(&rows);

        assert_eq!(map.len(), 2, "Two distinct contest IDs expected");

        let contest1 = map.get("contest-1").unwrap();
        assert_eq!(
            contest1.len(),
            2,
            "Both round-1 and round-3 entries must be present"
        );
        let rounds: Vec<Option<u64>> = contest1.iter().map(|d| d.round_number).collect();
        assert!(rounds.contains(&Some(1)));
        assert!(rounds.contains(&Some(3)));

        let contest2 = map.get("contest-2").unwrap();
        assert_eq!(contest2.len(), 1);
        assert_eq!(
            contest2[0].resolved_by_candidate_id.as_deref(),
            Some("candidate-b")
        );
    }

    /// Rows with a missing contest_id or no resolution_data must be skipped.
    #[test]
    fn test_build_tie_resolutions_map_skips_incomplete_rows() {
        let mut no_contest_id = make_resolution(
            "contest-1",
            1,
            "candidate-a",
            TallySessionResolutionType::IrvTieBreak,
        );
        no_contest_id.contest_id = None;

        let mut no_resolution = make_resolution(
            "contest-1",
            2,
            "candidate-a",
            TallySessionResolutionType::IrvTieBreak,
        );
        no_resolution.resolution_data = None;

        let map = build_tie_resolutions_map(&[no_contest_id, no_resolution]);
        assert!(map.is_empty(), "Incomplete rows must produce an empty map");
    }

    /// An empty slice must produce an empty map (no panics).
    #[test]
    fn test_build_tie_resolutions_map_empty_input() {
        let map = build_tie_resolutions_map(&[]);
        assert!(map.is_empty());
    }

    // -------------------------------------------------------------------------
    // Tests for pending_resolution_exists
    // These cover the (contest_id, round_number) uniqueness key used when
    // ProcessBallotsAll is set at area level and the same contest can tie in
    // different rounds across areas within a single tally run.
    // -------------------------------------------------------------------------

    fn make_pending_resolution(contest_id: &str, round_number: u64) -> TallySessionResolution {
        TallySessionResolution {
            id: uuid::Uuid::new_v4().to_string(),
            tenant_id: "tenant-1".to_string(),
            election_event_id: "event-1".to_string(),
            tally_session_id: "session-1".to_string(),
            contest_id: Some(contest_id.to_string()),
            created_at: None,
            last_updated_at: None,
            resolution_type: TallySessionResolutionType::IrvTieBreak,
            status: TallySessionResolutionStatus::Pending,
            resolution_data: Some(TallySessionResolutionData {
                round_number: Some(round_number),
                tied_candidate_ids: vec![],
                vote_count: 0,
                method_used: TieBreakingMethod::ExternalProcedure,
                resolved_by_candidate_id: None,
            }),
            resolved_by_user: None,
            resolved_at: None,
            labels: None,
            annotations: None,
        }
    }

    fn make_irv_metadata(round_number: u64) -> TallySessionResolutionData {
        TallySessionResolutionData {
            round_number: Some(round_number),
            tied_candidate_ids: vec![],
            vote_count: 0,
            method_used: TieBreakingMethod::ExternalProcedure,
            resolved_by_candidate_id: None,
        }
    }

    /// Empty list must never report an existing resolution (default SkipCandidateResults path).
    #[test]
    fn test_pending_resolution_exists_false_when_list_empty() {
        let tie_metadata = make_irv_metadata(2);
        assert!(!pending_resolution_exists(&[], "contest-x", &tie_metadata));
    }

    /// Same contest and same round — the existing pending record must be found.
    #[test]
    fn test_pending_resolution_exists_true_for_same_contest_and_round() {
        let existing = vec![make_pending_resolution("contest-x", 2)];
        let tie_metadata = make_irv_metadata(2);
        assert!(pending_resolution_exists(
            &existing,
            "contest-x",
            &tie_metadata
        ));
    }

    /// Same contest but different round — area-level ProcessBallotsAll can produce
    /// independent ties per round; they must be treated as distinct and both created.
    #[test]
    fn test_pending_resolution_exists_false_for_same_contest_different_round() {
        let existing = vec![make_pending_resolution("contest-x", 2)];
        let tie_metadata = make_irv_metadata(3);
        assert!(!pending_resolution_exists(
            &existing,
            "contest-x",
            &tie_metadata
        ));
    }

    /// Different contest, same round — must not collide.
    #[test]
    fn test_pending_resolution_exists_false_for_different_contest() {
        let existing = vec![make_pending_resolution("contest-x", 2)];
        let tie_metadata = make_irv_metadata(2);
        assert!(!pending_resolution_exists(
            &existing,
            "contest-y",
            &tie_metadata
        ));
    }

    /// Two areas produce ties for the same contest in different rounds
    /// (ProcessBallotsAll at area level). Both rounds must be independently
    /// detectable — neither should suppress the other.
    #[test]
    fn test_pending_resolution_exists_area_level_different_rounds_are_independent() {
        let round2 = make_pending_resolution("contest-x", 2);
        let round3 = make_pending_resolution("contest-x", 3);
        let existing = vec![round2, round3];

        // Querying for round 2 finds it.
        assert!(pending_resolution_exists(
            &existing,
            "contest-x",
            &make_irv_metadata(2)
        ));

        // Querying for round 3 finds it.
        assert!(pending_resolution_exists(
            &existing,
            "contest-x",
            &make_irv_metadata(3)
        ));

        // Round 4 has no record yet.
        assert!(!pending_resolution_exists(
            &existing,
            "contest-x",
            &make_irv_metadata(4)
        ));
    }
}
