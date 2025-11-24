// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use uuid::Uuid;

use crate::pipes::pipe_inputs::AreaConfig;

#[allow(unused)]
pub fn get_area_config(
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    election_id: &Uuid,
    census: u64,
    auditable_votes: u64,
    parent_id: Option<Uuid>,
    area_id: Option<String>,
) -> AreaConfig {
    let area_uuid = area_id
        .map(|val| Uuid::parse_str(&val).unwrap())
        .unwrap_or(Uuid::new_v4());
    AreaConfig {
        id: area_uuid,
        name: "".into(),
        tenant_id: *tenant_id,
        election_event_id: *election_event_id,
        election_id: *election_id,
        census,
        auditable_votes,
        parent_id,
    }
}
