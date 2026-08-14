// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Reading an election event exported from the Admin Portal back as a plan.
//!
//! **The inverse of what `build` writes, and deliberately written against the
//! emitter rather than against a description of the format.** A plan becomes an
//! importable document by going `Blueprint` → `to_workbook` → `build`, through
//! handlebars templates, across forty-odd fields. Anything here that guessed at
//! the shape would be a second opinion about what those templates emit, and the
//! way that fails is an election that imports cleanly and then behaves in a way
//! nobody chose. So the test that matters is a round trip — compile a plan, read
//! it back, compare — and it lives in `architect_tests` beside the emitter.
//!
//! **What an export cannot carry, and why that is not a defect.** Trustees,
//! contacts, the key-ceremony policy, voter messages, the census and the logo's
//! bytes are the *architect's* own material: they are not part of an election
//! event as the platform stores one, and they travel in a delivery's auxiliary
//! files. An export read here comes back as a plan with those parts empty, and
//! `validate_plan` then says so in the wizard's own words. That is the honest
//! outcome — the alternative is inventing a returning officer.
//!
//! An election event *archive* carries more than the JSON does — a voter CSV, the
//! images, the support-material files — and `open` reads those and hands them in.
//! The JSON alone is one file and yields one plan.

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use super::architect::{
    Blueprint, PlannedArea, PlannedCandidate, PlannedContest, PlannedElection,
    Translated, BLUEPRINT_VERSION,
};
use super::plan_from_workbook::ReadPlan;
use super::policy::{Behaviour, Overrides};
use super::problem::{Code, Problem, Report};
use crate::types::ceremonies::CeremoniesPolicy;

/// Where a value was, for a problem that has to name it.
const AT: &str = "plan";

/// Read an exported election event as a plan.
///
/// Errors come back as `Err(Report)` and warnings ride in [`ReadPlan::report`] —
/// the same split [`super::plan_from_workbook::plan_from_workbook`] and
/// [`super::build::build`] make, so every door into the wizard treats a bad
/// document the same way.
pub fn plan_from_event(document: &Value) -> Result<ReadPlan, Report> {
    let mut report = Report::default();

    let Some(event) = document.get("election_event").and_then(Value::as_object)
    else {
        let mut refused = Report::default();
        refused.push(Problem::error(
            Code::InvalidValue,
            AT,
            "this JSON has no `election_event`, so it is not an election event \
             export. If you meant to open a plan, open the .json the wizard \
             downloaded; if you meant an export, take the one whose name starts \
             `export_election_event`.",
        ));
        return Err(refused);
    };

    let presentation = object(event, "presentation");

    // The event's own presentation *is* the plan's defaults: `build` renders the
    // event row from `plan.defaults`, so reading them straight back is the
    // inverse rather than an interpretation.
    let defaults = behaviour_from(&presentation);

    let language = languages(&presentation);

    let mut plan = Blueprint {
        version: BLUEPRINT_VERSION,
        external_id: text(event, "external_id"),
        name: translated(&presentation, "name"),
        description: translated(&presentation, "description"),
        languages: language.offered.clone(),
        default_language: language.default.clone(),
        elections_order: string_or(&presentation, "elections_order", "custom"),
        show_cast_vote_logs: string_or(
            &presentation,
            "show_cast_vote_logs",
            "show-logs-tab",
        ),
        // `Option`, because a plan distinguishes "the client chose no" from "the
        // client never said" — and a profile can hide the control entirely. An
        // export always states them, so they always come back as `Some`.
        show_user_profile: presentation
            .get("show_user_profile")
            .and_then(Value::as_bool),
        skip_election_list: presentation
            .get("skip_election_list")
            .and_then(Value::as_bool),
        language_detection_policy: presentation
            .get("language_conf")
            .and_then(Value::as_object)
            .map(|conf| string_or(conf, "language_detection_policy", ""))
            .filter(|value| !value.is_empty()),
        logo_url: presentation
            .get("logo_url")
            .and_then(Value::as_str)
            .filter(|url| !url.is_empty())
            .map(str::to_string),
        materials_activated: presentation
            .get("materials")
            .and_then(Value::as_object)
            .and_then(|materials| materials.get("activated"))
            .and_then(Value::as_bool),
        // Parsed through the enum rather than kept as a string, so a value the
        // platform stopped offering is the event's own default here instead of a
        // word that reaches a ballot.
        ceremony_policy: presentation
            .get("ceremonies_policy")
            .and_then(Value::as_str)
            .and_then(|value| {
                serde_json::from_value(Value::String(value.to_string())).ok()
            })
            .unwrap_or(CeremoniesPolicy::default()),
        voting_channels: serde_json::from_value(
            event.get("voting_channels").cloned().unwrap_or(Value::Null),
        )
        .unwrap_or_default(),
        defaults,
        ..Blueprint::default()
    };

    if plan.external_id.is_empty() {
        report.push(Problem::warning(
            Code::MissingField,
            AT,
            "the export names no identifier for the event, so this plan has none \
             until you give it one.",
        ));
    }
    if plan.name.is_empty() {
        report.push(Problem::warning(
            Code::MissingField,
            "name",
            "the export carries no name for the event in any language.",
        ));
    }

    plan.areas = areas(document, &mut report);
    plan.elections =
        elections(document, &plan.defaults, &language, &mut report);

    // Said once, plainly, rather than left for somebody to discover on the
    // Contacts screen. An export is an election event; these are the architect's
    // own material and no export has ever carried them.
    report.push(Problem::warning(
        Code::MissingField,
        AT,
        "an election event export carries no trustees, contacts, voter messages \
         or ceremony dates — those are the wizard's own and travel in a delivery \
         rather than in an export. The screens for them are empty and the checks \
         below will say so.",
    ));

    Ok(ReadPlan { plan, report })
}

/// Read a flat bag of platform keys as the three groups a `Behaviour` has.
///
/// **The nesting is the whole point of this function.** The export writes every
/// rule flat, in one `presentation` object; `Behaviour` groups them as
/// `{policies, tally, layout}` with no `#[serde(flatten)]`. Handing the flat
/// object straight to `serde_json::from_value::<Behaviour>` therefore succeeds and
/// returns *every field at its struct default* — no error, no missing key, just
/// silently the wrong answer. That version passed a round trip over a plan whose
/// contests overrode nothing, because both sides were defaults, and failed the
/// moment a contest disagreed with its event. Hence
/// `an_exported_event_reads_back_as_the_plan_that_made_it` runs over a plan with
/// overrides in it.
///
/// The same bag is handed to all three groups on purpose: each picks out the keys
/// it knows and ignores the rest, so a rule added to the platform arrives here
/// without a second list of rule names to keep in step.
fn behaviour_from(flat: &Map<String, Value>) -> Behaviour {
    serde_json::from_value(Value::Object(
        [
            ("policies", flat.clone()),
            ("tally", flat.clone()),
            ("layout", flat.clone()),
        ]
        .into_iter()
        .map(|(group, keys)| (group.to_string(), Value::Object(keys)))
        .collect(),
    ))
    .unwrap_or_default()
}

/// The languages an event offers, and which one it falls back to.
pub(super) struct Languages {
    pub offered: Vec<String>,
    pub default: Option<String>,
}

fn languages(presentation: &Map<String, Value>) -> Languages {
    let conf = presentation
        .get("language_conf")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let offered: Vec<String> = conf
        .get("enabled_language_codes")
        .and_then(Value::as_array)
        .map(|codes| {
            codes
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    Languages {
        // Never empty: a plan with no languages renders an English-only ballot
        // and says so, which is better than a ballot with no language at all.
        offered: if offered.is_empty() {
            vec!["en".to_string()]
        } else {
            offered
        },
        default: conf
            .get("default_language_code")
            .and_then(Value::as_str)
            .filter(|code| !code.is_empty())
            .map(str::to_string),
    }
}

fn areas(document: &Value, report: &mut Report) -> Vec<PlannedArea> {
    let rows = array(document, "areas");
    let mut named: BTreeMap<String, String> = BTreeMap::new();

    let mut areas: Vec<PlannedArea> = Vec::new();
    for row in &rows {
        let Some(row) = row.as_object() else { continue };
        let id = text(row, "id");
        let external = text(row, "external_id");
        named.insert(id, external.clone());
        areas.push(PlannedArea {
            external_id: external,
            name: text(row, "name"),
            ..PlannedArea::default()
        });
    }

    // The parent, resolved from the identifier the export uses to the external id
    // a plan uses — a second pass, because a child can appear before its parent.
    for (area, row) in areas.iter_mut().zip(rows.iter()) {
        let Some(row) = row.as_object() else { continue };
        let parent = row.get("parent_id").and_then(Value::as_str);
        if let Some(parent) = parent {
            match named.get(parent) {
                Some(external) => {
                    area.parent_external_id = Some(external.clone())
                }
                None => report.push(Problem::warning(
                    Code::InvalidValue,
                    "areas",
                    format!(
                        "the area `{}` names a parent the export does not \
                         contain, so it comes back with no parent.",
                        area.external_id
                    ),
                )),
            }
        }
    }

    areas
}

fn elections(
    document: &Value,
    defaults: &Behaviour,
    language: &Languages,
    report: &mut Report,
) -> Vec<PlannedElection> {
    let areas = array(document, "areas");
    let contests = array(document, "contests");
    let candidates = array(document, "candidates");
    let links = array(document, "area_contests");

    // `id` → `external_id`, so a contest can say which areas it serves in the
    // words a plan uses rather than in the platform's identifiers.
    let area_names: BTreeMap<String, String> = areas
        .iter()
        .filter_map(Value::as_object)
        .map(|row| (text(row, "id"), text(row, "external_id")))
        .collect();

    let mut serves: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for link in links.iter().filter_map(Value::as_object) {
        let contest = text(link, "contest_id");
        let area = text(link, "area_id");
        if let Some(external) = area_names.get(&area) {
            serves.entry(contest).or_default().push(external.clone());
        }
    }

    array(document, "elections")
        .iter()
        .filter_map(Value::as_object)
        .map(|row| {
            let id = text(row, "id");
            let presentation = object(row, "presentation");

            PlannedElection {
                external_id: text(row, "external_id"),
                name: translated(&presentation, "name"),
                description: translated(&presentation, "description"),
                num_allowed_revotes: row
                    .get("num_allowed_revotes")
                    .and_then(Value::as_i64)
                    .unwrap_or(1),
                spoil_ballot_option: flag(row, "spoil_ballot_option"),
                contests_order: string_or(
                    &presentation,
                    "contests_order",
                    "custom",
                ),
                permission_label: text(row, "permission_label"),
                contests: contests_of(
                    &id,
                    &contests,
                    &candidates,
                    &serves,
                    defaults,
                    language,
                    report,
                ),
                ..PlannedElection::default()
            }
        })
        .collect()
}

fn contests_of(
    election: &str,
    contests: &[Value],
    candidates: &[Value],
    serves: &BTreeMap<String, Vec<String>>,
    defaults: &Behaviour,
    language: &Languages,
    report: &mut Report,
) -> Vec<PlannedContest> {
    contests
        .iter()
        .filter_map(Value::as_object)
        .filter(|row| text(row, "election_id") == election)
        .map(|row| {
            let id = text(row, "id");
            let presentation = object(row, "presentation");

            PlannedContest {
                external_id: text(row, "external_id"),
                name: translated(&presentation, "name"),
                description: description_of(row, &presentation),
                max_votes: row
                    .get("max_votes")
                    .and_then(Value::as_i64)
                    .unwrap_or(1),
                winners: row
                    .get("winning_candidates_num")
                    .and_then(Value::as_i64)
                    .unwrap_or(1),
                areas: serves.get(&id).cloned().unwrap_or_default(),
                overrides: overrides_of(row, &presentation, defaults),
                candidates: candidates_of(&id, candidates, language),
                ..PlannedContest::default()
            }
        })
        .collect()
}

/// What a contest says about its own behaviour, over the event's defaults.
///
/// **Computed as a difference rather than read field by field**, and that is the
/// whole reason this is trustworthy. A contest's row carries every rule it ends up
/// with, whether it chose it or inherited it; a plan wants only what it chose. So
/// both are deserialized into a whole `Behaviour`, serialized back to JSON, and
/// compared leaf by leaf — the keys that differ *are* the overrides.
///
/// The alternative is a hand-written list of every policy, which is a second list
/// of the policies, and the way a second list fails is a policy added to the
/// platform and forgotten here — silently inherited instead of overridden.
fn overrides_of(
    row: &Map<String, Value>,
    presentation: &Map<String, Value>,
    defaults: &Behaviour,
) -> Overrides {
    // Everything a contest's behaviour is spelled out of: its presentation, its
    // tally configuration, and the three columns that sit on the row itself.
    let mut whole = presentation.clone();
    if let Some(tally) =
        row.get("tally_configuration").and_then(Value::as_object)
    {
        for (key, value) in tally {
            whole.insert(key.clone(), value.clone());
        }
    }
    for key in [
        "counting_algorithm",
        "voting_type",
        "min_votes",
        "is_encrypted",
    ] {
        if let Some(value) = row.get(key) {
            whole.insert(key.to_string(), value.clone());
        }
    }

    let mine = behaviour_from(&whole);

    let base = serde_json::to_value(defaults).unwrap_or(Value::Null);
    let want = serde_json::to_value(&mine).unwrap_or(Value::Null);

    serde_json::from_value(difference(&base, &want)).unwrap_or_default()
}

/// The leaves of `want` that `base` does not already say, with the same nesting.
fn difference(base: &Value, want: &Value) -> Value {
    match (base, want) {
        (Value::Object(base), Value::Object(want)) => {
            let mut out = Map::new();
            for (key, value) in want {
                let was = base.get(key).unwrap_or(&Value::Null);
                if was == value {
                    continue;
                }
                let deeper = difference(was, value);
                // An object whose every leaf matched contributes nothing; a
                // changed leaf contributes itself.
                match &deeper {
                    Value::Object(inner) if inner.is_empty() => {}
                    _ => {
                        out.insert(key.clone(), deeper);
                    }
                }
            }
            Value::Object(out)
        }
        _ => want.clone(),
    }
}

/// A contest's description, which the format keeps in two places.
///
/// `presentation.i18n.<lang>.description` is what the Admin Portal edits per
/// language, and the flat `description` column is a mirror of the English one.
/// Preferring the translated form and falling back to the column is what
/// `PlannedContest::description`'s own `translated_or_plain` deserializer already
/// does for a plan; this is the same rule on the way in.
fn description_of(
    row: &Map<String, Value>,
    presentation: &Map<String, Value>,
) -> Translated {
    let translated = translated(presentation, "description");
    if !translated.is_empty() {
        return translated;
    }
    let flat = text(row, "description");
    if flat.is_empty() {
        Translated::default()
    } else {
        Translated::new(&flat)
    }
}

fn candidates_of(
    contest: &str,
    candidates: &[Value],
    language: &Languages,
) -> Vec<PlannedCandidate> {
    let _ = language;
    candidates
        .iter()
        .filter_map(Value::as_object)
        .filter(|row| text(row, "contest_id") == contest)
        .map(|row| {
            let presentation = object(row, "presentation");
            PlannedCandidate {
                external_id: text(row, "external_id"),
                name: translated(&presentation, "name"),
                description: description_of(row, &presentation),
                explicit_blank: flag(&presentation, "is_explicit_blank"),
                explicit_invalid: flag(&presentation, "is_explicit_invalid"),
                disabled: flag(&presentation, "is_disabled"),
                // The photograph is deliberately absent here. `image_document_id`
                // on the row names a file that travels *beside* the JSON, as an
                // `export_S3_files/` member of the archive — so only the archive
                // door can fill it in, and it does. A `CandidateImage` invented
                // from an identifier with no bytes behind it would be a ballot
                // with a broken picture on it.
                ..PlannedCandidate::default()
            }
        })
        .collect()
}

// --------------------------------------------------------------- small readers

fn object(row: &Map<String, Value>, key: &str) -> Map<String, Value> {
    row.get(key)
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
}

fn array(document: &Value, key: &str) -> Vec<Value> {
    document
        .get(key)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn text(row: &Map<String, Value>, key: &str) -> String {
    row.get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn string_or(row: &Map<String, Value>, key: &str, fallback: &str) -> String {
    row.get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

fn flag(row: &Map<String, Value>, key: &str) -> bool {
    row.get(key).and_then(Value::as_bool).unwrap_or_default()
}

/// One field of `presentation.i18n`, as every language that has it.
///
/// The format nests language *outside* field — `i18n.en.name`, `i18n.es.name` —
/// where a plan nests field outside language, so this transposes rather than
/// copies. A language whose text is blank is left out: `Translated::get` falls
/// back to English, and an empty string would defeat that and put a blank line on
/// a ballot.
fn translated(presentation: &Map<String, Value>, field: &str) -> Translated {
    let mut by_language = BTreeMap::new();
    if let Some(i18n) = presentation.get("i18n").and_then(Value::as_object) {
        for (code, texts) in i18n {
            let Some(text) = texts.get(field).and_then(Value::as_str) else {
                continue;
            };
            if text.is_empty() {
                continue;
            }
            by_language.insert(code.clone(), text.to_string());
        }
    }
    Translated { by_language }
}

#[cfg(test)]
#[path = "plan_from_event_tests.rs"]
mod plan_from_event_tests;

/// Everything an election event *archive* carries beside its JSON.
///
/// The bare document is one file and describes one election. The archive around it
/// also holds the census, the candidates' photographs and the support-material
/// files — so a plan read from the archive can be complete where a plan read from
/// the JSON alone cannot, and this is what makes the difference between the two
/// doors worth telling apart.
#[derive(Debug, Default)]
pub struct Beside {
    /// `export_voters-<id>.csv`, verbatim.
    pub voters: Option<String>,
    /// Every `images/…` and `export_S3_files/…` entry, by its entry name.
    pub files: Vec<(String, Vec<u8>)>,
}

/// Fill in what travelled beside the document.
///
/// Separate from [`plan_from_event`] on purpose: reading a zip needs a zip reader,
/// and the mapping itself needs neither that nor a spreadsheet. Keeping them apart
/// is what lets `plan_from_event` compile under `election_config_templates` alone.
pub fn fill_from_archive(
    plan: &mut Blueprint,
    document: &Value,
    beside: &Beside,
    report: &mut Report,
) {
    if let Some(csv) = beside.voters.as_deref() {
        voters_into(plan, csv, report);
    }
    images_into(plan, document, beside, report);
    materials_into(plan, document, beside, report);
}

/// The census, out of the export's own CSV.
fn voters_into(plan: &mut Blueprint, csv: &str, report: &mut Report) {
    // Read through the same reader the census screen uses for a dropped file, so
    // an import and a paste disagree about nothing — quoting, blank lines, a
    // trailing newline, a BOM.
    let mut reader = match super::census_csv::CensusCsv::new(csv) {
        Ok(reader) => reader,
        Err(error) => {
            report.push(Problem::warning(
                Code::InvalidValue,
                "voters",
                format!(
                    "the voter list inside this export could not be read, so the \
                     census is empty: {error}"
                ),
            ));
            return;
        }
    };

    let columns: Vec<String> = reader.header().columns.clone();
    let at = |name: &str| columns.iter().position(|column| column == name);
    let (username, email) = (at("username"), at("email"));
    if username.is_none() {
        report.push(Problem::warning(
            Code::MissingField,
            "voters",
            "the voter list inside this export has no `username` column, so \
             there is no way to tell one voter from another and the census is \
             empty.",
        ));
        return;
    }

    // Columns the wizard has its own field for. Everything else rides in `extra`,
    // which is how a client keeps a reporting breakout column without a code
    // change — the same rule the workbook door follows.
    let known = [
        "username",
        "email",
        "first_name",
        "last_name",
        "area_name",
        "id",
        "email_verified",
        "enabled",
        "authorized-election-ids",
    ];

    // The export's CSV names a voter's area by its *name*, because that is the
    // column the platform's own importer reads. A plan keys it by `external_id`
    // (version 3), so this resolves one to the other through the areas the
    // document just gave us. A name nothing matches is kept rather than dropped —
    // `check_census` reports it against the row, which is more use than a voter
    // who silently belongs nowhere.
    let by_name: BTreeMap<String, String> = plan
        .areas
        .iter()
        .map(|area| (area.name.clone(), area.external_id.clone()))
        .collect();

    let pick = |row: &[String], index: Option<usize>| -> String {
        index
            .and_then(|at| row.get(at))
            .cloned()
            .unwrap_or_default()
    };

    let mut people: Vec<super::architect::PlannedVoter> = Vec::new();
    loop {
        let batch = match reader.next_batch(BATCH) {
            Ok(batch) if batch.is_empty() => break,
            Ok(batch) => batch,
            Err(error) => {
                report.push(Problem::warning(
                    Code::InvalidValue,
                    "voters",
                    format!(
                        "the voter list stopped being readable after {} rows: \
                         {error}",
                        people.len()
                    ),
                ));
                break;
            }
        };

        for row in batch {
            let mut extra = BTreeMap::new();
            for (index, column) in columns.iter().enumerate() {
                if known.contains(&column.as_str()) {
                    continue;
                }
                let Some(value) = row.get(index) else {
                    continue;
                };
                if value.is_empty() {
                    continue;
                }
                extra.insert(column.clone(), value.clone());
            }

            let named = pick(&row, at("area_name"));
            people.push(super::architect::PlannedVoter {
                username: pick(&row, username),
                email: pick(&row, email),
                first_name: pick(&row, at("first_name")),
                last_name: pick(&row, at("last_name")),
                area_external_id: by_name.get(&named).cloned().unwrap_or(named),
                extra,
            });
        }
    }

    plan.voters = people;
}

/// How many census rows to read at a time. The reader's own unit of work.
const BATCH: usize = 5_000;

/// A candidate's photograph, matched to the entry that carries its bytes.
///
/// **Matched by the identifier the row names, unanchored.** The platform's own
/// exporter prefixes an image entry with twelve characters of tempfile name —
/// `enGgihs9azd5document_…` — so an entry name is only guaranteed to *contain*
/// `document_<id>_`, never to start with it. That is the same rule the importer's
/// `extract_document_uuid` follows, and anchoring here would silently drop every
/// photograph from a real export while passing against one this crate wrote.
fn images_into(
    plan: &mut Blueprint,
    document: &Value,
    beside: &Beside,
    report: &mut Report,
) {
    let wanted: BTreeMap<String, String> = array(document, "candidates")
        .iter()
        .filter_map(Value::as_object)
        .filter_map(|row| {
            let id = row.get("image_document_id").and_then(Value::as_str)?;
            Some((text(row, "external_id"), id.to_string()))
        })
        .collect();
    if wanted.is_empty() {
        return;
    }

    let mut missing = Vec::new();
    for election in &mut plan.elections {
        for contest in &mut election.contests {
            for candidate in &mut contest.candidates {
                let Some(id) = wanted.get(&candidate.external_id) else {
                    continue;
                };
                match found(&beside.files, "images/", id) {
                    Some((name, bytes)) => {
                        candidate.image =
                            Some(super::architect::CandidateImage {
                                file_name: name,
                                bytes,
                            })
                    }
                    None => missing.push(candidate.external_id.clone()),
                }
            }
        }
    }

    if !missing.is_empty() {
        report.push(Problem::warning(
            Code::MissingField,
            "elections",
            format!(
                "this export names a photograph for {} but the archive does not \
                 contain it, so those candidates have none: {}",
                if missing.len() == 1 {
                    "a candidate".to_string()
                } else {
                    format!("{} candidates", missing.len())
                },
                missing.join(", ")
            ),
        ));
    }
}

/// The voter-facing help documents, and the files behind them.
fn materials_into(
    plan: &mut Blueprint,
    document: &Value,
    beside: &Beside,
    report: &mut Report,
) {
    let mut missing = Vec::new();
    for row in array(document, "support_materials")
        .iter()
        .filter_map(Value::as_object)
    {
        let id = text(row, "document_id");
        let presentation = object(row, "presentation");
        let file = found(&beside.files, "export_S3_files/", &id);

        if file.is_none() {
            missing.push(text(row, "external_id"));
        }

        let (file_name, bytes) = file.unwrap_or_default();
        plan.materials.push(super::architect::PlannedMaterial {
            external_id: text(row, "external_id"),
            title: translated(&presentation, "title"),
            kind: text(row, "kind"),
            file_name,
            bytes,
            is_hidden: flag(row, "is_hidden"),
        });
    }

    if !missing.is_empty() {
        report.push(Problem::warning(
            Code::MissingField,
            "materials",
            format!(
                "this export lists support material the archive does not \
                 contain, so those rows have no file behind them: {}",
                missing.join(", ")
            ),
        ));
    }
}

/// The entry under `folder` whose name carries `document_<id>_`, and its bytes.
///
/// Returns the file's own name — everything after `document_<id>_` — rather than
/// the entry name, so a plan rebuilt from this produces the same entry rather than
/// one with a tempfile prefix baked into it.
fn found(
    files: &[(String, Vec<u8>)],
    folder: &str,
    id: &str,
) -> Option<(String, Vec<u8>)> {
    let marker = format!("document_{id}_");
    files.iter().find_map(|(name, bytes)| {
        if !name.starts_with(folder) {
            return None;
        }
        let at = name.find(&marker)?;
        Some((name[at + marker.len()..].to_string(), bytes.clone()))
    })
}
