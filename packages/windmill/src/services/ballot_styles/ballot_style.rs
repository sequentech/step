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
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::database::get_hasura_pool;
use crate::types::error::{Error, Result};
use anyhow::{anyhow, Context, Result as AnyhowResult};
use chrono::Duration;
use deadpool_postgres::{Client as DbClient, Transaction};
use futures::try_join;
use sequent_core::types::hasura::core::{
    self as hasura_type, Area, AreaContest, BallotPublication, Candidate, Contest, Election,
    ElectionEvent,
};
use sequent_core::types::scheduled_event::ScheduledEvent;

use std::collections::{HashMap, HashSet};
use tracing::{event, instrument, Level};
use uuid::Uuid;

use crate::services::date::ISO8601;
use crate::services::pg_lock::PgLock;

use sequent_core::services::area_tree::TreeNode;

/**
 * Returns a HashMap<election_id, set<contest_id>> with all
 * the election ids and contest ids related to an area,
 * taking into consideration the parent areas as well.
 */
pub fn get_elections_contests_map_for_area(
    area: &Area,
    areas_tree: &TreeNode,
    ballot_publication: &BallotPublication,
    contests_map: &HashMap<String, Contest>,
    area_contests_map: &HashMap<String, AreaContest>,
) -> AnyhowResult<HashMap<String, HashSet<String>>> {
    let election_ids = ballot_publication.election_ids.clone().unwrap_or(vec![]);
    if 0 == election_ids.len() {
        return Err(anyhow!("No election ids"));
    }
    let area_ids: Vec<String> = areas_tree
        .find_path_to_area(&area.id)
        .ok_or(anyhow!("area not found in tree"))?
        .into_iter()
        .map(|area| area.id)
        .collect();
    let area_contests: Vec<AreaContest> = area_contests_map
        .values()
        .filter(|area_contest| area_ids.contains(&area_contest.area_id))
        .map(|val| val.clone())
        .collect();
    // election_id, set<contest>
    let mut election_contest_map: HashMap<String, HashSet<String>> = HashMap::new();

    for area_contest in area_contests.iter() {
        let Some(contest) = contests_map.get(&area_contest.contest_id) else {
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
            .and_modify(|contest_ids| {
                contest_ids.insert(contest.id.clone());
            })
            .or_insert_with(|| {
                let mut set = HashSet::new();
                set.insert(contest.id.clone());
                set
            });
    }
    Ok(election_contest_map)
}

pub async fn create_ballot_style_postgres(
    transaction: &Transaction<'_>,
    area: &Area,
    areas_tree: &TreeNode,
    tenant_id: &str,
    election_event: &ElectionEvent,
    ballot_publication: &BallotPublication,
    elections_map: &HashMap<String, Election>,
    contests_map: &HashMap<String, Contest>,
    candidates_map: &HashMap<String, Candidate>,
    area_contests_map: &HashMap<String, AreaContest>,
    scheduled_events: &Vec<ScheduledEvent>,
) -> Result<()> {
    let election_contest_map = get_elections_contests_map_for_area(
        area,
        areas_tree,
        ballot_publication,
        contests_map,
        area_contests_map,
    )?;

    for (election_id, contest_ids) in election_contest_map.into_iter() {
        let election = elections_map
            .get(&election_id)
            .ok_or(anyhow!("election id not found {}", election_id))?;
        let contests: Vec<Contest> = contest_ids
            .iter()
            .map(|contest_id| {
                contests_map
                    .get(contest_id)
                    .map(|val| val.clone())
                    .ok_or(Error::String(format!("Can't find contest {}", contest_id)))
            })
            .collect::<Result<Vec<Contest>>>()?;
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
            scheduled_events.clone(),
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
    let (election_event, elections, contests, candidates, areas, area_contests, scheduled_events) =
        try_join!(
            get_election_event_by_id(&transaction, tenant_id, election_event_id),
            export_elections(&transaction, tenant_id, election_event_id),
            export_contests(&transaction, tenant_id, election_event_id),
            export_candidates(&transaction, tenant_id, election_event_id),
            get_event_areas(&transaction, tenant_id, election_event_id),
            export_area_contests(&transaction, tenant_id, election_event_id),
            find_scheduled_event_by_election_event_id(&transaction, tenant_id, election_event_id),
        )?;

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
        .map(|area_contest| (area_contest.id.clone(), area_contest.clone()))
        .collect();

    let basic_areas = areas.iter().map(|area| area.into()).collect();
    let areas_tree = TreeNode::from_areas(basic_areas)?;

    for area in &areas {
        create_ballot_style_postgres(
            &transaction,
            area,
            &areas_tree,
            &tenant_id,
            &election_event,
            &ballot_publication,
            &elections_map,
            &contests_map,
            &candidates_map,
            &area_contests_map,
            &scheduled_events,
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
