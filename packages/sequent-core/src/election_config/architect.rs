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

use crate::election_config::archive::{Artifact, Layout};
use crate::election_config::build::{build, BuildOptions, Bundle};
use crate::election_config::paths::Cell;
use crate::election_config::policy::{Behaviour, Overrides};
use crate::election_config::problem::{Code, Problem, Report, Severity};
use crate::election_config::profile::{apply_profile, check_required, Profile};
use crate::election_config::render::TemplateSet;
use crate::election_config::schema::ImportElectionEventSchema;
use crate::election_config::sheet::{Sheet, Workbook};
use crate::election_config::time::{self, Timestamp};
use serde::{Deserialize, Serialize};

/// The plan format's version.
///
/// Written into every saved plan and checked on load. A plan is a document
/// somebody spent an afternoon on; being able to say "this is from an older
/// version" beats failing to deserialize it with a serde error about a missing
/// field.
pub const BLUEPRINT_VERSION: u32 = 2;

/// Bring a version 1 plan up to date, in place.
///
/// Version 1 carried `policies`, a three-value `allowed | warn | restricted`
/// per policy, applied to every contest identically. Version 2 carries the
/// platform's own values, per contest.
///
/// The mapping below is **version 1's own**, reproduced exactly — including
/// where it was wrong. `restricted` for an under-vote produced
/// `warn-only-in-review` and `restricted` for a blank or invalid vote produced
/// `not-allowed`, and a plan that compiled to those bytes yesterday has to
/// compile to them today. Getting it *right* for an old plan would silently
/// change an election somebody has already reviewed; new plans get the
/// considered defaults instead.
fn migrate_v1(document: &mut serde_json::Value) {
    let Some(object) = document.as_object_mut() else {
        return;
    };
    if object.get("version").and_then(serde_json::Value::as_u64) != Some(1) {
        return;
    }

    let old = object.remove("policies").unwrap_or(serde_json::Value::Null);
    let says = |key: &str| -> &str {
        old.get(key)
            .and_then(serde_json::Value::as_str)
            .unwrap_or("warn")
    };

    let mut policies = serde_json::Map::new();
    policies.insert(
        "over_vote".to_string(),
        serde_json::json!(match says("over_vote") {
            "allowed" => "allowed",
            "restricted" => "not-allowed-with-msg-and-disable",
            _ => "allowed-with-msg",
        }),
    );
    policies.insert(
        "under_vote".to_string(),
        serde_json::json!(match says("under_vote") {
            "allowed" => "allowed",
            "restricted" => "warn-only-in-review",
            _ => "warn",
        }),
    );
    for (key, restricted) in [
        ("blank_vote", "not-allowed"),
        ("invalid_vote", "not-allowed"),
    ] {
        policies.insert(
            key.to_string(),
            serde_json::json!(match says(key) {
                "allowed" => "allowed",
                "restricted" => restricted,
                _ => "warn",
            }),
        );
    }

    object.insert(
        "defaults".to_string(),
        serde_json::json!({"policies": policies}),
    );
    object.insert("version".to_string(), serde_json::json!(2));
}

/// Read a saved plan, bringing an older one up to date.
///
/// The only way a plan should be deserialized. A plan from a *newer* version is
/// left alone here and refused by [`validate_plan`], which can say so as a
/// problem rather than as a serde error about a field nobody has heard of.
pub fn read_plan(document: &str) -> Result<Blueprint, Problem> {
    let mut value: serde_json::Value =
        serde_json::from_str(document).map_err(|error| {
            Problem::error(
                Code::InvalidValue,
                "plan",
                format!("this is not an election plan: {error}"),
            )
        })?;

    migrate_v1(&mut value);

    serde_json::from_value(value).map_err(|error| {
        Problem::error(
            Code::InvalidValue,
            "plan",
            format!("this plan cannot be read: {error}"),
        )
    })
}

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
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
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

    /// The order the elections appear in, for a voter with more than one.
    ///
    /// The same three values again. `custom` is the order on the Ballot screen.
    #[serde(default = "custom_order")]
    pub elections_order: String,

    /// What the whole event is, for voters.
    ///
    /// Translatable, like the name: the platform keeps it at
    /// `presentation.i18n.<lang>.description` and mirrors the English one into a
    /// flat `description` column, which is what the Admin Portal's list views
    /// read. Both are written.
    #[serde(default)]
    pub description: Translated,

    /// BCP 47 or ISO 639-2/T codes, in the order the picker should show them.
    #[serde(default)]
    pub languages: Vec<String>,

    /// Which of [`Self::languages`] a voter gets before choosing.
    ///
    /// `None` means the first one, which is what this used to be unconditionally
    /// — `event_sheet` wrote `languages.first()` and nothing could say otherwise.
    /// So the languages were configurable and the default was not, and a client
    /// wanting Spanish first had to reorder the list without being told that was
    /// what the order meant.
    ///
    /// It is the default *ballot* language, not the wizard's own. Those are
    /// different things and the guide says so: this one is what voters read.
    #[serde(default)]
    pub default_language: Option<String>,

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

    /// The voters, when the plan carries them.
    ///
    /// Optional on purpose. A plan describes an election and a census is a
    /// separate, much larger, much more sensitive thing — so most plans have
    /// none and the wizard says so. But a client whose membership does not
    /// change between elections has every reason to keep the two together, and
    /// telling them to hold the list somewhere else is telling them to hold it
    /// somewhere worse.
    ///
    /// The columns are `build_tables::VOTER_LEADING_COLUMNS` plus whatever else
    /// the client carries; anything unrecognised becomes a Keycloak user
    /// attribute, which is how a reporting breakout arrives without a code
    /// change. That is the same passthrough the workbook path has, so a census
    /// exported from one route imports through the other.
    #[serde(default)]
    pub voters: Vec<PlannedVoter>,

    /// Which of the four authentication flows this event uses.
    ///
    /// `None` leaves the environment's own configuration alone, which is the
    /// safe default and what every plan written before this field did.
    ///
    /// Carried in the plan rather than only in `CompileOptions` because a
    /// choice that vanishes when you reopen a plan is a choice somebody makes
    /// twice and gets differently the second time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_preset: Option<String>,

    /// How contests behave unless they say otherwise.
    ///
    /// Was the only place policies could be set, and applied to every contest
    /// identically; now the bottom of a three-level resolution.
    #[serde(default)]
    pub defaults: Behaviour,

    /// Anything the wizard has no field for. Carried, not interpreted.
    #[serde(default)]
    pub notes: String,
}

fn default_threshold() -> u32 {
    2
}

/// One voter, as a plan carries them.
///
/// Deliberately close to a row of `export_voters.csv` rather than to a domain
/// model: this is a census on its way through, and every field it does not
/// recognise it keeps. Reshaping it into something tidier would mean deciding
/// which of a client's own columns matter, which is not a decision this has any
/// basis for making.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PlannedVoter {
    /// What they sign in as. The only field that must be there.
    pub username: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub email: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub first_name: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub last_name: String,

    /// Which area's ballot they get. Matched **by name** against `areas`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub area_name: String,

    /// Everything else the client carries, passed through as user attributes.
    #[serde(flatten)]
    pub extra: std::collections::BTreeMap<String, String>,
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

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Contact {
    pub name: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub email: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PlannedArea {
    pub external_id: String,

    /// What the importer matches a voter's `area_name` against.
    #[serde(default)]
    pub name: String,

    /// The area this one sits inside, if any.
    ///
    /// A tree, because that is how districting is actually described — a local
    /// inside a region inside a state — and because the platform models it that
    /// way.
    ///
    /// **A contest assigned to a parent is on every descendant's ballot.** That
    /// is not this module's choice: when a publication is generated, windmill
    /// walks the path from the root down to each area and gathers every
    /// `area_contest` on the way — see
    /// [`crate::ballot_style::elections_contests_for_area`], which is now the one
    /// implementation of that walk and is what the preview calls. So assigning a
    /// contest to "National" and nothing else still puts it in front of every
    /// voter in North and South.
    ///
    /// This doc comment used to claim the opposite, and nothing tested it either
    /// way. `preview_tests::each_area_gets_the_contests_it_and_its_parents_vote_on`
    /// does now.
    #[serde(default)]
    pub parent_external_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlannedElection {
    pub external_id: String,
    #[serde(default)]
    pub name: Translated,

    /// What this election is about, for voters. Translatable.
    #[serde(default)]
    pub description: Translated,

    /// How many times a voter may change a vote they have already cast.
    ///
    /// One means cast once and that is final; two means one change. The platform
    /// keeps the *last* ballot and discards the earlier ones, so this is not "how
    /// many votes" — it is how many attempts.
    ///
    /// A real client decision, and the Admin Portal's election form exposes it,
    /// so a wizard-built election took whatever `election.hbs` said.
    #[serde(default = "one")]
    pub num_allowed_revotes: i64,

    /// Whether a voter may throw away a ballot they have already cast.
    ///
    /// Different from a contest's "I spoil my ballot" option, which is a choice on
    /// the paper. This discards a *cast* ballot and lets the voter start again, so
    /// it only means anything alongside a revote allowance.
    #[serde(default)]
    pub spoil_ballot_option: bool,

    /// Whether voting closes on the deadline or a little after it.
    ///
    /// `no-grace-period` or `grace-period-without-alert`, per
    /// `EGracePeriodPolicy`. A grace period lets a voter who opened the ballot
    /// before the close finish casting it, which is a decision about fairness
    /// rather than about software.
    #[serde(default = "no_grace_period")]
    pub grace_period_policy: String,

    /// How long that grace period lasts, in seconds. Zero without one.
    #[serde(default)]
    pub grace_period_secs: i64,

    /// Whether the voting screen is titled after this election or the whole event.
    ///
    /// `election` or `election-event`, per `EStartScreenTitlePolicy`. For an event
    /// with one election the event's name usually reads better; for one with
    /// several, the election's does.
    #[serde(default = "title_from_event")]
    pub start_screen_title_policy: String,

    /// The order this election's contests appear in.
    ///
    /// `custom`, `alphabetical` or `random`, the same three values as a contest's
    /// `candidates_order` — the Voting Portal sorts both through the same WASM
    /// helper. `custom` is the order on the Ballot screen, which the wizard
    /// already writes as each contest's `presentation.sort_order`; the other two
    /// tell the portal to ignore it.
    ///
    /// Exposed in the Admin Portal's election form and unsettable from a plan, so
    /// a wizard-built election could arrange its contests and not say whether the
    /// arrangement was honoured.
    #[serde(default = "custom_order")]
    pub contests_order: String,

    /// Which permission label gates access to this election.
    ///
    /// Internal: it is how the importer links a voter's authorisation to an
    /// election, and a client has no reason to choose one. Present because a
    /// delivery occasionally has to match an existing label, and empty means the
    /// builder derives it.
    #[serde(default)]
    pub permission_label: String,

    /// One set for every contest here, replacing whatever the contests say.
    ///
    /// `None` — the normal case — means each contest resolves the event default
    /// against its own overrides. `Some` is the old wizard's
    /// `samePolicyForAllContests`: edited once, and a contest's own overrides
    /// are not consulted. One field rather than a flag beside a value, because
    /// "shared is on but there is no shared value" is a state somebody would
    /// eventually produce and nobody could explain.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shared: Option<Overrides>,
    #[serde(default)]
    pub contests: Vec<PlannedContest>,
}

/// The values `#[serde(default = "…")]` gives a plan that omits them.
///
/// Written out rather than derived, because `derive(Default)` would give zero
/// revote attempts — an election nobody can vote in — and an empty
/// `grace_period_policy`, which is not one of the platform's two values. Those
/// attributes apply only when *deserialising*; a struct literal in Rust gets
/// this, which is what every fixture uses.
impl Default for PlannedElection {
    fn default() -> Self {
        PlannedElection {
            external_id: String::new(),
            name: Translated::default(),
            description: Translated::default(),
            num_allowed_revotes: 1,
            spoil_ballot_option: false,
            grace_period_policy: no_grace_period(),
            grace_period_secs: 0,
            start_screen_title_policy: title_from_event(),
            contests_order: custom_order(),
            permission_label: String::new(),
            shared: None,
            contests: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PlannedContest {
    pub external_id: String,
    #[serde(default)]
    pub name: Translated,
    /// What this contest is about, shown under its name on the ballot.
    ///
    /// Translatable. It was a `String`, which was wrong: the Admin Portal edits
    /// `presentation.i18n.<lang>.description` per language and keeps the flat
    /// column as a mirror of the English one, so a plan carrying one text put the
    /// same words in front of every voter whatever language they read.
    #[serde(default, deserialize_with = "translated_or_plain")]
    pub description: Translated,

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

    /// What this contest says about how it behaves, over the event's defaults.
    #[serde(default, skip_serializing_if = "Overrides::is_empty")]
    pub overrides: Overrides,

    /// Whether a voter may type a name the ballot does not list.
    ///
    /// The ballot codec has carried write-ins from the beginning —
    /// `raw_ballot::encode` packs the text into the ballot integer a byte at a
    /// time and `contest_context` reserves the bases for it — and no plan could
    /// ask for one, so the feature was reachable only by editing a bundle.
    ///
    /// It is a pair, and both halves have to agree: this switch, and one
    /// candidate marked `is_write_in` per slot a voter may fill. `validate`
    /// refuses either half alone, because the codec silently does nothing useful
    /// with a mismatch — a write-in candidate with the switch off is a nameless
    /// option on the ballot, and the switch on with no such candidate offers a
    /// voter nothing to write in.
    #[serde(default)]
    pub allow_writeins: bool,

    /// How many names a voter may add, when write-ins are allowed.
    ///
    /// One candidate row is minted per slot. Zero with `allow_writeins` on is
    /// the mismatch above, and `validate` says so rather than emitting a contest
    /// the codec cannot use.
    #[serde(default)]
    pub write_in_slots: i64,

    /// Which areas put this contest on their ballot.
    ///
    /// Empty means every area, which is what a plan that has never thought about
    /// districting wants and what one with a single area always wants. Naming
    /// areas explicitly is how a contest becomes local to some of them.
    #[serde(default)]
    pub areas: Vec<String>,
}

/// The order somebody arranged, which is what the wizard is for.
fn custom_order() -> String {
    "custom".to_string()
}

fn no_grace_period() -> String {
    "no-grace-period".to_string()
}

/// The event's name, not the election's.
///
/// Most events have one election, where the event's name is the one the client
/// wrote and the election's is a subdivision nobody outside the platform thinks
/// about.
fn title_from_event() -> String {
    "election-event".to_string()
}

fn one() -> i64 {
    1
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PlannedCandidate {
    pub external_id: String,
    #[serde(default)]
    pub name: Translated,

    /// A line about the candidate, shown under their name. Translatable.
    #[serde(default)]
    pub description: Translated,
    /// A "none of the above" option rather than a person.
    #[serde(default)]
    pub explicit_blank: bool,
    /// A "spoil my ballot" option rather than a person.
    #[serde(default)]
    pub explicit_invalid: bool,
}

/// What a contest ends up with.
///
/// Most specific last, with one exception: an election that has claimed the
/// decision does not consult its contests at all. That is a statement about the
/// election rather than a value copied onto each contest and then silently
/// stale, which is what would happen if "shared" merely pre-filled them.
pub fn resolve(
    plan: &Blueprint,
    election: &PlannedElection,
    contest: &PlannedContest,
) -> Behaviour {
    let claimed = election.shared.as_ref().unwrap_or(&contest.overrides);
    plan.defaults.apply(claimed)
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

    check_languages(plan, &mut report);
    check_trustees(plan, &mut report);
    check_schedule(plan, &mut report);
    check_areas(plan, &mut report);
    check_census(plan, &mut report);
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

/// The ballot's languages, and which one a voter starts in.
fn check_languages(plan: &Blueprint, report: &mut Report) {
    if plan.languages.is_empty() {
        report.push(Problem::warning(
            Code::MissingField,
            "languages",
            "no languages, so the ballot falls back to English. That is a safety \
             net rather than a choice.",
        ));
        return;
    }

    let Some(chosen) = plan.default_language.as_deref() else {
        return;
    };

    if !plan.languages.iter().any(|each| each == chosen) {
        // An error, not a warning. The builder would fall back to the first
        // language, so this imports cleanly and opens in a language nobody
        // chose — which is exactly the class of failure nobody notices until a
        // voter says the ballot came up in the wrong language.
        report.push(Problem::error(
            Code::InvalidValue,
            "default_language",
            format!(
                "'{chosen}' is not one of the languages this ballot is offered \
                 in ({}). Voters would get the first one instead.",
                plan.languages.join(", ")
            ),
        ));
    }
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

/// The census, when the plan carries one.
///
/// Two failures matter, and both are the kind nobody sees until a voter cannot
/// vote:
///
///   - **A duplicate username.** The importer derives a voter's id from it, so
///     two rows sharing one produce one account, and whichever row came second
///     silently replaced the first.
///   - **An area name no area has.** Voters are matched to areas *by name*, so
///     a misspelling gives a voter no ballot at all — and this is the one place
///     where a name is doing real work rather than labelling something.
///
/// A census with no voters in it is not an error: a plan may carry the shape of
/// an election long before anybody has the membership list.
fn check_census(plan: &Blueprint, report: &mut Report) {
    if plan.voters.is_empty() {
        return;
    }

    let named: std::collections::BTreeSet<&str> =
        plan.areas.iter().map(|area| area.name.as_str()).collect();

    let mut seen: std::collections::BTreeMap<&str, usize> =
        std::collections::BTreeMap::new();

    for (index, voter) in plan.voters.iter().enumerate() {
        let username = voter.username.trim();

        if username.is_empty() {
            report.push(Problem::error(
                Code::MissingField,
                format!("voters[{index}].username"),
                "a voter needs a username; it is what they sign in as and what                  their account is derived from",
            ));
            continue;
        }

        if let Some(first) = seen.insert(username, index) {
            report.push(Problem::error(
                Code::DuplicateId,
                format!("voters[{index}].username"),
                format!(
                    "'{username}' is also row {first}. Two voters sharing a                      username become one account, and this one would replace                      the other without saying so."
                ),
            ));
        }

        // Blank means the default area, which is what a plan with no areas
        // gets. Naming one that does not exist is the mistake worth catching.
        let area = voter.area_name.trim();
        if !area.is_empty() && !named.contains(area) {
            report.push(Problem::error(
                Code::DanglingReference,
                format!("voters[{index}].area_name"),
                format!(
                    "no area is called '{area}'. Voters are matched to their                      area by name, so this voter would get no ballot. Copy the                      name from the area rather than retyping it."
                ),
            ));
        }
    }

    // Said once rather than per voter: a census loaded against the wrong plan
    // produces one of these for every row, and ten thousand copies of the same
    // sentence is a report nobody reads.
    if !plan.areas.is_empty()
        && plan
            .voters
            .iter()
            .all(|voter| voter.area_name.trim().is_empty())
    {
        report.push(Problem::warning(
            Code::MissingField,
            "voters",
            "this election has areas but no voter names one, so every voter \
             gets the default ballot. If the districting is meant to apply, the \
             census needs an area column.",
        ));
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

    if let Some(voters) = voters_sheet(plan)? {
        sheets.push(voters);
    }

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
/// Build one sheet, and refuse a ragged one.
///
/// Every sheet in this file is a `Vec<String>` of column names built in one place
/// and a `Vec<Cell>` per row built in another, and nothing tied the two together:
/// a column added without its value — or the reverse — shifted every later cell
/// one place left, so `presentation.sort_order` was read as a language code and
/// the failure surfaced as "no entry found for key" in an unrelated test.
///
/// That happened once while adding `elections_order`. The two lists are still
/// separate, because pairing them would mean a `Vec<(String, Cell)>` per row and
/// the column names repeated on every row; this catches the mistake at the one
/// point every sheet passes through instead.
fn sheet_of(
    name: &str,
    columns: Vec<String>,
    rows: Vec<Vec<Cell>>,
) -> Result<Sheet, Problem> {
    for (index, row) in rows.iter().enumerate() {
        if row.len() != columns.len() {
            return Err(Problem::error(
                Code::ConflictingColumns,
                format!("{name}[{index}]"),
                format!(
                    "{} cells under {} columns — a column was added without its \
                     value, or a value without its column",
                    row.len(),
                    columns.len()
                ),
            ));
        }
    }

    let mut grid =
        vec![columns.iter().map(|c| Cell::text(c.clone())).collect()];
    grid.extend(rows);
    Sheet::from_grid(name, &grid)
}

/// `presentation.i18n.<lang>.<field>` columns, one per language.
fn i18n_columns(
    prefix: &str,
    field: &str,
    languages: &[String],
) -> Vec<String> {
    languages
        .iter()
        .map(|language| format!("{prefix}.i18n.{language}.{field}"))
        .collect()
}

/// A `Translated`, or the plain string an older plan carried.
///
/// `PlannedContest.description` was a `String` for one release. A plan saved then
/// has `"description": "One seat"` where a map now belongs, and serde would refuse
/// it — so a client who saved a plan and came back would be told their own file was
/// not a plan.
///
/// Read into English, which is what the flat column meant.
fn translated_or_plain<'de, D>(input: D) -> Result<Translated, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Either {
        Plain(String),
        ByLanguage(Translated),
    }

    Ok(match Either::deserialize(input)? {
        Either::ByLanguage(text) => text,
        Either::Plain(text) if text.is_empty() => Translated::default(),
        Either::Plain(text) => Translated::new(&text),
    })
}

/// The English text, for the flat column beside the i18n block.
///
/// Both are written because the Admin Portal keeps both in step and its list
/// views read the flat one. `Translated::get` falls back, so a plan that lists no
/// English still puts something there rather than a blank.
fn english_of(text: &Translated) -> Cell {
    match text.get("en") {
        Some(value) => Cell::text(value),
        None => Cell::Blank,
    }
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
    columns.extend(i18n_columns("presentation", "name", languages));
    columns.extend(i18n_columns("presentation", "description", languages));
    // The flat column too, mirroring the English one. The Admin Portal keeps
    // both in step — `newEvent.description = presentation.i18n.en.description` —
    // and a bundle with only the i18n block leaves every list view blank.
    columns.push("description".to_string());
    columns.push("presentation.elections_order".to_string());
    columns
        .push("presentation.language_conf.enabled_language_codes".to_string());
    columns
        .push("presentation.language_conf.default_language_code".to_string());

    let mut row = vec![Cell::text(plan.external_id.clone())];
    row.extend(i18n_values(&plan.name, languages));
    row.extend(i18n_values(&plan.description, languages));
    row.push(english_of(&plan.description));
    row.push(Cell::text(plan.elections_order.clone()));
    // A JSON array in one cell: the reader parses bracketed text as JSON, which is
    // how a list fits in a spreadsheet and therefore in a synthesised one too.
    row.push(Cell::text(
        serde_json::to_string(languages).unwrap_or_else(|_| "[]".to_string()),
    ));
    // The chosen default, or the first language when nobody chose. A default
    // naming a language the ballot is not offered in is refused by
    // `check_languages` rather than written here, so this cannot emit one.
    row.push(Cell::text(
        plan.default_language
            .clone()
            .filter(|chosen| languages.iter().any(|each| each == chosen))
            .or_else(|| languages.first().cloned())
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
    columns.extend(i18n_columns("presentation", "name", languages));
    columns.extend(i18n_columns("presentation", "description", languages));
    columns.push("description".to_string());
    columns.push("num_allowed_revotes".to_string());
    columns.push("spoil_ballot_option".to_string());
    columns.push("permission_label".to_string());
    columns.push("presentation.grace_period_policy".to_string());
    columns.push("presentation.grace_period_secs".to_string());
    columns.push("presentation.start_screen_title_policy".to_string());
    columns.push("presentation.contests_order".to_string());

    let rows = plan
        .elections
        .iter()
        .map(|election| {
            let mut row = vec![Cell::text(election.external_id.clone())];
            row.extend(i18n_values(&election.name, languages));
            row.extend(i18n_values(&election.description, languages));
            row.push(english_of(&election.description));
            row.push(Cell::Int(election.num_allowed_revotes));
            row.push(Cell::Bool(election.spoil_ballot_option));
            // Blank rather than an empty string, so the builder derives one
            // instead of setting the label to "".
            row.push(if election.permission_label.is_empty() {
                Cell::Blank
            } else {
                Cell::text(election.permission_label.clone())
            });
            row.push(Cell::text(election.grace_period_policy.clone()));
            row.push(Cell::Int(election.grace_period_secs));
            row.push(Cell::text(election.start_screen_title_policy.clone()));
            row.push(Cell::text(election.contests_order.clone()));
            row
        })
        .collect();

    sheet_of("Elections", columns, rows)
}

fn contests_sheet(
    plan: &Blueprint,
    languages: &[String],
) -> Result<Sheet, Problem> {
    // The behaviour columns come from the policy module rather than being
    // listed here, so a new policy is one declaration rather than a column, a
    // cell and two places to forget.
    let behaviour = Behaviour::default().columns();

    let mut columns = vec![
        "external_id".to_string(),
        "election.external_id".to_string(),
        "max_votes".to_string(),
        "winning_candidates_num".to_string(),
        "description".to_string(),
        "presentation.sort_order".to_string(),
        "presentation.allow_writeins".to_string(),
    ];
    columns.extend(i18n_columns("presentation", "description", languages));
    columns.extend(behaviour.iter().map(|(column, _)| (*column).to_string()));
    columns.extend(i18n_columns("presentation", "name", languages));

    let mut rows = Vec::new();
    for election in &plan.elections {
        for (order, contest) in election.contests.iter().enumerate() {
            let mut row = vec![
                Cell::text(contest.external_id.clone()),
                Cell::text(election.external_id.clone()),
                Cell::Int(contest.max_votes),
                Cell::Int(contest.winners),
                english_of(&contest.description),
                Cell::Int(order as i64),
                Cell::Bool(contest.allow_writeins),
            ];
            row.extend(i18n_values(&contest.description, languages));
            row.extend(
                resolve(plan, election, contest)
                    .columns()
                    .into_iter()
                    .map(|(_, cell)| cell),
            );
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
        "presentation.is_write_in".to_string(),
    ];
    columns.extend(i18n_columns("presentation", "name", languages));
    columns.extend(i18n_columns("presentation", "description", languages));
    columns.push("description".to_string());

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
                    Cell::Bool(false),
                ];
                row.extend(i18n_values(&candidate.name, languages));
                row.extend(i18n_values(&candidate.description, languages));
                row.push(english_of(&candidate.description));
                rows.push(row);
            }

            // One row per write-in slot, after the real candidates.
            //
            // Minted here rather than kept in the plan, because a write-in slot
            // is not a candidate anybody named: it is a blank line, and the
            // number of them is the whole decision. Keeping them in
            // `contest.candidates` would put empty rows in the ballot's own list
            // and make "how many candidates are standing" wrong.
            if contest.allow_writeins {
                let start = contest.candidates.len();
                for slot in 0..contest.write_in_slots {
                    let mut row = vec![
                        Cell::text(format!(
                            "{}-write-in-{}",
                            contest.external_id,
                            slot + 1
                        )),
                        Cell::text(contest.external_id.clone()),
                        Cell::Int((start as i64) + slot),
                        Cell::Bool(false),
                        Cell::Bool(false),
                        Cell::Bool(true),
                    ];
                    // Named, because the Voting Portal draws the name as the
                    // slot's label and an unnamed one is a blank box with no
                    // indication of what it is for.
                    let mut named = Translated::default();
                    for language in languages {
                        named.by_language.insert(
                            language.clone(),
                            format!("Write-in {}", slot + 1),
                        );
                    }
                    row.extend(i18n_values(&named, languages));
                    // A slot has no description; the columns still have to line
                    // up or the sheet is ragged and `build_tables` reads the
                    // wrong value into the wrong field.
                    row.extend(i18n_values(&Translated::default(), languages));
                    row.push(Cell::Blank);
                    rows.push(row);
                }
            }
        }
    }

    sheet_of("Candidates", columns, rows)
}

/// The plan's areas, or the one that covers everybody.
///
/// A plan that has never thought about districting still needs an area and a
/// ballot link, or no voter sees anything. See [`DEFAULT_AREA_EXTERNAL_ID`].
/// The census, when the plan carries one.
///
/// `None` rather than an empty sheet when it does not: `build_tables` treats a
/// present-but-empty Voters sheet as "this election has no voters", which is a
/// different claim from "this plan does not carry the census" and produces a
/// bundle that imports an election nobody can vote in.
///
/// Columns are `VOTER_LEADING_COLUMNS` minus the two the builder derives — `id`
/// comes from `ids::uid` and `authorized-election-ids` from the areas — plus
/// whatever else the client carries, in a stable order so two builds of one
/// census diff cleanly. Everything unrecognised is passed through, which is how
/// a reporting breakout column survives the round trip.
fn voters_sheet(plan: &Blueprint) -> Result<Option<Sheet>, Problem> {
    if plan.voters.is_empty() {
        return Ok(None);
    }

    /// The columns `PlannedVoter` names in its own right.
    ///
    /// Spelled here rather than borrowed from `build_tables::VOTER_LEADING_COLUMNS`
    /// because that module sits behind `election_config_templates` and this one
    /// does not — reaching for it would drag the whole builder into a front end
    /// that only wants to describe a plan. The two lists are checked against
    /// each other by `the_voters_sheet_matches_what_the_builder_reads`.
    const NAMED: &[&str] =
        &["username", "email", "first_name", "last_name", "area_name"];

    // Sorted, so a census that gains a column does not reorder the ones it had.
    let mut extra: Vec<&str> = plan
        .voters
        .iter()
        .flat_map(|voter| voter.extra.keys().map(String::as_str))
        .filter(|key| {
            !NAMED.contains(key)
                && !["id", "enabled", "email_verified"].contains(key)
                && *key != "area.external_id"
                && *key != "authorized-election-ids"
        })
        .collect();
    extra.sort_unstable();
    extra.dedup();

    let mut columns: Vec<String> =
        NAMED.iter().map(|name| (*name).to_string()).collect();
    columns.extend(extra.iter().map(|key| (*key).to_string()));

    let rows = plan
        .voters
        .iter()
        .map(|voter| {
            let mut row = vec![
                Cell::text(voter.username.clone()),
                text_or_blank(&voter.email),
                text_or_blank(&voter.first_name),
                text_or_blank(&voter.last_name),
                text_or_blank(&voter.area_name),
            ];
            row.extend(extra.iter().map(|key| {
                voter
                    .extra
                    .get(*key)
                    .map(|value| Cell::text(value.clone()))
                    .unwrap_or(Cell::Blank)
            }));
            row
        })
        .collect();

    sheet_of("Voters", columns, rows).map(Some)
}

/// A cell, or nothing at all — so a blank stays distinguishable from `""`.
fn text_or_blank(value: &str) -> Cell {
    if value.is_empty() {
        Cell::Blank
    } else {
        Cell::text(value.to_string())
    }
}

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

/// Everything a plan produces.
#[derive(Debug, Clone)]
pub struct Compiled {
    pub bundle: Bundle,

    /// The files, split into what goes inside the importable archive and what
    /// must not. [`side_files`] is already folded into `auxiliary`.
    pub layout: Layout,

    /// Warnings from every pass. Errors are returned as `Err` instead, so a
    /// caller holding a `Compiled` is holding something worth writing out.
    pub report: Report,
}

/// Turn a plan into what it is for.
///
/// This is the function the wizard and the CLI both call, and until it existed
/// there was nothing joining the two halves of this module: [`to_workbook`] and
/// [`side_files`] had no callers outside the tests, so a plan could be validated
/// and mapped but never actually built.
///
/// Six steps, none of them new — which is the point. A plan becomes the same rows
/// a spreadsheet produces, and everything after that is the existing builder:
///
/// 1. [`validate_plan`], in the plan's own vocabulary. Errors stop here, because
///    a problem phrased as `contests[2].winning_candidates_num` is no use to
///    somebody looking at a wizard.
/// 2. [`to_workbook`].
/// 3. [`super::build`].
/// 4. Deserialize the export into [`ImportElectionEventSchema`] and
///    [`super::validate`] it — the same second pass `step-cli` and
///    `buildFromWorkbook` each make. Two implementations that merely looked
///    similar would not survive this.
/// 5. [`super::archive::layout`].
/// 6. [`side_files`] into `auxiliary`, because a ceremony schedule is not part of
///    an import and putting it inside the archive would suggest otherwise.
pub fn compile_plan(
    plan: &Blueprint,
    templates: &TemplateSet,
    options: &BuildOptions,
    profile: Option<&Profile>,
) -> Result<Compiled, Report> {
    // The profile first, so a locked value is the one that gets validated and
    // the one that gets built. Checking the plan as written and then forcing the
    // value afterwards would report problems about text nobody will ship.
    let applied;
    let plan = match profile {
        Some(profile) => {
            applied = apply_profile(plan, profile)?;
            &applied
        }
        None => plan,
    };

    let mut report = validate_plan(plan);
    if let Some(profile) = profile {
        for problem in profile.warnings.problems.clone() {
            report.push(problem);
        }
        check_required(plan, profile, &mut report);
    }
    if report.has_errors() {
        return Err(report);
    }

    let workbook = to_workbook(plan).map_err(|problem| {
        let mut failed = Report::default();
        failed.push(problem);
        failed
    })?;

    let bundle = build(&workbook, templates, options)?;

    for problem in bundle.warnings.problems.clone() {
        report.push(problem);
    }

    match serde_json::from_value::<ImportElectionEventSchema>(
        bundle.export.clone(),
    ) {
        Ok(schema) => {
            for problem in super::validate(&schema).problems {
                report.push(problem);
            }
        }
        Err(error) => report.push(Problem::error(
            Code::InvalidValue,
            "bundle",
            format!(
                "the built bundle does not match the import schema, which is a \
                 bug in this tool rather than in the plan: {error}"
            ),
        )),
    }

    if report.has_errors() {
        return Err(report);
    }

    let mut layout = super::archive::layout(&bundle);
    for (name, contents) in side_files(plan) {
        layout.auxiliary.push(Artifact {
            name,
            bytes: contents.into_bytes(),
        });
    }

    Ok(Compiled {
        bundle,
        layout,
        report,
    })
}

#[cfg(test)]
#[path = "architect_tests.rs"]
mod architect_tests;
