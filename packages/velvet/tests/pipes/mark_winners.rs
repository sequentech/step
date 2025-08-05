
use velvet::pipes::mark_winners::*;
use velvet::pipes::do_tally::{CandidateResult, ContestResult, InvalidVotes};
use sequent_core::ballot::{Candidate, Contest};

#[test]
fn test_get_winners() {
    // note, all these are v4 uuids
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
        name_i18n: None,
        description: None,
        description_i18n: None,
        alias: None,
        alias_i18n: None,
        candidate_type: None,
        presentation: None,
        annotations: None,
    };
    let contest_result = ContestResult {
        contest: Contest {
            id: contest_id.clone(),
            tenant_id: tenant_id.clone(),
            election_event_id: election_event_id.clone(),
            election_id: election_id.clone(),
            name: Some("Event".to_string()),
            name_i18n: None,
            description: None,
            description_i18n: None,
            alias: None,
            alias_i18n: None,
            max_votes: 1,
            min_votes: 0,
            winning_candidates_num: 1,
            voting_type: Some("non-preferential".to_string()),
            counting_algorithm: Some("plurality-at-large".to_string()), /* plurality-at-large|borda-nauru|borda|borda-mas-madrid|desborda3|desborda2|desborda|cumulative */
            is_encrypted: true,
            candidates: vec![candidate_a.clone()],//Vec<Candidate>,
            presentation: None,
            created_at: None,
            annotations: None,
        },
        census: 1,
        percentage_census: 1.0,
        auditable_votes: 0,
        percentage_auditable_votes: 0.0,
        total_votes: 1,
        percentage_total_votes: 1.0,
        total_valid_votes: 1,
        percentage_total_valid_votes: 1.0,
        total_invalid_votes: 0,
        percentage_total_invalid_votes: 0.0,
        total_blank_votes: 0,
        percentage_total_blank_votes: 0.0,
        invalid_votes: InvalidVotes {
            explicit: 0,
            implicit: 0,
        },
        percentage_invalid_votes_explicit: 0.0,
        percentage_invalid_votes_implicit: 0.0,
        candidate_result: vec![
            CandidateResult {
                candidate: candidate_a.clone(),
                percentage_votes: 1.0,
                total_count: 1,
            }
        ],
        extended_metrics: None,
    };

    let winners = MarkWinners::get_winners(&contest_result);
    assert_eq!(winners.len(), 1);
    let winner_a = winners.get(0).unwrap();
    assert_eq!(winner_a.candidate.id, candidate_a.id);
    assert_eq!(winner_a.total_count, 1);
    assert_eq!(winner_a.winning_position, 1);
}