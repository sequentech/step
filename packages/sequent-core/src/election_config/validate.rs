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

use super::problem::{Code, Problem, Report};
use super::schema::ImportElectionEventSchema;

/// `CountingAlgType` in `crate::types::ceremonies`, by its serde rename.
pub const COUNTING_ALGORITHMS: &[&str] = &[
    "plurality-at-large",
    "instant-runoff",
    "borda-nauru",
    "borda",
    "borda-mas-madrid",
    "pairwise-beta",
    "desborda3",
    "desborda2",
    "desborda",
    "cumulative",
];

/// The algorithms `CountingAlgType::is_preferential` returns true for.
///
/// The split is load-bearing rather than cosmetic: ballot encoding follows the
/// algorithm, so a preferential contest counted by plurality imports cleanly and
/// then reads the rankings a voter entered as unordered selections.
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

/// `IVotingType` in the Admin Portal. Rust carries `voting_type` as a free-form
/// `String`, so the portal's enum is the only authority on what it may hold.
pub const VOTING_TYPES: &[&str] = &["preferential", "non-preferential"];

/// `ITieBreakingPolicy` in `ui-core`.
///
/// Two values, and the difference matters: `random` settles the tie inside the
/// tally, and `external-procedure` leaves it in the result for whoever runs the
/// election to settle under their own rules.
pub const TIE_BREAKING_POLICIES: &[&str] = &["random", "external-procedure"];

/// `ECollapsibleLists`, for a contest whose candidates are grouped into lists.
pub const COLLAPSIBLE_LISTS: &[&str] =
    &["disabled", "enabled-expanded", "enabled-collapsed"];

/// `EEnableCheckableLists` — what a voter may tick when candidates are grouped.
pub const CHECKABLE_LISTS: &[&str] = &[
    "disabled",
    "allow-selecting-candidates",
    "allow-selecting-lists",
    "allow-selecting-candidates-and-lists",
];

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

        for (field, value) in [
            ("min_votes", min_votes),
            ("max_votes", max_votes),
            ("winning_candidates_num", winners),
        ] {
            if value.is_none() {
                report.push(
                    Problem::error(
                        Code::MissingField,
                        path(field),
                        format!("a contest needs {field}"),
                    )
                    .about(about),
                );
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
            Some(value) if COUNTING_ALGORITHMS.contains(&value) => {
                let preferential = PREFERENTIAL_ALGORITHMS.contains(&value);
                match voting_type {
                    Some("preferential") if !preferential => report.push(
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
                    Some("non-preferential") if preferential => report.push(
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
        check_tie_breaking(contest, &path, about, report);
        check_layout(contest, &path, about, report);

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
/// The tie-breaking policy, which lives under `tally_configuration`.
///
/// Not in `check_presentation_policies` because it is not on the presentation:
/// how a tie is settled is a counting decision, and the Admin Portal keeps it in
/// the same place.
fn check_tie_breaking(
    contest: &crate::types::hasura::core::Contest,
    path: &impl Fn(&str) -> String,
    about: Option<&str>,
    report: &mut Report,
) {
    let Some(value) = contest
        .tally_configuration
        .as_ref()
        .and_then(|configuration| configuration.as_object())
        .and_then(|configuration| configuration.get("tie_breaking_policy"))
    else {
        return;
    };
    if value.is_null() {
        return;
    }
    match value.as_str() {
        Some(text) if TIE_BREAKING_POLICIES.contains(&text) => {}
        other => report.push(
            Problem::error(
                Code::InvalidValue,
                path("tally_configuration.tie_breaking_policy"),
                format!(
                    "{} is not a tie-breaking policy; expected one of {}",
                    other
                        .map(|v| format!("'{v}'"))
                        .unwrap_or("a non-string".into()),
                    TIE_BREAKING_POLICIES.join(", ")
                ),
            )
            .about(about),
        ),
    }
}

/// The numbers that describe how a ballot is laid out.
///
/// Nothing here changes what a vote *means*, which is why these are warnings
/// rather than errors where a value is merely strange, and errors only where the
/// portal would have nothing sensible to draw.
fn check_layout(
    contest: &crate::types::hasura::core::Contest,
    path: &impl Fn(&str) -> String,
    about: Option<&str>,
    report: &mut Report,
) {
    let Some(presentation) = contest
        .presentation
        .as_ref()
        .and_then(|value| value.as_object())
    else {
        return;
    };

    if let Some(columns) = presentation
        .get("columns")
        .and_then(serde_json::Value::as_i64)
    {
        // Zero columns is a contest nothing can be drawn in; the portal falls
        // back, but to something nobody chose. Above four is legal and unusable
        // on a phone, which is where most voters are.
        if columns < 1 {
            report.push(
                Problem::error(
                    Code::InvalidValue,
                    path("presentation.columns"),
                    format!("{columns} columns is not a layout"),
                )
                .about(about),
            );
        } else if columns > 4 {
            report.push(
                Problem::warning(
                    Code::InvalidValue,
                    path("presentation.columns"),
                    format!(
                        "{columns} columns will be unreadable on a phone, which \
                         is how most voters vote"
                    ),
                )
                .about(about),
            );
        }
    }

    if let Some(cap) = presentation
        .get("max_selections_per_type")
        .and_then(serde_json::Value::as_i64)
    {
        if cap < 0 {
            report.push(
                Problem::error(
                    Code::InvalidValue,
                    path("presentation.max_selections_per_type"),
                    format!("{cap} is not a number of selections"),
                )
                .about(about),
            );
        } else if cap > 0 && contest.max_votes.is_some_and(|max| cap > max) {
            // A per-type cap above the contest's own maximum can never bind, so
            // somebody who set it believes a limit is in force that is not.
            report.push(
                Problem::warning(
                    Code::ContestArithmetic,
                    path("presentation.max_selections_per_type"),
                    format!(
                        "a cap of {cap} per type never applies in a contest \
                         where a voter may choose {} in total",
                        contest.max_votes.unwrap_or(0)
                    ),
                )
                .about(about),
            );
        }
    }
}

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
        // Not enums in this crate — their value space belongs to the platform
        // and lives in `ui-core`, so it is a list here rather than a third copy
        // of a type. Checked all the same: an unrecognised one imports and then
        // renders as whatever the portal's fallback is.
        ("collapsible_lists", COLLAPSIBLE_LISTS.to_vec()),
        ("enable_checkable_lists", CHECKABLE_LISTS.to_vec()),
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
    let directly_linked: HashSet<&str> = bundle
        .area_contests
        .iter()
        .map(|link| link.area_id.as_str())
        .collect();
    let parents: HashSet<&str> = bundle
        .areas
        .iter()
        .filter_map(|area| area.parent_id.as_deref())
        .collect();

    // An area also votes on everything its ancestors do. That is the platform's
    // rule, not an assumption: when a ballot publication is generated, windmill
    // walks the path from the root down to each area and gathers every
    // `area_contest` on the way — see
    // `crate::ballot_style::elections_contests_for_area`, which is that walk and
    // is what the Election Architect's preview calls.
    //
    // This check used to look only at direct links, and so warned that a child
    // area's voters "would see an empty ballot" when in fact they get every
    // contest assigned to their parent. It was found by previewing a two-area
    // plan and seeing two ballots next to a warning saying there would be one.
    let mut parent_of: HashMap<&str, &str> = HashMap::new();
    for area in &bundle.areas {
        if let Some(parent) = area.parent_id.as_deref() {
            parent_of.insert(area.id.as_str(), parent);
        }
    }
    let inherits_a_contest = |area_id: &str| -> bool {
        let mut at = area_id;
        // Bounded by the number of areas, so a cycle cannot spin here. A cycle is
        // its own error, reported by `check_area_tree`.
        for _ in 0..bundle.areas.len() {
            if directly_linked.contains(at) {
                return true;
            }
            match parent_of.get(at) {
                Some(parent) => at = parent,
                None => return false,
            }
        }
        false
    };

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
        if parents.contains(area.id.as_str())
            || inherits_a_contest(area.id.as_str())
        {
            continue;
        }
        report.push(Problem::warning(
            Code::BallotCoverage,
            format!("areas[{index}]"),
            format!(
                "area '{}' votes on no contest — not its own and none inherited \
                 from an area it sits inside — so its voters would see an empty \
                 ballot",
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
    let mut labels: Vec<String> = Vec::new();

    for election in &bundle.elections {
        if let Some(label) = election.permission_label.as_deref() {
            if !label.trim().is_empty()
                && !labels.iter().any(|seen| seen == label)
            {
                labels.push(label.to_string());
            }
        }
    }
    for report_definition in &bundle.reports {
        for label in report_definition.permission_label.iter().flatten() {
            if !label.trim().is_empty()
                && !labels.iter().any(|seen| seen == label)
            {
                labels.push(label.clone());
            }
        }
    }

    if !labels.is_empty() {
        report.push(Problem::warning(
            Code::PermissionLabel,
            "elections[].permission_label",
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
