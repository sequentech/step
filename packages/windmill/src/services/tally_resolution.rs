// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::types::ceremonies::{
    TallySessionResolution, TallySessionResolutionData, TallySessionResolutionStatus,
    TallySessionResolutionType,
};
use std::collections::HashMap;
use tracing::info;
use uuid::Uuid;

use crate::postgres::tally_session_resolution::{
    create_tally_session_resolution, get_pending_resolutions,
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

#[cfg(test)]
pub(crate) mod tests {
    use crate::tasks::execute_tally_session::{
        build_tie_resolutions_map, parse_pending_tie_from_annotations, pending_resolution_exists,
    };
    use sequent_core::types::ceremonies::{
        TallySessionResolution, TallySessionResolutionData,
        TallySessionResolutionStatus, TallySessionResolutionType, TieBreakingMethod,
    };

    fn make_resolution(
        contest_id: &str,
        round_number: u64,
        candidate_id: &str,
        resolution_type: ResolutionType,
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
                resolved_by_candidate_id: None,
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
        let rounds: Vec<u64> = contest1
            .iter()
            .filter_map(|v| v.get("round_number").and_then(|n| n.as_u64()))
            .collect();
        assert!(rounds.contains(&1));
        assert!(rounds.contains(&3));

        let contest2 = map.get("contest-2").unwrap();
        assert_eq!(contest2.len(), 1);
        assert_eq!(
            contest2[0]
                .get("resolved_by_candidate_id")
                .and_then(|v| v.as_str()),
            Some("candidate-b")
        );
    }

    /// Rows whose resolution_type is not IrvTieBreak must be ignored.
    #[test]
    fn test_build_tie_resolutions_map_ignores_non_irv_rows() {
        let rows = vec![
            make_resolution(
                "contest-1",
                1,
                "candidate-a",
                TallySessionResolutionType::IrvTieBreak,
            ),
            make_resolution(
                "contest-1",
                2,
                "candidate-b",
                TallySessionResolutionType::ManualRecount,
            ),
        ];
        let map = build_tie_resolutions_map(&rows);
        let contest1 = map.get("contest-1").unwrap();
        assert_eq!(contest1.len(), 1, "ManualRecount row must be excluded");
    }

    /// Rows with a missing contest_id or no resolution must be skipped.
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
        no_resolution.resolution = None;

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
    // Tests for parse_pending_tie_from_annotations
    // -------------------------------------------------------------------------

    /// Well-formed annotations must return the pending_tie_resolution object.
    #[test]
    fn test_parse_pending_tie_from_annotations_extracts_pending() {
        let tie_metadata = serde_json::json!({
            "round_number": 2,
            "tied_candidate_ids": ["candidate-a", "candidate-c"],
        });
        let annotations = serde_json::json!({
            "process_results": {
                "pending_tie_resolution": tie_metadata,
            }
        });

        let result = parse_pending_tie_from_annotations(&annotations);
        assert!(result.is_some(), "Should extract pending_tie_resolution");
        let extracted = result.unwrap();
        assert_eq!(extracted["round_number"], 2);
        assert_eq!(
            extracted["tied_candidate_ids"],
            serde_json::json!(["candidate-a", "candidate-c"])
        );
    }

    /// Annotations without pending_tie_resolution must return None.
    #[test]
    fn test_parse_pending_tie_from_annotations_returns_none_when_absent() {
        let annotations = serde_json::json!({"process_results": {"some_other_key": 1}});
        assert!(parse_pending_tie_from_annotations(&annotations).is_none());

        let no_process_results = serde_json::json!({"other": "data"});
        assert!(parse_pending_tie_from_annotations(&no_process_results).is_none());
    }

    /// When process_results is not a JSON object (e.g. a string or array) the
    /// as_object() guard must prevent a panic and return None.
    #[test]
    fn test_parse_pending_tie_when_process_results_is_not_an_object() {
        let string_value = serde_json::json!({"process_results": "unexpected string"});
        assert!(parse_pending_tie_from_annotations(&string_value).is_none());

        let array_value = serde_json::json!({"process_results": [1, 2, 3]});
        assert!(parse_pending_tie_from_annotations(&array_value).is_none());
    }

    /// pending_tie_resolution: null is stored as JSON null; the function must
    /// return Some(Value::Null) because the key exists.
    #[test]
    fn test_parse_pending_tie_returns_null_value_when_key_is_null() {
        let annotations = serde_json::json!({
            "process_results": {"pending_tie_resolution": null}
        });
        let result = parse_pending_tie_from_annotations(&annotations);
        assert!(
            result.is_some(),
            "Key present with null value must return Some"
        );
        assert!(result.unwrap().is_null());
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

    /// A pending record with a non-IrvTieBreak type must not match even if contest
    /// and round match.
    #[test]
    fn test_pending_resolution_exists_ignores_non_irv_type() {
        let mut r = make_pending_resolution("contest-x", 2);
        r.resolution_type = TallySessionResolutionType::IrvTieBreak;
        let tie_metadata = make_irv_metadata(2);
        assert!(!pending_resolution_exists(&[r], "contest-x", &tie_metadata));
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
