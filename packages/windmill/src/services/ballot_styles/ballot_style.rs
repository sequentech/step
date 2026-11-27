// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::publication_files;
use crate::postgres::area::get_event_areas;
use crate::postgres::area_contest::export_area_contests;
use crate::postgres::ballot_publication::{get_ballot_publication_by_id, lock_publication_event};
use crate::postgres::ballot_style::{insert_ballot_styles, NewBallotStyle};
use crate::postgres::candidate::export_candidates;
use crate::postgres::contest::export_contests;
use crate::postgres::election::export_elections;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::keys_ceremony::get_keys_ceremonies;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::ballot_box_key::ballot_box_key_for_publication;
use crate::services::ceremonies::keys_ceremony::{
    verify_keys_ceremony_public_key, CeremonyPublicKeyState,
};
use crate::services::database::get_hasura_pool;
use crate::services::election_dates::get_election_dates;
use crate::services::external::utils::remove_datafix_annotations;
use crate::types::error::{Error, Result};
use anyhow::{anyhow, Context, Result as AnyhowResult};
use chrono::Duration;
use deadpool_postgres::{Client as DbClient, Transaction};
use futures::try_join;
use rocket::http::Status;
use sequent_core::ballot::{
    BallotBoxKey, ContestEncryptionPolicy, ElectionEventPresentation, StringifiedPeriodDates,
};
use sequent_core::ballot_codec::multi_ballot::BallotChoices;
use sequent_core::ballot_style::AreaContestIndex;
use sequent_core::types::hasura::core::{
    self as hasura_type, Area, AreaContest, BallotPublication, BallotStyle, Candidate, Contest,
    Election, ElectionEvent, KeysCeremony,
};
use sequent_core::types::scheduled_event::ScheduledEvent;
use serde::{Deserialize, Serialize};

use std::collections::{HashMap, HashSet};
use tokio_postgres::IsolationLevel;
use tracing::{event, instrument, Level};
use uuid::Uuid;

use crate::services::pg_lock::PgLock;
use sequent_core::services::date::ISO8601;

use sequent_core::services::area_tree::TreeNode;

pub const EVENT_CONFIG_FILE_NAME: &str = "election_event_config.json";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ElectionEventConfig {
    pub id: String,
    pub election_event_id: String,
    pub tenant_id: String,
    pub election_event_presentation: ElectionEventPresentation,
}

/// Ballot styles inserted per statement.
const INSERT_BATCH_SIZE: usize = 64;

/// What the voters of one election see: its dates and its public key.
struct ElectionInputs {
    dates: StringifiedPeriodDates,
    public_key: Option<String>,
}

fn election_inputs(
    elections: &HashMap<String, Election>,
    scheduled_events: &[ScheduledEvent],
    keys_ceremonies: &HashMap<String, KeysCeremony>,
) -> HashMap<String, ElectionInputs> {
    elections
        .values()
        .map(|election| {
            let public_key = election
                .keys_ceremony_id
                .as_ref()
                .and_then(|id| keys_ceremonies.get(id))
                .and_then(|keys_ceremony| keys_ceremony.status().ok())
                .and_then(|status| status.public_key);
            let inputs = ElectionInputs {
                dates: get_election_dates(election, scheduled_events.to_vec()).unwrap_or_default(),
                public_key,
            };
            (election.id.clone(), inputs)
        })
        .collect()
}

fn candidates_by_contest(candidates: &[Candidate]) -> HashMap<&str, Vec<&Candidate>> {
    let mut by_contest: HashMap<&str, Vec<&Candidate>> = HashMap::new();
    for candidate in candidates {
        if let Some(contest_id) = candidate.contest_id.as_deref() {
            by_contest.entry(contest_id).or_default().push(candidate);
        }
    }
    by_contest
}

fn contest_candidates(
    candidates_by_contest: &HashMap<&str, Vec<&Candidate>>,
    contest_ids: &HashSet<String>,
) -> Vec<Candidate> {
    contest_ids
        .iter()
        .filter_map(|contest_id| candidates_by_contest.get(contest_id.as_str()))
        .flatten()
        .map(|candidate| (*candidate).clone())
        .collect()
}

/// The event's rows for one generation, read and indexed once.
struct Generation<'a> {
    tenant_id: &'a str,
    election_event_id: &'a str,
    election_ids: &'a [String],
    area_contests: AreaContestIndex<'a>,
    elections: &'a HashMap<String, Election>,
    contests: &'a HashMap<String, Contest>,
    all_contests: &'a [Contest],
    candidates_by_contest: HashMap<&'a str, Vec<&'a Candidate>>,
    election_inputs: HashMap<String, ElectionInputs>,
    voter_election_event: ElectionEvent,
    ballot_box_key: Option<&'a BallotBoxKey>,
}

impl Generation<'_> {
    /// Builds the ballot style of each of the publication's elections that
    /// has contests in the area. `configuration_ids` keeps each keys
    /// ceremony's DKG configuration id, read from its board once.
    async fn area_ballot_styles(
        &self,
        transaction: &Transaction<'_>,
        area: &Area,
        configuration_ids: &mut HashMap<String, String>,
    ) -> Result<Vec<NewBallotStyle>> {
        let mut styles = Vec::new();
        for (election_id, contest_ids) in self
            .area_contests
            .elections_contests(&area.id, self.election_ids)?
        {
            let election = self
                .elections
                .get(&election_id)
                .ok_or(anyhow!("election id not found {}", election_id))?;
            let inputs = self
                .election_inputs
                .get(&election_id)
                .ok_or(anyhow!("election id not found {}", election_id))?;
            let contests: Vec<Contest> = contest_ids
                .iter()
                .map(|contest_id| {
                    self.contests
                        .get(contest_id)
                        .map(|val| val.clone())
                        .ok_or(Error::String(format!("Can't find contest {}", contest_id)))
                })
                .collect::<Result<Vec<Contest>>>()?;

            let ballot_style_id = Uuid::new_v4();
            let mut election_dto = sequent_core::ballot_style::create_ballot_style(
                ballot_style_id.to_string(),
                area.clone(),
                self.voter_election_event.clone(),
                election.clone(),
                contests,
                self.all_contests,
                contest_candidates(&self.candidates_by_contest, &contest_ids),
                inputs.dates.clone(),
                inputs.public_key.clone(),
                // windmill has an environment; let it read DEMO_PUBLIC_KEY from
                // there. A browser does not, which is why the parameter exists.
                None,
            )?;
            if let Some(published) = election_dto.public_key.as_mut().filter(|key| !key.is_demo) {
                let ceremony_id = election
                    .keys_ceremony_id
                    .as_deref()
                    .context("Election has a public key without a keys ceremony")?;
                let configuration_id = match configuration_ids.get(ceremony_id) {
                    Some(configuration_id) => configuration_id.clone(),
                    None => {
                        let dkg = crate::services::protocol_board::load_dkg_board(
                            transaction,
                            self.tenant_id,
                            self.election_event_id,
                            ceremony_id,
                        )
                        .await?;
                        let configuration_id = dkg.configuration.configuration_id()?.to_string();
                        configuration_ids.insert(ceremony_id.to_owned(), configuration_id.clone());
                        configuration_id
                    }
                };
                published.configuration_id = Some(configuration_id);
            }
            election_dto.ballot_box_key = self.ballot_box_key.cloned();

            let is_multi_contest = election_dto
                .election_event_presentation
                .as_ref()
                .and_then(|presentation| presentation.contest_encryption_policy.clone())
                == Some(ContestEncryptionPolicy::MULTIPLE_CONTESTS);

            if is_multi_contest {
                let max_bytes = BallotChoices::maximum_size_bytes(
                    &election_dto.contests,
                    election_dto.decline_to_vote_enabled(),
                    election_dto.blank_ballots_enabled(),
                    election_dto.multi_contest_encoding_mode.unwrap_or_default(),
                )?;

                if max_bytes > BallotChoices::MAX_SIZE_BYTES {
                    return Err(Error::String(format!(
                        "Ballot style for election {} in area {} needs {} bytes, exceeding the {}-byte ballot size limit. Reduce the number of candidates or contests, or adjust over-vote policies.",
                        election.id,
                        area.id,
                        max_bytes,
                        BallotChoices::MAX_SIZE_BYTES
                    )));
                }
            }

            styles.push(NewBallotStyle {
                id: ballot_style_id,
                election_id: Uuid::parse_str(&election.id).context("Invalid election id")?,
                area_id: Uuid::parse_str(&area.id).context("Invalid area id")?,
                ballot_eml: publication_files::ordered_json_string(&election_dto)
                    .map_err(|error| Error::String(error.to_string()))?,
            });
        }
        Ok(styles)
    }
}

#[instrument(err)]
pub async fn update_election_event_ballot_styles(
    tenant_id: &str,
    election_event_id: &str,
    ballot_publication_id: &str,
) -> AnyhowResult<()> {
    let lock = PgLock::acquire(
        format!("create_ballot_style-{tenant_id}-{election_event_id}-{ballot_publication_id}"),
        Uuid::new_v4().to_string(),
        ISO8601::now() + Duration::seconds(publication_files::GENERATION_LEASE_SECONDS),
    )
    .await?;

    let result = async {
        let Some(data) = generate_election_event_ballot_styles(
            tenant_id,
            election_event_id,
            ballot_publication_id,
            &lock,
        )
        .await?
        else {
            return Ok(());
        };
        let pool = get_hasura_pool().await;
        publication_files::prepare_publication_files(
            &pool,
            tenant_id,
            election_event_id,
            ballot_publication_id,
            &lock,
            data,
        )
        .await
    }
    .await;

    // Release on both paths so the lock does not leak until expiry on error.
    // A failed release must not mask the outcome of the work itself: the lock
    // is time-bounded and will expire on its own, whereas reporting a failure
    // here would mark an already-generated ballot publication as failed.
    if let Err(release_error) = lock.release().await {
        event!(
            Level::ERROR,
            "Failed to release the ballot style generation lock for election event {}: {:?}",
            election_event_id,
            release_error
        );
    }
    result
}

async fn generate_election_event_ballot_styles(
    tenant_id: &str,
    election_event_id: &str,
    ballot_publication_id: &str,
    lease: &PgLock,
) -> AnyhowResult<Option<publication_files::PublicationData>> {
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .with_context(|| "Error getting hasura db pool")?;

    // The producer enqueues this task before committing the publication and
    // holds the event lock until then. Wait for it before taking the snapshot.
    let barrier = hasura_db_client
        .transaction()
        .await
        .with_context(|| "Error starting hasura transaction")?;
    lock_publication_event(&barrier, tenant_id, election_event_id).await?;
    barrier
        .rollback()
        .await
        .with_context(|| "Error releasing the publication event lock")?;

    let transaction = hasura_db_client
        .build_transaction()
        .isolation_level(IsolationLevel::RepeatableRead)
        .start()
        .await
        .with_context(|| "Error starting hasura transaction")?;

    publication_files::lock_generation(&transaction, lease).await?;

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
    if !publication_needs_generation(&ballot_publication)? {
        publication_files::require_publication_files(&ballot_publication)?;
        return Ok(None);
    }
    if transaction.query_opt(
        "SELECT 1 FROM sequent_backend.ballot_style WHERE tenant_id=$1 AND election_event_id=$2 AND ballot_publication_id=$3 LIMIT 1",
        &[&Uuid::parse_str(tenant_id)?, &Uuid::parse_str(election_event_id)?, &Uuid::parse_str(ballot_publication_id)?],
    ).await?.is_some() {
        return Err(anyhow!("Previous ballot generation did not complete; generate a new publication"));
    }
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

    super::slates::validate_elections_slates(
        &ballot_publication.election_ids.clone().unwrap_or_default(),
        &elections_map,
        &contests_map.values().cloned().collect::<Vec<Contest>>(),
        &candidates_map.values().cloned().collect::<Vec<Candidate>>(),
    )?;
    let mut verified_keys_ceremonies = HashSet::new();
    for election_id in ballot_publication.election_ids.iter().flatten() {
        let Some(keys_ceremony) = elections_map
            .get(election_id)
            .and_then(|election| election.keys_ceremony_id.as_ref())
            .and_then(|keys_ceremony_id| keys_ceremonies_map.get(keys_ceremony_id))
        else {
            continue;
        };
        if verified_keys_ceremonies.insert(keys_ceremony.id.clone())
            && verify_keys_ceremony_public_key(
                &transaction,
                tenant_id,
                election_event_id,
                keys_ceremony,
            )
            .await?
                == CeremonyPublicKeyState::NotOnBoard
        {
            return Err(anyhow!(
                "Keys ceremony {} reports a public key that is not on its board",
                keys_ceremony.id
            ));
        }
    }

    let basic_areas = areas.iter().map(|area| area.into()).collect();
    let areas_tree = TreeNode::from_areas(basic_areas)?;
    let ballot_box_key = ballot_box_key_for_publication(&transaction, &election_event).await?;
    if let Some(receipt_key) = ballot_box_key.as_ref() {
        crate::services::public_ballot_keys::for_publication(
            &transaction,
            &election_event,
            receipt_key,
        )
        .await?;
    }

    let election_ids = ballot_publication.election_ids.clone().unwrap_or_default();
    let all_contests: Vec<Contest> = contests_map.values().cloned().collect();
    let all_candidates: Vec<Candidate> = candidates_map.values().cloned().collect();
    let mut voter_election_event = election_event.clone();
    remove_datafix_annotations(voter_election_event.annotations.as_mut());
    let generation = Generation {
        tenant_id,
        election_event_id,
        election_ids: &election_ids,
        area_contests: AreaContestIndex::new(&areas_tree, &contests_map, &area_contests_map),
        elections: &elections_map,
        contests: &contests_map,
        all_contests: &all_contests,
        candidates_by_contest: candidates_by_contest(&all_candidates),
        election_inputs: election_inputs(&elections_map, &scheduled_events, &keys_ceremonies_map),
        voter_election_event,
        ballot_box_key: ballot_box_key.as_ref(),
    };
    let tenant = Uuid::parse_str(tenant_id)?;
    let event = Uuid::parse_str(election_event_id)?;
    let publication = Uuid::parse_str(&ballot_publication.id)?;
    let mut configuration_ids = HashMap::new();
    let mut batch = Vec::with_capacity(INSERT_BATCH_SIZE);
    for area in &areas {
        batch.extend(
            generation
                .area_ballot_styles(&transaction, area, &mut configuration_ids)
                .await?,
        );
        if batch.len() >= INSERT_BATCH_SIZE {
            insert_ballot_styles(&transaction, tenant, event, publication, &batch).await?;
            batch.clear();
        }
    }
    if !batch.is_empty() {
        insert_ballot_styles(&transaction, tenant, event, publication, &batch).await?;
    }
    let data = publication_files::publication_data(
        &transaction,
        tenant_id,
        election_event_id,
        ballot_publication_id,
    )
    .await?;
    publication_files::renew_generation(&transaction, lease).await?;

    transaction
        .commit()
        .await
        .with_context(|| "Commit failed")?;
    Ok(Some(data))
}

/// Completed publications are immutable; repeated task delivery is a no-op.
fn publication_needs_generation(publication: &BallotPublication) -> AnyhowResult<bool> {
    if publication.deleted_at.is_some() {
        return Err(anyhow!("Cannot generate a deleted ballot publication"));
    }
    if publication.published_at.is_some() && !publication.is_generated.unwrap_or(false) {
        return Err(anyhow!(
            "Published ballot publication is missing generated content"
        ));
    }
    Ok(!publication.is_generated.unwrap_or(false))
}

#[cfg(test)]
mod generation_tests {
    use super::*;

    fn publication() -> BallotPublication {
        serde_json::from_value(serde_json::json!({
            "id": "publication", "tenant_id": "tenant", "election_event_id": "event"
        }))
        .unwrap()
    }

    #[test]
    fn completed_generation_is_not_repeated() {
        let mut publication = publication();
        assert!(publication_needs_generation(&publication).unwrap());
        publication.is_generated = Some(false);
        assert!(publication_needs_generation(&publication).unwrap());
        publication.is_generated = Some(true);
        assert!(!publication_needs_generation(&publication).unwrap());
        publication.published_at = Some(ISO8601::now());
        assert!(!publication_needs_generation(&publication).unwrap());
    }

    #[test]
    fn deleted_and_incomplete_published_content_cannot_be_regenerated() {
        let mut publication = publication();
        publication.published_at = Some(ISO8601::now());
        assert!(publication_needs_generation(&publication).is_err());
        publication.is_generated = Some(true);
        publication.deleted_at = Some(ISO8601::now());
        assert!(publication_needs_generation(&publication).is_err());
    }
}
