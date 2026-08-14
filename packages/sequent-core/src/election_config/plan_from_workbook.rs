// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Reading a [`Workbook`] back as a [`Blueprint`].
//!
//! The inverse of [`super::architect::to_workbook`], and the reason the wizard can
//! open a spreadsheet at all. One file rather than several, because it is the
//! mirror of one function: splitting it would put the two halves of a single
//! mapping in two places, and the failure mode of that is a column somebody added
//! to the writer and forgot here — which the round-trip test exists to catch and
//! could not if the halves drifted apart quietly.
//!
//! **Every error is collected, never returned early.** A workbook with four hundred
//! problems is the normal case for a first attempt, and the screen that shows them
//! groups by tab and by kind. A reader that stopped at the first would make that
//! screen pointless and turn one afternoon's fixing into four hundred rounds.
//!
//! **What cannot come back exactly**, all of it behaviour-preserving:
//!
//! - The Contests sheet carries *resolved* behaviour — event defaults, then the
//!   election's shared set or the contest's own overrides, flattened by
//!   `resolve`. Read back, every contest carries its values explicitly. The ballot
//!   behaves identically; the plan is shaped differently, and `blueprint.json`
//!   remains the record of which level a value was set at.
//! - A contest on every area was written as one row per area, and comes back as an
//!   explicit list rather than the empty "all of them".
//! - A plan with no areas emitted the synthesised `all-voters`, which comes back as
//!   a real area.
//! - **A missing translation becomes a real one.** `Translated::get` falls back to
//!   English on purpose — a blank is never the right answer for a candidate's name
//!   — so the writer fills every language column, and reading them back
//!   materialises the fallback as text. A plan with an English-only candidate comes
//!   back with that name in every language. Every screen and every ballot shows
//!   exactly what it showed before; only the plan's shape differs.
//! - Bytes never travel in a cell. A logo, a candidate's photograph and a support
//!   material keep their *file names* and lose their contents, which is why the
//!   wizard says so when a workbook is opened.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::election_config::architect::{
    Blueprint, Contact, MessageKind, MessageSchedule, Milestone, PlannedArea,
    PlannedCandidate, PlannedContest, PlannedElection, PlannedMaterial,
    PlannedMessage, PlannedVoter, Translated, Trustee, VotingChannelSet,
    BLUEPRINT_VERSION,
};
use crate::election_config::paths::cell_text;
use crate::election_config::problem::{Code, Problem, Report};
use crate::election_config::sheet::{Origin, Row, Sheet, Workbook};
use crate::election_config::time::Timestamp;
use crate::types::ceremonies::CeremoniesPolicy;

/// A plan read out of a workbook, and anything odd about it.
#[derive(Debug, Clone)]
pub struct ReadPlan {
    pub plan: Blueprint,
    /// Warnings only. Errors come back as `Err(Report)`.
    pub report: Report,
}

/// Sheets the wizard has no screens for, carried rather than interpreted.
const PLATFORM_SHEETS: &[&str] = &[
    "parameters",
    "adminusers",
    "permissions",
    "templates",
    "reports",
];

/// Read a workbook as a plan.
///
/// Errors come back as `Err(Report)` and warnings ride in [`ReadPlan::report`] —
/// the same split [`super::build::build`] makes, so the wizard and the janitor
/// treat a bad document the same way.
pub fn plan_from_workbook(workbook: &Workbook) -> Result<ReadPlan, Report> {
    let mut report = Report::default();
    let mut plan = Blueprint {
        version: BLUEPRINT_VERSION,
        ..Blueprint::default()
    };

    read_event(workbook, &mut plan, &mut report);

    let areas = read_areas(workbook, &mut report);
    let (mut elections, by_election) = read_elections(workbook, &mut report);
    let by_contest =
        read_contests(workbook, &mut elections, &by_election, &mut report);
    read_candidates(workbook, &mut elections, &by_contest, &mut report);
    read_area_contests(
        workbook,
        &areas,
        &mut elections,
        &by_contest,
        &mut report,
    );

    plan.areas = areas;
    plan.elections = elections;
    plan.voters = read_voters(workbook, &plan.areas, &mut report);
    plan.materials = read_materials(workbook, &plan.languages, &mut report);

    read_schedule(workbook, &mut plan, &mut report);
    read_ceremony(workbook, &mut plan, &mut report);
    plan.messages = read_messages(workbook, &mut report);
    plan.contacts = read_contacts(workbook);
    plan.trustees = read_trustees(workbook);
    plan.notes = read_notes(workbook);
    plan.platform = carried(workbook);

    if report.has_errors() {
        return Err(report);
    }
    Ok(ReadPlan { plan, report })
}

// -- saying what is wrong ------------------------------------------------------

/// Every complaint goes through here, so none can be raised without its cell.
///
/// The same discipline `Builder::problem` enforces on the other side: a locator
/// that is optional at the call site is a locator somebody forgets, and the screen
/// that groups these by tab then has a bucket labelled "somewhere".
fn refuse(
    report: &mut Report,
    row: &Row,
    column: &str,
    code: Code,
    message: impl Into<String>,
) {
    let origin = row.origin(Some(column));
    report.push(Problem::error(code, origin.to_string(), message).at(&origin));
}

fn warn_about(
    report: &mut Report,
    row: &Row,
    column: &str,
    message: impl Into<String>,
) {
    let origin = row.origin(Some(column));
    report.push(
        Problem::warning(Code::InvalidValue, origin.to_string(), message)
            .at(&origin),
    );
}

// -- reading cells -------------------------------------------------------------

/// A cell as text, whatever it is.
///
/// Not `Row::text`, which answers only for a string: a `key | value` sheet holds a
/// threshold as a *number*, and the ceremony's offset likewise, so reading only
/// strings there finds an empty cell and complains that `''` is not a number of
/// trustees. `cell_text` is the writer's own rendering, which makes this pair
/// symmetrical rather than nearly so.
fn text(row: &Row, column: &str) -> String {
    match row.get(column) {
        None => String::new(),
        Some(Value::String(value)) => value.trim().to_string(),
        Some(other) => cell_text(other, false),
    }
}

fn flag(row: &Row, column: &str) -> Option<bool> {
    match row.get(column) {
        Some(Value::Bool(value)) => Some(*value),
        Some(Value::String(value)) => {
            match value.trim().to_lowercase().as_str() {
                "true" | "yes" | "1" | "x" => Some(true),
                "false" | "no" | "0" => Some(false),
                _ => None,
            }
        }
        Some(Value::Number(value)) => Some(value.as_i64() != Some(0)),
        _ => None,
    }
}

fn whole(row: &Row, column: &str) -> Option<i64> {
    match row.get(column) {
        Some(Value::Number(value)) => value.as_i64(),
        Some(Value::String(value)) => value.trim().parse().ok(),
        _ => None,
    }
}

/// The i18n columns for one field, gathered back into a `Translated`.
fn translated(row: &Row, prefix: &str, field: &str) -> Translated {
    let mut by_language = BTreeMap::new();
    for (header, value) in &row.cells {
        let wanted = format!("{prefix}.i18n.");
        let Some(rest) = header.strip_prefix(&wanted) else {
            continue;
        };
        let Some((language, name)) = rest.split_once('.') else {
            continue;
        };
        if name != field {
            continue;
        }
        if let Value::String(text) = value {
            if !text.trim().is_empty() {
                by_language.insert(language.to_string(), text.clone());
            }
        }
    }
    Translated { by_language }
}

// -- the sheets ----------------------------------------------------------------

fn read_event(workbook: &Workbook, plan: &mut Blueprint, report: &mut Report) {
    let rows = workbook.rows("electionevent");
    let Some(row) = rows.first() else {
        // No row to point at, but there is a tab — which is what the screen
        // groups by, so a sheet-level complaint still belongs somewhere rather
        // than in a bucket labelled "somewhere".
        report.push(
            Problem::error(
                Code::MissingField,
                "sheet 'ElectionEvent'",
                "a configuration needs an ElectionEvent sheet with one row in it",
            )
            .at(&Origin::sheet("ElectionEvent")),
        );
        return;
    };
    if rows.len() > 1 {
        warn_about(
            report,
            &rows[1],
            "external_id",
            "only the first ElectionEvent row is read; the rest are ignored",
        );
    }

    plan.external_id = text(row, "external_id");
    if plan.external_id.is_empty() {
        refuse(
            report,
            row,
            "external_id",
            Code::MissingField,
            "the election event needs an external_id: every generated \
             identifier is derived from it",
        );
    }

    plan.name = translated(row, "presentation", "name");
    plan.description = translated(row, "presentation", "description");

    plan.languages =
        match row.get("presentation.language_conf.enabled_language_codes") {
            Some(Value::Array(items)) => items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect(),
            Some(Value::String(one)) if !one.trim().is_empty() => {
                vec![one.trim().to_string()]
            }
            _ => Vec::new(),
        };
    let default_language =
        text(row, "presentation.language_conf.default_language_code");
    plan.default_language =
        (!default_language.is_empty()).then_some(default_language);
    let detection =
        text(row, "presentation.language_conf.language_detection_policy");
    plan.language_detection_policy =
        (!detection.is_empty()).then_some(detection);

    plan.elections_order = text(row, "presentation.elections_order");
    plan.show_cast_vote_logs = text(row, "presentation.show_cast_vote_logs");

    plan.voting_channels = VotingChannelSet {
        online: flag(row, "voting_channels.online").unwrap_or(true),
        kiosk: flag(row, "voting_channels.kiosk").unwrap_or(false),
        telephone: flag(row, "voting_channels.telephone").unwrap_or(false),
        early_voting: flag(row, "voting_channels.early_voting")
            .unwrap_or(false),
    };

    let logo_url = text(row, "presentation.logo_url");
    plan.logo_url = (!logo_url.is_empty()).then_some(logo_url);
    // The *name* of a logo file, with no bytes behind it. Said out loud rather
    // than dropped silently: somebody who rebuilds from this workbook alone gets
    // an election event with no logo, and would otherwise find out by looking.
    let logo_file = text(row, "presentation.logo_file");
    if !logo_file.is_empty() {
        warn_about(
            report,
            row,
            "presentation.logo_file",
            format!(
                "'{logo_file}' names an image this file cannot carry. Choose it \
                 again on the Election Event screen, or the build has no logo."
            ),
        );
    }

    plan.skip_election_list = flag(row, "presentation.skip_election_list");
    plan.show_user_profile = flag(row, "presentation.show_user_profile");
    plan.materials_activated = flag(row, "presentation.materials.activated");
    plan.materials_title = translated(row, "presentation", "materialsTitle");
    plan.materials_subtitle =
        translated(row, "presentation", "materialsSubtitle");
}

fn read_areas(workbook: &Workbook, report: &mut Report) -> Vec<PlannedArea> {
    workbook
        .rows("areas")
        .iter()
        .filter_map(|row| {
            let external_id = text(row, "external_id");
            if external_id.is_empty() {
                refuse(
                    report,
                    row,
                    "external_id",
                    Code::MissingField,
                    "an area needs an external_id, because voters and contests \
                     name it by one",
                );
                return None;
            }
            Some(PlannedArea {
                external_id,
                name: text(row, "name"),
                parent_external_id: {
                    let parent = text(row, "parent.external_id");
                    (!parent.is_empty()).then_some(parent)
                },
                allow_early_voting: flag(
                    row,
                    "presentation.allow_early_voting",
                )
                .unwrap_or(false),
            })
        })
        .collect()
}

fn read_elections(
    workbook: &Workbook,
    report: &mut Report,
) -> (Vec<PlannedElection>, BTreeMap<String, usize>) {
    let mut elections = Vec::new();
    let mut by_id = BTreeMap::new();

    for row in workbook.rows("elections") {
        let external_id = text(row, "external_id");
        if external_id.is_empty() {
            refuse(
                report,
                row,
                "external_id",
                Code::MissingField,
                "an election needs an external_id: its contests name it by one",
            );
            continue;
        }
        if by_id.contains_key(&external_id) {
            refuse(
                report,
                row,
                "external_id",
                Code::DuplicateId,
                format!("'{external_id}' names two elections"),
            );
            continue;
        }

        by_id.insert(external_id.clone(), elections.len());
        elections.push(PlannedElection {
            external_id,
            name: translated(row, "presentation", "name"),
            description: translated(row, "presentation", "description"),
            num_allowed_revotes: whole(row, "num_allowed_revotes").unwrap_or(1),
            spoil_ballot_option: flag(row, "spoil_ballot_option")
                .unwrap_or(false),
            permission_label: text(row, "permission_label"),
            grace_period_policy: text(row, "presentation.grace_period_policy"),
            grace_period_secs: whole(row, "presentation.grace_period_secs")
                .unwrap_or_default(),
            start_screen_title_policy: text(
                row,
                "presentation.start_screen_title_policy",
            ),
            contests_order: text(row, "presentation.contests_order"),
            ..PlannedElection::default()
        });
    }

    (elections, by_id)
}

fn read_contests(
    workbook: &Workbook,
    elections: &mut [PlannedElection],
    by_election: &BTreeMap<String, usize>,
    report: &mut Report,
) -> BTreeMap<String, (usize, usize)> {
    let mut by_id = BTreeMap::new();

    for row in workbook.rows("contests") {
        let external_id = text(row, "external_id");
        let election_id = text(row, "election.external_id");

        if external_id.is_empty() {
            refuse(
                report,
                row,
                "external_id",
                Code::MissingField,
                "a contest needs an external_id: its candidates name it by one",
            );
            continue;
        }
        let Some(&at) = by_election.get(&election_id) else {
            // Dropped rather than attached to the first election, which would
            // put a contest on a ballot nobody asked for.
            refuse(
                report,
                row,
                "election.external_id",
                Code::DanglingReference,
                format!("no election is called '{election_id}'"),
            );
            continue;
        };

        let contest = PlannedContest {
            external_id: external_id.clone(),
            name: translated(row, "presentation", "name"),
            description: translated(row, "presentation", "description"),
            max_votes: whole(row, "max_votes").unwrap_or(1),
            winners: whole(row, "winning_candidates_num").unwrap_or(1),
            allow_writeins: flag(row, "presentation.allow_writeins")
                .unwrap_or(false),
            ..PlannedContest::default()
        };

        by_id.insert(external_id, (at, elections[at].contests.len()));
        elections[at].contests.push(contest);
    }

    by_id
}

fn read_candidates(
    workbook: &Workbook,
    elections: &mut [PlannedElection],
    by_contest: &BTreeMap<String, (usize, usize)>,
    report: &mut Report,
) {
    for row in workbook.rows("candidates") {
        let contest_id = text(row, "contest.external_id");
        let Some(&(election, contest)) = by_contest.get(&contest_id) else {
            refuse(
                report,
                row,
                "contest.external_id",
                Code::DanglingReference,
                format!("no contest is called '{contest_id}'"),
            );
            continue;
        };

        // A write-in row is a *slot*, not a candidate. Reading them as candidates
        // gives a contest people called "Write-in 1" standing in it, and makes
        // "how many are standing" wrong on every screen that counts.
        if flag(row, "presentation.is_write_in").unwrap_or(false) {
            let target = &mut elections[election].contests[contest];
            target.allow_writeins = true;
            target.write_in_slots += 1;
            continue;
        }

        let external_id = text(row, "external_id");
        if external_id.is_empty() {
            refuse(
                report,
                row,
                "external_id",
                Code::MissingField,
                "a candidate needs an external_id",
            );
            continue;
        }

        elections[election].contests[contest].candidates.push(
            PlannedCandidate {
                external_id,
                name: translated(row, "presentation", "name"),
                description: translated(row, "presentation", "description"),
                explicit_blank: flag(row, "presentation.is_explicit_blank")
                    .unwrap_or(false),
                explicit_invalid: flag(row, "presentation.is_explicit_invalid")
                    .unwrap_or(false),
                disabled: flag(row, "presentation.is_disabled")
                    .unwrap_or(false),
                // Bytes never travel in a cell. The identifier the writer put
                // here is the platform's, derived from the candidate, so there is
                // nothing to keep: a photograph has to be chosen again.
                image: None,
            },
        );
    }
}

fn read_area_contests(
    workbook: &Workbook,
    areas: &[PlannedArea],
    elections: &mut [PlannedElection],
    by_contest: &BTreeMap<String, (usize, usize)>,
    report: &mut Report,
) {
    let mut wanted: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for row in workbook.rows("areacontests") {
        let contest_id = text(row, "contest.external_id");
        let area_id = text(row, "area.external_id");

        if !by_contest.contains_key(&contest_id) {
            refuse(
                report,
                row,
                "contest.external_id",
                Code::DanglingReference,
                format!("no contest is called '{contest_id}'"),
            );
            continue;
        }
        if !areas.iter().any(|area| area.external_id == area_id) {
            refuse(
                report,
                row,
                "area.external_id",
                Code::DanglingReference,
                format!("no area is called '{area_id}'"),
            );
            continue;
        }
        wanted.entry(contest_id).or_default().push(area_id);
    }

    for (contest_id, mut on) in wanted {
        let Some(&(election, contest)) = by_contest.get(&contest_id) else {
            continue;
        };
        on.sort();
        on.dedup();
        // Every area means "no restriction", which is how the writer expanded an
        // empty list in the first place. Anything less stays explicit.
        let all: Vec<String> =
            areas.iter().map(|area| area.external_id.clone()).collect();
        let mut sorted_all = all.clone();
        sorted_all.sort();
        elections[election].contests[contest].areas =
            if on == sorted_all { Vec::new() } else { on };
    }
}

/// A voter's area, by identifier, however the sheet spelled it.
///
/// `area.external_id` when it is there. Otherwise the older `area_name`, resolved
/// against the plan's own areas — and an unmatched name is **kept as written**
/// rather than blanked, so validation reports "no area has external_id 'North
/// Local 4'" against the row that has it. A blank would be a voter who silently
/// gets no ballot, which is the same information with nowhere to act on it.
fn area_of(row: &Row, areas: &[PlannedArea]) -> String {
    let by_id = text(row, "area.external_id");
    if !by_id.trim().is_empty() {
        return by_id;
    }

    let name = text(row, "area_name");
    let name = name.trim();
    if name.is_empty() {
        return String::new();
    }
    areas
        .iter()
        .find(|area| area.name.trim() == name)
        .map(|area| area.external_id.clone())
        .unwrap_or_else(|| name.to_string())
}

/// The voters sheet, keyed to areas by `area.external_id`.
///
/// `areas` is passed in for one reason: a file written before this column existed
/// carries `area_name` instead, and every census a client already has is such a
/// file. Read strictly, all of them would open with every voter's area blank —
/// thousands of rows, each of them refused by the build, and nothing on screen to
/// say the column had simply moved. So a sheet with no `area.external_id` falls
/// back to resolving the name, once, on the way in.
fn read_voters(
    workbook: &Workbook,
    areas: &[PlannedArea],
    report: &mut Report,
) -> Vec<PlannedVoter> {
    let known = [
        "username",
        "email",
        "first_name",
        "last_name",
        "area_name",
        "area.external_id",
    ];

    workbook
        .rows("voters")
        .iter()
        .filter_map(|row| {
            let username = text(row, "username");
            if username.is_empty() {
                refuse(
                    report,
                    row,
                    "username",
                    Code::MissingField,
                    "a voter needs a username: it is what they sign in as",
                );
                return None;
            }

            let extra = row
                .cells
                .iter()
                .filter(|(header, _)| !known.contains(&header.as_str()))
                .filter_map(|(header, value)| {
                    value
                        .as_str()
                        .map(|text| (header.clone(), text.to_string()))
                })
                .collect();

            Some(PlannedVoter {
                username,
                email: text(row, "email"),
                first_name: text(row, "first_name"),
                last_name: text(row, "last_name"),
                area_external_id: area_of(row, areas),
                extra,
            })
        })
        .collect()
}

fn read_materials(
    workbook: &Workbook,
    languages: &[String],
    report: &mut Report,
) -> Vec<PlannedMaterial> {
    let _ = languages;
    workbook
        .rows("materials")
        .iter()
        .map(|row| {
            let file_name = text(row, "file");
            if !file_name.is_empty() {
                warn_about(
                    report,
                    row,
                    "file",
                    format!(
                        "'{file_name}' names a document this file cannot carry. \
                         Add it again on the Election Event screen, or the build \
                         has no support materials."
                    ),
                );
            }
            PlannedMaterial {
                external_id: text(row, "external_id"),
                title: translated(row, "presentation", "title"),
                kind: text(row, "kind"),
                file_name,
                is_hidden: flag(row, "is_hidden").unwrap_or(false),
                bytes: Vec::new(),
            }
        })
        .collect()
}

fn read_schedule(
    workbook: &Workbook,
    plan: &mut Blueprint,
    report: &mut Report,
) {
    for row in workbook.rows("scheduledevents") {
        let kind = text(row, "event_type");
        let when = text(row, "scheduled_datetime");
        if when.is_empty() {
            continue;
        }
        // Written as an instant, because the platform's scheduler parses it with
        // `parse_from_rfc3339` and an offset-less value never fires. Read back the
        // same way; a cell somebody typed by hand is a plain wall clock, so that
        // is the fallback rather than a refusal.
        let stamp = Timestamp::from_rfc3339(&when)
            .unwrap_or_else(|_| Timestamp::utc(when.clone()));

        match kind.as_str() {
            "START_VOTING_PERIOD" => plan.schedule.voting_opens = Some(stamp),
            "END_VOTING_PERIOD" => plan.schedule.voting_closes = Some(stamp),
            other => warn_about(
                report,
                row,
                "event_type",
                format!(
                    "'{other}' is not a voting-window event, so the wizard has \
                     no screen for it. It is kept in the file it came from."
                ),
            ),
        }
    }

    plan.schedule.milestones = workbook
        .rows("milestones")
        .iter()
        .map(|row| Milestone {
            event: text(row, "event"),
            date: text(row, "date"),
        })
        .collect();
}

fn read_ceremony(
    workbook: &Workbook,
    plan: &mut Blueprint,
    report: &mut Report,
) {
    let mut said: BTreeMap<String, (String, Row)> = BTreeMap::new();
    for row in workbook.rows("ceremony") {
        let key = text(row, "key");
        if !key.is_empty() {
            said.insert(key, (text(row, "value"), row.clone()));
        }
    }
    let value = |name: &str| said.get(name).map(|(text, _)| text.clone());

    if let Some(threshold) = value("threshold") {
        match threshold.parse::<u32>() {
            Ok(parsed) => plan.trustee_threshold = parsed,
            Err(_) => {
                if let Some((_, row)) = said.get("threshold") {
                    refuse(
                        report,
                        row,
                        "value",
                        Code::InvalidValue,
                        format!("'{threshold}' is not a number of trustees"),
                    );
                }
            }
        }
    }

    if let Some(policy) = value("policy") {
        match policy.as_str() {
            "manual-ceremonies" => {
                plan.ceremony_policy = CeremoniesPolicy::MANUAL_CEREMONIES
            }
            "automated-ceremonies" => {
                plan.ceremony_policy = CeremoniesPolicy::AUTOMATED_CEREMONIES
            }
            other => {
                if let Some((_, row)) = said.get("policy") {
                    refuse(
                        report,
                        row,
                        "value",
                        Code::InvalidValue,
                        format!(
                            "'{other}' is not a way to run a ceremony. It is \
                             either manual-ceremonies or automated-ceremonies."
                        ),
                    );
                }
            }
        }
    }

    let stamp = |name: &str| -> Option<Timestamp> {
        let local = value(name)?;
        Some(Timestamp {
            local,
            zone: value(&format!("{name}.zone")).unwrap_or_default(),
            offset_minutes: value(&format!("{name}.offset_minutes"))
                .and_then(|minutes| minutes.parse().ok())
                .unwrap_or(0),
        })
    };
    plan.schedule.key_ceremony = stamp("key_ceremony");
    plan.schedule.tally_ceremony = stamp("tally_ceremony");

    // The voting window's zone *name*, put back on the stamps `read_schedule`
    // already built from RFC3339. It cannot come from there: RFC3339 carries the
    // offset and not the name, so a window written in Los Angeles reopens as an
    // instant with no place attached, and the next edit resolves against whatever
    // zone the browser is in. Runs after `read_schedule` for that reason — it is
    // patching what that produced, not producing it.
    for (name, when) in [
        ("voting_opens", &mut plan.schedule.voting_opens),
        ("voting_closes", &mut plan.schedule.voting_closes),
    ] {
        if let (Some(stamp), Some(zone)) =
            (when.as_mut(), value(&format!("{name}.zone")))
        {
            stamp.zone = zone;
        }
    }
}

/// The messages, out of the sheet the writer put them in.
///
/// `schedule` is one JSON cell, which is the format's own convention for a
/// structured value — a send schedule is a list of timestamps each carrying a zone
/// and an offset, and spreading that across parallel `||` columns would be exact
/// only while every send shared a zone.
fn read_messages(
    workbook: &Workbook,
    report: &mut Report,
) -> Vec<PlannedMessage> {
    workbook
        .rows("messages")
        .iter()
        .filter_map(|row| {
            let alias = text(row, "kind");
            let kind = match alias.as_str() {
                "invitation-to-vote" => MessageKind::InvitationToVote,
                "get-out-the-vote" => MessageKind::GetOutTheVote,
                other => {
                    refuse(
                        report,
                        row,
                        "kind",
                        Code::InvalidValue,
                        format!(
                            "'{other}' is not a message the wizard sends. It is \
                             invitation-to-vote or get-out-the-vote."
                        ),
                    );
                    return None;
                }
            };

            let schedule = match row.get("schedule") {
                Some(value) => serde_json::from_value(value.clone())
                    .unwrap_or_else(|_| {
                        warn_about(
                            report,
                            row,
                            "schedule",
                            "this send schedule could not be read, so the \
                             message is loaded with none",
                        );
                        MessageSchedule::default()
                    }),
                None => MessageSchedule::default(),
            };

            Some(PlannedMessage {
                kind,
                subject: translated(row, "presentation", "subject"),
                body: translated(row, "presentation", "body"),
                html: translated(row, "presentation", "html"),
                schedule,
            })
        })
        .collect()
}

fn read_contacts(workbook: &Workbook) -> Vec<Contact> {
    workbook
        .rows("contacts")
        .iter()
        .map(|row| Contact {
            name: text(row, "name"),
            role: text(row, "role"),
            email: text(row, "email"),
        })
        .collect()
}

fn read_trustees(workbook: &Workbook) -> Vec<Trustee> {
    workbook
        .rows("trustees")
        .iter()
        .map(|row| Trustee {
            name: text(row, "name"),
            email: text(row, "email"),
        })
        .collect()
}

fn read_notes(workbook: &Workbook) -> String {
    workbook
        .rows("notes")
        .first()
        .map(|row| text(row, "notes"))
        .unwrap_or_default()
}

/// The sheets the wizard has no screens for, kept exactly as they came.
///
/// See [`Blueprint::platform`]: `build` already knows what every one of these
/// means, so the wizard's job is to hold on to them rather than to have an opinion.
fn carried(workbook: &Workbook) -> Vec<Sheet> {
    workbook
        .sheets()
        .iter()
        .filter(|sheet| PLATFORM_SHEETS.contains(&sheet.key.as_str()))
        .cloned()
        .collect()
}

#[cfg(test)]
#[path = "plan_from_workbook_tests.rs"]
mod plan_from_workbook_tests;
