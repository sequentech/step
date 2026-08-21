// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Whether a bundle can be imported, and whether it should be.
//!
//! Pure: no database, no IO, no clock. That is what lets the same code answer in
//! a browser before an upload and on the server before a transaction, and it is
//! the constraint to keep in mind when adding a rule. Anything needing the
//! database — does this tenant exist, is this area name already taken — belongs
//! in windmill on top of this pass, not here.
//!
//! The rules come from two places: what the importer rejects, and what janitor
//! learned the hard way. The second kind matters most. A bundle can satisfy every
//! type in the schema and still be wrong in a way nobody notices until election
//! day — a contest on no ballot, rankings counted by plurality, an election
//! scoped to a permission label that no administrator holds. Those import
//! cleanly and fail silently, so they are checked here.

use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use strum::VariantNames;

use super::problem::{Code, Problem, Report};
use super::schema::ImportElectionEventSchema;
use crate::types::ceremonies::CountingAlgType;

/// Every value `CountingAlgType` accepts, taken from the enum itself.
///
/// **Not written out here.** It was — ten strings, copied from the enum's serde
/// renames — and a list written twice is a list that drifts: the way it fails is a
/// bundle this accepts and the importer does not, or an algorithm the platform
/// gained and this went on rejecting.
pub const COUNTING_ALGORITHMS: &[&str] = CountingAlgType::VARIANTS;

/// The algorithms `CountingAlgType::is_preferential` returns true for.
///
/// The split is load-bearing rather than cosmetic: ballot encoding follows the
/// algorithm, so a preferential contest counted by plurality imports cleanly and
/// then reads the rankings a voter entered as unordered selections.
///
/// Still spelled out, because a `&'static [&'static str]` is what the browser is
/// handed and `is_preferential` cannot be called in a const. `the_preferential_list_
/// matches_the_enum` fails the moment the two disagree, which is the drift this
/// would otherwise invite.
pub const PREFERENTIAL_ALGORITHMS: &[&str] = &[
    "instant-runoff",
    "borda",
    "borda-nauru",
    "borda-mas-madrid",
    "pairwise-beta",
    "desborda",
    "desborda2",
    "desborda3",
];

/// A contest whose ballot carries an order.
pub const PREFERENTIAL: &str = "preferential";

/// A contest whose ballot carries a set of marks.
pub const NON_PREFERENTIAL: &str = "non-preferential";

/// `IVotingType` in the Admin Portal. Rust carries `voting_type` as a free-form
/// `String`, so the portal's enum is the only authority on what it may hold.
pub const VOTING_TYPES: &[&str] = &[PREFERENTIAL, NON_PREFERENTIAL];

/// Check a bundle and report everything wrong with it.
///
/// Never stops at the first problem: fixing a configuration one error per run is
/// miserable, and the caller usually wants the whole list at once.
pub fn validate(bundle: &ImportElectionEventSchema) -> Report {
    let mut report = Report::default();

    check_identity(bundle, &mut report);
    check_references(bundle, &mut report);
    check_area_tree(bundle, &mut report);
    check_contests(bundle, &mut report);
    check_ballot_coverage(bundle, &mut report);
    check_permission_labels(bundle, &mut report);
    check_unique_ids(bundle, &mut report);

    report
}

fn check_identity(bundle: &ImportElectionEventSchema, report: &mut Report) {
    // The schema carries tenant_id as a String so the module stays WASM-safe;
    // this is where the format check it lost comes back, as a readable problem
    // rather than an opaque serde error.
    if bundle.tenant_id.trim().is_empty() {
        report.push(Problem::error(
            Code::MissingField,
            "tenant_id",
            "the bundle has no tenant id",
        ));
    } else if !looks_like_uuid(&bundle.tenant_id) {
        report.push(Problem::error(
            Code::InvalidValue,
            "tenant_id",
            format!("'{}' is not a UUID", bundle.tenant_id),
        ));
    }

    let event = &bundle.election_event;
    if event.id.trim().is_empty() {
        report.push(Problem::error(
            Code::MissingField,
            "election_event.id",
            "the election event has no id",
        ));
    }
    if event.encryption_protocol.trim().is_empty() {
        report.push(Problem::error(
            Code::MissingField,
            "election_event.encryption_protocol",
            "the election event has no encryption protocol",
        ));
    }

    if bundle.elections.is_empty() {
        report.push(Problem::error(
            Code::MissingField,
            "elections",
            "an election event needs at least one election",
        ));
    }
    if bundle.areas.is_empty() {
        report.push(Problem::error(
            Code::MissingField,
            "areas",
            "an election event needs at least one area; every voter belongs to one",
        ));
    }
}

fn check_references(bundle: &ImportElectionEventSchema, report: &mut Report) {
    let election_ids: HashSet<&str> =
        bundle.elections.iter().map(|e| e.id.as_str()).collect();
    let contest_ids: HashSet<&str> =
        bundle.contests.iter().map(|c| c.id.as_str()).collect();
    let area_ids: HashSet<&str> =
        bundle.areas.iter().map(|a| a.id.as_str()).collect();

    for (index, contest) in bundle.contests.iter().enumerate() {
        if !election_ids.contains(contest.election_id.as_str()) {
            report.push(
                Problem::error(
                    Code::DanglingReference,
                    format!("contests[{index}].election_id"),
                    "points at an election that is not in the bundle",
                )
                .about(contest.external_id.as_deref()),
            );
        }
    }

    for (index, candidate) in bundle.candidates.iter().enumerate() {
        match candidate.contest_id.as_deref() {
            None => report.push(
                Problem::error(
                    Code::MissingField,
                    format!("candidates[{index}].contest_id"),
                    "a candidate must belong to a contest",
                )
                .about(candidate.external_id.as_deref()),
            ),
            Some(id) if !contest_ids.contains(id) => report.push(
                Problem::error(
                    Code::DanglingReference,
                    format!("candidates[{index}].contest_id"),
                    "points at a contest that is not in the bundle",
                )
                .about(candidate.external_id.as_deref()),
            ),
            Some(_) => {}
        }
    }

    for (index, link) in bundle.area_contests.iter().enumerate() {
        if !area_ids.contains(link.area_id.as_str()) {
            report.push(Problem::error(
                Code::DanglingReference,
                format!("area_contests[{index}].area_id"),
                "points at an area that is not in the bundle",
            ));
        }
        if !contest_ids.contains(link.contest_id.as_str()) {
            report.push(Problem::error(
                Code::DanglingReference,
                format!("area_contests[{index}].contest_id"),
                "points at a contest that is not in the bundle",
            ));
        }
    }
}

fn check_area_tree(bundle: &ImportElectionEventSchema, report: &mut Report) {
    let parents: HashMap<&str, Option<&str>> = bundle
        .areas
        .iter()
        .map(|area| (area.id.as_str(), area.parent_id.as_deref()))
        .collect();

    for (index, area) in bundle.areas.iter().enumerate() {
        let Some(parent) = area.parent_id.as_deref() else {
            continue;
        };

        if !parents.contains_key(parent) {
            report.push(Problem::error(
                Code::DanglingReference,
                format!("areas[{index}].parent_id"),
                "points at a parent area that is not in the bundle",
            ));
            continue;
        }

        // An infinite tree hangs the Admin Portal rather than failing the import.
        let mut seen: HashSet<&str> = HashSet::from([area.id.as_str()]);
        let mut cursor = Some(parent);
        while let Some(current) = cursor {
            if !seen.insert(current) {
                report.push(Problem::error(
                    Code::AreaCycle,
                    format!("areas[{index}].parent_id"),
                    format!(
                        "area '{}' is part of a parent cycle",
                        area.name.as_deref().unwrap_or(&area.id)
                    ),
                ));
                break;
            }
            cursor = parents.get(current).copied().flatten();
        }
    }
}

fn check_contests(bundle: &ImportElectionEventSchema, report: &mut Report) {
    let mut candidates_per_contest: HashMap<&str, usize> = HashMap::new();
    for candidate in &bundle.candidates {
        if let Some(contest_id) = candidate.contest_id.as_deref() {
            *candidates_per_contest.entry(contest_id).or_insert(0) += 1;
        }
    }

    for (index, contest) in bundle.contests.iter().enumerate() {
        let path = |field: &str| format!("contests[{index}].{field}");
        let about = contest.external_id.as_deref();

        let min_votes = contest.min_votes;
        let max_votes = contest.max_votes;
        let winners = contest.winning_candidates_num;

        // Present, and not below zero. **Only the relations were checked**, and every
        // one of them is a comparison — `min > max`, `winners > available` — so a
        // contest asking for -1 winners satisfied all of them and imported. The
        // platform's column is a signed integer and takes it; what it means is
        // nothing, and the count reaches ballot encoding and the tally.
        for (field, value) in [
            ("min_votes", min_votes),
            ("max_votes", max_votes),
            ("winning_candidates_num", winners),
        ] {
            match value {
                None => report.push(
                    Problem::error(
                        Code::MissingField,
                        path(field),
                        format!("a contest needs {field}"),
                    )
                    .about(about),
                ),
                Some(number) if number < 0 => report.push(
                    Problem::error(
                        Code::InvalidValue,
                        path(field),
                        format!("{field} is {number}, and a count cannot be negative"),
                    )
                    .about(about),
                ),
                Some(_) => {}
            }
        }

        if let (Some(min), Some(max)) = (min_votes, max_votes) {
            if min > max {
                report.push(
                    Problem::error(
                        Code::ContestArithmetic,
                        path("min_votes"),
                        format!("min_votes {min} is above max_votes {max}"),
                    )
                    .about(about),
                );
            }
        }

        let voting_type = contest.voting_type.as_deref();
        match voting_type {
            Some(value) if VOTING_TYPES.contains(&value) => {}
            other => report.push(
                Problem::error(
                    Code::InvalidValue,
                    path("voting_type"),
                    format!(
                        "{} is not a voting type; expected one of {}",
                        other
                            .map(|v| format!("'{v}'"))
                            .unwrap_or("nothing".into()),
                        VOTING_TYPES.join(", ")
                    ),
                )
                .about(about),
            ),
        }

        let algorithm = contest.counting_algorithm.as_deref();
        match algorithm {
            // Matched exactly, then parsed. `CountingAlgType::from_str` is
            // `ascii_case_insensitive`, so parsing *first* would accept `Borda` —
            // which Rust reads correctly and `ICountingAlgorithm` in `ui-core`, which
            // compares the string, does not. So: the enum says which values exist,
            // and this says they have to be spelled the way the platform spells them.
            Some(value) if COUNTING_ALGORITHMS.contains(&value) => {
                let preferential = CountingAlgType::from_str(value)
                    .map(|algorithm| algorithm.is_preferential())
                    .unwrap_or(false);
                match voting_type {
                    Some(PREFERENTIAL) if !preferential => report.push(
                        Problem::error(
                            Code::TallyMismatch,
                            path("counting_algorithm"),
                            format!(
                                "a preferential contest counted by '{value}', which \
                                 ignores rankings"
                            ),
                        )
                        .about(about),
                    ),
                    Some(NON_PREFERENTIAL) if preferential => report.push(
                        Problem::error(
                            Code::TallyMismatch,
                            path("counting_algorithm"),
                            format!(
                                "a non-preferential contest counted by '{value}', \
                                 which needs ranked ballots"
                            ),
                        )
                        .about(about),
                    ),
                    _ => {}
                }
            }
            other => report.push(
                Problem::error(
                    Code::InvalidValue,
                    path("counting_algorithm"),
                    format!(
                        "{} is not a counting algorithm; expected one of {}",
                        other
                            .map(|v| format!("'{v}'"))
                            .unwrap_or("nothing".into()),
                        COUNTING_ALGORITHMS.join(", ")
                    ),
                )
                .about(about),
            ),
        }

        check_presentation_policies(contest, &path, about, report);

        let available = candidates_per_contest
            .get(contest.id.as_str())
            .copied()
            .unwrap_or(0);
        if available == 0 {
            report.push(
                Problem::warning(
                    Code::BallotCoverage,
                    path("id"),
                    "the contest has no candidates, so nobody can vote in it",
                )
                .about(about),
            );
        } else {
            if let Some(winners) = winners {
                if winners > available as i64 {
                    report.push(
                        Problem::error(
                            Code::ContestArithmetic,
                            path("winning_candidates_num"),
                            format!(
                                "elects {winners} of {available} candidates"
                            ),
                        )
                        .about(about),
                    );
                }
            }
            if let Some(max) = max_votes {
                if max > available as i64 {
                    report.push(
                        Problem::error(
                            Code::ContestArithmetic,
                            path("max_votes"),
                            format!("allows {max} selections among {available} candidates"),
                        )
                        .about(about),
                    );
                }
            }
        }
    }
}

/// Every `presentation.*_policy` against the values the platform has.
///
/// A policy the Admin Portal does not know imports without complaint and then
/// behaves as whatever the voting portal falls back to — which is a ballot
/// behaving in a way nobody chose, discovered by a voter. Both hand-written
/// mappings that fed this format got at least one of them wrong, so it is worth
/// checking the bundle rather than trusting whoever produced it.
fn check_presentation_policies(
    contest: &crate::types::hasura::core::Contest,
    path: &impl Fn(&str) -> String,
    about: Option<&str>,
    report: &mut Report,
) {
    use crate::election_config::policy::{
        BlankVote, CandidatesOrder, DuplicatedRank, InvalidVote, OverVote,
        PolicyValue, PreferenceGaps, UnderVote,
    };
    use strum::IntoEnumIterator;

    let Some(presentation) = contest
        .presentation
        .as_ref()
        .and_then(|value| value.as_object())
    else {
        return;
    };

    fn known<T: PolicyValue + IntoEnumIterator>() -> Vec<&'static str> {
        T::iter().map(PolicyValue::as_str).collect()
    }

    let checks: Vec<(&str, Vec<&'static str>)> = vec![
        ("over_vote_policy", known::<OverVote>()),
        ("blank_vote_policy", known::<BlankVote>()),
        ("under_vote_policy", known::<UnderVote>()),
        ("invalid_vote_policy", known::<InvalidVote>()),
        ("duplicated_rank_policy", known::<DuplicatedRank>()),
        ("preference_gaps_policy", known::<PreferenceGaps>()),
        ("candidates_order", known::<CandidatesOrder>()),
    ];

    for (key, allowed) in checks {
        // Absent is fine: the platform has its own default for each of these,
        // and a bundle is not obliged to state one.
        let Some(value) = presentation.get(key) else {
            continue;
        };
        if value.is_null() {
            continue;
        }
        let Some(text) = value.as_str() else {
            report.push(
                Problem::error(
                    Code::InvalidValue,
                    path(&format!("presentation.{key}")),
                    format!("{key} should be text, and is {value}"),
                )
                .about(about),
            );
            continue;
        };
        if !allowed.contains(&text) {
            report.push(
                Problem::error(
                    Code::InvalidValue,
                    path(&format!("presentation.{key}")),
                    format!(
                        "'{text}' is not a {key}; expected one of {}",
                        allowed.join(", ")
                    ),
                )
                .about(about),
            );
        }
    }
}

fn check_ballot_coverage(
    bundle: &ImportElectionEventSchema,
    report: &mut Report,
) {
    // Neither of these breaks the import. Both mean somebody's vote is missing on
    // election day, which is the most expensive time to find out.
    let linked_contests: HashSet<&str> = bundle
        .area_contests
        .iter()
        .map(|link| link.contest_id.as_str())
        .collect();
    let linked_areas: HashSet<&str> = bundle
        .area_contests
        .iter()
        .map(|link| link.area_id.as_str())
        .collect();
    let parents: HashSet<&str> = bundle
        .areas
        .iter()
        .filter_map(|area| area.parent_id.as_deref())
        .collect();

    for (index, contest) in bundle.contests.iter().enumerate() {
        if !linked_contests.contains(contest.id.as_str()) {
            report.push(
                Problem::warning(
                    Code::BallotCoverage,
                    format!("contests[{index}]"),
                    "appears on no area's ballot, so nobody can vote in it",
                )
                .about(contest.external_id.as_deref()),
            );
        }
    }

    for (index, area) in bundle.areas.iter().enumerate() {
        // A parent area is a grouping; only leaf areas carry a ballot.
        if linked_areas.contains(area.id.as_str())
            || parents.contains(area.id.as_str())
        {
            continue;
        }
        report.push(Problem::warning(
            Code::BallotCoverage,
            format!("areas[{index}]"),
            format!(
                "area '{}' has no contests and is not a parent of another area, so \
                 its voters would see an empty ballot",
                area.name.as_deref().unwrap_or(&area.id)
            ),
        ));
    }
}

fn check_permission_labels(
    bundle: &ImportElectionEventSchema,
    report: &mut Report,
) {
    // Hasura filters election and report on
    //   permission_label IS NULL OR permission_label IN X-Hasura-Permission-Labels
    // so an entity carrying a label is invisible to every administrator who does
    // not hold it — including whoever runs the import. The bundle cannot know who
    // holds what, so this is a warning naming the label rather than a refusal.
    // Per collection, because `Problem::path` is where the thing *is*: a front end
    // turns it into a wizard step or a spreadsheet cell. One warning pointing at
    // `elections[]` for a label that only a report carries sends somebody to a screen
    // where there is nothing to change.
    let mut from_elections: Vec<String> = Vec::new();
    let mut from_reports: Vec<String> = Vec::new();

    for election in &bundle.elections {
        if let Some(label) = election.permission_label.as_deref() {
            if !label.trim().is_empty()
                && !from_elections.iter().any(|seen| seen == label)
            {
                from_elections.push(label.to_string());
            }
        }
    }
    for report_definition in &bundle.reports {
        for label in report_definition.permission_label.iter().flatten() {
            if !label.trim().is_empty()
                && !from_reports.iter().any(|seen| seen == label)
            {
                from_reports.push(label.clone());
            }
        }
    }

    for (path, labels) in [
        ("elections[].permission_label", &from_elections),
        ("reports[].permission_label", &from_reports),
    ] {
        if labels.is_empty() {
            continue;
        }
        report.push(Problem::warning(
            Code::PermissionLabel,
            path,
            format!(
                "permission labels in use: {}. Anything carrying a label is hidden \
                 from every administrator without it, so whoever imports this needs \
                 one of them on their own 'permission_labels' attribute or the Admin \
                 Portal will show them an empty list.",
                labels.join(", ")
            ),
        ));
    }
}

fn check_unique_ids(bundle: &ImportElectionEventSchema, report: &mut Report) {
    // Two entities sharing an id means one silently overwrites the other. The ids
    // are checked across every collection at once, not per collection: a contest
    // and an area sharing one collides just as badly.
    let groups: [(&str, Vec<&str>); 5] = [
        (
            "elections",
            bundle.elections.iter().map(|e| e.id.as_str()).collect(),
        ),
        (
            "contests",
            bundle.contests.iter().map(|c| c.id.as_str()).collect(),
        ),
        (
            "candidates",
            bundle.candidates.iter().map(|c| c.id.as_str()).collect(),
        ),
        (
            "areas",
            bundle.areas.iter().map(|a| a.id.as_str()).collect(),
        ),
        (
            "area_contests",
            bundle.area_contests.iter().map(|l| l.id.as_str()).collect(),
        ),
    ];

    let mut seen: HashMap<&str, &str> = HashMap::new();
    for (kind, ids) in &groups {
        for id in ids {
            if let Some(previous) = seen.insert(id, kind) {
                report.push(Problem::error(
                    Code::DuplicateId,
                    *kind,
                    format!("id {id} is also used by {previous}"),
                ));
            }
        }
    }
}

/// Whether a string is shaped like a hyphenated UUID.
///
/// Deliberately not `Uuid::parse_str`: that crate is only enabled by the
/// `keycloak` feature here, and pulling it into `default_features` would put
/// `getrandom` in the WASM build to check a string's shape.
fn looks_like_uuid(value: &str) -> bool {
    let groups: Vec<&str> = value.split('-').collect();
    groups.len() == 5
        && [8usize, 4, 4, 4, 12]
            .iter()
            .zip(&groups)
            .all(|(expected, group)| {
                group.len() == *expected
                    && group
                        .chars()
                        .all(|character| character.is_ascii_hexdigit())
            })
}
