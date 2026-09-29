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
//! zero, which the reader builds.

use chrono::{DateTime, Duration, Offset, Timelike, Utc};
use chrono_tz::Tz;
use sequent_core::monitoring::config::Settings;
use sequent_core::monitoring::payload::{
    Bucket, Counts, Cube, CubeCell, GroupRow, Notice, PostRow, ScopePayload, UNKNOWN_KEY,
};
use sequent_core::monitoring::scope::ScopeKey;
use sequent_core::monitoring::sources::{
    BuiltinDimension, DataSourceId, Measure, PendingProducer, PostState, Producer,
};
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
    pub region: Option<String>,
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

/// What a pass read of an event.
#[derive(Debug, Clone)]
pub struct EventFacts<'a> {
    pub settings: &'a Settings,
    pub zone: Tz,
    pub posts: Vec<Post>,
    /// Sorted by voter.
    pub voters: Vec<VoterRow>,
    /// The elections each area votes in.
    pub area_elections: HashMap<Uuid, Vec<Uuid>>,
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

/// Every source's figures for every set.
pub fn produce(facts: &EventFacts<'_>, sets: &[ElectionSet]) -> Vec<SourceFigures> {
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

    fn payload(
        self,
        cube_dimensions: Option<&[String]>,
        zone: Tz,
        series_measures: &[Measure],
        labels: &HashMap<String, String>,
    ) -> ScopePayload {
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
                        counts,
                    })
                    .collect();
                (dimension, rows)
            })
            .collect();
        ScopePayload {
            totals: self.totals,
            groups,
            cube: cube_dimensions.map(|dimensions| Cube {
                dimensions: dimensions.to_vec(),
                cells: self
                    .cube
                    .into_iter()
                    .map(|(values, counts)| CubeCell { values, counts })
                    .collect(),
            }),
            posts: Vec::new(),
            series: series(zone, &self.series, series_measures),
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

fn post_labels(facts: &EventFacts<'_>) -> HashMap<String, String> {
    facts
        .posts
        .iter()
        .map(|post| (post.id.to_string(), post.name.clone()))
        .collect()
}

/// Turnout, enrollment decisions and voting and enrollment activity.
fn voter_sources(
    facts: &EventFacts<'_>,
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
                    tally.group(
                        BuiltinDimension::Reason,
                        whole.reason.unwrap_or(UNKNOWN_KEY),
                        &[(Measure::Disapproved, 1)].into(),
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
            let cube = (source == DataSourceId::VoterTurnout && !dimensions.is_empty())
                .then_some(dimensions.as_slice());
            let scopes = scopes
                .into_iter()
                .map(|(scope, tally)| {
                    (
                        scope,
                        tally.payload(cube, facts.zone, series_measures(source), &labels),
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

fn post_sources(facts: &EventFacts<'_>, set: &ElectionSet) -> PostSources {
    let mut posts: Vec<&Post> = facts
        .posts
        .iter()
        .filter(|post| set.elections.contains(&post.id))
        .collect();
    posts.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
    let for_source = |source: DataSourceId, state: fn(&Post) -> PostState| {
        let spec = source.spec();
        let mut scopes: BTreeMap<String, Vec<PostRow>> = BTreeMap::new();
        for post in &posts {
            let state = state(post);
            let row = PostRow {
                post_id: post.id.to_string(),
                post: post.name.clone(),
                region: post.region.clone(),
                state: Some(state),
                counts: spec
                    .measures
                    .iter()
                    .map(|measure| (*measure, u64::from(state.has_reached(*measure))))
                    .collect(),
            };
            let mut keys = vec![
                canonical(None, None, None),
                canonical(None, Some(post.id), None),
            ];
            if let Some(region) = &post.region {
                keys.push(canonical(Some(region), None, None));
            }
            for key in keys {
                scopes.entry(key).or_default().push(row.clone());
            }
        }
        scopes
            .into_iter()
            .map(|(scope, rows)| {
                let mut totals: Counts =
                    spec.measures.iter().map(|measure| (*measure, 0)).collect();
                let mut regions: BTreeMap<String, Counts> = BTreeMap::new();
                let mut by_post = Vec::new();
                for row in &rows {
                    add(&mut totals, &row.counts);
                    add(
                        regions
                            .entry(
                                row.region
                                    .clone()
                                    .unwrap_or_else(|| UNKNOWN_KEY.to_string()),
                            )
                            .or_default(),
                        &row.counts,
                    );
                    by_post.push(GroupRow {
                        key: row.post_id.clone(),
                        label: Some(row.post.clone()),
                        counts: row.counts.clone(),
                    });
                }
                let groups = [
                    (
                        BuiltinDimension::Region.to_string(),
                        regions
                            .into_iter()
                            .map(|(key, counts)| GroupRow {
                                key,
                                label: None,
                                counts,
                            })
                            .collect(),
                    ),
                    (BuiltinDimension::Post.to_string(), by_post),
                ]
                .into();
                let payload = ScopePayload {
                    totals,
                    groups,
                    posts: rows,
                    ..ScopePayload::default()
                };
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

/// Sign-in attempts. An attempt belongs to the Posts its voter's area votes
/// in; one by an unknown username, or a voter with no area, belongs to no
/// Post and is counted for the whole event only.
fn access_security(facts: &EventFacts<'_>, set: &ElectionSet) -> BTreeMap<String, ScopePayload> {
    let measures = DataSourceId::AccessSecurity.spec().measures;
    let region_of: HashMap<Uuid, Option<&str>> = facts
        .posts
        .iter()
        .map(|post| (post.id, post.region.as_deref()))
        .collect();
    let mut tallies: BTreeMap<String, Tally> = BTreeMap::new();
    let event_scope = canonical(None, None, None);
    tallies.entry(event_scope.clone()).or_default();
    for login in &facts.logins {
        let Some(measure) = login_measure(&login.event_type) else {
            continue;
        };
        let counts: Counts = [(measure, login.attempts)].into();
        let posts: Vec<Uuid> = login
            .area_id
            .filter(|_| login.registered)
            .and_then(|area| facts.area_elections.get(&area))
            .map(|elections| {
                elections
                    .iter()
                    .copied()
                    .filter(|election| set.elections.contains(election))
                    .collect()
            })
            .unwrap_or_default();
        let at_event_only = !login.registered || login.area_id.is_none();
        if posts.is_empty() && !at_event_only {
            // A voter of Posts this set does not see.
            continue;
        }
        let region = |post: &Uuid| region_of.get(post).copied().flatten();
        let mut scopes: BTreeMap<String, Vec<Uuid>> = BTreeMap::new();
        scopes.insert(event_scope.clone(), posts.clone());
        for post in &posts {
            scopes
                .entry(canonical(None, Some(*post), None))
                .or_default()
                .push(*post);
            if let Some(region) = region(post) {
                scopes
                    .entry(canonical(Some(region), None, None))
                    .or_default()
                    .push(*post);
            }
        }
        for (scope, posts) in scopes {
            let tally = tallies.entry(scope).or_default();
            add(&mut tally.totals, &counts);
            tally.event(facts.zone, measure, login.bucket_start, login.attempts);
            let regions: BTreeSet<&str> = posts
                .iter()
                .map(|post| region(post).unwrap_or(UNKNOWN_KEY))
                .collect();
            for post in &posts {
                tally.group(BuiltinDimension::Post, &post.to_string(), &counts);
            }
            for region in regions {
                tally.group(BuiltinDimension::Region, region, &counts);
            }
        }
    }
    let labels = post_labels(facts);
    tallies
        .into_iter()
        .map(|(scope, mut tally)| {
            for measure in measures {
                tally.totals.entry(*measure).or_default();
            }
            tally.notices.insert(if scope == event_scope {
                Notice::UnregisteredAttemptsAtEventScopeOnly
            } else {
                Notice::UnregisteredAttemptsExcluded
            });
            let payload = tally.payload(
                None,
                facts.zone,
                series_measures(DataSourceId::AccessSecurity),
                &labels,
            );
            (scope, payload)
        })
        .collect()
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
