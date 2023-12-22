use uuid::Uuid;

use crate::pipes::pipe_inputs::AreaConfig;

pub fn get_area_config(
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    election_id: &Uuid,
    census: u64,
) -> AreaConfig {
    AreaConfig {
        id: Uuid::new_v4(),
        tenant_id: *tenant_id,
        election_event_id: *election_event_id,
        election_id: *election_id,
        census,
    }
}
