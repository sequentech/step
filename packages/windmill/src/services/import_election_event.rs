// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use ::keycloak::types::RealmRepresentation;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::services::keycloak;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tracing::instrument;
use uuid::Uuid;

use sequent_core::types::hasura_types::{
    Area as AreaData, Candidate as CandidateData, Contest as ContestData, Election as ElectionData,
    ElectionEvent as ElectionEventData,
};

#[derive(Debug, Deserialize)]
pub struct Election {
    id: Uuid,
    election_event_id: Uuid,
    data: ElectionData,
}

#[derive(Debug, Deserialize)]
pub struct Contest {
    id: Uuid,
    election_id: Uuid,
    data: ContestData,
    area_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct Candidate {
    id: Uuid,
    contest_id: Uuid,
    data: CandidateData,
}

#[derive(Debug, Deserialize)]
pub struct AreaContest {
    area_id: Uuid,
    contest: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct ImportElectionEventSchema {
    keycloak_event_realm: RealmRepresentation,
    election_event_data: ElectionEventData,
    elections: Vec<Election>,
    contests: Vec<Contest>,
    candidates: Vec<Candidate>,
    areas: Vec<AreaData>,
    area_contest: Vec<AreaContest>,
}

pub async fn process(data: &ImportElectionEventSchema) {
    dbg!(&data);
}
