// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::ballot_styles;
use crate::pipes::pipe_inputs::ElectionConfig;
use sequent_core::services::area_tree::TreeNodeArea;
use uuid::Uuid;

#[allow(unused)]
pub fn get_election_config_1(election_event_id: &Uuid, areas: Vec<Uuid>) -> ElectionConfig {
    let tenant_id = Uuid::new_v4();
    let election_id = Uuid::new_v4();

    let first_area_id = areas.first().cloned().unwrap();
    let ballot_style = ballot_styles::get_ballot_style_1(
        &tenant_id,
        election_event_id,
        &election_id,
        &first_area_id,
    );

    ElectionConfig {
        id: election_id,
        name: "Election 1".to_string(),
        alias: "Election 1 alias".to_string(),
        description: "".to_string(),
        dates: None,
        annotations: Default::default(),
        election_event_annotations: Default::default(),
        tenant_id,
        election_event_id: *election_event_id,
        census: 0,
        total_votes: 0,
        ballot_styles: vec![ballot_style],
        areas: areas
            .iter()
            .map(|area| TreeNodeArea {
                id: area.to_string(),
                tenant_id: tenant_id.to_string(),
                annotations: Default::default(),
                election_event_id: election_event_id.to_string(),
                parent_id: None,
            })
            .collect(),
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
        alias: "Election 2 alias".to_string(),
        description: "".to_string(),
        annotations: Default::default(),
        election_event_annotations: Default::default(),
        dates: None,
        tenant_id,
        election_event_id,
        census: 0,
        total_votes: 0,
        ballot_styles: vec![ballot_style1, ballot_style2],
        areas: vec![TreeNodeArea {
            id: area_id.to_string(),
            tenant_id: tenant_id.to_string(),
            annotations: Default::default(),
            election_event_id: election_event_id.to_string(),
            parent_id: None,
        }],
    }
}

#[allow(unused)]
pub fn get_election_config_3(
    election_event_id: &Uuid,
    areas: Vec<(Uuid, Option<Uuid>)>,
) -> ElectionConfig {
    let tenant_id = Uuid::new_v4();
    let election_id = Uuid::new_v4();

    let first_area_id = areas.first().cloned().unwrap().0;
    let ballot_style = ballot_styles::get_ballot_style_1(
        &tenant_id,
        election_event_id,
        &election_id,
        &first_area_id,
    );

    ElectionConfig {
        id: election_id,
        name: "Election 3".to_string(),
        alias: "Election 3 alias".to_string(),
        description: "".to_string(),
        annotations: Default::default(),
        election_event_annotations: Default::default(),
        dates: None,
        tenant_id,
        election_event_id: *election_event_id,
        census: 0,
        total_votes: 0,
        ballot_styles: vec![ballot_style],
        areas: areas
            .iter()
            .map(|(area_id, parent_area_id)| TreeNodeArea {
                id: area_id.to_string(),
                tenant_id: tenant_id.to_string(),
                annotations: Default::default(),
                election_event_id: election_event_id.to_string(),
                parent_id: parent_area_id.map(|a| a.to_string()),
            })
            .collect(),
    }
}
