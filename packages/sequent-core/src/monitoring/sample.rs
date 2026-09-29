// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Made-up payloads in the shape each source's producer writes.
//!
//! For the places that need figures before any snapshot exists: the preset
//! tests, which evaluate every widget against them; Harvest's in-memory
//! fakes; and the renderer's tests, which draw the boards built from them.
//! Deterministic, so a board built from them can be compared byte for byte.
//!
//! The figures are consistent the way real ones are: a voter source's totals
//! are the sum of its cube, a Post source's the sum of its Posts, an activity
//! source's the sum of its hours. They are never shown to a viewer.

use super::config::Settings;
use super::payload::{
    Bucket, Counts, Cube, CubeCell, GroupRow, Notice, PostRow, ScopePayload,
    UNKNOWN_KEY,
};
use super::sources::{
    BuiltinDimension, DataSourceId, Measure, QueryTemplate, VoterDimensions,
};

/// The days the sample series covers, in order.
pub const SAMPLE_DAYS: [&str; 2] = ["2026-05-04", "2026-05-05"];

/// The offset the sample hours are in. Samples are not in any real zone.
const SAMPLE_OFFSET: &str = "+00:00";

/// A payload for `source` at the whole event, with a cube over the voter
/// dimensions `settings` configure.
pub fn sample_payload(
    source: DataSourceId,
    settings: Option<&Settings>,
) -> ScopePayload {
    let spec = source.spec();
    let measures = spec.measures;
    let mut payload = ScopePayload::default();

    let cube = match (spec.voter_dimensions, settings) {
        (VoterDimensions::Configured, Some(settings))
            if !settings.dimensions.is_empty() =>
        {
            Some(sample_cube(measures, settings))
        }
        _ => None,
    };

    let stated = !spec.states.is_empty();
    if stated {
        payload.posts = sample_posts(source);
    }

    if spec.has_template(QueryTemplate::Timeseries) {
        payload.series = sample_series(measures);
    }

    payload.totals = if let Some(cube) = &cube {
        sum(cube.cells.iter().map(|cell| &cell.counts))
    } else if stated {
        sum(payload.posts.iter().map(|post| &post.counts))
    } else if !payload.series.is_empty() {
        sum(payload.series.iter().map(|bucket| &bucket.counts))
    } else {
        measures
            .iter()
            .map(|measure| (*measure, base(*measure)))
            .collect()
    };
    payload.cube = cube;

    for dimension in spec.builtin_dimensions {
        let groups = match dimension {
            // Computed from the Posts when a query asks.
            BuiltinDimension::State => continue,
            // A Post source counts Posts, so a group is its Posts there.
            BuiltinDimension::Region | BuiltinDimension::Post if stated => {
                post_groups(*dimension, &payload.posts)
            }
            _ => sample_groups(*dimension, &payload.totals),
        };
        payload.groups.insert(dimension.to_string(), groups);
    }

    if source == DataSourceId::AccessSecurity {
        payload.notices = vec![Notice::UnregisteredAttemptsAtEventScopeOnly];
    }
    payload
}

/// A plausible count for a measure at the whole event.
fn base(measure: Measure) -> u64 {
    use Measure::*;
    match measure {
        Registered => 1200,
        PreEnrolled => 780,
        CredentialsIssued => 610,
        TestVoted => 140,
        Voted => 512,
        Applications => 900,
        Pending => 60,
        Approved => 780,
        Disapproved => 60,
        Posts => 1,
        Initialized | Opened | Tested | Tallied | Transmitted => 1,
        Paused | Closed | LockedDown | TransmissionFailed => 0,
        Logins => 2400,
        LoginFailures => 180,
        PasswordResets => 40,
        Detections => 7,
        Issues => 25,
        PendingIssues => 6,
    }
}

/// The values the sample gives a configured dimension: its bands, else its
/// labelled values, else two of its own; always some voters with none.
fn values_of(settings: &Settings, dimension: &str) -> Vec<String> {
    let mapping = &settings.dimensions[dimension];
    let mut values: Vec<String> = if !mapping.age_bands.is_empty() {
        mapping
            .age_bands
            .iter()
            .map(|band| band.label.clone())
            .collect()
    } else if !mapping.labels.is_empty() {
        mapping.labels.keys().cloned().collect()
    } else {
        vec![format!("{dimension}-a"), format!("{dimension}-b")]
    };
    values.push(UNKNOWN_KEY.to_string());
    values
}

fn sample_cube(measures: &[Measure], settings: &Settings) -> Cube {
    let dimensions: Vec<String> = settings.dimensions.keys().cloned().collect();
    let values: Vec<Vec<String>> = dimensions
        .iter()
        .map(|dimension| values_of(settings, dimension))
        .collect();
    // Every combination of values, as the position of each in its dimension.
    let mut cells: Vec<Vec<usize>> = vec![Vec::new()];
    for of_dimension in &values {
        cells = cells
            .into_iter()
            .flat_map(|cell| {
                (0..of_dimension.len()).map(move |at| {
                    let mut cell = cell.clone();
                    cell.push(at);
                    cell
                })
            })
            .collect();
    }
    let count = cells.len() as u64;
    let cells = cells
        .into_iter()
        .enumerate()
        .map(|(position, at)| {
            // Spread each measure over the cells unevenly but reproducibly.
            // A source lists its measures in funnel order, so each stage
            // keeps a share of the one before it that depends on the cell's
            // value in every dimension: groups turn out differently, and no
            // stage outgrows the one before.
            let weight = 1 + (position as u64 * 7) % 5;
            let mut before: Option<u64> = None;
            let counts = measures
                .iter()
                .enumerate()
                .map(|(stage, measure)| {
                    let share = base(*measure) * weight / (3 * count);
                    let share = match before {
                        None => share,
                        Some(before) => {
                            let kept = at.iter().enumerate().fold(
                                share,
                                |kept, (dimension, value)| {
                                    let tenths = 10
                                        - (value + dimension + stage) as u64
                                            % 4;
                                    kept * tenths / 10
                                },
                            );
                            kept.min(before)
                        }
                    };
                    before = Some(share);
                    (*measure, share)
                })
                .collect();
            let values = at
                .iter()
                .enumerate()
                .map(|(dimension, value)| values[dimension][*value].clone())
                .collect();
            CubeCell { values, counts }
        })
        .collect();
    Cube { dimensions, cells }
}

fn sample_posts(source: DataSourceId) -> Vec<PostRow> {
    let spec = source.spec();
    let places = [
        ("post-madrid", "Madrid", Some("Europe")),
        ("post-tokyo", "Tokyo", Some("Asia Pacific")),
        ("post-dubai", "Dubai", Some("Middle East")),
        ("post-riyadh", "Riyadh", None),
    ];
    places
        .iter()
        .enumerate()
        .map(|(position, (id, name, region))| {
            // Each Post a step further along, so every state shows.
            let state = spec.states[(position + 1) % spec.states.len()];
            PostRow {
                post_id: id.to_string(),
                post: name.to_string(),
                region: region.map(str::to_string),
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

fn sample_series(measures: &[Measure]) -> Vec<Bucket> {
    SAMPLE_DAYS
        .iter()
        .flat_map(|day| {
            (0..24).map(move |hour| Bucket {
                start: format!("{day}T{hour:02}:00:00"),
                day: day.to_string(),
                utc_offset: SAMPLE_OFFSET.to_string(),
                counts: measures
                    .iter()
                    .map(|measure| {
                        // Busier by day than by night.
                        let shape = [
                            0, 0, 0, 0, 0, 1, 2, 4, 6, 8, 9, 9, 8, 8, 9, 9, 8,
                            7, 6, 5, 4, 2, 1, 0,
                        ];
                        (*measure, base(*measure) * shape[hour] / 240)
                    })
                    .collect(),
            })
        })
        .collect()
}

/// A Post source's groups by region or by Post, summed from its Posts.
fn post_groups(
    dimension: BuiltinDimension,
    posts: &[PostRow],
) -> Vec<GroupRow> {
    let mut groups: Vec<GroupRow> = Vec::new();
    for post in posts {
        let (key, label) = match dimension {
            BuiltinDimension::Post => {
                (post.post_id.clone(), Some(post.post.clone()))
            }
            _ => (
                post.region
                    .clone()
                    .unwrap_or_else(|| UNKNOWN_KEY.to_string()),
                None,
            ),
        };
        let at = match groups.iter().position(|group| group.key == key) {
            Some(at) => at,
            None => {
                groups.push(GroupRow {
                    key,
                    label,
                    counts: Counts::new(),
                });
                groups.len() - 1
            }
        };
        for (measure, count) in &post.counts {
            *groups[at].counts.entry(*measure).or_default() += count;
        }
    }
    groups
}

fn sample_groups(
    dimension: BuiltinDimension,
    totals: &Counts,
) -> Vec<GroupRow> {
    let rows: &[(&str, Option<&str>, u64)] = match dimension {
        BuiltinDimension::Region => &[
            ("Europe", None, 5),
            ("Asia Pacific", None, 4),
            ("Middle East", None, 3),
            (UNKNOWN_KEY, None, 1),
        ],
        BuiltinDimension::Post => &[
            ("post-madrid", Some("Madrid"), 5),
            ("post-tokyo", Some("Tokyo"), 4),
            ("post-dubai", Some("Dubai"), 3),
            ("post-riyadh", Some("Riyadh"), 2),
        ],
        BuiltinDimension::Country => &[
            ("Spain", None, 5),
            ("Japan", None, 4),
            ("United Arab Emirates", None, 3),
            (UNKNOWN_KEY, None, 1),
        ],
        BuiltinDimension::Reason => &[
            ("document-unreadable", Some("Document unreadable"), 5),
            ("details-mismatch", Some("Details do not match"), 3),
            ("not-eligible", Some("Not eligible"), 1),
            (UNKNOWN_KEY, None, 1),
        ],
        BuiltinDimension::State => &[],
    };
    rows.iter()
        .map(|(key, label, weight)| GroupRow {
            key: key.to_string(),
            label: label.map(str::to_string),
            counts: totals
                .iter()
                .map(|(measure, total)| (*measure, total * weight / 14))
                .collect(),
        })
        .collect()
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

#[cfg(test)]
#[path = "sample_tests.rs"]
mod sample_tests;
