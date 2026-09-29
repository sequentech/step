// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The governed data sources.
//!
//! A data source is a set of facts with a fixed counting unit: *distinct voters
//! at the selected scope*, *Posts in scope*, *attempts, not people*. The unit
//! is what makes a figure mean the same thing on every dashboard, so it is code
//! and not configuration. Configuration picks a source, one of its templates
//! and that template's parameters; it never says how to count.
//!
//! A new data source is therefore a code change, reviewed like one.

use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumIter, EnumString};

/// Every data source the platform knows, whether or not its producer exists.
///
/// A source whose producer is owned by another team is listed anyway, so a
/// preset can place its widget today and the widget says "Not connected"
/// until the producer lands — never zero.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    EnumIter,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum DataSourceId {
    VoterTurnout,
    TestVoting,
    EnrollmentDecisions,
    VotingCredentials,
    PollStatus,
    FinalTestingLockdown,
    CountingTransmission,
    VotingEnrollmentActivity,
    AccessSecurity,
    AttackDetections,
    Helpdesk,
}

/// What one unit of a source's figures is.
///
/// The Admin Portal names this beside the data source ("Counts distinct
/// voters; counting rules are fixed by the data source"), so it is an enum the
/// portal can translate rather than a sentence.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display, EnumIter,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum CountingUnit {
    /// Each voter once at the selected scope, never summed across elections
    /// or groups.
    DistinctVoters,
    /// Each pre-enrolled voter once.
    DistinctPreEnrolledVoters,
    /// Each voter's latest enrollment decision.
    LatestDecisionPerVoter,
    /// Each approved voter once.
    ApprovedVoters,
    /// Each Post (election) in scope once.
    PostsInScope,
    /// Each voter's first valid vote or approval, in the bucket it happened.
    FirstEventPerVoter,
    /// Attempts, not people: two failed logins by one voter count twice.
    Attempts,
    Detections,
    ReportedIssues,
}

/// A number a source can report.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    EnumIter,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Measure {
    /// Voters in the census for the scope.
    Registered,
    PreEnrolled,
    CredentialsIssued,
    TestVoted,
    /// Voters with at least one valid vote. Revotes do not count again.
    Voted,
    Applications,
    Pending,
    Approved,
    Disapproved,
    Posts,
    Initialized,
    Opened,
    Paused,
    Closed,
    Tested,
    LockedDown,
    Tallied,
    Transmitted,
    TransmissionFailed,
    Logins,
    LoginFailures,
    PasswordResets,
    Detections,
    Issues,
    PendingIssues,
}

impl Measure {
    /// Shown for the measure unless a query relabels it.
    pub fn default_label(self) -> &'static str {
        match self {
            Measure::Registered => "Registered",
            Measure::PreEnrolled => "Pre-enrolled",
            Measure::CredentialsIssued => "Credentials issued",
            Measure::TestVoted => "Test voted",
            Measure::Voted => "Voted",
            Measure::Applications => "Applications",
            Measure::Pending => "Pending",
            Measure::Approved => "Approved",
            Measure::Disapproved => "Disapproved",
            Measure::Posts => "Posts",
            Measure::Initialized => "Initialized",
            Measure::Opened => "Opened",
            Measure::Paused => "Paused",
            Measure::Closed => "Closed",
            Measure::Tested => "Tested",
            Measure::LockedDown => "Locked down",
            Measure::Tallied => "Tallied",
            Measure::Transmitted => "Transmitted",
            Measure::TransmissionFailed => "Transmission failed",
            Measure::Logins => "Logins",
            Measure::LoginFailures => "Login failures",
            Measure::PasswordResets => "Password resets",
            Measure::Detections => "Detections",
            Measure::Issues => "Issues",
            Measure::PendingIssues => "Pending issues",
        }
    }
}

/// The shapes of result a source can return. Each fixes its output columns,
/// which is what lets a chart refer to them by name.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    EnumIter,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum QueryTemplate {
    /// One row of totals for the scope.
    Summary,
    /// One row per value of a dimension.
    ByGroup,
    /// One row per Post in scope, with its state.
    ByPost,
    /// One row per time bucket.
    Timeseries,
    /// One row per listed measure, for a pie of outcomes or a funnel.
    ByMeasure,
}

/// How wide a time bucket is. Buckets are `[start, end)` in the event's time
/// zone, so they sum to the totals.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    EnumIter,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum TimeGrain {
    Hour,
    Day,
}

/// A dimension every deployment has, because it comes from the platform's own
/// entities rather than from a voter attribute.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    EnumIter,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum BuiltinDimension {
    Region,
    Post,
    Country,
    /// Why an application was disapproved.
    Reason,
    /// Where a Post stands, for a source that counts Posts. Worked out from
    /// the Posts in scope, each in one state.
    State,
}

/// Whether a source's rows carry the voter dimensions a deployment configures
/// (sex, age band, status abroad — whatever its settings declare).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VoterDimensions {
    Configured,
    NotApplicable,
}

/// Where a source's facts come from, and whether that producer exists yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Producer {
    Available,
    Pending(PendingProducer),
}

/// A producer another part of the platform has still to deliver. The widget
/// names it in its "Not connected" state.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display, EnumIter,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum PendingProducer {
    /// Elections cannot yet be marked as test elections.
    TestElectionDesignation,
    /// Issuing credentials does not yet record an event.
    CredentialIssuedEvent,
    /// Final testing and lockdown have no recorded state per Post yet.
    FinalTestingLockdownState,
    /// No attack detection feed is connected.
    AttackDetectionFeed,
    /// No helpdesk system is connected.
    HelpdeskIntegration,
}

/// Where a Post stands in a Post-counting source.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    EnumIter,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum PostState {
    NotInitialized,
    Initialized,
    Opened,
    Paused,
    Closed,
    NotTested,
    /// Final testing passed; not yet locked down.
    Tested,
    LockedDown,
    NotTallied,
    Tallied,
    Transmitted,
    TransmissionFailed,
}

impl PostState {
    /// Whether a Post in this state counts towards `measure`: the one place
    /// a milestone is defined. A milestone stays reached as the Post moves
    /// on — a closed Post was opened — while a pause or a failed
    /// transmission counts only while it lasts.
    pub fn has_reached(self, measure: Measure) -> bool {
        use PostState::*;
        match measure {
            Measure::Posts => true,
            Measure::Initialized => {
                matches!(self, Initialized | Opened | Paused | Closed)
            }
            Measure::Opened => matches!(self, Opened | Paused | Closed),
            Measure::Paused => self == Paused,
            Measure::Closed => self == Closed,
            Measure::Tested => matches!(self, Tested | LockedDown),
            Measure::LockedDown => self == LockedDown,
            Measure::Tallied => {
                matches!(self, Tallied | Transmitted | TransmissionFailed)
            }
            Measure::Transmitted => self == Transmitted,
            Measure::TransmissionFailed => self == TransmissionFailed,
            _ => false,
        }
    }

    /// Shown unless a query relabels it.
    pub fn default_label(self) -> &'static str {
        match self {
            PostState::NotInitialized => "Not initialized",
            PostState::Initialized => "Initialized",
            PostState::Opened => "Opened",
            PostState::Paused => "Paused",
            PostState::Closed => "Closed",
            PostState::NotTested => "Not tested",
            PostState::Tested => "Tested",
            PostState::LockedDown => "Locked down",
            PostState::NotTallied => "Not tallied",
            PostState::Tallied => "Tallied",
            PostState::Transmitted => "Transmitted",
            PostState::TransmissionFailed => "Transmission failed",
        }
    }
}

/// Everything configuration may ask of a source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SourceSpec {
    pub id: DataSourceId,
    pub counting_unit: CountingUnit,
    pub measures: &'static [Measure],
    pub templates: &'static [QueryTemplate],
    pub builtin_dimensions: &'static [BuiltinDimension],
    pub voter_dimensions: VoterDimensions,
    /// The states a Post can be in, for a source that counts Posts.
    pub states: &'static [PostState],
    pub producer: Producer,
}

use BuiltinDimension as D;
use Measure as M;
use QueryTemplate as T;

const VOTER_TEMPLATES: &[QueryTemplate] =
    &[T::Summary, T::ByGroup, T::Timeseries, T::ByMeasure];
const POST_TEMPLATES: &[QueryTemplate] =
    &[T::Summary, T::ByGroup, T::ByPost, T::ByMeasure];
const SCOPE: &[BuiltinDimension] = &[D::Region, D::Post, D::Country];
const POST_SCOPE: &[BuiltinDimension] = &[D::Region, D::Post];
const POSTS_BY_STATE: &[BuiltinDimension] = &[D::Region, D::Post, D::State];

impl DataSourceId {
    /// The source's contract. The only place a counting rule is stated.
    pub const fn spec(self) -> SourceSpec {
        use DataSourceId::*;
        let states: &'static [PostState] = match self {
            PollStatus => &[
                PostState::NotInitialized,
                PostState::Initialized,
                PostState::Opened,
                PostState::Paused,
                PostState::Closed,
            ],
            FinalTestingLockdown => &[
                PostState::NotTested,
                PostState::Tested,
                PostState::LockedDown,
            ],
            CountingTransmission => &[
                PostState::NotTallied,
                PostState::Tallied,
                PostState::Transmitted,
                PostState::TransmissionFailed,
            ],
            _ => &[],
        };
        let (counting_unit, measures, templates, builtin, voter, producer): (
            CountingUnit,
            &'static [Measure],
            &'static [QueryTemplate],
            &'static [BuiltinDimension],
            VoterDimensions,
            Producer,
        ) = match self {
            VoterTurnout => (
                CountingUnit::DistinctVoters,
                &[M::Registered, M::PreEnrolled, M::Voted],
                VOTER_TEMPLATES,
                SCOPE,
                VoterDimensions::Configured,
                Producer::Available,
            ),
            TestVoting => (
                CountingUnit::DistinctPreEnrolledVoters,
                &[M::PreEnrolled, M::TestVoted],
                VOTER_TEMPLATES,
                SCOPE,
                VoterDimensions::Configured,
                Producer::Pending(PendingProducer::TestElectionDesignation),
            ),
            EnrollmentDecisions => (
                CountingUnit::LatestDecisionPerVoter,
                &[M::Applications, M::Pending, M::Approved, M::Disapproved],
                VOTER_TEMPLATES,
                &[D::Region, D::Post, D::Country, D::Reason],
                VoterDimensions::NotApplicable,
                Producer::Available,
            ),
            VotingCredentials => (
                CountingUnit::ApprovedVoters,
                &[M::Approved, M::CredentialsIssued],
                VOTER_TEMPLATES,
                SCOPE,
                VoterDimensions::Configured,
                Producer::Pending(PendingProducer::CredentialIssuedEvent),
            ),
            PollStatus => (
                CountingUnit::PostsInScope,
                &[M::Posts, M::Initialized, M::Opened, M::Paused, M::Closed],
                POST_TEMPLATES,
                POSTS_BY_STATE,
                VoterDimensions::NotApplicable,
                Producer::Available,
            ),
            FinalTestingLockdown => (
                CountingUnit::PostsInScope,
                &[M::Posts, M::Tested, M::LockedDown],
                POST_TEMPLATES,
                POSTS_BY_STATE,
                VoterDimensions::NotApplicable,
                Producer::Pending(PendingProducer::FinalTestingLockdownState),
            ),
            CountingTransmission => (
                CountingUnit::PostsInScope,
                &[M::Posts, M::Tallied, M::Transmitted, M::TransmissionFailed],
                POST_TEMPLATES,
                POSTS_BY_STATE,
                VoterDimensions::NotApplicable,
                Producer::Available,
            ),
            VotingEnrollmentActivity => (
                CountingUnit::FirstEventPerVoter,
                &[M::Approved, M::Voted],
                &[T::Summary, T::Timeseries, T::ByMeasure],
                SCOPE,
                VoterDimensions::NotApplicable,
                Producer::Available,
            ),
            AccessSecurity => (
                CountingUnit::Attempts,
                &[M::Logins, M::LoginFailures, M::PasswordResets],
                &[T::Summary, T::ByGroup, T::Timeseries, T::ByMeasure],
                POST_SCOPE,
                VoterDimensions::NotApplicable,
                Producer::Available,
            ),
            AttackDetections => (
                CountingUnit::Detections,
                &[M::Detections],
                &[T::Summary, T::Timeseries, T::ByMeasure],
                &[],
                VoterDimensions::NotApplicable,
                Producer::Pending(PendingProducer::AttackDetectionFeed),
            ),
            Helpdesk => (
                CountingUnit::ReportedIssues,
                &[M::Issues, M::PendingIssues],
                &[T::Summary, T::ByGroup, T::Timeseries, T::ByMeasure],
                POST_SCOPE,
                VoterDimensions::NotApplicable,
                Producer::Pending(PendingProducer::HelpdeskIntegration),
            ),
        };
        SourceSpec {
            id: self,
            counting_unit,
            measures,
            templates,
            builtin_dimensions: builtin,
            voter_dimensions: voter,
            states,
            producer,
        }
    }
}

impl SourceSpec {
    pub fn has_measure(&self, measure: Measure) -> bool {
        self.measures.contains(&measure)
    }

    pub fn has_template(&self, template: QueryTemplate) -> bool {
        self.templates.contains(&template)
    }

    /// Whether `group_by: <name>` is a dimension of this source. Voter
    /// dimensions are only known once the deployment's settings are, so a
    /// name that is not built in is accepted here when the source carries
    /// voter dimensions, and checked against the settings by
    /// [`crate::monitoring::policy::validate_set`].
    pub fn may_group_by(&self, name: &str) -> bool {
        match name.parse::<BuiltinDimension>() {
            Ok(builtin) => self.builtin_dimensions.contains(&builtin),
            Err(_) => self.voter_dimensions == VoterDimensions::Configured,
        }
    }
}

#[cfg(test)]
#[path = "sources_tests.rs"]
mod sources_tests;
