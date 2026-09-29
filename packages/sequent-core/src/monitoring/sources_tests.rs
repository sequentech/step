// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use strum::IntoEnumIterator;

#[test]
fn only_sources_whose_facts_the_platform_records_are_connected() {
    use DataSourceId::*;
    let producer = |source: DataSourceId| source.spec().producer;
    for source in [
        VoterTurnout,
        EnrollmentDecisions,
        PollStatus,
        CountingTransmission,
        VotingEnrollmentActivity,
        AccessSecurity,
    ] {
        assert_eq!(producer(source), Producer::Available, "{source}");
    }
    let pending = [
        (TestVoting, PendingProducer::TestElectionDesignation),
        (VotingCredentials, PendingProducer::CredentialIssuedEvent),
        (
            FinalTestingLockdown,
            PendingProducer::FinalTestingLockdownState,
        ),
        (AttackDetections, PendingProducer::AttackDetectionFeed),
        (Helpdesk, PendingProducer::HelpdeskIntegration),
    ];
    for (source, reason) in pending {
        assert_eq!(producer(source), Producer::Pending(reason), "{source}");
    }
    assert_eq!(DataSourceId::iter().count(), 11);
}

/// "First valid vote or approval per voter": credentials are issued by a
/// producer that does not exist yet, so the activity source must not offer
/// them — a connected source would show a count it never made.
#[test]
fn activity_counts_first_votes_and_approvals_only() {
    let spec = DataSourceId::VotingEnrollmentActivity.spec();
    assert_eq!(spec.counting_unit, CountingUnit::FirstEventPerVoter);
    assert_eq!(spec.measures, [Measure::Approved, Measure::Voted]);
}

#[test]
fn a_connected_source_offers_no_measure_only_a_pending_producer_counts() {
    let pending_only = [Measure::CredentialsIssued, Measure::TestVoted];
    for source in DataSourceId::iter() {
        let spec = source.spec();
        if spec.producer != Producer::Available {
            continue;
        }
        for measure in pending_only {
            assert!(!spec.has_measure(measure), "{source} offers {measure}");
        }
    }
}

#[test]
fn every_source_can_be_queried_and_post_sources_say_how_posts_stand() {
    for source in DataSourceId::iter() {
        let spec = source.spec();
        assert!(!spec.measures.is_empty(), "{source}");
        assert!(spec.has_template(QueryTemplate::Summary), "{source}");
        let counts_posts = spec.counting_unit == CountingUnit::PostsInScope;
        assert_eq!(
            spec.has_template(QueryTemplate::ByPost),
            counts_posts,
            "{source}"
        );
        assert_eq!(!spec.states.is_empty(), counts_posts, "{source}");
    }
}

#[test]
fn a_milestone_stays_reached_but_a_pause_or_failure_only_lasts() {
    use Measure as M;
    use PostState as S;
    assert!(S::Closed.has_reached(M::Opened));
    assert!(S::Closed.has_reached(M::Initialized));
    assert!(!S::Closed.has_reached(M::Paused));
    assert!(S::Paused.has_reached(M::Opened));
    assert!(!S::NotInitialized.has_reached(M::Initialized));
    assert!(S::TransmissionFailed.has_reached(M::Tallied));
    assert!(!S::TransmissionFailed.has_reached(M::Transmitted));
    assert!(S::LockedDown.has_reached(M::Tested));
    // Every Post-counting source: each state is a Post, and each milestone
    // measure is reached by at least one of its states.
    for source in DataSourceId::iter() {
        let spec = source.spec();
        if spec.states.is_empty() {
            continue;
        }
        for measure in spec.measures {
            assert!(
                spec.states.iter().any(|state| state.has_reached(*measure)),
                "{source}: no state reaches {measure}"
            );
        }
    }
}

/// Helpdesk issues and attack detections come in categories, which a
/// widget breaks them down by.
#[test]
fn issues_and_detections_can_be_broken_down_by_category() {
    for source in [DataSourceId::Helpdesk, DataSourceId::AttackDetections] {
        assert!(source.spec().may_group_by("category"), "{source}");
        assert!(
            source.spec().has_template(QueryTemplate::ByGroup),
            "{source}"
        );
    }
    assert!(!DataSourceId::VoterTurnout.spec().may_group_by("category"));
}
