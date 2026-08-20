// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Pipeline-side tally helpers.
//!
//! `update_extended_metrics` (along with its private `calculate_*`
//! helpers) lives in `velvet-core` and is re-exported here so existing
//! algorithm imports (`use ...counting_algorithm::utils::*`) keep
//! working. The remaining helpers — `get_contest_tally_operation`,
//! `get_area_tally_operation`, `get_area_weight` — are pipeline-side
//! orchestration consumed directly by `do_tally.rs`.

pub use velvet_core::counting::update_extended_metrics;

use sequent_core::{
    ballot::{BallotStyle, Contest, Weight},
    types::ceremonies::{CountingAlgType, TallyOperation},
};
use std::str::FromStr;
use tracing::instrument;
use uuid::Uuid;

#[instrument(skip_all)]
pub fn get_contest_tally_operation(contest: &Contest) -> TallyOperation {
    let default_tally_op = contest
        .get_counting_algorithm()
        .get_default_tally_operation_for_contest();
    let annotations = contest.annotations.clone().unwrap_or_default();
    let operation = annotations
        .get("tally_operation")
        .map(|val| val.clone())
        .unwrap_or_default();
    TallyOperation::from_str(&operation).unwrap_or(default_tally_op)
}

#[instrument(skip_all)]
pub fn get_area_tally_operation(
    ballot_styles: &Vec<BallotStyle>,
    counting_alg: CountingAlgType,
    area_id: &Uuid,
) -> TallyOperation {
    let area_ballot_style: Option<&BallotStyle> = ballot_styles
        .iter()
        .find(|bs| bs.area_id == area_id.to_string());

    match area_ballot_style
        .and_then(|bs| bs.area_annotations.as_ref())
        .and_then(|area_annotations| area_annotations.tally_operation)
    {
        Some(tally_op) => tally_op,
        None => counting_alg.get_default_tally_operation_for_area(),
    }
}

#[instrument(skip_all)]
pub fn get_area_weight(ballot_styles: &Vec<BallotStyle>, area_id: &Uuid) -> Weight {
    let area_ballot_style: Option<&BallotStyle> = ballot_styles
        .iter()
        .find(|bs| bs.area_id == area_id.to_string());

    area_ballot_style
        .map(|bs| {
            bs.area_annotations
                .as_ref()
                .map(|area_annotations| area_annotations.get_weight())
        })
        .flatten()
        .unwrap_or_default()
}



#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn ballot_style_with(area_id: &Uuid, weight: Option<u64>) -> BallotStyle {
        BallotStyle {
            id: Uuid::new_v4().to_string(),
            tenant_id: Uuid::new_v4().to_string(),
            election_event_id: Uuid::new_v4().to_string(),
            election_id: Uuid::new_v4().to_string(),
            num_allowed_revotes: None,
            description: None,
            public_key: None,
            area_id: area_id.to_string(),
            area_presentation: None,
            contests: vec![],
            election_event_annotations: Default::default(),
            election_annotations: Default::default(),
            election_event_presentation: None,
            election_presentation: None,
            election_dates: None,
            area_annotations: weight.map(|weight| {
                serde_json::from_value(serde_json::json!({ "weight": weight }))
                    .expect("area annotations parse")
            }),
            multi_contest_encoding_mode: None,
        }
    }

    /// An area weight is applied whenever the published ballot style carries
    /// one, with no reference to the weighted voting policy. That is why
    /// `create_tally_ceremony` refuses to start a voters-weighted tally while
    /// any published ballot style still carries a weight: nothing downstream
    /// would stop the two being applied on top of each other.
    #[test]
    fn area_weight_is_applied_whenever_the_ballot_style_carries_one() {
        let area_id = Uuid::new_v4();
        let ballot_styles = vec![ballot_style_with(&area_id, Some(5))];
        assert_eq!(*get_area_weight(&ballot_styles, &area_id), Some(5));
    }

    /// An area with no ballot style, and one whose ballot style carries no
    /// annotations, both fall back to the neutral weight rather than zero.
    #[test]
    fn area_weight_falls_back_to_the_neutral_weight() {
        let area_id = Uuid::new_v4();
        assert_eq!(*get_area_weight(&vec![], &area_id), Some(1));

        let ballot_styles = vec![ballot_style_with(&area_id, None)];
        assert_eq!(*get_area_weight(&ballot_styles, &area_id), Some(1));
    }
}
