// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::monitoring::config::Settings;
use crate::monitoring::payload::UNKNOWN_KEY;
use crate::monitoring::sources::{DataSourceId, Measure};
use std::collections::{BTreeMap, BTreeSet};

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
