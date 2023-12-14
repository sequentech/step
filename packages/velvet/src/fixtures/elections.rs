use sequent_core::ballot::BallotStyle;
use uuid::Uuid;

use super::ballot_styles;

struct Election {
    tenant_id: Uuid,
    election_event_id: Uuid,
    election_id: Uuid,
    ballot_styles: Vec<BallotStyle>,
}

pub fn get_election1() -> Election {
    let tenant_id = Uuid::new_v4();
    let election_event_id = Uuid::new_v4();
    let election_id = Uuid::new_v4();

    let area_id = Uuid::new_v4();
    let ballot_style = ballot_styles::get_ballot_style_1(
        &tenant_id.to_string(),
        &election_event_id.to_string(),
        &election_id.to_string(),
        &area_id.to_string(),
    );

    Election {
        tenant_id,
        election_event_id,
        election_id,
        ballot_styles: vec![ballot_style],
    }
}

pub fn get_election2() -> Election {
    let tenant_id = Uuid::new_v4();
    let election_event_id = Uuid::new_v4();
    let election_id = Uuid::new_v4();

    let area_id = Uuid::new_v4();
    let ballot_style1 = ballot_styles::get_ballot_style_1(
        &tenant_id.to_string(),
        &election_event_id.to_string(),
        &election_id.to_string(),
        &area_id.to_string(),
    );

    let area_id = Uuid::new_v4();
    let ballot_style2 = ballot_styles::get_ballot_style_1(
        &tenant_id.to_string(),
        &election_event_id.to_string(),
        &election_id.to_string(),
        &area_id.to_string(),
    );

    Election {
        tenant_id,
        election_event_id,
        election_id,
        ballot_styles: vec![ballot_style1, ballot_style2],
    }
}
