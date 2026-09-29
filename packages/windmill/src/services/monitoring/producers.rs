// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What each data source counts, from what one pass of the snapshot job
//! read: the payload of every scope, for every set of elections viewers may
//! see.
//!
//! Every figure is counted at its own scope from the rows in it, never added
//! up from narrower scopes, so a voter who can vote in two Posts of a region
//! is one voter of the region. A voter belongs to a scope when any of their
//! rows does, and what they count there is read from those rows only: they
//! voted in a Post when they voted in that election.
//!
//! Scopes are the ones [`ScopeKey`] names: the whole event, each region,
//! Post and country, and each country within a region or a Post. Only
//! scopes with something in them get a payload; a scope with none shows as
//! zero, which the reader builds with [`empty_payload`].
//!
//! Every payload has the same shape whatever is in it: every measure of its
//! source, every group its source can be grouped by (empty when nobody is
//! in it) and, for a source with voter dimensions, the cube. A query never
//! finds a figure missing only because nobody is in it.

use chrono::{DateTime, Duration, Offset, Timelike, Utc};
use chrono_tz::Tz;
use sequent_core::monitoring::config::Settings;
use sequent_core::monitoring::payload::{
    Bucket, Counts, Cube, CubeCell, GroupRow, Notice, PostRow, ScopePayload, UNKNOWN_KEY,
};
use sequent_core::monitoring::scope::ScopeKey;
use sequent_core::monitoring::sources::{
    BuiltinDimension, DataSourceId, Measure, PendingProducer, PostState, Producer, QueryTemplate,
    VoterDimensions,
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use strum::IntoEnumIterator;
use uuid::Uuid;

/// A voter's latest application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Enrollment {
    Pending,
    Accepted,
    Rejected,
}

/// One `monitoring_voter` row: a voter in one election.
#[derive(Debug, Clone, PartialEq)]
pub struct VoterRow {
    pub voter_id: String,
    pub election_id: Uuid,
    pub region: Option<String>,
    pub country: Option<String>,
    pub dims: BTreeMap<String, String>,
    pub pre_enrolled_at: Option<DateTime<Utc>>,
    pub first_voted_at: Option<DateTime<Utc>>,
    pub enrollment: Option<Enrollment>,
    pub enrollment_reason: Option<String>,
    pub enrollment_decided_at: Option<DateTime<Utc>>,
}

/// An election of the event: a Post.
#[derive(Debug, Clone, PartialEq)]
pub struct Post {
    pub id: Uuid,
    pub name: String,
    /// Every region the Post is in: its own annotation, its areas' or its
    /// voters', as the settings read regions. None: Unknown.
    pub regions: BTreeSet<String>,
    /// Where it stands in the poll.
    pub poll: PostState,
    /// Where it stands in counting and transmission.
    pub counting: PostState,
}

/// Sign-in attempts of one kind in one 15-minute bucket.
#[derive(Debug, Clone, PartialEq)]
pub struct LoginRow {
    pub bucket_start: DateTime<Utc>,
    pub event_type: String,
    /// Whether the username named a voter.
    pub registered: bool,
    pub area_id: Option<Uuid>,
    pub attempts: u64,
}

/// A set of elections a viewer may see, and its key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElectionSet {
    pub key: String,
    pub elections: BTreeSet<Uuid>,
}

/// What a pass read of an event. Owned, so a pass counts it off the async
/// runtime.
#[derive(Debug, Clone)]
pub struct EventFacts {
    pub settings: Settings,
    pub zone: Tz,
    pub posts: Vec<Post>,
    /// Sorted by voter.
    pub voters: Vec<VoterRow>,
    /// The elections each area votes in.
    pub area_elections: HashMap<Uuid, Vec<Uuid>>,
    /// Each area's region, when the settings read regions from an area
    /// annotation; empty otherwise.
    pub area_regions: HashMap<Uuid, String>,
    pub logins: Vec<LoginRow>,
}

/// Whether a source was counted, and why not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceStatus {
    Connected,
    NotConnected(PendingProducer),
}

/// A source's figures for one set of elections.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceFigures {
    pub source: DataSourceId,
    pub election_set_key: String,
    pub status: SourceStatus,
    /// By [`ScopeKey::canonical`].
    pub scopes: BTreeMap<String, ScopePayload>,
}

/// The builtin dimensions a source's payloads hold groups of: those its
/// by_group template reads from the payload's groups. Posts by state are
/// worked out from the Posts, not stored.
pub fn group_dimensions(source: DataSourceId) -> Vec<BuiltinDimension> {
    let spec = source.spec();
    if !spec.has_template(QueryTemplate::ByGroup) {
        return Vec::new();
    }
    spec.builtin_dimensions
        .iter()
        .copied()
        .filter(|dimension| *dimension != BuiltinDimension::State)
        .collect()
}

/// The voter dimensions a source's cube has, in the settings' order; `None`
/// when the source has no cube or the settings name no dimension.
pub fn cube_dimensions(source: DataSourceId, settings: &Settings) -> Option<Vec<String>> {
    (source.spec().voter_dimensions == VoterDimensions::Configured
        && !settings.dimensions.is_empty())
    .then(|| settings.dimensions.keys().cloned().collect())
}

/// A counted scope with nothing in it: every measure of the source at zero,
/// every group empty, and the cube with no cells. Built by the reader for a
/// scope a run holds no payload of, which for sign-ins is never the whole
/// event (always written), so it says unregistered attempts are excluded.
pub fn empty_payload(source: DataSourceId, settings: &Settings) -> ScopePayload {
    ScopePayload {
        totals: zeros(source),
        groups: group_dimensions(source)
            .into_iter()
            .map(|dimension| (dimension.to_string(), Vec::new()))
            .collect(),
        cube: cube_dimensions(source, settings).map(|dimensions| Cube {
            dimensions,
            cells: Vec::new(),
        }),
        notices: if source == DataSourceId::AccessSecurity {
            vec![Notice::UnregisteredAttemptsExcluded]
        } else {
            Vec::new()
        },
        ..ScopePayload::default()
    }
}

/// Every measure of `source` at zero.
fn zeros(source: DataSourceId) -> Counts {
    source
        .spec()
        .measures
        .iter()
        .map(|measure| (*measure, 0))
        .collect()
}

/// `counts` with every measure of `source`, those it lacks at zero.
fn complete(source: DataSourceId, counts: Counts) -> Counts {
    let mut all = zeros(source);
    add(&mut all, &counts);
    all
}

/// Every source's figures for every set.
pub fn produce(facts: &EventFacts, sets: &[ElectionSet]) -> Vec<SourceFigures> {
    let mut figures = Vec::new();
    for set in sets {
        let mut voters = voter_sources(facts, set);
        let posts = post_sources(facts, set);
        let logins = access_security(facts, set);
        for source in DataSourceId::iter() {
            let (status, scopes) = match source.spec().producer {
                Producer::Pending(pending) => {
                    (SourceStatus::NotConnected(pending), BTreeMap::new())
                }
                Producer::Available => (
                    SourceStatus::Connected,
                    match source {
                        DataSourceId::PollStatus => posts.poll.clone(),
                        DataSourceId::CountingTransmission => posts.counting.clone(),
                        DataSourceId::AccessSecurity => logins.clone(),
                        _ => voters.remove(&source).unwrap_or_default(),
                    },
                ),
            };
            figures.push(SourceFigures {
                source,
                election_set_key: set.key.clone(),
                status,
                scopes,
            });
        }
    }
    figures
}

fn canonical(region: Option<&str>, post: Option<Uuid>, country: Option<&str>) -> String {
    ScopeKey {
        region: region.map(str::to_string),
        post: post.map(|post| post.to_string()),
        country: country.map(str::to_string),
    }
    .canonical()
}

/// The scopes a row lies in.
fn row_scopes(row: &VoterRow) -> Vec<String> {
    let region = row.region.as_deref();
    let country = row.country.as_deref();
    let post = Some(row.election_id);
    let mut scopes = vec![canonical(None, None, None), canonical(None, post, None)];
    if region.is_some() {
        scopes.push(canonical(region, None, None));
    }
    if country.is_some() {
        scopes.push(canonical(None, None, country));
        scopes.push(canonical(None, post, country));
        if region.is_some() {
            scopes.push(canonical(region, None, country));
        }
    }
    scopes
}

/// What one voter counts as over some of their rows.
struct Facts<'r> {
    pre_enrolled_at: Option<DateTime<Utc>>,
    first_voted_at: Option<DateTime<Utc>>,
    dims: &'r BTreeMap<String, String>,
    enrollment: Option<Enrollment>,
    reason: Option<&'r str>,
    decided_at: Option<DateTime<Utc>>,
}

fn facts_of<'r>(rows: &[&'r VoterRow]) -> Facts<'r> {
    let first = rows[0];
    Facts {
        pre_enrolled_at: rows.iter().filter_map(|row| row.pre_enrolled_at).min(),
        first_voted_at: rows.iter().filter_map(|row| row.first_voted_at).min(),
        dims: &first.dims,
        enrollment: first.enrollment,
        reason: first.enrollment_reason.as_deref(),
        decided_at: first.enrollment_decided_at,
    }
}

const VOTER_SOURCES: [DataSourceId; 3] = [
    DataSourceId::VoterTurnout,
    DataSourceId::EnrollmentDecisions,
    DataSourceId::VotingEnrollmentActivity,
];

/// A source's counts of one voter, and when each happened, for the series.
fn voter_counts(
    source: DataSourceId,
    facts: &Facts<'_>,
) -> (Counts, Vec<(Measure, DateTime<Utc>)>) {
    let one = |yes: bool| u64::from(yes);
    let accepted = facts.enrollment == Some(Enrollment::Accepted);
    let rejected = facts.enrollment == Some(Enrollment::Rejected);
    let mut events = Vec::new();
    let counts: Counts = match source {
        DataSourceId::VoterTurnout => {
            events.extend(facts.first_voted_at.map(|at| (Measure::Voted, at)));
            [
                (Measure::Registered, 1),
                (Measure::PreEnrolled, one(facts.pre_enrolled_at.is_some())),
                (Measure::Voted, one(facts.first_voted_at.is_some())),
            ]
            .into()
        }
        DataSourceId::EnrollmentDecisions => {
            if let Some(at) = facts.decided_at {
                if accepted {
                    events.push((Measure::Approved, at));
                } else if rejected {
                    events.push((Measure::Disapproved, at));
                }
            }
            [
                (Measure::Applications, one(facts.enrollment.is_some())),
                (
                    Measure::Pending,
                    one(facts.enrollment == Some(Enrollment::Pending)),
                ),
                (Measure::Approved, one(accepted)),
                (Measure::Disapproved, one(rejected)),
            ]
            .into()
        }
        DataSourceId::VotingEnrollmentActivity => {
            if accepted {
                events.extend(facts.decided_at.map(|at| (Measure::Approved, at)));
            }
            events.extend(facts.first_voted_at.map(|at| (Measure::Voted, at)));
            [
                (Measure::Approved, one(accepted)),
                (Measure::Voted, one(facts.first_voted_at.is_some())),
            ]
            .into()
        }
        _ => Counts::new(),
    };
    (counts, events)
}

fn add(into: &mut Counts, counts: &Counts) {
    for (measure, count) in counts {
        *into.entry(*measure).or_default() += count;
    }
}

/// A scope's payload while it is counted.
#[derive(Default)]
struct Tally {
    totals: Counts,
    cube: BTreeMap<Vec<String>, Counts>,
    groups: BTreeMap<String, BTreeMap<String, Counts>>,
    series: BTreeMap<DateTime<Utc>, Counts>,
    notices: BTreeSet<Notice>,
}

impl Tally {
    fn group(&mut self, dimension: BuiltinDimension, key: &str, counts: &Counts) {
        add(
            self.groups
                .entry(dimension.to_string())
                .or_default()
                .entry(key.to_string())
                .or_default(),
            counts,
        );
    }

    fn event(&mut self, zone: Tz, measure: Measure, at: DateTime<Utc>, count: u64) {
        *self
            .series
            .entry(hour_start(zone, at))
            .or_default()
            .entry(measure)
            .or_default() += count;
    }

    /// The payload of `source`, in the shape of [`empty_payload`].
    fn payload(
        mut self,
        source: DataSourceId,
        settings: &Settings,
        zone: Tz,
        labels: &HashMap<String, String>,
    ) -> ScopePayload {
        for dimension in group_dimensions(source) {
            self.groups.entry(dimension.to_string()).or_default();
        }
        let groups = self
            .groups
            .into_iter()
            .map(|(dimension, rows)| {
                let rows = rows
                    .into_iter()
                    .map(|(key, counts)| GroupRow {
                        label: (dimension == BuiltinDimension::Post.to_string())
                            .then(|| labels.get(&key).cloned())
                            .flatten(),
                        key,
                        counts: complete(source, counts),
                    })
                    .collect();
                (dimension, rows)
            })
            .collect();
        let cells = self.cube;
        ScopePayload {
            totals: complete(source, self.totals),
            groups,
            cube: cube_dimensions(source, settings).map(|dimensions| Cube {
                dimensions,
                cells: cells
                    .into_iter()
                    .map(|(values, counts)| CubeCell {
                        values,
                        counts: complete(source, counts),
                    })
                    .collect(),
            }),
            posts: Vec::new(),
            series: series(zone, &self.series, series_measures(source)),
            notices: self.notices.into_iter().collect(),
        }
    }
}

/// The UTC instant a local hour of `zone` starts, for the hour `at` is in.
pub fn hour_start(zone: Tz, at: DateTime<Utc>) -> DateTime<Utc> {
    let local = at.with_timezone(&zone);
    at - Duration::seconds(i64::from(local.minute() * 60 + local.second()))
        - Duration::nanoseconds(i64::from(local.nanosecond()))
}

/// Hourly buckets from the first to the last with any count, none missing
/// between, each carrying every measure the source puts in series.
fn series(zone: Tz, hours: &BTreeMap<DateTime<Utc>, Counts>, measures: &[Measure]) -> Vec<Bucket> {
    let (Some(first), Some(last)) = (hours.keys().next(), hours.keys().next_back()) else {
        return Vec::new();
    };
    let mut buckets = Vec::new();
    let mut hour = *first;
    while hour <= *last {
        let local = hour.with_timezone(&zone);
        let offset = local.offset().fix().local_minus_utc();
        let mut counts: Counts = measures.iter().map(|measure| (*measure, 0)).collect();
        if let Some(found) = hours.get(&hour) {
            add(&mut counts, found);
        }
        buckets.push(Bucket {
            start: local.format("%Y-%m-%dT%H:%M:%S").to_string(),
            day: local.format("%Y-%m-%d").to_string(),
            utc_offset: format!(
                "{}{:02}:{:02}",
                if offset < 0 { '-' } else { '+' },
                offset.abs() / 3600,
                offset.abs() % 3600 / 60
            ),
            counts,
        });
        let mut next = hour_start(zone, hour + Duration::hours(1));
        if next <= hour {
            next = hour_start(zone, hour + Duration::hours(2));
        }
        hour = next;
    }
    buckets
}

fn series_measures(source: DataSourceId) -> &'static [Measure] {
    match source {
        DataSourceId::VoterTurnout => &[Measure::Voted],
        DataSourceId::EnrollmentDecisions => &[Measure::Approved, Measure::Disapproved],
        DataSourceId::VotingEnrollmentActivity => &[Measure::Approved, Measure::Voted],
        DataSourceId::AccessSecurity => &[
            Measure::Logins,
            Measure::LoginFailures,
            Measure::PasswordResets,
        ],
        _ => &[],
    }
}

fn post_labels(facts: &EventFacts) -> HashMap<String, String> {
    facts
        .posts
        .iter()
        .map(|post| (post.id.to_string(), post.name.clone()))
        .collect()
}

/// Turnout, enrollment decisions and voting and enrollment activity.
fn voter_sources(
    facts: &EventFacts,
    set: &ElectionSet,
) -> BTreeMap<DataSourceId, BTreeMap<String, ScopePayload>> {
    let dimensions: Vec<String> = facts.settings.dimensions.keys().cloned().collect();
    let mut tallies: BTreeMap<DataSourceId, BTreeMap<String, Tally>> = BTreeMap::new();
    let rows: Vec<&VoterRow> = facts
        .voters
        .iter()
        .filter(|row| set.elections.contains(&row.election_id))
        .collect();
    for voter in rows.chunk_by(|a, b| a.voter_id == b.voter_id) {
        let mut scopes: BTreeMap<String, Vec<&VoterRow>> = BTreeMap::new();
        for row in voter {
            for scope in row_scopes(row) {
                scopes.entry(scope).or_default().push(*row);
            }
        }
        for (scope, rows) in scopes {
            let whole = facts_of(&rows);
            let groups: [(BuiltinDimension, Vec<(String, Vec<&VoterRow>)>); 3] = [
                (
                    BuiltinDimension::Region,
                    partition(&rows, |row| row.region.clone()),
                ),
                (
                    BuiltinDimension::Post,
                    partition(&rows, |row| Some(row.election_id.to_string())),
                ),
                (
                    BuiltinDimension::Country,
                    partition(&rows, |row| row.country.clone()),
                ),
            ];
            for source in VOTER_SOURCES {
                let tally = tallies
                    .entry(source)
                    .or_default()
                    .entry(scope.clone())
                    .or_default();
                let (counts, events) = voter_counts(source, &whole);
                add(&mut tally.totals, &counts);
                for (measure, at) in events {
                    tally.event(facts.zone, measure, at, 1);
                }
                if source == DataSourceId::VoterTurnout && !dimensions.is_empty() {
                    let values = dimensions
                        .iter()
                        .map(|name| {
                            whole
                                .dims
                                .get(name)
                                .cloned()
                                .unwrap_or_else(|| UNKNOWN_KEY.to_string())
                        })
                        .collect();
                    add(tally.cube.entry(values).or_default(), &counts);
                }
                if source == DataSourceId::EnrollmentDecisions
                    && whole.enrollment == Some(Enrollment::Rejected)
                {
                    // The voters disapproved for the reason: each one
                    // application, disapproved.
                    tally.group(
                        BuiltinDimension::Reason,
                        whole.reason.unwrap_or(UNKNOWN_KEY),
                        &counts,
                    );
                }
                if source == DataSourceId::VotingEnrollmentActivity {
                    continue;
                }
                for (dimension, parts) in &groups {
                    for (key, part) in parts {
                        let (counts, _) = voter_counts(source, &facts_of(part));
                        tally.group(*dimension, key, &counts);
                    }
                }
            }
        }
    }
    let labels = post_labels(facts);
    tallies
        .into_iter()
        .map(|(source, scopes)| {
            let scopes = scopes
                .into_iter()
                .map(|(scope, tally)| {
                    (
                        scope,
                        tally.payload(source, &facts.settings, facts.zone, &labels),
                    )
                })
                .collect();
            (source, scopes)
        })
        .collect()
}

/// Rows by the value `key` reads, a missing one under [`UNKNOWN_KEY`].
fn partition<'r>(
    rows: &[&'r VoterRow],
    key: impl Fn(&VoterRow) -> Option<String>,
) -> Vec<(String, Vec<&'r VoterRow>)> {
    let mut parts: BTreeMap<String, Vec<&'r VoterRow>> = BTreeMap::new();
    for row in rows {
        parts
            .entry(key(row).unwrap_or_else(|| UNKNOWN_KEY.to_string()))
            .or_default()
            .push(*row);
    }
    parts.into_iter().collect()
}

struct PostSources {
    poll: BTreeMap<String, ScopePayload>,
    counting: BTreeMap<String, ScopePayload>,
}

/// A Post's regions as one row shows them: the region of a region scope,
/// else all of them.
fn shown_region(post: &Post, region_scope: Option<&str>) -> Option<String> {
    match region_scope {
        Some(region) => Some(region.to_string()),
        None if post.regions.is_empty() => None,
        None => Some(post.regions.iter().cloned().collect::<Vec<_>>().join(", ")),
    }
}

/// Poll status and counting and transmission. A Post is counted in every
/// region it is in, so at a scope with Posts of several regions the region
/// groups may add up to more than the Posts: each group is counted on its
/// own, as every group is.
fn post_sources(facts: &EventFacts, set: &ElectionSet) -> PostSources {
    let mut posts: Vec<&Post> = facts
        .posts
        .iter()
        .filter(|post| set.elections.contains(&post.id))
        .collect();
    posts.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
    let labels = post_labels(facts);
    let for_source = |source: DataSourceId, state: fn(&Post) -> PostState| {
        let spec = source.spec();
        // Each scope's Posts, and the region it is of, if it is a region's.
        let mut scopes: BTreeMap<String, (Option<&str>, Vec<&Post>)> = BTreeMap::new();
        for post in &posts {
            scopes
                .entry(canonical(None, None, None))
                .or_default()
                .1
                .push(post);
            scopes
                .entry(canonical(None, Some(post.id), None))
                .or_default()
                .1
                .push(post);
            for region in &post.regions {
                let entry = scopes
                    .entry(canonical(Some(region), None, None))
                    .or_insert_with(|| (Some(region.as_str()), Vec::new()));
                entry.1.push(post);
            }
        }
        scopes
            .into_iter()
            .map(|(scope, (region_scope, in_scope))| {
                let mut tally = Tally::default();
                let mut rows = Vec::new();
                for post in in_scope {
                    let state = state(post);
                    let counts: Counts = spec
                        .measures
                        .iter()
                        .map(|measure| (*measure, u64::from(state.has_reached(*measure))))
                        .collect();
                    add(&mut tally.totals, &counts);
                    tally.group(BuiltinDimension::Post, &post.id.to_string(), &counts);
                    match region_scope {
                        Some(region) => tally.group(BuiltinDimension::Region, region, &counts),
                        None if post.regions.is_empty() => {
                            tally.group(BuiltinDimension::Region, UNKNOWN_KEY, &counts)
                        }
                        None => {
                            for region in &post.regions {
                                tally.group(BuiltinDimension::Region, region, &counts);
                            }
                        }
                    }
                    rows.push(PostRow {
                        post_id: post.id.to_string(),
                        post: post.name.clone(),
                        region: shown_region(post, region_scope),
                        state: Some(state),
                        counts,
                    });
                }
                let mut payload = tally.payload(source, &facts.settings, facts.zone, &labels);
                // Groups keyed by Post keep the Posts' order.
                if let Some(by_post) = payload.groups.get_mut(&BuiltinDimension::Post.to_string()) {
                    let order: HashMap<&str, usize> = rows
                        .iter()
                        .enumerate()
                        .map(|(at, row)| (row.post_id.as_str(), at))
                        .collect();
                    by_post.sort_by_key(|row| order.get(row.key.as_str()).copied());
                }
                payload.posts = rows;
                (scope, payload)
            })
            .collect()
    };
    PostSources {
        poll: for_source(DataSourceId::PollStatus, |post| post.poll),
        counting: for_source(DataSourceId::CountingTransmission, |post| post.counting),
    }
}

/// The measure a Keycloak event type counts towards.
pub fn login_measure(event_type: &str) -> Option<Measure> {
    match event_type {
        "LOGIN" => Some(Measure::Logins),
        "LOGIN_ERROR" => Some(Measure::LoginFailures),
        "RESET_PASSWORD" | "UPDATE_PASSWORD" => Some(Measure::PasswordResets),
        _ => None,
    }
}

/// Sign-in attempts. An attempt by a voter belongs to the Posts their area
/// votes in, and to the region of the area when the settings read regions
/// from areas, else to the regions of those Posts. One by an unknown
/// username belongs to no Post and is counted for the whole event only. One
/// by a voter with no area, or whose area votes in no Post, is too, but only
/// for the set of every election: a set restricted to some Posts cannot
/// tell whether the voter is one of theirs.
fn access_security(facts: &EventFacts, set: &ElectionSet) -> BTreeMap<String, ScopePayload> {
    let source = DataSourceId::AccessSecurity;
    let regions_of: HashMap<Uuid, &BTreeSet<String>> = facts
        .posts
        .iter()
        .map(|post| (post.id, &post.regions))
        .collect();
    let full = facts
        .posts
        .iter()
        .all(|post| set.elections.contains(&post.id));
    let by_area = facts.settings.scope.region.area_annotation.is_some();
    let mut tallies: BTreeMap<String, Tally> = BTreeMap::new();
    let event_scope = canonical(None, None, None);
    tallies.entry(event_scope.clone()).or_default();
    for login in &facts.logins {
        let Some(measure) = login_measure(&login.event_type) else {
            continue;
        };
        let counts: Counts = [(measure, login.attempts)].into();
        let area = login.area_id.filter(|_| login.registered);
        let elections = area.and_then(|area| facts.area_elections.get(&area));
        let posts: Vec<Uuid> = elections
            .into_iter()
            .flatten()
            .copied()
            .filter(|election| set.elections.contains(election))
            .collect();
        let placed = elections.is_some_and(|elections| !elections.is_empty());
        let at_event_only = !login.registered || !placed;
        if at_event_only {
            if login.registered && !full {
                // A voter this set may not see.
                continue;
            }
        } else if posts.is_empty() {
            // A voter of Posts this set does not see.
            continue;
        }
        let regions: BTreeSet<&str> = if at_event_only {
            BTreeSet::new()
        } else if by_area {
            area.and_then(|area| facts.area_regions.get(&area))
                .map(String::as_str)
                .into_iter()
                .collect()
        } else {
            posts
                .iter()
                .filter_map(|post| regions_of.get(post))
                .flat_map(|regions| regions.iter().map(String::as_str))
                .collect()
        };
        let mut scopes: Vec<(String, Option<&str>, Vec<Uuid>)> =
            vec![(event_scope.clone(), None, posts.clone())];
        for post in &posts {
            scopes.push((canonical(None, Some(*post), None), None, vec![*post]));
        }
        for region in &regions {
            scopes.push((
                canonical(Some(region), None, None),
                Some(*region),
                posts.clone(),
            ));
        }
        for (scope, region_scope, posts) in scopes {
            let tally = tallies.entry(scope).or_default();
            add(&mut tally.totals, &counts);
            tally.event(facts.zone, measure, login.bucket_start, login.attempts);
            if at_event_only {
                continue;
            }
            for post in &posts {
                tally.group(BuiltinDimension::Post, &post.to_string(), &counts);
            }
            match region_scope {
                Some(region) => tally.group(BuiltinDimension::Region, region, &counts),
                None if regions.is_empty() => {
                    tally.group(BuiltinDimension::Region, UNKNOWN_KEY, &counts)
                }
                None => {
                    for region in &regions {
                        tally.group(BuiltinDimension::Region, region, &counts);
                    }
                }
            }
        }
    }
    let labels = post_labels(facts);
    tallies
        .into_iter()
        .map(|(scope, mut tally)| {
            tally.notices.insert(if scope == event_scope {
                Notice::UnregisteredAttemptsAtEventScopeOnly
            } else {
                Notice::UnregisteredAttemptsExcluded
            });
            let payload = tally.payload(source, &facts.settings, facts.zone, &labels);
            (scope, payload)
        })
        .collect()
}

/// The channels a Post can be voted in, as its status names them.
const VOTING_CHANNELS: [&str; 4] = [
    "voting_status",
    "kiosk_voting_status",
    "early_voting_status",
    "telephone_voting_status",
];

/// Where a Post stands over every channel it is voted in: open while any
/// channel is, else paused while any is, else closed once any was, else not
/// started.
pub fn voting_status(status: Option<&Value>) -> &'static str {
    let channels: Vec<&str> = VOTING_CHANNELS
        .iter()
        .filter_map(|channel| status?.get(channel)?.as_str())
        .collect();
    ["OPEN", "PAUSED", "CLOSED"]
        .into_iter()
        .find(|wanted| channels.contains(wanted))
        .unwrap_or("NOT_STARTED")
}

/// Where a Post stands in the poll, from its status and whether its
/// initialization report was generated.
pub fn poll_state(voting_status: Option<&str>, initialized: bool) -> PostState {
    match voting_status {
        Some("CLOSED") => PostState::Closed,
        Some("PAUSED") => PostState::Paused,
        Some("OPEN") => PostState::Opened,
        _ if initialized => PostState::Initialized,
        _ => PostState::NotInitialized,
    }
}

/// Zone-aware formatting needs a zone the settings name; one the database
/// does not know counts in UTC.
pub fn zone_of(settings: &Settings) -> Tz {
    settings.time_zone.parse().unwrap_or(Tz::UTC)
}

#[cfg(test)]
#[path = "producers_tests.rs"]
mod producers_tests;
