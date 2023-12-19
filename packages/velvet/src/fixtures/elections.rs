use uuid::Uuid;
use crate::pipes::pipe_inputs::ElectionConfig;
use super::ballot_styles;

pub fn get_election1(election_event_id: &Uuid) -> ElectionConfig {
    let tenant_id = Uuid::new_v4();
    let election_id = Uuid::new_v4();

    let area_id = Uuid::new_v4();
    let ballot_style =
        ballot_styles::get_ballot_style_1(&tenant_id, election_event_id, &election_id, &area_id);

    ElectionConfig {
        id: election_id,
        tenant_id,
        election_event_id: election_event_id.clone(),
        ballot_styles: vec![ballot_style],
    }
}

pub fn get_election2() -> ElectionConfig {
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
        tenant_id,
        election_event_id,
        ballot_styles: vec![ballot_style1, ballot_style2],
    }
}
