use sequent_core::ballot::{Candidate, CandidatePresentation, Contest};
use velvet::pipes::do_tally::{CandidateResult, ContestResult, InvalidVotes};
use velvet::pipes::mark_winners::*;

#[test]
fn test_get_winners() {
    let tenant_id = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5".to_string();
    let election_event_id = "9f48606b-3159-4c6a-9c3f-f7f49badfd8b".to_string();
    let election_id = "09af46e9-a40e-4ad5-a209-90654d3aecc2".to_string();
    let contest_id = "1dea377d-4ec0-4436-aa2c-0c8f5ec7a5be".to_string();

    let candidate_a = Candidate {
        id: "ca39ad00-2927-4279-a0fc-1d9010900b76".to_string(),
        tenant_id: tenant_id.clone(),
        election_event_id: election_event_id.clone(),
        election_id: election_id.clone(),
        contest_id: contest_id.clone(),
        name: Some("A".to_string()),
        ..Default::default()
    };

    let candidate_b = Candidate {
        id: "bb39ad00-2927-4279-a0fc-1d9010900b77".to_string(),
        tenant_id: tenant_id.clone(),
        election_event_id: election_event_id.clone(),
        election_id: election_id.clone(),
        contest_id: contest_id.clone(),
        name: Some("B".to_string()),
        ..Default::default()
    };

    let candidate_invalid = Candidate {
        id: "cc39ad00-2927-4279-a0fc-1d9010900b78".to_string(),
        tenant_id: tenant_id.clone(),
        election_event_id: election_event_id.clone(),
        election_id: election_id.clone(),
        contest_id: contest_id.clone(),
        name: Some("Invalid".to_string()),
        presentation: Some(CandidatePresentation {
            is_explicit_invalid: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };

    let candidate_blank = Candidate {
        id: "dd39ad00-2927-4279-a0fc-1d9010900b79".to_string(),
        tenant_id: tenant_id.clone(),
        election_event_id: election_event_id.clone(),
        election_id: election_id.clone(),
        contest_id: contest_id.clone(),
        name: Some("Blank".to_string()),
        presentation: Some(CandidatePresentation {
            is_explicit_blank: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };

    let contest_result = ContestResult {
        contest: Contest {
            id: contest_id.clone(),
            tenant_id: tenant_id.clone(),
            election_event_id: election_event_id.clone(),
            election_id: election_id.clone(),
            name: Some("Event".to_string()),
            winning_candidates_num: 2,
            candidates: vec![
                candidate_a.clone(),
                candidate_b.clone(),
                candidate_invalid.clone(),
                candidate_blank.clone(),
            ],
            ..Default::default()
        },
        total_votes: 20,
        total_valid_votes: 6,
        total_invalid_votes: 8,
        total_blank_votes: 6,
        invalid_votes: InvalidVotes {
            explicit: 8,
            implicit: 0,
        },
        candidate_result: vec![
            CandidateResult {
                candidate: candidate_a.clone(),
                percentage_votes: 0.10,
                total_count: 2,
            },
            CandidateResult {
                candidate: candidate_b.clone(),
                percentage_votes: 0.20,
                total_count: 4,
            },
            CandidateResult {
                candidate: candidate_invalid.clone(),
                percentage_votes: 0.40,
                total_count: 8,
            },
            CandidateResult {
                candidate: candidate_blank.clone(),
                percentage_votes: 0.30,
                total_count: 6,
            },
        ],
        ..Default::default()
    };

    let expected_winners = vec![
        WinnerResult {
            candidate: candidate_b.clone(),
            total_count: 4,
            winning_position: 1,
        },
        WinnerResult {
            candidate: candidate_a.clone(),
            total_count: 2,
            winning_position: 2,
        },
    ];

    let winners = MarkWinners::get_winners(&contest_result);
    assert_eq!(winners, expected_winners);
}
