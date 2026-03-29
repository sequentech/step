// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use uuid::Uuid;

use crate::pipes::pipe_inputs::AreaConfig;

#[allow(unused)]
/// Generate an `AreaConfig` for test fixtures.
///
/// # Arguments
/// * `tenant_id` - Tenant UUID
/// * `election_event_id` - Election event UUID
/// * `election_id` - Election UUID
/// * `census` - Census value
/// * `auditable_votes` - Auditable votes value
/// * `parent_id` - Optional parent area UUID
/// * `area_id` - Optional area UUID as string
///
/// # Panics
/// Panics if `area_id` is provided but is not a valid UUID string.
pub fn get_area_config(
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    election_id: &Uuid,
    census: u64,
    auditable_votes: u64,
    parent_id: Option<Uuid>,
    area_id: Option<String>,
) -> AreaConfig {
    let area_uuid = area_id.map_or_else(Uuid::new_v4, |val| {
        Uuid::parse_str(&val).expect("Invalid UUID in area_id")
    });
    AreaConfig {
        id: area_uuid,
        name: String::new(),
        tenant_id: *tenant_id,
        election_event_id: *election_event_id,
        election_id: *election_id,
        census,
        auditable_votes,
        parent_id,
    }
}
