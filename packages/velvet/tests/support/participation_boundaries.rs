// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Participation counters must fail past the largest exact count instead of
//! wrapping, or being rounded, to a total that could appear valid in an
//! election report.
use super::*;

#[test]
fn participation_checks_both_additions_at_the_largest_exact_count() {
    let mut result = ContestResult {
        total_votes: MAX_COUNT - 2,
        auditable_votes: 1,
        extended_metrics: Some(ExtendedMetricsContest {
            total_declined_to_vote: 1,
            ..Default::default()
        }),
        ..Default::default()
    };
    assert_eq!(participation_total(&result).unwrap(), MAX_COUNT);
    result.total_votes = MAX_COUNT;
    assert!(
        participation_total(&result).is_err(),
        "auditable ballots must use checked addition"
    );
    result.auditable_votes = 0;
    assert!(
        participation_total(&result).is_err(),
        "declined ballots must use checked addition too"
    );
}

#[test]
fn merging_channel_counts_stop_at_the_largest_exact_count_and_absent_metrics_add_nothing() {
    let channel = sequent_core::types::participation::ParticipationChannel::from(
        sequent_core::types::tally_sheets::VotingChannel::PAPER,
    );
    let mut aggregate = VotesByChannel::from([(channel.clone(), MAX_COUNT - 1)]);
    let one = VotesByChannel::from([(channel.clone(), 1)]);
    merge_votes_by_channel(&mut aggregate, &one).unwrap();
    assert_eq!(aggregate[&channel], MAX_COUNT);
    assert!(merge_votes_by_channel(&mut aggregate, &one).is_err());
    assert_eq!(
        aggregate[&channel], MAX_COUNT,
        "overflow must not replace the counter with zero"
    );
    merge_result_votes_by_channel(&mut aggregate, &ContestResult::default()).unwrap();
    assert_eq!(aggregate.len(), 1);
    assert_eq!(aggregate[&channel], MAX_COUNT);
}
