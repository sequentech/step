// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Tests for [`super`] at the size of a large event: every widget of every
//! shipped preset, for every choice a viewer can make, evaluated against a
//! payload with 100 Posts, 150 countries, 10 regions, 40 disapproval
//! reasons, a cube of more than two thousand cells and thirty days of
//! hourly buckets. Each board must fit what the renderer accepts (its body
//! limit and its query limits) and be computed within a generous budget.
//!
//! The boards of the largest cases are also written to `fixtures/scale`,
//! where the renderer's tests draw them within the renderer's time limit.
//! `MONITORING_UPDATE_BOARDS=1` rewrites them.
//!
//! Run with `--nocapture` to see what each widget measured:
//! `[scale] preset=.. widget=.. compute_ms=.. board_bytes=..`.

use super::presets_tests::sorted_keys;
use super::*;
use crate::monitoring::compute::{evaluate, QueryResult};
use crate::monitoring::config::{DynamicOptions, Widget, DEFAULT_THEME};
use crate::monitoring::payload::{
    Bucket, Counts, Cube, CubeCell, GroupRow, PostRow, ScopePayload,
    UNKNOWN_KEY,
};
use crate::monitoring::render_request::build_board;
use crate::monitoring::resolve::{resolve_widget, DynamicOptionValues};
use crate::monitoring::sample::sample_payload;
use crate::monitoring::sources::{
    DataSourceId, Measure, QueryTemplate, TimeGrain,
};
use indexmap::IndexMap;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const POSTS: usize = 100;
const COUNTRIES: usize = 150;
const REGIONS: usize = 10;
const REASONS: usize = 40;
const DAYS: usize = 30;
/// Values of a free-text voter dimension: as many as there are countries.
const FREE_TEXT_VALUES: usize = COUNTRIES;
const OFFSET: &str = "+08:00";

/// Voters per hour of a day, relative: busier by day than by night.
const HOURLY_SHAPE: [u64; 24] = [
    0, 0, 0, 0, 0, 1, 2, 4, 6, 8, 9, 9, 8, 8, 9, 9, 8, 7, 6, 5, 4, 2, 1, 0,
];

/// What the renderer accepts; see `monitoring_renderer/app.py`
/// (`MAX_BODY_BYTES`) and `monitoring_renderer/policy.py` (`MAX_ROWS`,
/// `MAX_COLUMNS`, `MAX_QUERIES`, `MAX_CELL_CHARS`).
const RENDERER_MAX_BODY_BYTES: usize = 256 * 1024;
const RENDERER_MAX_ROWS: usize = 5000;
const RENDERER_MAX_COLUMNS: usize = 64;
const RENDERER_MAX_QUERIES: usize = 16;
const RENDERER_MAX_CELL_CHARS: usize = 1024;

/// The widest a chart is drawn, as Harvest's width buckets allow.
const WIDEST: u32 = 2400;

/// Resolving, evaluating and building one board at this size, in a debug
/// build on a slow CI machine. Measured at about 11 ms at most (an hourly
/// series of 720 buckets).
const COMPUTE_BUDGET: Duration = Duration::from_millis(250);

/// A board's request body at this size may take at most this share of the
/// renderer's limit, so a larger event still fits.
const BODY_HEADROOM: usize = 2;

fn monitoring_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/monitoring")
}

fn loaded(id: &str) -> Preset {
    match load(id).unwrap_or_else(|| panic!("no preset {id}")) {
        Ok(preset) => preset,
        Err(report) => panic!("preset {id} refused:\n{report}"),
    }
}

fn all_presets() -> Vec<Preset> {
    PRESETS.iter().map(|source| loaded(source.id)).collect()
}

fn days() -> Vec<String> {
    (0..DAYS)
        .map(|day| format!("2026-05-{:02}", day + 1))
        .collect()
}

fn dynamic() -> DynamicOptionValues {
    DynamicOptionValues { event_days: days() }
}

fn region(at: usize) -> String {
    format!("Region {:02}", at + 1)
}

/// A share of `total` that differs by position, never all of it.
fn share(total: u64, at: usize) -> u64 {
    total * (1 + (at as u64 * 7) % 13) / 40
}

/// The voter funnel over one cell, as the producer counts it: each stage a
/// part of the one before.
fn voter_counts(measures: &BTreeSet<Measure>, at: usize) -> Counts {
    let registered = 40 + (at as u64 * 37) % 400;
    let pre_enrolled = registered * (6 + at as u64 % 4) / 10;
    let voted = pre_enrolled * (3 + at as u64 % 5) / 10;
    measures
        .iter()
        .map(|measure| {
            let count = match measure {
                Measure::Registered => registered,
                Measure::PreEnrolled | Measure::Approved => pre_enrolled,
                Measure::CredentialsIssued => pre_enrolled * 9 / 10,
                Measure::TestVoted => pre_enrolled / 5,
                Measure::Voted => voted,
                Measure::VotedPreEnrolled => voted * 8 / 10,
                _ => share(registered, at),
            };
            (*measure, count)
        })
        .collect()
}

/// The cube's values of each dimension: the sample's configured values,
/// and a free-text dimension given as many values as there are countries.
fn cube_values(sample: &Cube) -> Vec<Vec<String>> {
    (0..sample.dimensions.len())
        .map(|at| {
            let mut values: Vec<String> = Vec::new();
            for cell in &sample.cells {
                if !values.contains(&cell.values[at]) {
                    values.push(cell.values[at].clone());
                }
            }
            let dimension = &sample.dimensions[at];
            let free_text = values
                .iter()
                .any(|value| *value == format!("{dimension}-a"));
            if free_text {
                values = (0..FREE_TEXT_VALUES)
                    .map(|value| format!("{dimension} {:03}", value + 1))
                    .collect();
                values.push(UNKNOWN_KEY.to_string());
            }
            values
        })
        .collect()
}

fn scaled_cube(sample: &Cube) -> Cube {
    let measures: BTreeSet<Measure> = sample
        .cells
        .iter()
        .flat_map(|cell| cell.counts.keys().copied())
        .collect();
    let values = cube_values(sample);
    let mut cells: Vec<Vec<String>> = vec![Vec::new()];
    for of_dimension in &values {
        cells = cells
            .into_iter()
            .flat_map(|cell| {
                of_dimension.iter().map(move |value| {
                    let mut cell = cell.clone();
                    cell.push(value.clone());
                    cell
                })
            })
            .collect();
    }
    Cube {
        dimensions: sample.dimensions.clone(),
        cells: cells
            .into_iter()
            .enumerate()
            .map(|(at, values)| CubeCell {
                values,
                counts: voter_counts(&measures, at),
            })
            .collect(),
    }
}

fn sum<'c>(counts: impl Iterator<Item = &'c Counts>) -> Counts {
    let mut totals = Counts::new();
    for counts in counts {
        for (measure, value) in counts {
            *totals.entry(*measure).or_default() += value;
        }
    }
    totals
}

fn scaled_posts(source: DataSourceId) -> Vec<PostRow> {
    let spec = source.spec();
    (0..POSTS)
        .map(|at| {
            let state = spec.states[at % spec.states.len()];
            PostRow {
                post_id: format!("post-{:03}", at + 1),
                post: format!("Consulate General {:03}", at + 1),
                // A few Posts have no region.
                region: (at % 25 != 24).then(|| region(at % REGIONS)),
                state: Some(state),
                counts: spec
                    .measures
                    .iter()
                    .map(|measure| {
                        (*measure, u64::from(state.has_reached(*measure)))
                    })
                    .collect(),
            }
        })
        .collect()
}

/// A dimension's keys and labels at this size; `None` for one whose values
/// do not grow with the event (a category).
fn scaled_keys(dimension: &str) -> Option<Vec<(String, Option<String>)>> {
    let own = |count: usize, key: &dyn Fn(usize) -> String| {
        let mut keys: Vec<(String, Option<String>)> =
            (0..count).map(|at| (key(at), None)).collect();
        keys.push((UNKNOWN_KEY.to_string(), None));
        keys
    };
    match dimension {
        "region" => Some(own(REGIONS, &region)),
        "country" => {
            Some(own(COUNTRIES, &|at| format!("Country {:03}", at + 1)))
        }
        "reason" => Some(
            (0..REASONS)
                .map(|at| {
                    (
                        format!("reason-{:02}", at + 1),
                        Some(format!("Disapproval reason {:02}", at + 1)),
                    )
                })
                .chain([(UNKNOWN_KEY.to_string(), None)])
                .collect(),
        ),
        "post" => Some(
            (0..POSTS)
                .map(|at| {
                    (
                        format!("post-{:03}", at + 1),
                        Some(format!("Consulate General {:03}", at + 1)),
                    )
                })
                .collect(),
        ),
        _ => None,
    }
}

/// A Post source's groups by region or by Post, summed from its Posts.
fn post_groups(dimension: &str, posts: &[PostRow]) -> Vec<GroupRow> {
    let mut groups: IndexMap<String, GroupRow> = IndexMap::new();
    for post in posts {
        let (key, label) = if dimension == "post" {
            (post.post_id.clone(), Some(post.post.clone()))
        } else {
            let key = post
                .region
                .clone()
                .unwrap_or_else(|| UNKNOWN_KEY.to_string());
            (key, None)
        };
        let row = groups.entry(key.clone()).or_insert_with(|| GroupRow {
            key,
            label,
            counts: Counts::new(),
        });
        for (measure, count) in &post.counts {
            *row.counts.entry(*measure).or_default() += count;
        }
    }
    groups.into_values().collect()
}

/// Thirty days of hours, busier by day than by night.
fn scaled_series(totals: &Counts) -> Vec<Bucket> {
    days()
        .iter()
        .flat_map(|day| (0..24).map(move |hour| (day.clone(), hour)))
        .enumerate()
        .map(|(at, (day, hour))| {
            let weight = HOURLY_SHAPE[hour];
            Bucket {
                start: format!("{day}T{hour:02}:00:00"),
                day,
                utc_offset: OFFSET.to_string(),
                counts: totals
                    .iter()
                    .map(|(measure, total)| {
                        let count = total * weight / (DAYS as u64 * 24 * 5);
                        (*measure, count + (at as u64 % 3))
                    })
                    .collect(),
            }
        })
        .collect()
}

/// The payload a snapshot of a large event writes for `source`: the
/// sample's shape, at the size of the constants above.
fn scaled_payload(
    source: DataSourceId,
    settings: Option<&crate::monitoring::config::Settings>,
) -> ScopePayload {
    let sample = sample_payload(source, settings);
    let stated = !source.spec().states.is_empty();
    let mut payload = ScopePayload {
        notices: sample.notices.clone(),
        ..ScopePayload::default()
    };
    if stated {
        payload.posts = scaled_posts(source);
    }
    payload.cube = sample.cube.as_ref().map(scaled_cube);
    payload.totals = match (&payload.cube, stated) {
        (Some(cube), _) => sum(cube.cells.iter().map(|cell| &cell.counts)),
        (None, true) => sum(payload.posts.iter().map(|post| &post.counts)),
        (None, false) => sample
            .totals
            .iter()
            .map(|(measure, total)| (*measure, total * 1000))
            .collect(),
    };
    if !sample.series.is_empty() {
        payload.series = scaled_series(&payload.totals);
    }
    for (dimension, rows) in &sample.groups {
        let groups = match scaled_keys(dimension) {
            _ if stated && matches!(dimension.as_str(), "region" | "post") => {
                post_groups(dimension, &payload.posts)
            }
            Some(keys) => {
                let measures: Vec<Measure> = rows
                    .first()
                    .map(|row| row.counts.keys().copied().collect())
                    .unwrap_or_default();
                keys.into_iter()
                    .enumerate()
                    .map(|(at, (key, label))| GroupRow {
                        key,
                        label,
                        counts: measures
                            .iter()
                            .map(|measure| {
                                let total = payload.totals[measure];
                                (*measure, share(total, at) / 4)
                            })
                            .collect(),
                    })
                    .collect()
            }
            None => rows.clone(),
        };
        payload.groups.insert(dimension.clone(), groups);
    }
    payload
}

/// Every set of selector values a viewer can pick, hidden selectors left
/// out.
fn combinations(widget: &Widget) -> Vec<IndexMap<String, String>> {
    let mut combinations = vec![IndexMap::new()];
    for (name, selector) in &widget.selectors {
        let options: Vec<String> = match selector.options_from {
            Some(DynamicOptions::EventDays) => days(),
            None => selector.options.keys().cloned().collect(),
        };
        combinations = combinations
            .into_iter()
            .flat_map(|chosen: IndexMap<String, String>| {
                let shown = selector.when.as_ref().map_or(true, |when| {
                    chosen
                        .get(&when.selector)
                        .is_some_and(|value| when.one_of.contains(value))
                });
                if !shown {
                    return vec![chosen];
                }
                options
                    .iter()
                    .map(|option| {
                        let mut chosen = chosen.clone();
                        chosen.insert(name.clone(), option.clone());
                        chosen
                    })
                    .collect()
            })
            .collect();
    }
    combinations
}

/// Which hours a timeseries shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Hours {
    /// As the widget's selectors say: one day's hours at most.
    AsSelected,
    /// Every hour of the event, as a widget with no day selector shows.
    EveryHour,
}

struct Drawn {
    board: Value,
    rows: usize,
    elapsed: Duration,
}

fn draw(
    preset: &Preset,
    widget: &Widget,
    requested: &IndexMap<String, String>,
    hours: Hours,
) -> Result<Drawn, String> {
    let settings = preset.set.settings.as_ref();
    let payload = scaled_payload(widget.source, settings);
    let started = Instant::now();
    let mut resolved =
        resolve_widget(widget, &IndexMap::new(), requested, &dynamic())
            .map_err(|report| format!("resolve: {report}"))?;
    if hours == Hours::EveryHour {
        for query in resolved.queries.values_mut() {
            if query.grain.is_some() {
                query.grain = Some(TimeGrain::Hour);
                query.day = None;
            }
        }
    }
    let mut data: IndexMap<String, QueryResult> = IndexMap::new();
    for (name, query) in &resolved.queries {
        let result = evaluate(widget.source, query, &payload, settings)
            .map_err(|report| format!("query {name}: {report}"))?;
        data.insert(name.clone(), result);
    }
    let board =
        build_board(widget, preset.set.themes.get(DEFAULT_THEME), &data);
    let elapsed = started.elapsed();
    let rows = data.values().map(|result| result.rows.len()).max();
    Ok(Drawn {
        board,
        rows: rows.unwrap_or(0),
        elapsed,
    })
}

/// The body Harvest posts for `board`, as its adapter writes it.
fn request_body(board: &Value, height: Option<u32>) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "board": board,
        "width": WIDEST,
        "height": height,
        "color_scheme": "light",
        "locale": "en-US",
    }))
    .unwrap()
}

/// What the renderer would refuse in `board`, as `path: why`.
fn renderer_refusals(board: &Value, body: usize) -> Vec<String> {
    let mut refused = Vec::new();
    if body * BODY_HEADROOM > RENDERER_MAX_BODY_BYTES {
        refused.push(format!(
            "body: {body} bytes, over 1/{BODY_HEADROOM} of {RENDERER_MAX_BODY_BYTES}"
        ));
    }
    let queries = board["queries"].as_object().cloned().unwrap_or_default();
    if queries.len() > RENDERER_MAX_QUERIES {
        refused.push(format!("queries: {}", queries.len()));
    }
    for (name, query) in &queries {
        let columns = query["columns"].as_array().map_or(0, Vec::len);
        if columns > RENDERER_MAX_COLUMNS {
            refused.push(format!("{name}: {columns} columns"));
        }
        let values = query["values"].as_array().cloned().unwrap_or_default();
        if values.len() > RENDERER_MAX_ROWS {
            refused.push(format!("{name}: {} rows", values.len()));
        }
        let long = values
            .iter()
            .flat_map(|row| row.as_array().cloned().unwrap_or_default())
            .filter_map(|cell| cell.as_str().map(str::len))
            .any(|chars| chars > RENDERER_MAX_CELL_CHARS);
        if long {
            refused.push(format!(
                "{name}: a cell over {RENDERER_MAX_CELL_CHARS} characters"
            ));
        }
    }
    refused
}

#[test]
fn the_scaled_payload_has_the_size_of_a_large_event() {
    let preset = loaded("comelec");
    let settings = preset.set.settings.as_ref();
    let turnout = scaled_payload(DataSourceId::VoterTurnout, settings);
    assert_eq!(turnout.groups["post"].len(), POSTS);
    assert_eq!(turnout.groups["country"].len(), COUNTRIES + 1);
    assert_eq!(turnout.groups["region"].len(), REGIONS + 1);
    let cube = turnout.cube.as_ref().unwrap();
    // Sex (two and Unknown) × age band (four and Unknown) × a free-text
    // dimension (150 and Unknown).
    assert_eq!(cube.cells.len(), 3 * 5 * (FREE_TEXT_VALUES + 1));
    assert!(cube.is_well_formed());
    let activity =
        scaled_payload(DataSourceId::VotingEnrollmentActivity, settings);
    assert_eq!(activity.series.len(), DAYS * 24);
    let decisions = scaled_payload(DataSourceId::EnrollmentDecisions, settings);
    assert_eq!(decisions.groups["reason"].len(), REASONS + 1);
    let polls = scaled_payload(DataSourceId::PollStatus, settings);
    assert_eq!(polls.posts.len(), POSTS);
}

#[test]
fn every_widget_draws_within_budget_and_the_renderer_s_limits_at_scale() {
    let mut failures = Vec::new();
    let mut drawn = 0;
    for preset in all_presets() {
        let id = &preset.manifest.id;
        for (key, widget) in &preset.set.widgets {
            let mut cases: Vec<(String, IndexMap<String, String>, Hours)> =
                combinations(widget)
                    .into_iter()
                    .map(|requested| {
                        (format!("{requested:?}"), requested, Hours::AsSelected)
                    })
                    .collect();
            let timeseries = widget
                .query
                .iter()
                .chain(widget.queries.values())
                .any(|query| query.template == QueryTemplate::Timeseries);
            if timeseries {
                cases.push((
                    "every hour".to_string(),
                    IndexMap::new(),
                    Hours::EveryHour,
                ));
            }
            let (mut slowest, mut largest, mut most_rows) =
                (Duration::ZERO, 0, 0);
            for (case, requested, hours) in cases {
                let drawn_case = match draw(&preset, widget, &requested, hours)
                {
                    Ok(drawn_case) => drawn_case,
                    Err(why) => {
                        failures.push(format!("{id}/{key} {case}: {why}"));
                        continue;
                    }
                };
                drawn += 1;
                let body = request_body(&drawn_case.board, widget.height).len();
                for why in renderer_refusals(&drawn_case.board, body) {
                    failures.push(format!("{id}/{key} {case}: {why}"));
                }
                if drawn_case.elapsed > COMPUTE_BUDGET {
                    failures.push(format!(
                        "{id}/{key} {case}: {:?}, over {COMPUTE_BUDGET:?}",
                        drawn_case.elapsed
                    ));
                }
                slowest = slowest.max(drawn_case.elapsed);
                largest = largest.max(body);
                most_rows = most_rows.max(drawn_case.rows);
            }
            println!(
                "[scale] preset={id} widget={key} compute_ms={:.2} board_bytes={largest} rows={most_rows}",
                slowest.as_secs_f64() * 1000.0
            );
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert!(drawn > 100, "only {drawn} boards drawn");
}

/// The largest boards of each shape, for the renderer's tests.
fn scale_cases() -> Vec<(&'static str, &'static str, &'static str, Hours)> {
    vec![
        // 100 Posts as a table.
        (
            "comelec",
            "turnout-by-post",
            "turnout-by-post",
            Hours::AsSelected,
        ),
        (
            "comelec",
            "status-by-post",
            "status-by-post",
            Hours::AsSelected,
        ),
        // 150 countries and Unknown as a table.
        (
            "comelec",
            "turnout-by-country",
            "turnout-by-country",
            Hours::AsSelected,
        ),
        // 150 values and Unknown as horizontal bars.
        (
            "comelec",
            "turnout-by-group",
            "turnout-by-group.status",
            Hours::AsSelected,
        ),
        // Every reason as horizontal bars.
        (
            "comelec",
            "disapproval-reasons",
            "disapproval-reasons.all",
            Hours::AsSelected,
        ),
        // 720 hourly buckets as bars and a line.
        (
            "comelec",
            "voting-activity",
            "voting-activity.every-hour",
            Hours::EveryHour,
        ),
        // 720 hourly buckets of three measures as lines.
        (
            "comelec",
            "login-outcomes",
            "login-outcomes.every-hour",
            Hours::EveryHour,
        ),
    ]
}

fn requested_for(file: &str) -> IndexMap<String, String> {
    match file {
        "turnout-by-group.status" => {
            IndexMap::from([("breakdown".to_string(), "status".to_string())])
        }
        "disapproval-reasons.all" => {
            IndexMap::from([("top".to_string(), "all".to_string())])
        }
        _ => IndexMap::new(),
    }
}

#[test]
fn the_scale_boards_match_the_files_the_renderer_is_tested_with() {
    let root = monitoring_dir().join("fixtures/scale");
    let update = std::env::var_os("MONITORING_UPDATE_BOARDS").is_some();
    let mut stale = Vec::new();
    for (preset_id, key, file, hours) in scale_cases() {
        let preset = loaded(preset_id);
        let widget = &preset.set.widgets[key];
        let drawn = draw(&preset, widget, &requested_for(file), hours)
            .unwrap_or_else(|why| panic!("{file}: {why}"));
        let mut text =
            serde_json::to_string(&sorted_keys(&drawn.board)).unwrap();
        text.push('\n');
        let path = root.join(format!("{preset_id}.{file}.json"));
        if update {
            std::fs::create_dir_all(&root).unwrap();
            std::fs::write(&path, &text).unwrap();
        }
        if std::fs::read_to_string(&path).ok().as_deref() != Some(&text) {
            stale.push(path.display().to_string());
        }
    }
    assert!(
        stale.is_empty(),
        "stale; run with MONITORING_UPDATE_BOARDS=1: {stale:?}"
    );
}
