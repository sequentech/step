use sequent_core::ballot::BallotStyle;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

use super::ballot_styles;

#[derive(Serialize, Deserialize, Clone)]
pub struct Election {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    pub ballot_styles: Vec<BallotStyle>,
}

pub fn get_election1(election_event_id: &Uuid) -> Election {
    let tenant_id = Uuid::new_v4();
    let election_id = Uuid::new_v4();

    let area_id = Uuid::new_v4();
    let ballot_style =
        ballot_styles::get_ballot_style_1(&tenant_id, election_event_id, &election_id, &area_id);

    Election {
        id: election_id,
        tenant_id,
        election_event_id: election_event_id.clone(),
        ballot_styles: vec![ballot_style],
    }
}

pub fn get_election2() -> Election {
    let tenant_id = Uuid::new_v4();
    let election_event_id = Uuid::new_v4();
    let election_id = Uuid::new_v4();

    let area_id = Uuid::new_v4();
    let ballot_style1 =
        ballot_styles::get_ballot_style_1(&tenant_id, &election_event_id, &election_id, &area_id);

    let area_id = Uuid::new_v4();
    let ballot_style2 =
        ballot_styles::get_ballot_style_1(&tenant_id, &election_event_id, &election_id, &area_id);

    Election {
        id: election_id,
        tenant_id,
        election_event_id,
        ballot_styles: vec![ballot_style1, ballot_style2],
    }
}
