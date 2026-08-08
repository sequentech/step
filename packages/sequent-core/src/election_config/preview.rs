// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What a voter will see, before anything has been imported.
//!
//! The Election Architect's review step shows the ballot. The temptation is to
//! read the plan and draw something that looks like a ballot, and that is the
//! one thing this must not do: a preview drawn from a second reading of the
//! configuration agrees with itself and not necessarily with the platform. Its
//! whole value is being able to say "that is what voters get", and a
//! re-implementation cannot honestly say it.
//!
//! So nothing here decides anything about a ballot. It calls
//! [`crate::ballot_style::create_ballot_style`] — the platform's own ballot-style
//! builder, the same function windmill runs when somebody presses *Generate
//! ballot publication* — over the entities the plan compiled into, and assembles
//! the result into the file the Voting Portal already knows how to open.
//!
//! Two consequences worth stating:
//!
//! * The input is [`ImportElectionEventSchema`], deserialized from the bundle
//!   that is about to be imported. Not the plan. So the preview is generated from
//!   the bytes that will be shipped, and cannot disagree with them.
//! * The output is byte-compatible with the publication preview windmill writes
//!   to the public bucket (`windmill::tasks::prepare_publication_preview`), which
//!   `voting-portal/src/routes/PreviewPublicationEvent.tsx` fetches and renders.
//!   So the Voting Portal itself can open it, at whichever version the client is
//!   running, and that render involves no code of ours at all.
//!
//! What is *not* real, and is labelled as such wherever it is shown:
//!
//! * The public key. A ballot style carries one; before the key ceremony there is
//!   none, so the preview carries a stand-in and the platform's own
//!   `is_demo: true` flag says so. Nothing can be cast against it.
//! * Ballot style ids. windmill mints a `Uuid::new_v4()` per style. A preview has
//!   to be reproducible — two previews of one plan being byte-identical is what
//!   makes them reviewable — so ids come from the plan's deterministic factory.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::ballot::{BallotStyle, PeriodDates};
use crate::ballot_style::{create_ballot_style, elections_contests_for_area};
use crate::services::area_tree::TreeNode;
use crate::types::hasura::core::{
    Area, AreaContest, Candidate, Contest, Election,
};
use crate::types::scheduled_event::{prepare_scheduled_dates, ScheduledEvent};

use super::build::{Bundle, JsonTable};
use super::emit::{JsonField, SCHEDULED_EVENT_COLUMNS};
use super::ids::IdFactory;
use super::problem::{Code, Problem, Report};
use super::schema::ImportElectionEventSchema;

/// The stand-in public key a preview carries.
///
/// Not a key: eight bytes of zeroes, base64. It exists because
/// [`BallotStyle::public_key`] is not optional and the Voting Portal reads it, and
/// it is deliberately not a *valid* key — the portal flags it `is_demo` and
/// nothing can be encrypted to it. Choosing a real-looking value here would be
/// the one way to make a preview dangerous.
pub const NOT_A_KEY: &str = "AAAAAAAAAAA=";

#[derive(Debug, Clone)]
pub struct PreviewOptions {
    /// Shown in place of the key the ceremony has not produced yet.
    ///
    /// Defaults to [`NOT_A_KEY`]. Overridable so a delivery engineer previewing
    /// against a tenant that *has* run its ceremony can paste the real key in and
    /// see the real thing.
    pub demo_public_key: String,
}

impl Default for PreviewOptions {
    fn default() -> Self {
        Self {
            demo_public_key: NOT_A_KEY.to_string(),
        }
    }
}

/// The file the Voting Portal opens.
///
/// The same five keys, in the same shape, as the JSON
/// `windmill::tasks::prepare_publication_preview` uploads and
/// `voting-portal/src/routes/PreviewPublicationEvent.tsx` fetches. Typed here
/// only as far as the two fields a preview has to get right; the other three are
/// carried as JSON because their contents come from the platform's own DB
/// projections (an election row there has an extra open-status field, for
/// instance) and inventing typed twins of those would be claiming to know more
/// than we do.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicationPreview {
    /// One per area × election, each the `ballot_eml` a `ballot_style` row holds.
    pub ballot_styles: Vec<BallotStyle>,
    pub election_event: Value,
    pub elections: Value,
    /// Voter-facing help documents. Always empty here: a plan carries no
    /// uploaded support material, and the field exists because the portal reads
    /// it.
    pub support_materials: Value,
    /// Uploaded files — logos, candidate photographs. Empty for the same reason.
    pub documents: Value,
}

impl PublicationPreview {
    /// The document, ready to write out or hand to JavaScript.
    ///
    /// Not `serde_json::to_string(&self)`, and the difference is not cosmetic. A
    /// ballot style carries its translations in `I18nContent`, which is a
    /// `HashMap`, and serializing a `HashMap` writes its keys in whatever order
    /// the hasher happened to produce — so two previews of one plan differ, in
    /// the same run, by nothing but the order of `{"en":…,"es":…}`. That defeats
    /// the point: a preview is a document somebody saves, forwards, and diffs
    /// against the one from last week.
    ///
    /// `serde_json::Value`'s object is a `BTreeMap`, so going through it sorts
    /// every key at every depth. One canonical form, and it is this method's
    /// whole job.
    pub fn to_document(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }
}

/// Generate the ballots a plan's voters would be given.
///
/// One [`BallotStyle`] per area × election, exactly as a publication produces
/// them: an area inherits its ancestors' contests, and an area that ends up with
/// no contests in an election gets no ballot style for it rather than an empty
/// one.
///
/// Errors are [`Report`] problems rather than an `anyhow` chain, because they are
/// shown next to the plan's other problems and have to name a field.
pub fn preview_publication(
    bundle: &Bundle,
    options: &PreviewOptions,
) -> Result<PublicationPreview, Report> {
    let mut report = Report::default();

    let schema: ImportElectionEventSchema =
        match serde_json::from_value(bundle.export.clone()) {
            Ok(schema) => schema,
            Err(error) => {
                report.push(Problem::error(
                    Code::InvalidValue,
                    "bundle",
                    format!(
                        "the built bundle does not match the import schema, so \
                         there is nothing to preview: {error}"
                    ),
                ));
                return Err(report);
            }
        };

    let scheduled = scheduled_events(&bundle.scheduled_events);

    let contests: HashMap<String, Contest> = schema
        .contests
        .iter()
        .map(|contest| (contest.id.clone(), contest.clone()))
        .collect();
    let candidates: Vec<Candidate> = schema.candidates.clone();
    let area_contests: HashMap<String, AreaContest> = schema
        .area_contests
        .iter()
        .map(|area_contest| (area_contest.id.clone(), area_contest.clone()))
        .collect();
    let elections: HashMap<String, Election> = schema
        .elections
        .iter()
        .map(|election| (election.id.clone(), election.clone()))
        .collect();
    let election_ids: Vec<String> = schema
        .elections
        .iter()
        .map(|each| each.id.clone())
        .collect();

    if election_ids.is_empty() {
        report.push(Problem::error(
            Code::MissingField,
            "elections",
            "there are no elections, so there is no ballot to show",
        ));
        return Err(report);
    }

    let tree: TreeNode = match TreeNode::from_areas(
        schema.areas.iter().map(|area| area.into()).collect(),
    ) {
        Ok(tree) => tree,
        Err(error) => {
            report.push(Problem::error(
                Code::InvalidValue,
                "areas",
                format!("the areas do not form a tree: {error}"),
            ));
            return Err(report);
        }
    };

    // Every contest in the event, which is what the encoding-mode resolution
    // wants — it is an election-wide decision, so all areas have to agree.
    let all_contests: Vec<Contest> = schema.contests.clone();

    // Deterministic, so two previews of one plan are the same bytes. windmill
    // mints a v4 uuid per style, which is right for a row in a table and wrong
    // for a document somebody diffs. Same factory, same namespace, so a preview
    // id is stable across runs and recognisably from this event.
    let Some(ids) = IdFactory::new(&bundle.event_external_id) else {
        report.push(Problem::error(
            Code::InvalidValue,
            "external_id",
            "the event has no identifier, so preview ballots cannot be named",
        ));
        return Err(report);
    };

    let mut styles: Vec<BallotStyle> = Vec::new();

    // Sorted, for the same reason the ids are deterministic.
    let mut areas: Vec<&Area> = schema.areas.iter().collect();
    areas.sort_by(|left, right| left.id.cmp(&right.id));

    for area in areas {
        let for_area = match elections_contests_for_area(
            area,
            &tree,
            &election_ids,
            &contests,
            &area_contests,
        ) {
            Ok(map) => map,
            Err(error) => {
                report.push(Problem::error(
                    Code::InvalidValue,
                    "areas",
                    format!(
                        "could not work out which contests area {} votes on: \
                         {error}",
                        area.id
                    ),
                ));
                return Err(report);
            }
        };

        let mut election_ids_for_area: Vec<&String> = for_area.keys().collect();
        election_ids_for_area.sort();

        for election_id in election_ids_for_area {
            let Some(election) = elections.get(election_id) else {
                continue;
            };
            let contest_ids = &for_area[election_id];

            let mut chosen: Vec<Contest> = contest_ids
                .iter()
                .filter_map(|contest_id| contests.get(contest_id).cloned())
                .collect();
            chosen.sort_by(|left, right| left.id.cmp(&right.id));

            let for_contests: Vec<Candidate> = candidates
                .iter()
                .filter(|candidate| {
                    candidate
                        .contest_id
                        .as_ref()
                        .is_some_and(|id| contest_ids.contains(id))
                })
                .cloned()
                .collect();

            let dates = {
                // Never started, never paused, never stopped — it has not been
                // imported yet. Only the schedule is real, and it comes from the
                // rows the importer will read.
                let mut dates = PeriodDates::default().to_string_fields();
                if let Ok(scheduled_dates) = prepare_scheduled_dates(
                    scheduled.clone(),
                    Some(&election.id),
                ) {
                    dates.scheduled_event_dates = Some(scheduled_dates);
                }
                dates
            };

            match create_ballot_style(
                ids.uid("ballot_style", &[&area.id, &election.id]),
                area.clone(),
                schema.election_event.clone(),
                election.clone(),
                chosen,
                &all_contests,
                for_contests,
                dates,
                // No ceremony has run, so there is no key. The stand-in is
                // flagged `is_demo` by the platform's own builder.
                None,
                Some(options.demo_public_key.clone()),
            ) {
                Ok(style) => styles.push(style),
                Err(error) => {
                    report.push(Problem::error(
                        Code::InvalidValue,
                        "elections",
                        format!(
                            "the platform could not build a ballot for area {} \
                             in election {}: {error}",
                            area.id, election.id
                        ),
                    ));
                    return Err(report);
                }
            }
        }
    }

    if styles.is_empty() {
        report.push(Problem::error(
            Code::MissingField,
            "areas",
            "no area votes on any contest, so no voter would be given a ballot",
        ));
        return Err(report);
    }

    Ok(PublicationPreview {
        ballot_styles: styles,
        election_event: serde_json::to_value(&schema.election_event)
            .unwrap_or(Value::Null),
        elections: serde_json::to_value(&schema.elections)
            .unwrap_or(Value::Array(Vec::new())),
        support_materials: json!([]),
        documents: json!([]),
    })
}

/// The scheduled events, read back out of the rows the importer reads.
///
/// Taken from the built CSV rather than from the plan, on purpose: the voting
/// window that matters is the one in `export_scheduled_events-<id>.csv`, and a
/// preview that read the plan's own schedule could show a window the bundle does
/// not carry. A row that will not deserialize is skipped — the schedule is
/// decoration on a ballot, and the validator already reports a malformed one.
fn scheduled_events(table: &JsonTable) -> Vec<ScheduledEvent> {
    table
        .rows
        .iter()
        .filter_map(|row| {
            let mut object = serde_json::Map::new();
            for (column, field) in SCHEDULED_EVENT_COLUMNS.iter().zip(row) {
                object.insert(
                    (*column).to_string(),
                    match field {
                        JsonField::Null => Value::Null,
                        JsonField::Value(value) => value.clone(),
                    },
                );
            }
            serde_json::from_value(Value::Object(object)).ok()
        })
        .collect()
}

#[cfg(test)]
#[path = "preview_tests.rs"]
mod preview_tests;
