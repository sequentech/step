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
//! The figures are consistent the way real ones are:
//! - one electorate: every voter source reads the same voters, so the voted
//!   total of turnout is the running total of voting activity;
//! - a voter source's totals are the sum of its cube, a Post source's the
//!   sum of its Posts, and every group list and series splits the totals
//!   exactly;
//! - each funnel stage stays below the one before, in every cell and group;
//! - groups turn out differently, and there are more Posts and reasons than
//!   a list's limit, so a chart that mixes up a ratio or ignores a limit
//!   shows it.
//!
//! They are never shown to a viewer.

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

/// The sample's Posts: id, name and region. One has no region.
const POSTS: [(&str, &str, Option<&str>); 12] = [
    ("post-madrid", "Madrid", Some("Europe")),
    ("post-rome", "Rome", Some("Europe")),
    ("post-london", "London", Some("Europe")),
    ("post-tokyo", "Tokyo", Some("Asia Pacific")),
    ("post-singapore", "Singapore", Some("Asia Pacific")),
    ("post-sydney", "Sydney", Some("Asia Pacific")),
    ("post-dubai", "Dubai", Some("Middle East")),
    ("post-doha", "Doha", Some("Middle East")),
    ("post-kuwait", "Kuwait", Some("Middle East")),
    ("post-toronto", "Toronto", Some("Americas")),
    ("post-washington", "Washington", Some("Americas")),
    ("post-riyadh", "Riyadh", None),
];

const REGIONS: [&str; 4] =
    ["Europe", "Asia Pacific", "Middle East", "Americas"];

const COUNTRIES: [&str; 11] = [
    "Spain",
    "Italy",
    "United Kingdom",
    "Japan",
    "Singapore",
    "Australia",
    "United Arab Emirates",
    "Qatar",
    "Kuwait",
    "Canada",
    "United States",
];

/// Disapproval reasons: key and label.
const REASONS: [(&str, &str); 12] = [
    ("document-unreadable", "Document unreadable"),
    ("details-mismatch", "Details do not match"),
    ("photo-unclear", "Photo unclear"),
    ("document-expired", "Document expired"),
    ("not-eligible", "Not eligible"),
    ("duplicate", "Duplicate application"),
    ("signature-missing", "Signature missing"),
    ("wrong-post", "Wrong Post"),
    ("form-incomplete", "Form incomplete"),
    ("address-unverified", "Address unverified"),
    ("name-mismatch", "Name does not match"),
    ("underage", "Under voting age"),
];

const ISSUE_CATEGORIES: [&str; 4] =
    ["Sign-in", "Enrollment", "Ballot", "Other"];

const ATTACK_CATEGORIES: [&str; 3] =
    ["Credential stuffing", "Brute force", "Bot traffic"];

/// Voters per hour of a day, relative: busier by day than by night.
const HOURLY_SHAPE: [u64; 24] = [
    0, 0, 0, 0, 0, 1, 2, 4, 6, 8, 9, 9, 8, 8, 9, 9, 8, 7, 6, 5, 4, 2, 1, 0,
];

/// A payload for `source` at the whole event, with a cube over the voter
/// dimensions `settings` configure.
pub fn sample_payload(
    source: DataSourceId,
    settings: Option<&Settings>,
) -> ScopePayload {
    let spec = source.spec();
    let measures = spec.measures;
    let electorate = settings
        .filter(|settings| !settings.dimensions.is_empty())
        .map(electorate);
    let mut payload = ScopePayload::default();

    let stated = !spec.states.is_empty();
    if stated {
        payload.posts = sample_posts(source);
    }
    if let (VoterDimensions::Configured, Some(electorate)) =
        (spec.voter_dimensions, &electorate)
    {
        payload.cube = Some(project(electorate, measures));
    }

    let event = event_totals(electorate.as_ref());
    payload.totals = if let Some(cube) = &payload.cube {
        sum(cube.cells.iter().map(|cell| &cell.counts))
    } else if stated {
        sum(payload.posts.iter().map(|post| &post.counts))
    } else {
        measures
            .iter()
            .map(|measure| (*measure, event[measure]))
            .collect()
    };

    if spec.has_template(QueryTemplate::Timeseries) {
        payload.series = sample_series(&payload.totals);
    }

    for dimension in spec.builtin_dimensions {
        let groups = match dimension {
            // Computed from the Posts when a query asks.
            BuiltinDimension::State => continue,
            // A Post source counts Posts, so a group is its Posts there.
            BuiltinDimension::Region | BuiltinDimension::Post if stated => {
                post_groups(*dimension, &payload.posts)
            }
            _ => sample_groups(source, *dimension, measures, &payload.totals),
        };
        payload.groups.insert(dimension.to_string(), groups);
    }

    if source == DataSourceId::AccessSecurity {
        payload.notices = vec![Notice::UnregisteredAttemptsAtEventScopeOnly];
    }
    payload
}

/// A plausible count for a measure at the whole event, before the voter
/// funnel narrows it.
fn base(measure: Measure) -> u64 {
    use Measure::*;
    match measure {
        Registered => 1200,
        PreEnrolled => 780,
        Approved => 780,
        CredentialsIssued => 610,
        TestVoted => 140,
        Voted => 512,
        Applications => 900,
        Pending => 60,
        Disapproved => 60,
        Posts => POSTS.len() as u64,
        Initialized | Opened | Tested | Tallied | Transmitted => 1,
        Paused | Closed | LockedDown | TransmissionFailed => 0,
        Logins => 2400,
        LoginFailures => 180,
        PasswordResets => 40,
        Detections => 40,
        Issues => 25,
        PendingIssues => 6,
    }
}

/// The measure each voter measure narrows, in the order the funnel runs.
/// Approved voters are exactly the pre-enrolled ones: validating a voter's
/// document is what approves them.
const VOTER_FUNNEL: [(Measure, Option<Measure>); 6] = [
    (Measure::Registered, None),
    (Measure::PreEnrolled, Some(Measure::Registered)),
    (Measure::Approved, Some(Measure::PreEnrolled)),
    (Measure::CredentialsIssued, Some(Measure::Approved)),
    (Measure::TestVoted, Some(Measure::PreEnrolled)),
    (Measure::Voted, Some(Measure::PreEnrolled)),
];

/// The totals every source agrees on: the electorate's where there is one,
/// and the applications the decisions add up to.
fn event_totals(electorate: Option<&Cube>) -> Counts {
    use strum::IntoEnumIterator;
    let mut totals: Counts = Measure::iter()
        .map(|measure| (measure, base(measure)))
        .collect();
    if let Some(electorate) = electorate {
        totals.extend(sum(electorate.cells.iter().map(|cell| &cell.counts)));
    }
    totals.insert(
        Measure::Applications,
        totals[&Measure::Approved]
            + totals[&Measure::Disapproved]
            + totals[&Measure::Pending],
    );
    totals
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

/// Every voter measure over every combination of the configured
/// dimensions' values.
fn electorate(settings: &Settings) -> Cube {
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
            // Voters spread over the cells unevenly but reproducibly. Each
            // funnel stage keeps a share of the one it narrows that depends
            // on the cell's value in every dimension, so groups turn out
            // differently and no stage outgrows the one before.
            let weight = 1 + (position as u64 * 7) % 5;
            let mut counts = Counts::new();
            for (stage, (measure, narrows)) in VOTER_FUNNEL.iter().enumerate() {
                let count = match (measure, narrows) {
                    (_, None) => base(*measure) * weight / (3 * count),
                    (Measure::Approved, Some(same)) => counts[same],
                    (_, Some(narrows)) => at.iter().enumerate().fold(
                        counts[narrows],
                        |kept, (dimension, value)| {
                            let tenths =
                                9 - (value + dimension + stage) as u64 % 4;
                            kept * tenths / 10
                        },
                    ),
                };
                counts.insert(*measure, count);
            }
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

/// The electorate's cube with only a source's measures.
fn project(electorate: &Cube, measures: &[Measure]) -> Cube {
    Cube {
        dimensions: electorate.dimensions.clone(),
        cells: electorate
            .cells
            .iter()
            .map(|cell| CubeCell {
                values: cell.values.clone(),
                counts: measures
                    .iter()
                    .map(|measure| (*measure, cell.counts[measure]))
                    .collect(),
            })
            .collect(),
    }
}

fn sample_posts(source: DataSourceId) -> Vec<PostRow> {
    let spec = source.spec();
    POSTS
        .iter()
        .enumerate()
        .map(|(position, (id, name, region))| {
            // Posts at every step along, so every state shows.
            let state = spec.states[position % spec.states.len()];
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

/// Each measure's total over the hours of the sample days, the first day
/// busier than the second.
fn sample_series(totals: &Counts) -> Vec<Bucket> {
    let hours: Vec<(&str, usize)> = SAMPLE_DAYS
        .iter()
        .flat_map(|day| (0..24).map(move |hour| (*day, hour)))
        .collect();
    let weights: Vec<u64> = hours
        .iter()
        .map(|(day, hour)| {
            let busier = if *day == SAMPLE_DAYS[0] { 2 } else { 1 };
            HOURLY_SHAPE[*hour] * busier
        })
        .collect();
    let shares: Vec<(Measure, Vec<u64>)> = totals
        .iter()
        .map(|(measure, total)| (*measure, apportion(*total, &weights)))
        .collect();
    hours
        .iter()
        .enumerate()
        .map(|(at, (day, hour))| Bucket {
            start: format!("{day}T{hour:02}:00:00"),
            day: day.to_string(),
            utc_offset: SAMPLE_OFFSET.to_string(),
            counts: shares
                .iter()
                .map(|(measure, shares)| (*measure, shares[at]))
                .collect(),
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

/// The groups a built-in dimension has in the sample: key and label.
fn group_keys(
    source: DataSourceId,
    dimension: BuiltinDimension,
) -> Vec<(String, Option<String>)> {
    let own = |keys: &[&str]| -> Vec<(String, Option<String>)> {
        keys.iter().map(|key| (key.to_string(), None)).collect()
    };
    let mut keys = match dimension {
        BuiltinDimension::Region => own(&REGIONS),
        BuiltinDimension::Country => own(&COUNTRIES),
        BuiltinDimension::Post => POSTS
            .iter()
            .map(|(id, name, _)| (id.to_string(), Some(name.to_string())))
            .collect(),
        BuiltinDimension::Reason => REASONS
            .iter()
            .map(|(key, label)| (key.to_string(), Some(label.to_string())))
            .collect(),
        BuiltinDimension::Category
            if source == DataSourceId::AttackDetections =>
        {
            own(&ATTACK_CATEGORIES)
        }
        BuiltinDimension::Category => own(&ISSUE_CATEGORIES),
        BuiltinDimension::State => Vec::new(),
    };
    // Every Post has a Post; anything else can be missing.
    if dimension != BuiltinDimension::Post {
        keys.push((UNKNOWN_KEY.to_string(), None));
    }
    keys
}

/// A built-in dimension's groups, splitting each measure's total exactly.
/// Each measure leans a little differently over the groups, so a group's
/// ratio is its own; a reason is only given for a disapproval.
fn sample_groups(
    source: DataSourceId,
    dimension: BuiltinDimension,
    measures: &[Measure],
    totals: &Counts,
) -> Vec<GroupRow> {
    let keys = group_keys(source, dimension);
    let counted: Vec<Measure> = match dimension {
        BuiltinDimension::Reason => vec![Measure::Disapproved],
        _ => measures.to_vec(),
    };
    let shares: Vec<(Measure, Vec<u64>)> = counted
        .iter()
        .enumerate()
        .map(|(stage, measure)| {
            let weights: Vec<u64> = (0..keys.len())
                .map(|at| {
                    // Fewer in later groups, and a lean of up to a seventh
                    // either way that differs by measure.
                    let size = 3 * keys.len() as u64 - 2 * at as u64;
                    let lean = 14 + ((at * 3 + stage) % 5) as u64 - 2;
                    size * lean
                })
                .collect();
            (*measure, apportion(totals[measure], &weights))
        })
        .collect();
    keys.into_iter()
        .enumerate()
        .map(|(at, (key, label))| GroupRow {
            key,
            label,
            counts: shares
                .iter()
                .map(|(measure, shares)| (*measure, shares[at]))
                .collect(),
        })
        .collect()
}

/// `total` split in proportion to `weights`, exactly: the largest
/// remainders take what rounding down leaves, the earliest first on a tie.
fn apportion(total: u64, weights: &[u64]) -> Vec<u64> {
    let whole: u64 = weights.iter().sum();
    if whole == 0 {
        return vec![0; weights.len()];
    }
    let mut shares: Vec<u64> = weights
        .iter()
        .map(|weight| total * weight / whole)
        .collect();
    let mut remainders: Vec<(u64, usize)> = weights
        .iter()
        .enumerate()
        .map(|(at, weight)| (total * weight % whole, at))
        .collect();
    remainders.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let left = total - shares.iter().sum::<u64>();
    for (_, at) in remainders.into_iter().take(left as usize) {
        shares[at] += 1;
    }
    shares
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
