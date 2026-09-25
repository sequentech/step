// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What voter-weighted voting cannot be combined with. Each weight is applied
//! by repeating a voter's ballot, so anything else that multiplies ballots or
//! counts them without a weight would give a wrong result.

use crate::domain::tally_ceremony::TallyValidationError;
use sequent_core::ballot::{
    BallotStyle, DecodedBallotsInclusionPolicy, DelegatedVotingPolicy, Weight,
};
use sequent_core::ballot_codec::multi_ballot::votable_contests;
use sequent_core::types::ceremonies::CountingAlgType;
use sequent_core::types::hasura::core::TallySheet;

/// When a voter-weighted tally is checked, which decides how a refusal is
/// worded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeightedVotingStage {
    /// The tally session is being created.
    Creation,
}

/// Refuses the election event policies that voter-weighted voting cannot be
/// combined with.
pub fn check_weighted_voting_policies(
    stage: WeightedVotingStage,
    delegated_voting_policy: &DelegatedVotingPolicy,
    decoded_ballots_inclusion_policy: &DecodedBallotsInclusionPolicy,
) -> Result<(), TallyValidationError> {
    // A delegate's ballot has no defined weighted semantics, and applying
    // both would silently compute weight * (1 + delegate_count).
    if *delegated_voting_policy == DelegatedVotingPolicy::ENABLED {
        return Err(match stage {
            WeightedVotingStage::Creation => TallyValidationError::new(
                "Delegated voting and voter-weighted voting cannot both be \
                 enabled on the same election event",
            ),
        });
    }
    // The mix batch no longer repeats a ciphertext, but the tally still
    // expands each batch's plaintexts by that batch's multiplier, so the
    // decoded ballots would carry each voter's weight as a run of identical
    // plaintexts. This closes the most direct disclosure; it does not make
    // the scheme secret-ballot safe on its own, since a ballot still
    // appears in one batch per bit of its weight and every batch is
    // public.
    if *decoded_ballots_inclusion_policy == DecodedBallotsInclusionPolicy::INCLUDED {
        return Err(match stage {
            WeightedVotingStage::Creation => TallyValidationError::new(
                "Decoded ballots cannot be included in the results when \
                 voter-weighted voting is enabled, because the repeated \
                 ballots would reveal each voter's weight",
            ),
        });
    }
    Ok(())
}

/// Refuses approved tally sheets of the elections being tallied.
pub fn check_weighted_voting_tally_sheets(
    stage: WeightedVotingStage,
    approved_tally_sheets: &[TallySheet],
    election_ids: &[String],
) -> Result<(), TallyValidationError> {
    // A tally sheet reports a count of paper ballots and has nowhere to
    // carry a weight, so velvet adds its votes to the weighted electronic
    // totals at one vote each. That does not just under-count them: it
    // decides contests, since a few hundred paper ballots worth 1 land
    // beside electronic ballots worth thousands. It also breaks the
    // published percentages, because a sheet contributes to the candidate
    // totals but not to the weighted base they are divided by.
    // Scoped to the elections being tallied, like the two refusals below.
    // An approved sheet belonging to a different election in the same event
    // says nothing about this tally, and refusing on it would name a remedy
    // -- withdraw the sheet -- that destroys that other election's paper
    // count.
    let counted_sheets = approved_tally_sheets
        .iter()
        // An empty list needs no fallback: a session that names no
        // elections has no contest rows, so there is no tally for a
        // sheet to be counted into.
        .filter(|sheet| election_ids.contains(&sheet.election_id))
        .count();
    if counted_sheets > 0 {
        return Err(match stage {
            WeightedVotingStage::Creation => TallyValidationError::new(format!(
                "Approved tally sheets cannot be counted when voter-weighted \
                 voting is enabled: a tally sheet reports a ballot count with no \
                 weight, so its votes would be added to the weighted totals at a \
                 weight of one each. {} approved tally sheet(s) exist for this \
                 election event",
                counted_sheets
            )),
        });
    }
    Ok(())
}

/// Refuses published ballots whose area still carries a weight, or whose
/// contests use a counting algorithm the weighted tally does not support.
pub fn check_weighted_voting_ballot_styles(
    stage: WeightedVotingStage,
    published_ballot_styles: &[BallotStyle],
) -> Result<(), TallyValidationError> {
    // Nothing downstream stops an area weight being applied on top of the
    // per-voter weight, so this refusal is the only thing that does.
    // It reads the ballot style snapshot frozen at publication, because
    // that is the value velvet will use: clearing the live area row without
    // republishing would otherwise satisfy the check while the tally still
    // double-counted every ballot.
    let mut weighted_areas: Vec<String> = Vec::new();
    let mut unsupported_contests: Vec<String> = Vec::new();
    for ballot_style in published_ballot_styles {
        // An absent weight and an explicit 1 are the same value, so only a
        // weight that would actually multiply is a conflict.
        let is_weighted = ballot_style
            .area_annotations
            .as_ref()
            .map(|annotations| annotations.get_weight())
            .is_some_and(|weight| weight != Weight::default());
        if is_weighted && !weighted_areas.contains(&ballot_style.area_id) {
            weighted_areas.push(ballot_style.area_id.clone());
        }

        // Duplicating a ballot is only defined for the algorithm this
        // feature was specified and tested for. Others would silently
        // accept repeated ballots with untested quota and elimination
        // behaviour. An unset algorithm resolves to plurality-at-large, and
        // could not have been published otherwise.
        // An acclaimed contest is never tallied, so its counting
        // algorithm cannot make a ballot style unsupported.
        for contest in votable_contests(&ballot_style.contests) {
            if contest.get_counting_algorithm() != CountingAlgType::PluralityAtLarge
                && !unsupported_contests.contains(&contest.id)
            {
                unsupported_contests.push(contest.id.clone());
            }
        }
    }
    if !weighted_areas.is_empty() {
        return Err(match stage {
            WeightedVotingStage::Creation => TallyValidationError::new(format!(
                "Voter-weighted voting cannot be used while published ballots \
                 still carry an area weight, because the two would multiply: \
                 {}. This has to be corrected \
                 before the ballots are published: set the weighted voting \
                 policy back to areas-weighted voting, which makes the weight \
                 editable again, clear it on these areas, set the policy to \
                 voters-weighted voting and publish. Once voting has begun the \
                 ballots cannot be republished, so at this point there is no \
                 remedy left",
                weighted_areas.join(", ")
            )),
        });
    }

    if !unsupported_contests.is_empty() {
        return Err(match stage {
            WeightedVotingStage::Creation => TallyValidationError::new(format!(
                "Voter-weighted voting only supports the plurality-at-large \
                 counting algorithm. These contests use another algorithm: {}",
                unsupported_contests.join(", ")
            )),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequent_core::ballot::Contest;
    use serde_json::json;

    const CREATION: WeightedVotingStage = WeightedVotingStage::Creation;

    fn contest(id: &str, counting_algorithm: CountingAlgType, is_acclaimed: bool) -> Contest {
        Contest {
            id: id.into(),
            election_id: "election".into(),
            counting_algorithm: Some(counting_algorithm),
            is_acclaimed: Some(is_acclaimed),
            ..Default::default()
        }
    }

    /// A ballot style of `area_id`, whose area annotations are `annotations`.
    fn ballot_style(
        area_id: &str,
        annotations: Option<serde_json::Value>,
        contests: Vec<Contest>,
    ) -> BallotStyle {
        BallotStyle {
            id: format!("style-{area_id}"),
            tenant_id: "tenant".into(),
            election_event_id: "event".into(),
            election_id: "election".into(),
            num_allowed_revotes: None,
            description: None,
            public_key: None,
            area_id: area_id.into(),
            area_presentation: None,
            contests,
            election_event_presentation: None,
            election_presentation: None,
            election_dates: None,
            election_event_annotations: None,
            election_annotations: None,
            area_annotations: annotations.map(|value| serde_json::from_value(value).unwrap()),
            multi_contest_encoding_mode: None,
        }
    }

    fn approved_sheet(election_id: &str) -> TallySheet {
        serde_json::from_value(json!({
            "id": format!("sheet-{election_id}"), "tenant_id": "tenant",
            "election_event_id": "event", "election_id": election_id,
            "contest_id": "contest", "area_id": "area", "created_by_user_id": "user",
            "status": "APPROVED", "version": 1,
        }))
        .unwrap()
    }

    fn message(result: Result<(), TallyValidationError>) -> String {
        result.unwrap_err().to_string()
    }

    #[test]
    fn delegated_voting_cannot_be_combined_with_voter_weighted_voting() {
        assert_eq!(
            message(check_weighted_voting_policies(
                CREATION,
                &DelegatedVotingPolicy::ENABLED,
                &DecodedBallotsInclusionPolicy::NOT_INCLUDED,
            )),
            "Delegated voting and voter-weighted voting cannot both be enabled on the same \
             election event"
        );
    }

    #[test]
    fn decoded_ballots_cannot_be_published_with_voter_weighted_voting() {
        assert_eq!(
            message(check_weighted_voting_policies(
                CREATION,
                &DelegatedVotingPolicy::DISABLED,
                &DecodedBallotsInclusionPolicy::INCLUDED,
            )),
            "Decoded ballots cannot be included in the results when voter-weighted voting is \
             enabled, because the repeated ballots would reveal each voter's weight"
        );
    }

    #[test]
    fn delegated_voting_is_reported_before_decoded_ballots() {
        assert!(message(check_weighted_voting_policies(
            CREATION,
            &DelegatedVotingPolicy::ENABLED,
            &DecodedBallotsInclusionPolicy::INCLUDED,
        ))
        .starts_with("Delegated voting"));
    }

    #[test]
    fn voter_weighted_voting_accepts_its_default_policies() {
        assert!(check_weighted_voting_policies(
            CREATION,
            &DelegatedVotingPolicy::default(),
            &DecodedBallotsInclusionPolicy::default(),
        )
        .is_ok());
    }

    #[test]
    fn approved_tally_sheets_of_the_tallied_elections_are_refused() {
        let sheets = [
            approved_sheet("tallied"),
            approved_sheet("other"),
            approved_sheet("tallied"),
        ];
        assert_eq!(
            message(check_weighted_voting_tally_sheets(
                CREATION,
                &sheets,
                &["tallied".to_string()],
            )),
            "Approved tally sheets cannot be counted when voter-weighted voting is enabled: a \
             tally sheet reports a ballot count with no weight, so its votes would be added to \
             the weighted totals at a weight of one each. 2 approved tally sheet(s) exist for \
             this election event"
        );
    }

    #[test]
    fn tally_sheets_of_other_elections_are_ignored() {
        let sheets = [approved_sheet("other")];
        assert!(check_weighted_voting_tally_sheets(CREATION, &sheets, &["tallied".into()]).is_ok());
        assert!(check_weighted_voting_tally_sheets(CREATION, &sheets, &[]).is_ok());
    }

    #[test]
    fn published_ballots_with_an_area_weight_are_refused_once_per_area() {
        let plurality = || vec![contest("contest", CountingAlgType::PluralityAtLarge, false)];
        let styles = [
            ballot_style("north", Some(json!({"weight": 3})), plurality()),
            ballot_style("south", Some(json!({"weight": 1})), plurality()),
            ballot_style("north", Some(json!({"weight": 3})), plurality()),
            ballot_style("east", Some(json!({"weight": 7})), plurality()),
        ];
        assert_eq!(
            message(check_weighted_voting_ballot_styles(CREATION, &styles)),
            "Voter-weighted voting cannot be used while published ballots still carry an area \
             weight, because the two would multiply: north, east. This has to be corrected \
             before the ballots are published: set the weighted voting policy back to \
             areas-weighted voting, which makes the weight editable again, clear it on these \
             areas, set the policy to voters-weighted voting and publish. Once voting has \
             begun the ballots cannot be republished, so at this point there is no remedy left"
        );
    }

    #[test]
    fn the_default_weight_does_not_count_as_an_area_weight() {
        let plurality = || vec![contest("contest", CountingAlgType::PluralityAtLarge, false)];
        let styles = [
            ballot_style("explicit-one", Some(json!({"weight": 1})), plurality()),
            ballot_style("no-weight", Some(json!({"weight": null})), plurality()),
            ballot_style("empty-annotations", Some(json!({})), plurality()),
            ballot_style("no-annotations", None, plurality()),
        ];
        assert!(check_weighted_voting_ballot_styles(CREATION, &styles).is_ok());
    }

    #[test]
    fn contests_counted_with_another_algorithm_are_refused_once_each() {
        let styles = [
            ballot_style(
                "north",
                None,
                vec![
                    contest("borda-contest", CountingAlgType::Borda, false),
                    contest(
                        "plurality-contest",
                        CountingAlgType::PluralityAtLarge,
                        false,
                    ),
                ],
            ),
            ballot_style(
                "south",
                None,
                vec![
                    contest("borda-contest", CountingAlgType::Borda, false),
                    contest("runoff-contest", CountingAlgType::InstantRunoff, false),
                ],
            ),
        ];
        assert_eq!(
            message(check_weighted_voting_ballot_styles(CREATION, &styles)),
            "Voter-weighted voting only supports the plurality-at-large counting algorithm. \
             These contests use another algorithm: borda-contest, runoff-contest"
        );
    }

    #[test]
    fn an_acclaimed_contest_does_not_need_a_supported_algorithm() {
        let styles = [ballot_style(
            "north",
            None,
            vec![
                contest("acclaimed-contest", CountingAlgType::Borda, true),
                contest(
                    "plurality-contest",
                    CountingAlgType::PluralityAtLarge,
                    false,
                ),
            ],
        )];
        assert!(check_weighted_voting_ballot_styles(CREATION, &styles).is_ok());
    }

    #[test]
    fn area_weights_are_reported_before_unsupported_algorithms() {
        let styles = [ballot_style(
            "north",
            Some(json!({"weight": 2})),
            vec![contest("borda-contest", CountingAlgType::Borda, false)],
        )];
        assert!(
            message(check_weighted_voting_ballot_styles(CREATION, &styles))
                .starts_with("Voter-weighted voting cannot be used while published ballots")
        );
    }
}
