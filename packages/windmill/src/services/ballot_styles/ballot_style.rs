// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::area::get_event_areas;
use crate::postgres::area_contest::export_area_contests;
use crate::postgres::ballot_publication::{
    get_ballot_publication_by_id, update_ballot_publication_status,
};
use crate::postgres::ballot_style::insert_ballot_style;
use crate::postgres::candidate::export_candidates;
use crate::postgres::contest::export_contests;
use crate::postgres::election::export_elections;
use crate::postgres::election_event::get_election_event_by_id;
use crate::services::database::get_hasura_pool;
use crate::services::import_election_event::AreaContest;
use crate::types::error::Result;
use anyhow::{anyhow, Context, Result as AnyhowResult};
use chrono::Duration;
use deadpool_postgres::{Client as DbClient, Transaction};
use futures::try_join;
use sequent_core::types::hasura::core::{
    self as hasura_type, Area, BallotPublication, Candidate, Contest, Election, ElectionEvent,
};

use std::collections::HashMap;
use tracing::{event, instrument, Level};
use uuid::Uuid;

use crate::services::date::ISO8601;
use crate::services::pg_lock::PgLock;

use super::area_tree::TreeNode;

pub async fn create_ballot_style_postgres(
    transaction: &Transaction<'_>,
    area: &Area,
    areas_map: &HashMap<String, Area>,
    areas_tree: &TreeNode,
    tenant_id: &str,
    election_event: &ElectionEvent,
    ballot_publication: &BallotPublication,
    elections_map: &HashMap<String, Election>,
    contests_map: &HashMap<String, Contest>,
    candidates_map: &HashMap<String, Candidate>,
    area_contests_map: &HashMap<String, AreaContest>,
) -> Result<()> {
    let election_ids = ballot_publication.election_ids.clone().unwrap_or(vec![]);
    if 0 == election_ids.len() {
        event!(Level::INFO, "No election ids",);
        return Ok(());
    }
    let area_ids: Vec<String> = areas_tree
        .find_path_to_area(&area.id)
        .ok_or(anyhow!("area not found in tree"))?
        .into_iter()
        .map(|area| area.id)
        .collect();
    let area_contests: Vec<AreaContest> = area_contests_map
        .values()
        .filter(|area_contest| area_ids.contains(&area_contest.area_id.to_string()))
        .map(|val| val.clone())
        .collect();
    // election_id, vec<contest>
    let mut election_contest_map: HashMap<String, Vec<Contest>> = HashMap::new();

    for area_contest in area_contests.iter() {
        let Some(contest) = contests_map.get(&area_contest.contest_id.to_string()) else {
            event!(
                Level::INFO,
                "missing contest for area contest: {}",
                area_contest.id
            );
            continue;
        };
        if !election_ids.contains(&contest.election_id) {
            continue;
        }
        election_contest_map
            .entry(contest.election_id.clone())
            .and_modify(|contests| contests.push(contest.clone()))
            .or_insert(vec![contest.clone()]);
    }

    for (election_id, contests) in election_contest_map.into_iter() {
        let election = elections_map
            .get(&election_id)
            .ok_or(anyhow!("election id not found {}", election_id))?;
        let contest_ids: Vec<String> = contests.iter().map(|contest| contest.id.clone()).collect();
        let candidates: Vec<Candidate> = candidates_map
            .values()
            .filter(|candidate| {
                let Some(contest_id) = candidate.contest_id.clone() else {
                    return false;
                };
                contest_ids.contains(&contest_id)
            })
            .map(|candidate| candidate.clone())
            .collect();

        let ballot_style_id = Uuid::new_v4();
        let election_dto = sequent_core::ballot_style::create_ballot_style(
            ballot_style_id.clone().to_string(),
            area.clone(),
            election_event.clone(),
            election.clone(),
            contests.clone(),
            candidates.clone(),
        )?;
        let election_dto_json_string = serde_json::to_string(&election_dto)?;
        let _created_ballot_style = insert_ballot_style(
            transaction,
            &ballot_style_id.to_string(),
            tenant_id,
            &election_event.id,
            &election.id,
            &area.id,
            Some(election_dto_json_string),
            None,
            &ballot_publication.id,
        )
        .await?;
    }

    Ok(())
}

#[instrument(err)]
pub async fn update_election_event_ballot_styles(
    tenant_id: &str,
    election_event_id: &str,
    ballot_publication_id: &str,
) -> AnyhowResult<()> {
    let lock = PgLock::acquire(
        format!("create_ballot_style-{}-{}", tenant_id, election_event_id),
        Uuid::new_v4().to_string(),
        ISO8601::now() + Duration::seconds(60),
    )
    .await?;
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .with_context(|| "Error getting hasura db pool")?;

    let transaction = hasura_db_client
        .transaction()
        .await
        .with_context(|| "Error starting hasura transaction")?;

    let Some(ballot_publication) = get_ballot_publication_by_id(
        &transaction,
        tenant_id,
        election_event_id,
        ballot_publication_id,
    )
    .await?
    else {
        return Err(anyhow!("can't find ballot publication"));
    };
    let (election_event, elections, contests, candidates, areas, area_contests) = try_join!(
        get_election_event_by_id(&transaction, tenant_id, election_event_id),
        export_elections(&transaction, tenant_id, election_event_id),
        export_contests(&transaction, tenant_id, election_event_id),
        export_candidates(&transaction, tenant_id, election_event_id),
        get_event_areas(&transaction, tenant_id, election_event_id),
        export_area_contests(&transaction, tenant_id, election_event_id),
    )?;
    let areas_map: HashMap<String, Area> = areas
        .clone()
        .into_iter()
        .map(|area: Area| (area.id.clone(), area.clone()))
        .collect();

    let elections_map: HashMap<String, Election> = elections
        .into_iter()
        .map(|election: Election| (election.id.clone(), election.clone()))
        .collect();

    let contests_map: HashMap<String, Contest> = contests
        .into_iter()
        .map(|contest| (contest.id.clone(), contest.clone()))
        .collect();

    let candidates_map: HashMap<String, Candidate> = candidates
        .into_iter()
        .map(|candidate: Candidate| (candidate.id.clone(), candidate.clone()))
        .collect();

    let area_contests_map: HashMap<String, AreaContest> = area_contests
        .into_iter()
        .map(|area_contest| (area_contest.id.to_string(), area_contest.clone()))
        .collect();

    let areas_tree = TreeNode::from_areas(areas.clone())?;

    for area in &areas {
        create_ballot_style_postgres(
            &transaction,
            area,
            &areas_map,
            &areas_tree,
            &tenant_id,
            &election_event,
            &ballot_publication,
            &elections_map,
            &contests_map,
            &candidates_map,
            &area_contests_map,
        )
        .await?;
    }
    update_ballot_publication_status(
        &transaction,
        tenant_id,
        election_event_id,
        ballot_publication_id,
        true,
        None,
    )
    .await?;

    let _commit = transaction.commit().await.with_context(|| "Commit failed");
    lock.release().await?;
    Ok(())
}
