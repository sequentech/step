// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
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
    parent_id: Option<Uuid>,
) -> AreaConfig {
    AreaConfig {
        id: Uuid::new_v4(),
        name: "".into(),
        tenant_id: *tenant_id,
        election_event_id: *election_event_id,
        election_id: *election_id,
        census,
        parent_id,
    }
}
