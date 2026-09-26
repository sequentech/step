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

use base64::engine::general_purpose::STANDARD;
use base64::Engine;

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
    /// Uploaded files. Always empty, but not for the same reason: a plan *does*
    /// carry logos and candidate photographs. They reach the preview as `data:`
    /// urls substituted by [`inline_images`], because a document row describes a
    /// file in a bucket and nothing has uploaded one yet.
    pub documents: Value,
}

/// Every object's keys in name order, at every depth.
///
/// Free-standing rather than a method because it is about `Value`, not about a
/// preview, and because the whole point is that it does not trust which map
/// implementation `serde_json` was compiled with.
fn canonical(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut pairs: Vec<(String, Value)> = map.into_iter().collect();
            pairs.sort_by(|(one, _), (two, _)| one.cmp(two));
            Value::Object(
                pairs
                    .into_iter()
                    .map(|(key, nested)| (key, canonical(nested)))
                    .collect(),
            )
        }
        Value::Array(items) => {
            Value::Array(items.into_iter().map(canonical).collect())
        }
        other => other,
    }
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
    /// So this sorts every object's keys at every depth, and **that has to be
    /// done rather than assumed**. Going through `serde_json::Value` was the
    /// original answer, on the grounds that its object is a `BTreeMap` — and it
    /// is, until something in the build turns on `serde_json`'s `preserve_order`,
    /// at which point it is an `IndexMap` that faithfully keeps the insertion
    /// order a `HashMap` gave it. A transitive dependency of `keycloak` does
    /// exactly that, and cargo unifies features across the workspace, so whether
    /// a preview was reproducible depended on whether an unrelated crate was
    /// compiled beside it. It was not reproducible in windmill, which is the one
    /// place that matters.
    pub fn to_document(&self) -> Value {
        canonical(serde_json::to_value(self).unwrap_or(Value::Null))
    }

    /// The areas these ballots belong to, named.
    ///
    /// Not part of the document, and deliberately: a ballot style names its area
    /// by id, that is what windmill writes, and adding a sixth key would mean the
    /// preview was no longer the same file the platform produces.
    ///
    /// It is here because a picker offering four uuids is not a choice anybody
    /// can make. The wizard shows these; the file it hands to a Voting Portal is
    /// untouched.
    pub fn areas(
        &self,
        schema: &ImportElectionEventSchema,
    ) -> Vec<PreviewArea> {
        let named: HashMap<&str, &str> = schema
            .areas
            .iter()
            .map(|area| {
                (area.id.as_str(), area.name.as_deref().unwrap_or_default())
            })
            .collect();

        let mut seen: Vec<PreviewArea> = Vec::new();
        for style in &self.ballot_styles {
            if seen.iter().any(|each| each.id == style.area_id) {
                continue;
            }
            seen.push(PreviewArea {
                id: style.area_id.clone(),
                name: named
                    .get(style.area_id.as_str())
                    .copied()
                    .unwrap_or_default()
                    .to_string(),
            });
        }
        seen
    }

    /// The elections these ballots belong to, named.
    ///
    /// The same reasoning as [`Self::areas`], and needed for the same reason: a
    /// ballot style names its election by the id windmill will write, and a
    /// picker offering uuids is not a choice anybody can make. The wizard's
    /// preview shows one area × one election at a time, because that is what a
    /// voter is handed — showing every election at once is a page no voter ever
    /// sees.
    ///
    /// In the document's own order rather than sorted, so the picker lists them
    /// the way the ballot does.
    pub fn elections(
        &self,
        schema: &ImportElectionEventSchema,
    ) -> Vec<PreviewArea> {
        // An election carries no `name` column — the platform keeps it under
        // `presentation.i18n.<lang>.name`, which is the same shape the authoring
        // workbook writes. Any language will do for a picker: they are the same
        // election, and falling back to `external_id` gives a word the author
        // typed rather than a uuid.
        let named: HashMap<&str, String> = schema
            .elections
            .iter()
            .map(|election| {
                let shown = election
                    .presentation
                    .as_ref()
                    .and_then(|presentation| presentation.get("i18n"))
                    .and_then(|i18n| i18n.as_object())
                    .and_then(|by_language| {
                        by_language.values().find_map(|texts| {
                            texts.get("name").and_then(Value::as_str)
                        })
                    })
                    .map(str::to_string)
                    .or_else(|| election.external_id.clone())
                    .unwrap_or_default();
                (election.id.as_str(), shown)
            })
            .collect();

        let mut seen: Vec<PreviewArea> = Vec::new();
        for style in &self.ballot_styles {
            if seen.iter().any(|each| each.id == style.election_id) {
                continue;
            }
            seen.push(PreviewArea {
                id: style.election_id.clone(),
                name: named
                    .get(style.election_id.as_str())
                    .cloned()
                    .unwrap_or_default(),
            });
        }
        seen
    }
}

/// One area with a ballot, for a picker to label.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewArea {
    pub id: String,
    pub name: String,
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

/// The bundle's own image bytes, keyed by the url the bundle points at them with.
///
/// A built bundle names a candidate's photograph as `tenant-…/document-…/file`,
/// which is *bucket-relative*: the Voting Portal prefixes `PUBLIC_BUCKET_URL` and
/// the file is there because the importer uploaded it. A preview has been through
/// neither step, so the browser resolves that path against whatever is serving the
/// wizard and gets a 404 and a broken picture — a candidate with a photograph
/// previewed as a candidate without one, which is the opposite of what a preview is
/// for.
///
/// The bytes are already in hand: `images` is what the archive's `images/` branch
/// will be built from. So the preview points at those instead, inline, and shows
/// the photograph the plan actually carries.
///
/// Only the preview. Nothing here touches [`Bundle::export`], so the bundle that
/// ships still carries bucket paths and imports exactly as before.
fn inline_images(bundle: &Bundle) -> HashMap<String, String> {
    bundle
        .images
        .iter()
        .map(|image| {
            // The extension, because a `data:` url carries its own type and the
            // bytes do not say what they are. The fallback is bare `image`, which
            // is what the wizard's own candidate photograph field uses: not a
            // registered type, and accepted by every browser through sniffing,
            // which beats guessing `image/png` at a `.jfif`.
            let mime = match image
                .file_name
                .rsplit_once('.')
                .map(|(_, ext)| ext.to_ascii_lowercase())
                .as_deref()
            {
                Some("png") => "image/png",
                Some("jpg" | "jpeg") => "image/jpeg",
                Some("gif") => "image/gif",
                Some("webp") => "image/webp",
                Some("svg") => "image/svg+xml",
                Some("avif") => "image/avif",
                _ => "image",
            };
            (
                image.public_path(&bundle.tenant_id),
                format!("data:{mime};base64,{}", STANDARD.encode(&image.bytes)),
            )
        })
        .collect()
}

pub fn preview_publication(
    bundle: &Bundle,
    options: &PreviewOptions,
) -> Result<PublicationPreview, Report> {
    let mut report = Report::default();

    // Before the schema is read: a built bundle points at a candidate's photograph
    // in the public bucket, which is right for the ballot a voter opens after
    // import and resolves to nothing in a preview. Substituted on the copy that is
    // about to be deserialized, so the platform's own ballot-style builder carries
    // the inline image through verbatim and nothing below this line has to know a
    // preview resolves images differently.
    //
    // `bundle.export` itself is untouched: the shipped document keeps its bucket
    // paths and imports exactly as before.
    let mut document = bundle.export.clone();
    let inline = inline_images(bundle);
    if !inline.is_empty() {
        if let Some(candidates) =
            document.get_mut("candidates").and_then(Value::as_array_mut)
        {
            for candidate in candidates.iter_mut() {
                let urls = candidate
                    .get_mut("presentation")
                    .and_then(|presentation| presentation.get_mut("urls"))
                    .and_then(Value::as_array_mut);
                for entry in urls.into_iter().flatten() {
                    if entry.get("is_image").and_then(Value::as_bool)
                        != Some(true)
                    {
                        continue;
                    }
                    let replacement = entry
                        .get("url")
                        .and_then(Value::as_str)
                        .and_then(|url| inline.get(url))
                        .cloned();
                    if let Some(data) = replacement {
                        entry["url"] = Value::String(data);
                    }
                }
            }
        }
        // The event logo travels as a plain string rather than in a `urls` list,
        // and it is a file in the same bucket, so it breaks the same way.
        let logo = document
            .get("election_event")
            .and_then(|event| event.get("presentation"))
            .and_then(|presentation| presentation.get("logo_url"))
            .and_then(Value::as_str)
            .and_then(|url| inline.get(url))
            .cloned();
        if let Some(data) = logo {
            document["election_event"]["presentation"]["logo_url"] =
                Value::String(data);
        }
    }

    let schema: ImportElectionEventSchema =
        match serde_json::from_value(document) {
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
