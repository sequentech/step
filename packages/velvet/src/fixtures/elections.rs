// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::ballot_styles;
use crate::pipes::pipe_inputs::ElectionConfig;
use uuid::Uuid;

#[allow(unused)]
pub fn get_election_config_1(election_event_id: &Uuid) -> ElectionConfig {
    let tenant_id = Uuid::new_v4();
    let election_id = Uuid::new_v4();

    let area_id = Uuid::new_v4();
    let ballot_style =
        ballot_styles::get_ballot_style_1(&tenant_id, election_event_id, &election_id, &area_id);

    ElectionConfig {
        id: election_id,
        name: "Election 1".to_string(),
        tenant_id,
        election_event_id: *election_event_id,
        census: 0,
        total_votes: 0,
        ballot_styles: vec![ballot_style],
    }
}

#[allow(unused)]
pub fn get_election_config_2() -> ElectionConfig {
    let tenant_id = Uuid::new_v4();
    let election_event_id = Uuid::new_v4();
    let election_id = Uuid::new_v4();

    let area_id = Uuid::new_v4();
    let ballot_style1 =
        ballot_styles::get_ballot_style_1(&tenant_id, &election_event_id, &election_id, &area_id);

    let area_id = Uuid::new_v4();
    let ballot_style2 =
        ballot_styles::get_ballot_style_1(&tenant_id, &election_event_id, &election_id, &area_id);

    ElectionConfig {
        id: election_id,
        name: "Election 2".to_string(),
        tenant_id,
        election_event_id,
        census: 0,
        total_votes: 0,
        ballot_styles: vec![ballot_style1, ballot_style2],
    }
}
