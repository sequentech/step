// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The Election Architect's plan, and how it becomes a bundle.
//!
//! The architect is a wizard: somebody answers questions and gets an importable
//! election event. This module is the half of it that decides what the answers
//! mean. The other half — what the questions look like — is React, in
//! `beyond/packages/election-architect`, and it contains no mapping, no CSV and no
//! zip.
//!
//! # Why this is so small
//!
//! A [`Blueprint`] does not become a bundle here. It becomes a
//! [`Workbook`] — the same rows the spreadsheet reader produces — and the existing
//! [`super::build`] takes it from there.
//!
//! That is the whole design. A wizard is not a different kind of election event;
//! it is a different way of filling in the same fields. Going through the workbook
//! shape means the architect inherits the entity templates, the deterministic ids,
//! the CSV byte shapes, the Keycloak realm handling, the archive layout and every
//! validation rule, for free and without a second copy of any of them. What is
//! left here is only what is genuinely the architect's own: its plan, the checks
//! that apply to a plan rather than to a bundle, and the three files it produces
//! that are not part of an import at all.
//!
//! # What the TypeScript version got wrong, and why
//!
//! Its output was not the importable format: `election_config.json` inside a
//! nested `official_election_setup.zip`, where the importer looks for
//! `export_election_event-<uuid>.json` at the archive root. Its scheduled-events
//! CSV was built by string interpolation, one of three hand-written copies of that
//! byte shape. It stamped `new Date()` into every entity, so no two runs of the
//! same answers produced the same file. It embedded a Keycloak realm copied from
//! one environment, which the importer takes wholesale and would have used to
//! replace whatever the target environment had provisioned. And it validated
//! nothing.
//!
//! None of those are mistakes anybody made twice. They are what happens when a
//! format is implemented a second time, which is why this implementation does not.

use std::cmp::Ordering;

use crate::election_config::paths::Cell;
use crate::election_config::problem::{Code, Problem, Report, Severity};
use crate::election_config::sheet::{Sheet, Workbook};
use crate::election_config::time::{self, Timestamp};
use serde::{Deserialize, Serialize};

/// The plan format's version.
///
/// Written into every saved plan and checked on load. A plan is a document
/// somebody spent an afternoon on; being able to say "this is from an older
/// version" beats failing to deserialize it with a serde error about a missing
/// field.
pub const BLUEPRINT_VERSION: u32 = 1;

/// What the wizard collected.
///
/// This is the artifact worth keeping. The bundle is derived from it and is
/// disposable; the plan is what somebody edits next month when a candidate
/// withdraws.
///
/// The TypeScript version had no such thing — it reconstructed the wizard's state
/// by parsing its own generated bundle back in. That loses every answer the bundle
/// has no field for (the trustee threshold, the ceremony dates, the points of
/// contact) and breaks whenever the bundle's shape changes. Saving the plan is both
/// simpler and lossless.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Blueprint {
    /// [`BLUEPRINT_VERSION`] at the time it was saved.
    pub version: u32,

    /// Stable identifier for the event, and the seed for every generated id.
    ///
    /// Not shown to voters. Two plans with the same one produce the same
    /// identifiers, which is what makes regenerating diffable.
    pub external_id: String,

    /// The event's name, per language. `en` is the fallback.
    #[serde(default)]
    pub name: Translated,

    /// BCP 47 or ISO 639-2/T codes, in the order the picker should show them.
    #[serde(default)]
    pub languages: Vec<String>,

    #[serde(default)]
    pub logo_url: Option<String>,

    #[serde(default)]
    pub contacts: Vec<Contact>,

    #[serde(default)]
    pub trustees: Vec<Trustee>,

    /// How many trustees must take part to open the tally.
    #[serde(default = "default_threshold")]
    pub trustee_threshold: u32,

    #[serde(default)]
    pub schedule: Schedule,

    /// The areas voters belong to.
    ///
    /// Empty means one ballot for everybody, and one is synthesised. See
    /// [`DEFAULT_AREA_EXTERNAL_ID`].
    #[serde(default)]
    pub areas: Vec<PlannedArea>,

    #[serde(default)]
    pub elections: Vec<PlannedElection>,

    #[serde(default)]
    pub policies: Policies,

    /// Anything the wizard has no field for. Carried, not interpreted.
    #[serde(default)]
    pub notes: String,
}

fn default_threshold() -> u32 {
    2
}

/// Text in as many languages as the plan enables.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Translated {
    /// `language code -> text`.
    #[serde(flatten)]
    pub by_language: std::collections::BTreeMap<String, String>,
}

impl Translated {
    pub fn new(english: &str) -> Self {
        let mut by_language = std::collections::BTreeMap::new();
        by_language.insert("en".to_string(), english.to_string());
        Translated { by_language }
    }

    /// The text in `language`, falling back to English and then to anything.
    ///
    /// A missing translation shows the English rather than an empty ballot line.
    /// Blank is never the right answer for a candidate's name.
    pub fn get(&self, language: &str) -> Option<&str> {
        self.by_language
            .get(language)
            .or_else(|| self.by_language.get("en"))
            .or_else(|| self.by_language.values().next())
            .map(String::as_str)
            .filter(|text| !text.is_empty())
    }

    pub fn is_empty(&self) -> bool {
        self.by_language.values().all(|text| text.is_empty())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Contact {
    pub name: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub email: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Trustee {
    pub name: String,
    #[serde(default)]
    pub email: String,
}

/// The dates an election runs to.
///
/// Each moment carries the zone it was written in, because the platform acts on
/// an instant and a wall clock is not one. See [`super::time`] — a plan that
/// says `2027-03-01T09:00` produced a `scheduled_date` the scheduler could not
/// parse, so voting never opened and nothing said why.
///
/// A plan written before zones existed still opens: a bare string reads as UTC,
/// which is what it always meant, and validation says so rather than guessing.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Schedule {
    /// When the trustees generate the election key. Not an imported event: it is
    /// something people have to attend, so it travels in the ceremony file.
    #[serde(default)]
    pub key_ceremony: Option<Timestamp>,

    #[serde(default)]
    pub voting_opens: Option<Timestamp>,

    #[serde(default)]
    pub voting_closes: Option<Timestamp>,

    #[serde(default)]
    pub tally_ceremony: Option<Timestamp>,

    /// Anything else with a date on it, for the schedule the client is handed.
    #[serde(default)]
    pub milestones: Vec<Milestone>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Milestone {
    pub event: String,
    pub date: String,
}

/// A group of voters who get the same ballot.
///
/// The name is plain text rather than translated, and that is not an oversight:
/// the voters CSV identifies a voter's area *by name*, so it is an identifier the
/// importer matches on. Two areas sharing a name would silently put voters in
/// whichever one the importer found first.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlannedArea {
    pub external_id: String,

    /// What the importer matches a voter's `area_name` against.
    #[serde(default)]
    pub name: String,

    /// The area this one sits inside, if any.
    ///
    /// A tree, because that is how districting is actually described — a local
    /// inside a region inside a state — and because the platform models it that
    /// way. A contest assigned to a parent is not automatically on its children's
    /// ballots; assignment is explicit, so that "who votes on this" is answerable
    /// by reading one list rather than walking a tree.
    #[serde(default)]
    pub parent_external_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlannedElection {
    pub external_id: String,
    #[serde(default)]
    pub name: Translated,
    #[serde(default)]
    pub contests: Vec<PlannedContest>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlannedContest {
    pub external_id: String,
    #[serde(default)]
    pub name: Translated,
    #[serde(default)]
    pub description: String,

    /// How many candidates a voter may choose.
    #[serde(default = "one")]
    pub max_votes: i64,

    /// How many candidates the contest elects.
    ///
    /// The TypeScript hard-coded this to 1 while letting `max_votes` be anything,
    /// so a "choose 3" contest silently elected one person.
    #[serde(default = "one")]
    pub winners: i64,

    #[serde(default)]
    pub candidates: Vec<PlannedCandidate>,

    /// Which areas put this contest on their ballot.
    ///
    /// Empty means every area, which is what a plan that has never thought about
    /// districting wants and what one with a single area always wants. Naming
    /// areas explicitly is how a contest becomes local to some of them.
    #[serde(default)]
    pub areas: Vec<String>,
}

fn one() -> i64 {
    1
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlannedCandidate {
    pub external_id: String,
    #[serde(default)]
    pub name: Translated,
    /// A "none of the above" option rather than a person.
    #[serde(default)]
    pub explicit_blank: bool,
    /// A "spoil my ballot" option rather than a person.
    #[serde(default)]
    pub explicit_invalid: bool,
}

/// What the ballot does when a voter does something unusual.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Policy {
    /// Let it happen without comment.
    Allowed,
    /// Let it happen, but say something first.
    Warn,
    /// Do not let it happen.
    Restricted,
}

impl Default for Policy {
    fn default() -> Self {
        Policy::Warn
    }
}

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize,
)]
pub struct Policies {
    /// Choosing more than `max_votes`.
    #[serde(default)]
    pub over_vote: Policy,
    /// Choosing nothing at all.
    #[serde(default)]
    pub blank_vote: Policy,
    /// Choosing fewer than `max_votes`.
    #[serde(default)]
    pub under_vote: Policy,
    /// Deliberately spoiling the ballot.
    #[serde(default)]
    pub invalid_vote: Policy,
}

impl Policies {
    /// The platform's `over_vote_policy` values.
    fn over_vote(self) -> &'static str {
        match self.over_vote {
            Policy::Allowed => "allowed",
            Policy::Warn => "allowed-with-msg",
            Policy::Restricted => "not-allowed-with-msg-and-disable",
        }
    }

    fn under_vote(self) -> &'static str {
        match self.under_vote {
            Policy::Allowed => "allowed",
            Policy::Warn => "warn",
            Policy::Restricted => "warn-only-in-review",
        }
    }

    fn blank_vote(self) -> &'static str {
        match self.blank_vote {
            Policy::Allowed => "allowed",
            Policy::Warn => "warn",
            Policy::Restricted => "not-allowed",
        }
    }

    fn invalid_vote(self) -> &'static str {
        match self.invalid_vote {
            Policy::Allowed => "allowed",
            Policy::Warn => "warn",
            Policy::Restricted => "not-allowed",
        }
    }
}

/// The area used when a plan names none.
///
/// A bundle needs an area and a ballot link or no voter sees anything, so a plan
/// that has never thought about districting gets one covering everybody.
///
/// Named rather than anonymous because the voters CSV resolves an area by name,
/// and because a delivery engineer opening the generated event should be able to
/// tell nobody chose it.
pub const DEFAULT_AREA_EXTERNAL_ID: &str = "all-voters";
pub const DEFAULT_AREA_NAME: &str = "All voters";

/// Check a plan, before anything is built from it.
///
/// These are questions about the *plan*, in the wizard's own vocabulary — a
/// trustee threshold higher than the number of trustees, a voting window that
/// closes before it opens. [`super::validate`] then checks the bundle, and a
/// problem there is phrased in the bundle's vocabulary. Both run; they are asking
/// different questions and an author needs both answers.
pub fn validate_plan(plan: &Blueprint) -> Report {
    let mut report = Report::default();

    if plan.version > BLUEPRINT_VERSION {
        report.push(Problem::error(
            Code::InvalidValue,
            "version",
            format!(
                "this plan was saved by a newer version ({} against {}). Opening \
                 it here would silently drop whatever that version added.",
                plan.version, BLUEPRINT_VERSION
            ),
        ));
    }

    if plan.external_id.trim().is_empty() {
        report.push(Problem::error(
            Code::MissingField,
            "external_id",
            "the event needs an identifier: every generated id is derived from it, \
             so without one nothing can be built twice the same way",
        ));
    }

    if plan.name.is_empty() {
        report.push(Problem::error(
            Code::MissingField,
            "name",
            "the event needs a name. Voters see it above the ballot, and it \
             becomes the login page's title.",
        ));
    }

    check_trustees(plan, &mut report);
    check_schedule(plan, &mut report);
    check_areas(plan, &mut report);
    check_ballot(plan, &mut report);

    if plan.contacts.is_empty() {
        report.push(Problem::warning(
            Code::MissingField,
            "contacts",
            "nobody is listed as a point of contact. On election day this is who \
             gets called.",
        ));
    }

    report
}

fn check_trustees(plan: &Blueprint, report: &mut Report) {
    if plan.trustees.is_empty() {
        report.push(Problem::warning(
            Code::MissingField,
            "trustees",
            "no trustees. The election key needs somebody to hold it, and the \
             tally needs them to come back.",
        ));
        return;
    }

    if plan.trustee_threshold == 0 {
        report.push(Problem::error(
            Code::InvalidValue,
            "trustee_threshold",
            "a threshold of zero means the tally can be opened by nobody at all",
        ));
    }

    if plan.trustee_threshold as usize > plan.trustees.len() {
        // The failure mode is the worst kind: everything works until the tally,
        // and then the result cannot be decrypted by anyone.
        report.push(Problem::error(
            Code::ContestArithmetic,
            "trustee_threshold",
            format!(
                "{} of {} trustees are required, which cannot be met. The key \
                 would be generated and the result could never be decrypted.",
                plan.trustee_threshold,
                plan.trustees.len()
            ),
        ));
    }

    if plan.trustee_threshold == 1 && plan.trustees.len() > 1 {
        report.push(Problem::warning(
            Code::InvalidValue,
            "trustee_threshold",
            "one trustee alone can open the tally, which is the same guarantee as \
             having a single trustee",
        ));
    }
}

fn check_schedule(plan: &Blueprint, report: &mut Report) {
    let schedule = &plan.schedule;

    // Each moment on its own first: an ordering complaint about a time that is
    // not a time would send somebody looking at the wrong field.
    let mut found = Vec::new();
    for (at, moment) in [
        ("schedule.key_ceremony", &schedule.key_ceremony),
        ("schedule.voting_opens", &schedule.voting_opens),
        ("schedule.voting_closes", &schedule.voting_closes),
        ("schedule.tally_ceremony", &schedule.tally_ceremony),
    ] {
        if let Some(moment) = moment {
            time::check(moment, at, &mut found);
        }
    }
    let unreadable = found
        .iter()
        .any(|problem| problem.severity == Severity::Error);
    for problem in found {
        report.push(problem);
    }
    if unreadable {
        return;
    }

    match (&schedule.voting_opens, &schedule.voting_closes) {
        (None, _) | (_, None) => report.push(Problem::warning(
            Code::MissingSchedule,
            "schedule",
            "the voting window is incomplete, so the period will have to be opened \
             or closed by hand in the Admin Portal",
        )),
        (Some(opens), Some(closes)) => {
            // By instant, not by text. Two moments in different zones sort by
            // their strings in whatever order the digits happen to fall.
            if !opens.is_empty()
                && !closes.is_empty()
                && time::compare(closes, opens) != Ordering::Greater
            {
                report.push(Problem::error(
                    Code::InvalidValue,
                    "schedule.voting_closes",
                    "voting closes before it opens, so it would never be open",
                ));
            }

            // Both endpoints in one zone but at different offsets means the
            // window crosses a daylight-saving change — legitimate, and worth
            // knowing about, because an hour moves under whoever planned it.
            if opens.zone == closes.zone
                && !opens.zone.trim().is_empty()
                && opens.offset_minutes != closes.offset_minutes
            {
                report.push(Problem::warning(
                    Code::InvalidValue,
                    "schedule",
                    "the voting window crosses a daylight-saving change, so it is \
                     an hour longer or shorter than the clock times suggest",
                ));
            }
        }
    }

    if let (Some(ceremony), Some(opens)) =
        (&schedule.key_ceremony, &schedule.voting_opens)
    {
        if !ceremony.is_empty()
            && !opens.is_empty()
            && time::compare(ceremony, opens) != Ordering::Less
        {
            report.push(Problem::error(
                Code::InvalidValue,
                "schedule.key_ceremony",
                "the key ceremony is not before voting opens. The election key has \
                 to exist before a vote can be encrypted with it.",
            ));
        }
    }

    if let (Some(tally), Some(closes)) =
        (&schedule.tally_ceremony, &schedule.voting_closes)
    {
        if !tally.is_empty()
            && !closes.is_empty()
            && time::compare(tally, closes) != Ordering::Greater
        {
            report.push(Problem::error(
                Code::InvalidValue,
                "schedule.tally_ceremony",
                "the tally ceremony is not after voting closes, so it would count \
                 votes that had not been cast yet",
            ));
        }
    }
}

/// Districting: the areas themselves, before any contest points at one.
fn check_areas(plan: &Blueprint, report: &mut Report) {
    for (index, area) in plan.areas.iter().enumerate() {
        let at = format!("areas[{index}]");

        if area.external_id.trim().is_empty() {
            report.push(Problem::error(
                Code::MissingField,
                &at,
                "an area needs an identifier",
            ));
        }

        if area.name.trim().is_empty() {
            report.push(
                Problem::error(
                    Code::MissingField,
                    format!("{at}.name"),
                    "an area needs a name: the voters CSV identifies a voter's \
                     area by name, not by id, so an unnamed area is one no voter \
                     can be put in",
                )
                .about(Some(&area.external_id)),
            );
        }

        // The voters CSV resolves by name, so a duplicate silently assigns voters
        // to whichever one the importer happens to find first.
        if let Some(earlier) = plan.areas[..index].iter().find(|other| {
            !other.name.trim().is_empty() && other.name == area.name
        }) {
            report.push(
                Problem::error(
                    Code::DuplicateId,
                    format!("{at}.name"),
                    format!(
                        "two areas are both named '{}' ('{}' and '{}'). The voters \
                         CSV resolves an area by name, so voters would land in \
                         whichever the importer found first.",
                        area.name, earlier.external_id, area.external_id
                    ),
                )
                .about(Some(&area.external_id)),
            );
        }

        if let Some(parent) = area
            .parent_external_id
            .as_ref()
            .filter(|parent| !parent.is_empty())
        {
            if parent == &area.external_id {
                report.push(
                    Problem::error(
                        Code::AreaCycle,
                        format!("{at}.parent_external_id"),
                        "an area cannot be inside itself",
                    )
                    .about(Some(&area.external_id)),
                );
            } else if !plan
                .areas
                .iter()
                .any(|other| &other.external_id == parent)
            {
                report.push(
                    Problem::error(
                        Code::DanglingReference,
                        format!("{at}.parent_external_id"),
                        format!("no area has the identifier '{parent}'"),
                    )
                    .about(Some(&area.external_id)),
                );
            }
        }
    }
}

fn check_ballot(plan: &Blueprint, report: &mut Report) {
    if plan.elections.is_empty() {
        report.push(Problem::error(
            Code::MissingField,
            "elections",
            "an election event needs at least one election",
        ));
        return;
    }

    for (index, election) in plan.elections.iter().enumerate() {
        let at = format!("elections[{index}]");

        if election.contests.is_empty() {
            report.push(
                Problem::warning(
                    Code::BallotCoverage,
                    &at,
                    "this election has no contests, so nobody votes in it",
                )
                .about(Some(&election.external_id)),
            );
        }

        for (contest_index, contest) in election.contests.iter().enumerate() {
            let at = format!("{at}.contests[{contest_index}]");
            let choices = contest
                .candidates
                .iter()
                .filter(|candidate| {
                    !candidate.explicit_blank && !candidate.explicit_invalid
                })
                .count();

            if contest.max_votes < 1 {
                report.push(
                    Problem::error(
                        Code::ContestArithmetic,
                        &at,
                        "a voter may choose fewer than one candidate, so there is \
                         nothing to vote for",
                    )
                    .about(Some(&contest.external_id)),
                );
            }

            if contest.winners < 1 {
                report.push(
                    Problem::error(
                        Code::ContestArithmetic,
                        &at,
                        "the contest elects nobody",
                    )
                    .about(Some(&contest.external_id)),
                );
            }

            // The bug the TypeScript shipped: winners was fixed at 1 while
            // max_votes was free, so "choose 3" quietly elected one person.
            if contest.winners > contest.max_votes {
                report.push(
                    Problem::error(
                        Code::ContestArithmetic,
                        &at,
                        format!(
                            "the contest elects {} but a voter may only choose {}",
                            contest.winners, contest.max_votes
                        ),
                    )
                    .about(Some(&contest.external_id)),
                );
            }

            for area in &contest.areas {
                if !plan
                    .areas
                    .iter()
                    .any(|planned| &planned.external_id == area)
                {
                    report.push(
                        Problem::error(
                            Code::DanglingReference,
                            format!("{at}.areas"),
                            format!("no area has the identifier '{area}'"),
                        )
                        .about(Some(&contest.external_id)),
                    );
                }
            }

            if choices == 0 {
                report.push(
                    Problem::warning(
                        Code::BallotCoverage,
                        &at,
                        "no candidates yet",
                    )
                    .about(Some(&contest.external_id)),
                );
            } else if (contest.winners as usize) > choices {
                report.push(
                    Problem::error(
                        Code::ContestArithmetic,
                        &at,
                        format!(
                            "the contest elects {} from a field of {choices}",
                            contest.winners
                        ),
                    )
                    .about(Some(&contest.external_id)),
                );
            }
        }
    }
}

/// Turn a plan into the rows the builder reads.
///
/// This is the only mapping in the architect, and it produces a
/// [`Workbook`] rather than a bundle so that everything downstream — templates,
/// ids, CSV shapes, the realm, the archive — is the code the workbook reader
/// already uses.
pub fn to_workbook(plan: &Blueprint) -> Result<Workbook, Problem> {
    let languages = plan.languages_or_english();

    let mut sheets = vec![
        event_sheet(plan, &languages)?,
        elections_sheet(plan, &languages)?,
        contests_sheet(plan, &languages)?,
        candidates_sheet(plan, &languages)?,
        areas_sheet(plan)?,
        area_contests_sheet(plan)?,
    ];

    if let Some(schedule) = scheduled_events_sheet(plan)? {
        sheets.push(schedule);
    }

    Workbook::new(sheets)
}

impl Blueprint {
    /// The languages to write, never empty.
    ///
    /// A plan with no language still has to produce a ballot somebody can read.
    fn languages_or_english(&self) -> Vec<String> {
        let chosen: Vec<String> = self
            .languages
            .iter()
            .map(|code| code.trim().to_string())
            .filter(|code| !code.is_empty())
            .collect();
        if chosen.is_empty() {
            vec!["en".to_string()]
        } else {
            chosen
        }
    }
}

/// A header and its column of values, as the sheet reader wants them.
fn sheet_of(
    name: &str,
    columns: Vec<String>,
    rows: Vec<Vec<Cell>>,
) -> Result<Sheet, Problem> {
    let mut grid =
        vec![columns.iter().map(|c| Cell::text(c.clone())).collect()];
    grid.extend(rows);
    Sheet::from_grid(name, &grid)
}

/// `presentation.i18n.<lang>.name` columns, one per language.
fn i18n_columns(prefix: &str, languages: &[String]) -> Vec<String> {
    languages
        .iter()
        .map(|language| format!("{prefix}.i18n.{language}.name"))
        .collect()
}

fn i18n_values(text: &Translated, languages: &[String]) -> Vec<Cell> {
    languages
        .iter()
        .map(|language| match text.get(language) {
            Some(value) => Cell::text(value),
            None => Cell::Blank,
        })
        .collect()
}

fn event_sheet(
    plan: &Blueprint,
    languages: &[String],
) -> Result<Sheet, Problem> {
    let mut columns = vec!["external_id".to_string()];
    columns.extend(i18n_columns("presentation", languages));
    columns
        .push("presentation.language_conf.enabled_language_codes".to_string());
    columns
        .push("presentation.language_conf.default_language_code".to_string());

    let mut row = vec![Cell::text(plan.external_id.clone())];
    row.extend(i18n_values(&plan.name, languages));
    // A JSON array in one cell: the reader parses bracketed text as JSON, which is
    // how a list fits in a spreadsheet and therefore in a synthesised one too.
    row.push(Cell::text(
        serde_json::to_string(languages).unwrap_or_else(|_| "[]".to_string()),
    ));
    row.push(Cell::text(
        languages
            .first()
            .cloned()
            .unwrap_or_else(|| "en".to_string()),
    ));

    if let Some(logo) = plan.logo_url.as_ref().filter(|url| !url.is_empty()) {
        columns.push("presentation.logo_url".to_string());
        row.push(Cell::text(logo.clone()));
    }

    sheet_of("ElectionEvent", columns, vec![row])
}

fn elections_sheet(
    plan: &Blueprint,
    languages: &[String],
) -> Result<Sheet, Problem> {
    let mut columns = vec!["external_id".to_string()];
    columns.extend(i18n_columns("presentation", languages));

    let rows = plan
        .elections
        .iter()
        .map(|election| {
            let mut row = vec![Cell::text(election.external_id.clone())];
            row.extend(i18n_values(&election.name, languages));
            row
        })
        .collect();

    sheet_of("Elections", columns, rows)
}

fn contests_sheet(
    plan: &Blueprint,
    languages: &[String],
) -> Result<Sheet, Problem> {
    let mut columns = vec![
        "external_id".to_string(),
        "election.external_id".to_string(),
        "max_votes".to_string(),
        "min_votes".to_string(),
        "winning_candidates_num".to_string(),
        "description".to_string(),
        "presentation.over_vote_policy".to_string(),
        "presentation.under_vote_policy".to_string(),
        "presentation.blank_vote_policy".to_string(),
        "presentation.invalid_vote_policy".to_string(),
        "presentation.sort_order".to_string(),
    ];
    columns.extend(i18n_columns("presentation", languages));

    let mut rows = Vec::new();
    for election in &plan.elections {
        for (order, contest) in election.contests.iter().enumerate() {
            let mut row = vec![
                Cell::text(contest.external_id.clone()),
                Cell::text(election.external_id.clone()),
                Cell::Int(contest.max_votes),
                // The wizard does not ask, and a required minimum is a way to
                // stop somebody voting at all.
                Cell::Int(0),
                Cell::Int(contest.winners),
                Cell::text(contest.description.clone()),
                Cell::text(plan.policies.over_vote()),
                Cell::text(plan.policies.under_vote()),
                Cell::text(plan.policies.blank_vote()),
                Cell::text(plan.policies.invalid_vote()),
                Cell::Int(order as i64),
            ];
            row.extend(i18n_values(&contest.name, languages));
            rows.push(row);
        }
    }

    sheet_of("Contests", columns, rows)
}

fn candidates_sheet(
    plan: &Blueprint,
    languages: &[String],
) -> Result<Sheet, Problem> {
    let mut columns = vec![
        "external_id".to_string(),
        "contest.external_id".to_string(),
        "presentation.sort_order".to_string(),
        "presentation.is_explicit_blank".to_string(),
        "presentation.is_explicit_invalid".to_string(),
    ];
    columns.extend(i18n_columns("presentation", languages));

    let mut rows = Vec::new();
    for election in &plan.elections {
        for contest in &election.contests {
            for (order, candidate) in contest.candidates.iter().enumerate() {
                let mut row = vec![
                    Cell::text(candidate.external_id.clone()),
                    Cell::text(contest.external_id.clone()),
                    Cell::Int(order as i64),
                    Cell::Bool(candidate.explicit_blank),
                    Cell::Bool(candidate.explicit_invalid),
                ];
                row.extend(i18n_values(&candidate.name, languages));
                rows.push(row);
            }
        }
    }

    sheet_of("Candidates", columns, rows)
}

/// The plan's areas, or the one that covers everybody.
///
/// A plan that has never thought about districting still needs an area and a
/// ballot link, or no voter sees anything. See [`DEFAULT_AREA_EXTERNAL_ID`].
fn areas_sheet(plan: &Blueprint) -> Result<Sheet, Problem> {
    let columns = vec![
        "external_id".to_string(),
        "name".to_string(),
        "parent.external_id".to_string(),
    ];

    if plan.areas.is_empty() {
        return sheet_of(
            "Areas",
            columns,
            vec![vec![
                Cell::text(DEFAULT_AREA_EXTERNAL_ID),
                Cell::text(DEFAULT_AREA_NAME),
                Cell::Blank,
            ]],
        );
    }

    let rows = plan
        .areas
        .iter()
        .map(|area| {
            vec![
                Cell::text(area.external_id.clone()),
                Cell::text(area.name.clone()),
                match &area.parent_external_id {
                    Some(parent) if !parent.is_empty() => {
                        Cell::text(parent.clone())
                    }
                    _ => Cell::Blank,
                },
            ]
        })
        .collect();

    sheet_of("Areas", columns, rows)
}

/// Which contests appear on which area's ballot.
fn area_contests_sheet(plan: &Blueprint) -> Result<Sheet, Problem> {
    let every_area: Vec<String> = if plan.areas.is_empty() {
        vec![DEFAULT_AREA_EXTERNAL_ID.to_string()]
    } else {
        plan.areas
            .iter()
            .map(|area| area.external_id.clone())
            .collect()
    };

    let mut rows = Vec::new();
    for contest in plan
        .elections
        .iter()
        .flat_map(|election| &election.contests)
    {
        // An empty list means everywhere. A contest nobody assigned is one
        // somebody has not got to yet, and dropping it off every ballot would be
        // a silent way of losing it.
        let on: Vec<&String> = if contest.areas.is_empty() {
            every_area.iter().collect()
        } else {
            contest.areas.iter().collect()
        };

        for area in on {
            // A contest may not be listed twice for the same area: both rows
            // would mint the same id and one would overwrite the other.
            let link = vec![
                Cell::text(area.clone()),
                Cell::text(contest.external_id.clone()),
            ];
            if !rows.contains(&link) {
                rows.push(link);
            }
        }
    }

    sheet_of(
        "AreaContests",
        vec![
            "area.external_id".to_string(),
            "contest.external_id".to_string(),
        ],
        rows,
    )
}

/// The voting window, or nothing.
///
/// Event-wide rather than per election: the wizard asks once, and an event-wide
/// scheduled event covers every election in it.
fn scheduled_events_sheet(plan: &Blueprint) -> Result<Option<Sheet>, Problem> {
    let mut rows = Vec::new();

    for (name, processor, at) in [
        (
            "Voting opens",
            "START_VOTING_PERIOD",
            &plan.schedule.voting_opens,
        ),
        (
            "Voting closes",
            "END_VOTING_PERIOD",
            &plan.schedule.voting_closes,
        ),
    ] {
        if let Some(at) = at.as_ref().filter(|at| !at.is_empty()) {
            // An instant, not the wall clock somebody typed. The scheduler reads
            // this back through `DateTime::parse_from_rfc3339`, which requires an
            // offset — so a bare `2027-03-01T09:00` yields no date, the poller
            // drops the event, and the voting period never opens. Nothing on that
            // path reports anything.
            rows.push(vec![
                Cell::text(name),
                Cell::text(processor),
                Cell::text(at.to_rfc3339()?),
            ]);
        }
    }

    if rows.is_empty() {
        return Ok(None);
    }

    sheet_of(
        "ScheduledEvents",
        vec![
            "event_name".to_string(),
            "event_type".to_string(),
            "scheduled_datetime".to_string(),
        ],
        rows,
    )
    .map(Some)
}

/// The files the architect produces that are not part of an import.
///
/// A ceremony schedule, a list of who to call, and a list of who holds the key.
/// None of them has a home in an election event, and all three are what the client
/// actually asks for — so they travel beside the archive, like the administrator
/// and template files the workbook reader produces.
///
/// The plan itself is written out too. That is what makes the wizard resumable
/// without parsing its own output back, which is how the TypeScript version did it
/// and why its round trip lost the trustee threshold and every ceremony date.
pub fn side_files(plan: &Blueprint) -> Vec<(String, String)> {
    let mut files = Vec::new();

    let plan_json =
        serde_json::to_string_pretty(plan).unwrap_or_else(|_| "{}".to_string());
    files.push(("blueprint.json".to_string(), plan_json + "\n"));

    let ceremony = serde_json::json!({
        "_comment": "Dates people have to attend. Not part of the import.",
        "key_ceremony": plan.schedule.key_ceremony,
        "tally_ceremony": plan.schedule.tally_ceremony,
        "voting_opens": plan.schedule.voting_opens,
        "voting_closes": plan.schedule.voting_closes,
        "milestones": plan.schedule.milestones,
    });
    files.push(("ceremony_schedule.json".to_string(), pretty(&ceremony)));

    if !plan.contacts.is_empty() {
        files.push((
            "points_of_contact.json".to_string(),
            pretty(&serde_json::json!(plan.contacts)),
        ));
    }

    if !plan.trustees.is_empty() {
        files.push((
            "trustees_list.json".to_string(),
            pretty(&serde_json::json!({
                "threshold": plan.trustee_threshold,
                "trustees": plan.trustees,
            })),
        ));
    }

    files
}

fn pretty(value: &serde_json::Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string())
        + "\n"
}

#[cfg(test)]
#[path = "architect_tests.rs"]
mod architect_tests;
