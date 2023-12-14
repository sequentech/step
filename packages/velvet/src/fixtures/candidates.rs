use sequent_core::ballot::{
    BallotStyle, Candidate, CandidatePresentation, Contest, ContestPresentation, PublicKeyConfig,
};
use uuid::Uuid;

pub fn get_candidate_0(
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    election_id: &Uuid,
    contest_id: &Uuid,
) -> Candidate {
    Candidate {
        id: "0".to_string(),
        tenant_id: tenant_id.to_string(),
        election_event_id: election_event_id.to_string(),
        election_id: election_id.to_string(),
        contest_id: contest_id.to_string(),
        name: Some("José Rabano Pimiento".into()),
        description: None,
        candidate_type: None,
        presentation: Some(CandidatePresentation {
            is_explicit_invalid: false,
            is_write_in: false,
            sort_order: Some(0),
            urls: None,
            invalid_vote_position: None,
            is_category_list: false,
        }),
    }
}

pub fn get_candidate_1(
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    election_id: &Uuid,
    contest_id: &Uuid,
) -> Candidate {
    Candidate {
        id: "1".to_string(),
        tenant_id: tenant_id.to_string(),
        election_event_id: election_event_id.to_string(),
        election_id: election_id.to_string(),
        contest_id: contest_id.to_string(),
        name: Some("Miguel Pimentel Inventado".into()),
        description: None,
        candidate_type: None,
        presentation: Some(CandidatePresentation {
            is_explicit_invalid: false,
            is_write_in: false,
            sort_order: Some(1),
            urls: None,
            invalid_vote_position: None,
            is_category_list: false,
        }),
    }
}

pub fn get_candidate_2(
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    election_id: &Uuid,
    contest_id: &Uuid,
) -> Candidate {
    Candidate {
        id: "2".to_string(),
        tenant_id: tenant_id.to_string(),
        election_event_id: election_event_id.to_string(),
        election_id: election_id.to_string(),
        contest_id: contest_id.to_string(),
        name: Some("Juan Iglesias Torquemada".into()),
        description: None,
        candidate_type: None,
        presentation: Some(CandidatePresentation {
            is_explicit_invalid: false,
            is_write_in: false,
            sort_order: Some(2),
            urls: None,
            invalid_vote_position: None,
            is_category_list: false,
        }),
    }
}

pub fn get_candidate_3(
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    election_id: &Uuid,
    contest_id: &Uuid,
) -> Candidate {
    Candidate {
        id: "3".to_string(),
        tenant_id: tenant_id.to_string(),
        election_event_id: election_event_id.to_string(),
        election_id: election_id.to_string(),
        contest_id: contest_id.to_string(),
        name: Some("Mari Pili Hernández Ordoñez".into()),
        description: None,
        candidate_type: None,
        presentation: Some(CandidatePresentation {
            is_explicit_invalid: false,
            is_write_in: false,
            sort_order: Some(3),
            urls: None,
            invalid_vote_position: None,
            is_category_list: false,
        }),
    }
}

pub fn get_candidate_4(
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    election_id: &Uuid,
    contest_id: &Uuid,
) -> Candidate {
    Candidate {
        id: "4".to_string(),
        tenant_id: tenant_id.to_string(),
        election_event_id: election_event_id.to_string(),
        election_id: election_id.to_string(),
        contest_id: contest_id.to_string(),
        name: Some("Juan Y Medio".into()),
        description: None,
        candidate_type: None,
        presentation: Some(CandidatePresentation {
            is_explicit_invalid: false,
            is_write_in: false,
            sort_order: Some(4),
            urls: None,
            invalid_vote_position: None,
            is_category_list: false,
        }),
    }
}

pub fn get_candidate_5(
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    election_id: &Uuid,
    contest_id: &Uuid,
) -> Candidate {
    Candidate {
        id: "5".to_string(),
        tenant_id: tenant_id.to_string(),
        election_event_id: election_event_id.to_string(),
        election_id: election_id.to_string(),
        contest_id: contest_id.to_string(),
        name: Some("Spiderman".into()),
        description: None,
        candidate_type: None,
        presentation: Some(CandidatePresentation {
            is_explicit_invalid: false,
            is_write_in: false,
            sort_order: Some(5),
            urls: None,
            invalid_vote_position: None,
            is_category_list: false,
        }),
    }
}
