// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::database::get_hasura_pool;
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

pub async fn create_ballot_style_postgres(
    transaction: &Transaction<'_>,
    area: &Area,
    tenant_id: &str,
    election_event: &ElectionEvent,
    ballot_publication: &BallotPublication,
    elections_input: &Vec<Election>,
    contests_input: &Vec<Contest>,
    candidates_input: &Vec<Candidate>,
    area_contests_input: &Vec<AreaContest>,
) -> Result<()> {
    let election_ids = ballot_publication.election_ids.clone().unwrap_or(vec![]);
    if 0 == election_ids.len() {
        event!(Level::INFO, "No election ids",);
        return Ok(());
    }
    let area_contests: Vec<AreaContest> = area_contests_input
        .clone()
        .into_iter()
        .filter(|area_contest| area_contest.area_id.to_string() == area.id)
        .collect();

    // election_id, vec<contest_ids>
    let mut election_contest_map: HashMap<String, Vec<String>> = HashMap::new();

    for area_contest in area_contests.iter() {
        let Some(contest) = contests_input
            .iter()
            .find(|contest| contest.id == area_contest.contest_id.to_string())
        else {
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
            .and_modify(|contest_ids| contest_ids.push(contest.id.clone()))
            .or_insert(vec![contest.id.clone()]);
    }

    for (election_id, contest_ids) in election_contest_map.into_iter() {
        let election = elections_input
            .iter()
            .find(|election| election.id == election_id)
            .with_context(|| format!("election id not found {}", election_id))?;
        let contests = contest_ids
            .clone()
            .into_iter()
            .map(|contest_id| -> Result<hasura_type::Contest> {
                let area_contest = area_contests
                    .iter()
                    .find(|area_contest| area_contest.contest_id.to_string() == contest_id)
                    .with_context(|| format!("contest id not found {}", contest_id))?;
                let contest = contests_input
                    .iter()
                    .find(|contest| contest.id == contest_id)
                    .with_context(|| format!("contest not found {}", contest_id))?;
                Ok(contest.clone())
            })
            .collect::<Result<Vec<hasura_type::Contest>>>()?;
        let candidates: Vec<hasura_type::Candidate> = contest_ids
            .into_iter()
            .map(|contest_id| -> Result<Vec<hasura_type::Candidate>> {
                let area_contest = area_contests
                    .iter()
                    .find(|area_contest| area_contest.contest_id.to_string() == contest_id)
                    .with_context(|| format!("contest id not found {}", contest_id))?;
                let area_candidates: Vec<Candidate> = candidates_input
                    .clone()
                    .into_iter()
                    .filter(|candidate| candidate.contest_id == Some(contest_id.clone()))
                    .collect();
                Ok(area_candidates)
            })
            .collect::<Result<Vec<Vec<hasura_type::Candidate>>>>()?
            .into_iter()
            .flatten()
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
        let _hasura_response = insert_ballot_style(
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

    for area in &areas {
        create_ballot_style_postgres(
            &transaction,
            area,
            &tenant_id,
            &election_event,
            &ballot_publication,
            &elections,
            &contests,
            &candidates,
            &area_contests,
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
