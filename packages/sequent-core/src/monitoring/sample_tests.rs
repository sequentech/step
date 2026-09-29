// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::monitoring::config::Settings;
use crate::monitoring::payload::UNKNOWN_KEY;
use crate::monitoring::sources::{DataSourceId, Measure};
use std::collections::{BTreeMap, BTreeSet};
use strum::IntoEnumIterator;

fn settings() -> Settings {
    serde_yaml::from_str(include_str!("fixtures/settings.yaml"))
        .expect("fixture settings")
}

#[test]
fn a_voter_cube_keeps_every_funnel_stage_below_the_one_before() {
    let payload = sample_payload(DataSourceId::VoterTurnout, Some(&settings()));
    let cube = payload.cube.expect("voter turnout has a cube");
    for cell in &cube.cells {
        let registered = cell.counts[&Measure::Registered];
        let pre_enrolled = cell.counts[&Measure::PreEnrolled];
        let voted = cell.counts[&Measure::Voted];
        assert!(
            voted <= pre_enrolled && pre_enrolled <= registered,
            "{cell:?}"
        );
    }
}

/// Groups that all share one ratio would hide a chart that mixes up
/// numerators and denominators.
#[test]
fn groups_of_voters_turn_out_differently() {
    let payload = sample_payload(DataSourceId::VoterTurnout, Some(&settings()));
    let cube = payload.cube.expect("voter turnout has a cube");
    for (at, dimension) in cube.dimensions.iter().enumerate() {
        let mut groups: BTreeMap<&str, (u64, u64)> = BTreeMap::new();
        for cell in &cube.cells {
            let group = groups.entry(cell.values[at].as_str()).or_default();
            group.0 += cell.counts[&Measure::Voted];
            group.1 += cell.counts[&Measure::Registered];
        }
        let ratios: BTreeSet<u64> = groups
            .values()
            .map(|(voted, registered)| voted * 1000 / registered)
            .collect();
        assert!(ratios.len() > 1, "{dimension}: {groups:?}");
    }
}

#[test]
fn the_totals_of_a_voter_source_are_the_sum_of_its_cube() {
    let payload = sample_payload(DataSourceId::VoterTurnout, Some(&settings()));
    let cube = payload.cube.as_ref().expect("voter turnout has a cube");
    for (measure, total) in &payload.totals {
        let summed: u64 =
            cube.cells.iter().map(|cell| cell.counts[measure]).sum();
        assert_eq!(summed, *total, "{measure}");
    }
}

/// A Post source counts Posts, so its groups by region and by Post are its
/// Posts counted there, never a share of a handful rounded to nothing.
#[test]
fn a_post_source_s_groups_count_its_posts() {
    for source in [
        DataSourceId::PollStatus,
        DataSourceId::CountingTransmission,
        DataSourceId::FinalTestingLockdown,
    ] {
        let payload = sample_payload(source, None);
        for dimension in ["region", "post"] {
            let groups = &payload.groups[dimension];
            let summed: u64 = groups
                .iter()
                .map(|group| group.counts[&Measure::Posts])
                .sum();
            assert_eq!(
                summed,
                payload.totals[&Measure::Posts],
                "{source} {dimension}"
            );
            assert!(
                groups.iter().all(|group| group.counts[&Measure::Posts] > 0),
                "{source} {dimension}: {groups:?}"
            );
        }
        let regions: Vec<&str> = payload.groups["region"]
            .iter()
            .map(|group| group.key.as_str())
            .collect();
        assert!(regions.contains(&UNKNOWN_KEY), "{regions:?}");
        let names: Vec<Option<&str>> = payload.groups["post"]
            .iter()
            .map(|group| group.label.as_deref())
            .collect();
        assert!(names.contains(&Some("Madrid")), "{names:?}");
    }
}

/// Every source reads the same electorate, so the figures on one dashboard
/// agree: the voted total is the running total of voting activity.
#[test]
fn every_source_counts_the_same_voters() {
    let settings = settings();
    let turnout = sample_payload(DataSourceId::VoterTurnout, Some(&settings));
    let activity =
        sample_payload(DataSourceId::VotingEnrollmentActivity, Some(&settings));
    let credentials =
        sample_payload(DataSourceId::VotingCredentials, Some(&settings));
    let decisions =
        sample_payload(DataSourceId::EnrollmentDecisions, Some(&settings));
    assert_eq!(
        activity.totals[&Measure::Voted],
        turnout.totals[&Measure::Voted]
    );
    assert_eq!(
        credentials.totals[&Measure::Approved],
        decisions.totals[&Measure::Approved]
    );
    assert_eq!(
        decisions.totals[&Measure::Approved],
        turnout.totals[&Measure::PreEnrolled]
    );
}

#[test]
fn every_series_adds_up_to_the_totals() {
    for source in DataSourceId::iter() {
        let payload = sample_payload(source, Some(&settings()));
        if payload.series.is_empty() {
            continue;
        }
        for (measure, total) in &payload.totals {
            let summed: u64 = payload
                .series
                .iter()
                .map(|bucket| bucket.counts[measure])
                .sum();
            assert_eq!(summed, *total, "{source} {measure}");
        }
    }
}

/// Each voter has one latest decision, and only a disapproval has a reason.
#[test]
fn decisions_add_up_to_the_applications() {
    let payload = sample_payload(DataSourceId::EnrollmentDecisions, None);
    let totals = &payload.totals;
    assert_eq!(
        totals[&Measure::Applications],
        totals[&Measure::Approved]
            + totals[&Measure::Disapproved]
            + totals[&Measure::Pending]
    );
    let reasons = &payload.groups["reason"];
    assert!(reasons.len() > 10, "a top ten leaves some out");
    assert!(reasons.iter().all(|reason| reason.counts.len() == 1));
    let summed: u64 = reasons
        .iter()
        .map(|reason| reason.counts[&Measure::Disapproved])
        .sum();
    assert_eq!(summed, totals[&Measure::Disapproved]);
}

/// Built-in groups split every measure exactly, keep the funnel in order,
/// and turn out differently.
#[test]
fn built_in_groups_split_the_totals_and_turn_out_differently() {
    let payload = sample_payload(DataSourceId::VoterTurnout, Some(&settings()));
    for dimension in ["region", "post", "country"] {
        let groups = &payload.groups[dimension];
        for (measure, total) in &payload.totals {
            let summed: u64 =
                groups.iter().map(|group| group.counts[measure]).sum();
            assert_eq!(summed, *total, "{dimension} {measure}");
        }
        for group in groups {
            let counts = &group.counts;
            assert!(
                counts[&Measure::Voted] <= counts[&Measure::PreEnrolled]
                    && counts[&Measure::PreEnrolled]
                        <= counts[&Measure::Registered],
                "{dimension}: {group:?}"
            );
        }
        let ratios: BTreeSet<u64> = groups
            .iter()
            .map(|group| {
                group.counts[&Measure::Voted] * 1000
                    / group.counts[&Measure::Registered]
            })
            .collect();
        assert!(ratios.len() > 2, "{dimension}: {ratios:?}");
    }
    assert!(
        payload.groups["post"].len() > 10,
        "a top ten leaves some out"
    );
}

#[test]
fn posts_stand_at_every_state() {
    for source in DataSourceId::iter() {
        let spec = source.spec();
        if spec.states.is_empty() {
            continue;
        }
        let payload = sample_payload(source, None);
        for state in spec.states {
            assert!(
                payload.posts.iter().any(|post| post.state == Some(*state)),
                "{source}: {state}"
            );
        }
    }
}
