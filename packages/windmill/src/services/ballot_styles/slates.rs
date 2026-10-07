// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Slate checks a ballot publication must pass, on top of the shared rules in
//! `sequent_core::election_config::slates`.

use super::ballot_publication::BallotPublicationValidationError;
use anyhow::{Context, Result};
use sequent_core::ballot::BallotStyle;
use sequent_core::election_config::slates::{
    canonicalize, check_ballot_style, check_election, parse, SLATES_ANNOTATION,
};
use sequent_core::election_config::Problem;
use sequent_core::types::hasura::core::{Candidate, Contest, Election};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

fn slates_path(election_id: &str) -> String {
    format!("election {election_id}: {SLATES_ANNOTATION}")
}

fn reason(problem: &Problem) -> String {
    format!("{}: {}", problem.path, problem.message)
}

fn reject_if_any(reasons: BTreeSet<String>) -> Result<()> {
    if reasons.is_empty() {
        Ok(())
    } else {
        Err(BallotPublicationValidationError::new(reasons.into_iter().collect()).into())
    }
}

/// Check every published election's slates against its whole inventory.
///
/// Run before any area's ballot style is generated: an area only sees the
/// contests it votes on, so a reference to a contest outside the election can
/// only be told apart from an out-of-area contest here.
pub fn validate_elections_slates(
    election_ids: &[String],
    elections: &HashMap<String, Election>,
    contests: &[Contest],
    candidates: &[Candidate],
) -> Result<()> {
    let reasons = election_ids
        .iter()
        .filter_map(|election_id| elections.get(election_id))
        .flat_map(|election| {
            check_election(election, contests, candidates, &slates_path(&election.id))
        })
        .map(|problem| reason(&problem))
        .collect();
    reject_if_any(reasons)
}

fn parse_ballot_styles(publication: &Value) -> Result<Vec<BallotStyle>> {
    publication
        .as_array()
        .context("Ballot publication must be an array of ballot styles")?
        .iter()
        .map(|ballot_style| {
            serde_json::from_value(ballot_style.clone()).context("Can't read stored ballot style")
        })
        .collect()
}

/// Check the slates stored in a generated publication before it goes live.
///
/// A draft generated before slates were validated, or edited directly, never
/// went through [`validate_elections_slates`]. Each stored ballot style is
/// checked as a voter will receive it, and a contest it does not carry must
/// still be one of its election's contests.
pub fn validate_publication_slates(publication: &Value, contests: &[Contest]) -> Result<()> {
    let mut election_contest_ids: HashMap<&str, HashSet<&str>> = HashMap::new();
    for contest in contests {
        election_contest_ids
            .entry(contest.election_id.as_str())
            .or_default()
            .insert(contest.id.as_str());
    }

    let mut reasons = BTreeSet::new();
    for ballot_style in parse_ballot_styles(publication)? {
        let path = slates_path(&ballot_style.election_id);
        let problems = check_ballot_style(&ballot_style, &path);
        let is_readable = problems.is_empty();
        reasons.extend(problems.iter().map(reason));
        if !is_readable {
            continue;
        }
        let Some(config) = ballot_style
            .election_annotations
            .as_ref()
            .and_then(|annotations| annotations.get(SLATES_ANNOTATION))
            .and_then(|text| parse(text, &path).ok())
        else {
            continue;
        };
        let known = election_contest_ids.get(ballot_style.election_id.as_str());
        for slate in &config.slates {
            for contest_id in slate.members.keys() {
                let in_style = ballot_style
                    .contests
                    .iter()
                    .any(|contest| &contest.id == contest_id);
                let in_election = known.is_some_and(|ids| ids.contains(contest_id.as_str()));
                if !in_style && !in_election {
                    reasons.insert(format!(
                        "{path}: slate '{}' lists contest {contest_id}, which is not in the election",
                        slate.id
                    ));
                }
            }
        }
    }
    reject_if_any(reasons)
}

/// The slate configuration each election publishes, in canonical form.
fn published_slates(publication: &Value) -> Result<BTreeMap<String, Option<String>>> {
    let mut slates = BTreeMap::new();
    for ballot_style in parse_ballot_styles(publication)? {
        let annotation = ballot_style
            .election_annotations
            .as_ref()
            .and_then(|annotations| annotations.get(SLATES_ANNOTATION))
            .map(|text| {
                canonicalize(text, &slates_path(&ballot_style.election_id))
                    .unwrap_or_else(|_| text.clone())
            });
        slates.entry(ballot_style.election_id).or_insert(annotation);
    }
    Ok(slates)
}

/// Reject a publication that changes the slates of an election whose voting
/// has started. `previous` holds the live ballot styles of started elections
/// only.
pub fn validate_slates_unchanged(previous: &Value, current: &Value) -> Result<()> {
    let previous = published_slates(previous)?;
    let current = published_slates(current)?;
    let reasons = current
        .iter()
        .filter(|(election_id, slates)| {
            previous
                .get(*election_id)
                .is_some_and(|published| published != *slates)
        })
        .map(|(election_id, _)| {
            format!(
                "{}: the slate configuration changed after voting started for this election. Restore the published configuration before publishing again.",
                slates_path(election_id)
            )
        })
        .collect();
    reject_if_any(reasons)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const ELECTION: &str = "election-officers";
    const PRESIDENT: &str = "contest-president";
    const TRUSTEES: &str = "contest-trustees";

    fn slates() -> Value {
        json!({
            "version": 1,
            "slates": [{
                "id": "forward-together",
                "name": {"en": "Forward Together"},
                "members": {PRESIDENT: ["president-a"], TRUSTEES: ["trustee-a"]},
            }],
        })
    }

    fn contests() -> Vec<Contest> {
        serde_json::from_value(json!([
            {
                "id": PRESIDENT, "tenant_id": "tenant", "election_event_id": "event",
                "election_id": ELECTION, "max_votes": 1,
                "counting_algorithm": "plurality-at-large"
            },
            {
                "id": TRUSTEES, "tenant_id": "tenant", "election_event_id": "event",
                "election_id": ELECTION, "max_votes": 3,
                "counting_algorithm": "plurality-at-large"
            },
        ]))
        .unwrap()
    }

    fn candidates() -> Vec<Candidate> {
        serde_json::from_value(json!([
            {
                "id": "president-a", "tenant_id": "tenant", "election_event_id": "event",
                "contest_id": PRESIDENT
            },
            {
                "id": "trustee-a", "tenant_id": "tenant", "election_event_id": "event",
                "contest_id": TRUSTEES
            },
        ]))
        .unwrap()
    }

    fn elections(annotations: Value) -> HashMap<String, Election> {
        let election: Election = serde_json::from_value(json!({
            "id": ELECTION, "tenant_id": "tenant", "election_event_id": "event",
            "annotations": annotations,
        }))
        .unwrap();
        HashMap::from([(election.id.clone(), election)])
    }

    /// One area's stored ballot style, carrying the trustees contest only.
    fn publication(slates: Option<&Value>) -> Value {
        let annotations = match slates {
            Some(slates) => json!({SLATES_ANNOTATION: slates.to_string()}),
            None => json!({}),
        };
        json!([{
            "id": "style", "tenant_id": "tenant", "election_event_id": "event",
            "election_id": ELECTION, "area_id": "area",
            "election_annotations": annotations,
            "contests": [{
                "id": TRUSTEES, "tenant_id": "tenant", "election_event_id": "event",
                "election_id": ELECTION, "max_votes": 3, "min_votes": 0,
                "winning_candidates_num": 3, "is_encrypted": true,
                "counting_algorithm": "plurality-at-large",
                "candidates": [{
                    "id": "trustee-a", "tenant_id": "tenant",
                    "election_event_id": "event", "election_id": ELECTION,
                    "contest_id": TRUSTEES
                }],
            }],
        }])
    }

    fn reasons(result: Result<()>) -> Vec<String> {
        result
            .unwrap_err()
            .downcast_ref::<BallotPublicationValidationError>()
            .expect("a publication validation error")
            .reasons
            .clone()
    }

    #[test]
    fn elections_without_slates_are_published_as_before() {
        let ids = vec![ELECTION.to_string()];
        assert!(
            validate_elections_slates(&ids, &elections(json!({})), &contests(), &candidates())
                .is_ok()
        );
        assert!(validate_publication_slates(&publication(None), &contests()).is_ok());
        assert!(validate_slates_unchanged(&publication(None), &publication(None)).is_ok());
    }

    #[test]
    fn valid_slates_are_published() {
        let ids = vec![ELECTION.to_string()];
        let annotations = json!({SLATES_ANNOTATION: slates().to_string()});
        assert!(validate_elections_slates(
            &ids,
            &elections(annotations),
            &contests(),
            &candidates()
        )
        .is_ok());
        assert!(validate_publication_slates(&publication(Some(&slates())), &contests()).is_ok());
    }

    #[test]
    fn generation_rejects_a_missing_candidate() {
        let mut slates = slates();
        slates["slates"][0]["members"][TRUSTEES] = json!(["trustee-gone"]);
        let annotations = json!({SLATES_ANNOTATION: slates.to_string()});
        let reasons = reasons(validate_elections_slates(
            &[ELECTION.to_string()],
            &elections(annotations),
            &contests(),
            &candidates(),
        ));
        assert_eq!(reasons.len(), 1);
        assert!(reasons[0].starts_with("election election-officers: sequent.slates"));
    }

    #[test]
    fn publishing_rejects_a_stored_contest_outside_the_election() {
        let mut slates = slates();
        slates["slates"][0]["members"]["contest-elsewhere"] = json!(["elsewhere-a"]);
        let reasons = reasons(validate_publication_slates(
            &publication(Some(&slates)),
            &contests(),
        ));
        assert_eq!(reasons.len(), 1);
        assert!(reasons[0].contains("contest-elsewhere"));
    }

    #[test]
    fn publishing_rejects_a_stored_member_missing_from_its_contest() {
        let mut slates = slates();
        slates["slates"][0]["members"][TRUSTEES] = json!(["trustee-gone"]);
        assert!(!reasons(validate_publication_slates(
            &publication(Some(&slates)),
            &contests()
        ))
        .is_empty());
    }

    #[test]
    fn publishing_rejects_an_unreadable_stored_configuration() {
        let publication = publication(Some(&json!({"version": 2, "slates": []})));
        assert!(!reasons(validate_publication_slates(&publication, &contests())).is_empty());
    }

    #[test]
    fn slates_cannot_change_once_voting_has_started() {
        let previous = publication(Some(&slates()));
        let mut renamed = slates();
        renamed["slates"][0]["name"]["en"] = json!("Forward");

        assert!(validate_slates_unchanged(&previous, &publication(Some(&slates()))).is_ok());
        for changed in [publication(Some(&renamed)), publication(None)] {
            let reasons = reasons(validate_slates_unchanged(&previous, &changed));
            assert_eq!(reasons.len(), 1);
        }
        assert_eq!(
            reasons(validate_slates_unchanged(
                &publication(None),
                &publication(Some(&slates()))
            ))
            .len(),
            1
        );
    }

    #[test]
    fn formatting_is_not_a_change() {
        let previous = publication(Some(&slates()));
        let mut current = publication(None);
        current[0]["election_annotations"] = json!({
            SLATES_ANNOTATION: serde_json::to_string_pretty(&slates()).unwrap()
        });
        assert!(validate_slates_unchanged(&previous, &current).is_ok());
    }

    #[test]
    fn an_election_that_has_not_started_may_change_its_slates() {
        let mut renamed = slates();
        renamed["slates"][0]["name"]["en"] = json!("Forward");
        assert!(validate_slates_unchanged(&json!([]), &publication(Some(&renamed))).is_ok());
    }
}
