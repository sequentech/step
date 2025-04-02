// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::area::get_event_areas;
use crate::postgres::area_contest::export_area_contests;
use crate::postgres::ballot_publication::{
    get_ballot_publication, get_ballot_publication_by_id, update_ballot_publication_status,
};
use crate::postgres::ballot_style::{
    get_ballot_styles_by_ballot_publication_by_id, insert_ballot_style,
};
use crate::postgres::candidate::export_candidates;
use crate::postgres::contest::export_contests;
use crate::postgres::election::{export_elections, get_election_by_id};
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::keys_ceremony::get_keys_ceremonies;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::database::get_hasura_pool;
use crate::services::election_dates::get_election_dates;
use crate::services::pg_lock::PgLock;
use crate::types::error::{Error, Result};
use anyhow::{anyhow, Context, Result as AnyhowResult};
use chrono::{Duration, Local};
use deadpool_postgres::{Client as DbClient, Transaction};
use futures::try_join;
use sequent_core::services::area_tree::TreeNode;
use sequent_core::services::date::ISO8601;
use sequent_core::services::s3;
use sequent_core::types::hasura::core::{
    self as hasura_type, Area, AreaContest, BallotPublication, BallotStyle, Candidate, Contest,
    Election, ElectionEvent, KeysCeremony,
};
use sequent_core::types::scheduled_event::ScheduledEvent;
use sequent_core::util::retry::retry_with_exponential_backoff;
use std::collections::{HashMap, HashSet};
use std::time::Duration as StdDuration;
use tracing::{event, info, instrument, Level};
use uuid::Uuid;
/**
 * Returns a HashMap<election_id, set<contest_id>> with all
 * the election ids and contest ids related to an area,
 * taking into consideration the parent areas as well.
 */
#[instrument(skip_all, err)]
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

#[instrument(skip_all, err)]
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
    keys_ceremonies_map: &HashMap<String, KeysCeremony>,
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
        let public_key = if let Some(keys_ceremony_id) = election.keys_ceremony_id.clone() {
            if let Some(keys_ceremony) = keys_ceremonies_map.get(&keys_ceremony_id) {
                if let Some(status) = keys_ceremony.status().ok() {
                    status.public_key
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let election_dates =
            get_election_dates(election, scheduled_events.clone()).unwrap_or_default();

        let ballot_style_id = Uuid::new_v4();
        let election_dto = sequent_core::ballot_style::create_ballot_style(
            ballot_style_id.clone().to_string(),
            area.clone(),
            election_event.clone(),
            election.clone(),
            contests.clone(),
            candidates.clone(),
            election_dates.clone(),
            public_key.clone(),
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
    let (
        election_event,
        elections,
        contests,
        candidates,
        areas,
        area_contests,
        scheduled_events,
        keys_ceremonies,
    ) = try_join!(
        get_election_event_by_id(&transaction, tenant_id, election_event_id),
        export_elections(&transaction, tenant_id, election_event_id),
        export_contests(&transaction, tenant_id, election_event_id),
        export_candidates(&transaction, tenant_id, election_event_id),
        get_event_areas(&transaction, tenant_id, election_event_id),
        export_area_contests(&transaction, tenant_id, election_event_id),
        find_scheduled_event_by_election_event_id(&transaction, tenant_id, election_event_id),
        get_keys_ceremonies(&transaction, tenant_id, election_event_id),
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

    let keys_ceremonies_map: HashMap<String, KeysCeremony> = keys_ceremonies
        .into_iter()
        .map(|keys_ceremony: KeysCeremony| (keys_ceremony.id.clone(), keys_ceremony.clone()))
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
            &keys_ceremonies_map,
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

#[instrument(err)]
pub async fn get_ballot_styles_for_authorized_elections(
    tenant_id: &str,
    election_event_id: &str,
    authorized_election_ids: &Vec<String>,
) -> AnyhowResult<Vec<BallotStyle>> {
    todo!()
}

/// Upload the files of this ublication id into the public bucket under the base_path "tenant-{tenant_id}/event-{election_event_id}/area-{area_id}"
/// elections.json, election-event.json, ballot-publications.json and ballot-style-election-{election_id}.json.
///
/// Only ballot-publications.json gets overwritten.
/// All other files will land under {base_path}/publication-{ballot_publication_id}, so a new file is added at every publication.
#[instrument(skip(hasura_transaction, election_event), err)]
pub async fn update_election_event_ballot_s3_files(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    ballot_publication_id: &str,
    election_event: &ElectionEvent,
) -> Result<()> {
    let s3_bucket = s3::get_public_bucket()?;

    let ballot_styles = get_ballot_styles_by_ballot_publication_by_id(
        hasura_transaction,
        tenant_id,
        election_event_id,
        ballot_publication_id,
    )
    .await?;

    let mut areas = HashSet::new();
    let mut election_ids = HashSet::new();

    // Upload ballot_style files and prepare the areas and election_ids to be unique.
    for ballot_style in &ballot_styles {
        let area_id = ballot_style
            .area_id
            .as_deref()
            .ok_or("No area_id found".to_string())?;
        let election_id = &ballot_style.election_id;
        areas.insert(area_id.to_string());
        election_ids.insert(election_id.to_string());

        let ballot_style_data = serde_json::to_string(ballot_style)
            .map_err(|err| format!("Error serializing ballot style to json: {err:?}"))?;

        let ballot_style_path = s3::get_public_ballot_style_file_path(
            tenant_id,
            election_event_id,
            area_id,
            ballot_publication_id,
            election_id,
        );
        upload_ballot_publication_files_to_s3_with_retry(
            &ballot_style_data,
            &ballot_style_path,
            &s3_bucket,
        )
        .await?;
    }

    // Prepare the files to upload for each area.
    let election_event_data = serde_json::to_string(&election_event)
        .map_err(|err| format!("Error serializing election event to json: {err:?}"))?;

    let mut elections: Vec<Election> = vec![];
    for election_id in &election_ids {
        let election = get_election_by_id(
            &hasura_transaction,
            tenant_id,
            election_event_id,
            election_id,
        )
        .await?
        .ok_or(anyhow!("can't find election: {election_id}"))?;
        elections.push(election);
    }

    let elections_data = serde_json::to_string(&elections)
        .map_err(|err| format!("Error serializing elections to json: {err:?}"))?;

    let ballot_publications: Vec<BallotPublication> =
        get_ballot_publication(&hasura_transaction, tenant_id, election_event_id)
            .await
            .map_err(|e| anyhow!("Error reading ballot publications: {e:?}"))?;

    let ballot_publications_data = serde_json::to_string(&ballot_publications)
        .map_err(|err| format!("Error serializing ballot publications to json: {err:?}"))?;

    for area_id in &areas {
        let election_event_path = s3::get_public_election_event_file_path(
            tenant_id,
            election_event_id,
            area_id,
            ballot_publication_id,
        );
        upload_ballot_publication_files_to_s3_with_retry(
            &election_event_data,
            &election_event_path,
            &s3_bucket,
        )
        .await?;

        // Upload elections data:
        let elections_file_path = s3::get_public_elections_file_path(
            tenant_id,
            election_event_id,
            area_id,
            ballot_publication_id,
        );
        upload_ballot_publication_files_to_s3_with_retry(
            &elections_data,
            &elections_file_path,
            &s3_bucket,
        )
        .await?;

        // Upload ballot publications file or replace it if it exists.
        let ballot_publications_file_path =
            s3::get_public_ballot_publications_file_path(tenant_id, election_event_id, area_id);
        upload_ballot_publication_files_to_s3_with_retry(
            &ballot_publications_data,
            &ballot_publications_file_path,
            &s3_bucket,
        )
        .await?;
    }

    Ok(())
}

#[instrument(skip(data), err)]
pub async fn upload_ballot_publication_files_to_s3_with_retry(
    data: &str,
    path: &str,
    s3_bucket: &str,
) -> Result<()> {
    retry_with_exponential_backoff(
        || async {
            s3::upload_data_to_s3(
                data.to_string().into_bytes().into(),
                path.to_string(),
                false, // False because it's windmill uploading, not a public interface
                s3_bucket.to_string(),
                "text/plain".to_string(),
                None,
                None,
            )
            .await
        },
        3,
        StdDuration::from_millis(100),
    )
    .await
    .map_err(|err| format!("Error uploading input document to S3, trying 3 times: {err:?}"))?;
    Ok(())
}
