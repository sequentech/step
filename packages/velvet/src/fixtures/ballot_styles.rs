// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use sequent_core::ballot::{BallotStyle, Contest, PublicKeyConfig};
use uuid::Uuid;

use super::contests;

#[allow(unused)]
pub fn get_ballot_style_1(
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    election_id: &Uuid,
    area_id: &Uuid,
) -> BallotStyle {
    BallotStyle {
        id: Uuid::new_v4().to_string(),
        tenant_id: tenant_id.to_string(),
        election_event_id: election_event_id.to_string(),
        election_id: election_id.to_string(),
        num_allowed_revotes: Some(1),
        description: Some("Write-ins simple".into()),
        public_key: Some(PublicKeyConfig {
            public_key: "ajR/I9RqyOwbpsVRucSNOgXVLCvLpfQxCgPoXGQ2RF4".into(),
            is_demo: false,
        }),
        area_id: area_id.to_string(),
        area_presentation: None,
        contests: vec![contests::get_contest_1(
            tenant_id,
            election_event_id,
            election_id,
        )],
        election_event_annotations: Default::default(),
        election_annotations: Default::default(),
        election_event_presentation: None,
        election_presentation: None,
        election_dates: None,
        area_annotations: None,
    }
}

#[allow(unused)]
pub fn generate_ballot_style(
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    election_id: &Uuid,
    area_id: &Uuid,
    contests: Vec<Contest>,
) -> BallotStyle {
    BallotStyle {
        id: Uuid::new_v4().to_string(),
        tenant_id: tenant_id.to_string(),
        election_event_id: election_event_id.to_string(),
        election_id: election_id.to_string(),
        num_allowed_revotes: Some(1),
        description: Some("Write-ins simple".into()),
        public_key: Some(PublicKeyConfig {
            public_key: "ajR/I9RqyOwbpsVRucSNOgXVLCvLpfQxCgPoXGQ2RF4".into(),
            is_demo: false,
        }),
        area_id: area_id.to_string(),
        area_presentation: None,
        contests,
        election_event_presentation: None,
        election_presentation: None,
        election_dates: None,
        election_event_annotations: Default::default(),
        election_annotations: Default::default(),
        area_annotations: None,
    }
}
