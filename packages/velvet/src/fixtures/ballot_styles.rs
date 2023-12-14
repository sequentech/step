use sequent_core::ballot::{
    BallotStyle, Candidate, CandidatePresentation, Contest, ContestPresentation, PublicKeyConfig,
};
use uuid::Uuid;

use super::contests;

pub fn get_ballot_style_1(
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    area_id: &str,
) -> BallotStyle {
    BallotStyle {
        id: Uuid::new_v4().to_string(),
        tenant_id: tenant_id.to_string(),
        election_event_id: election_event_id.to_string(),
        election_id: election_id.to_string(),
        description: Some("Write-ins simple".into()),
        public_key: Some(PublicKeyConfig {
            public_key: "ajR/I9RqyOwbpsVRucSNOgXVLCvLpfQxCgPoXGQ2RF4".into(),
            is_demo: false,
        }),
        area_id: area_id.to_string(),
        contests: vec![contests::get_contest_1(
            &tenant_id,
            &election_event_id,
            &election_id,
        )],
    }
}
