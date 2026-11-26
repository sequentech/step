// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::publication_files;
use crate::postgres::area::get_event_areas;
use crate::postgres::area_contest::export_area_contests;
use crate::postgres::ballot_publication::get_ballot_publication_by_id;
use crate::postgres::ballot_style::{insert_ballot_styles, NewBallotStyle};
use crate::postgres::candidate::export_candidates;
use crate::postgres::contest::export_contests;
use crate::postgres::election::export_elections;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::keys_ceremony::get_keys_ceremonies;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::database::get_hasura_pool;
use crate::services::datafix::utils::remove_datafix_annotations;
use crate::services::election_dates::get_election_dates;
use crate::types::error::{Error, Result};
use anyhow::{anyhow, Context, Result as AnyhowResult};
use chrono::Duration;
use deadpool_postgres::{Client as DbClient, Transaction};
use futures::try_join;
use rocket::http::Status;
use sequent_core::ballot::{
    ContestEncryptionPolicy, ElectionEventPresentation, StringifiedPeriodDates,
};
use sequent_core::ballot_codec::multi_ballot::BallotChoices;
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

/// The event's rows, indexed once per generation so that building an area's
/// ballot styles only reads the rows of that area.
struct GenerationIndex<'a> {
    publication_elections: HashSet<&'a str>,
    elections: HashMap<&'a str, &'a Election>,
    contests: HashMap<&'a str, &'a Contest>,
    candidates_by_contest: HashMap<&'a str, Vec<&'a Candidate>>,
    contests_by_area: HashMap<&'a str, Vec<&'a str>>,
    /// Each area with its ancestors, from the root down.
    area_paths: HashMap<&'a str, Vec<&'a str>>,
}

impl<'a> GenerationIndex<'a> {
    fn new(
        ballot_publication: &'a BallotPublication,
        elections: &'a [Election],
        contests: &'a [Contest],
        candidates: &'a [Candidate],
        area_contests: &'a [AreaContest],
        areas_tree: &'a TreeNode,
    ) -> Self {
        let mut candidates_by_contest: HashMap<&str, Vec<&Candidate>> = HashMap::new();
        for candidate in candidates {
            if let Some(contest_id) = candidate.contest_id.as_deref() {
                candidates_by_contest
                    .entry(contest_id)
                    .or_default()
                    .push(candidate);
            }
        }
        let mut contests_by_area: HashMap<&str, Vec<&str>> = HashMap::new();
        for area_contest in area_contests {
            contests_by_area
                .entry(area_contest.area_id.as_str())
                .or_default()
                .push(area_contest.contest_id.as_str());
        }
        let mut area_paths = HashMap::new();
        let mut pending: Vec<(&TreeNode, Vec<&str>)> = vec![(areas_tree, Vec::new())];
        while let Some((node, mut path)) = pending.pop() {
            if let Some(area) = &node.area {
                path.push(area.id.as_str());
                area_paths.insert(area.id.as_str(), path.clone());
            }
            for child in &node.children {
                pending.push((child, path.clone()));
            }
        }
        GenerationIndex {
            publication_elections: ballot_publication
                .election_ids
                .iter()
                .flatten()
                .map(String::as_str)
                .collect(),
            elections: elections
                .iter()
                .map(|election| (election.id.as_str(), election))
                .collect(),
            contests: contests
                .iter()
                .map(|contest| (contest.id.as_str(), contest))
                .collect(),
            candidates_by_contest,
            contests_by_area,
            area_paths,
        }
    }

    /// The contests of the publication's elections that apply to the area,
    /// including those of its ancestors, grouped by election.
    fn election_contests(&self, area_id: &str) -> AnyhowResult<HashMap<&'a str, HashSet<&'a str>>> {
        if self.publication_elections.is_empty() {
            return Err(anyhow!("No election ids"));
        }
        let path = self
            .area_paths
            .get(area_id)
            .ok_or(anyhow!("area not found in tree"))?;
        let mut election_contests: HashMap<&str, HashSet<&str>> = HashMap::new();
        for contest_id in path
            .iter()
            .filter_map(|area| self.contests_by_area.get(area))
            .flatten()
        {
            let Some(contest) = self.contests.get(contest_id) else {
                event!(
                    Level::INFO,
                    "missing contest {} for area {}",
                    contest_id,
                    area_id
                );
                continue;
            };
            if !self
                .publication_elections
                .contains(contest.election_id.as_str())
            {
                continue;
            }
            election_contests
                .entry(contest.election_id.as_str())
                .or_default()
                .insert(contest.id.as_str());
        }
        Ok(election_contests)
    }

    fn candidates(&self, contest_ids: &HashSet<&str>) -> Vec<Candidate> {
        contest_ids
            .iter()
            .filter_map(|contest_id| self.candidates_by_contest.get(contest_id))
            .flatten()
            .map(|candidate| (*candidate).clone())
            .collect()
    }
}

/// What the voters of one election see: its dates and its public key.
struct ElectionInputs {
    dates: StringifiedPeriodDates,
    public_key: Option<String>,
}

fn election_inputs(
    elections: &[Election],
    scheduled_events: &[ScheduledEvent],
    keys_ceremonies: &[KeysCeremony],
) -> HashMap<String, ElectionInputs> {
    elections
        .iter()
        .map(|election| {
            let public_key = election.keys_ceremony_id.as_ref().and_then(|id| {
                keys_ceremonies
                    .iter()
                    .find(|keys_ceremony| &keys_ceremony.id == id)
                    .and_then(|keys_ceremony| keys_ceremony.status().ok())
                    .and_then(|status| status.public_key)
            });
            let inputs = ElectionInputs {
                dates: get_election_dates(election, scheduled_events.to_vec()).unwrap_or_default(),
                public_key,
            };
            (election.id.clone(), inputs)
        })
        .collect()
}

/// Builds the ballot style of each of the publication's elections that has
/// contests in the area.
fn area_ballot_styles(
    area: &Area,
    index: &GenerationIndex<'_>,
    voter_election_event: &ElectionEvent,
    all_contests: &[Contest],
    election_inputs: &HashMap<String, ElectionInputs>,
) -> Result<Vec<NewBallotStyle>> {
    let mut styles = Vec::new();
    for (election_id, contest_ids) in index.election_contests(&area.id)? {
        let election = index
            .elections
            .get(election_id)
            .ok_or(anyhow!("election id not found {}", election_id))?;
        let inputs = election_inputs
            .get(election_id)
            .ok_or(anyhow!("election id not found {}", election_id))?;
        let contests = contest_ids
            .iter()
            .map(|contest_id| {
                index
                    .contests
                    .get(contest_id)
                    .map(|contest| (*contest).clone())
                    .ok_or(Error::String(format!("Can't find contest {}", contest_id)))
            })
            .collect::<Result<Vec<Contest>>>()?;

        let ballot_style_id = Uuid::new_v4();
        let election_dto = sequent_core::ballot_style::create_ballot_style(
            ballot_style_id.to_string(),
            area.clone(),
            voter_election_event.clone(),
            (*election).clone(),
            contests,
            all_contests,
            index.candidates(&contest_ids),
            inputs.dates.clone(),
            inputs.public_key.clone(),
        )?;

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
    insert_publication_ballot_styles(
        &transaction,
        tenant_id,
        election_event_id,
        &ballot_publication,
    )
    .await?;
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

/// Builds and inserts the ballot style of every area and election of the
/// publication, in the caller's transaction.
async fn insert_publication_ballot_styles(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    ballot_publication: &BallotPublication,
) -> AnyhowResult<()> {
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
        .iter()
        .map(|election| (election.id.clone(), election.clone()))
        .collect();
    super::slates::validate_elections_slates(
        &ballot_publication.election_ids.clone().unwrap_or_default(),
        &elections_map,
        &contests,
        &candidates,
    )?;

    let basic_areas = areas.iter().map(|area| area.into()).collect();
    let areas_tree = TreeNode::from_areas(basic_areas)?;
    let index = GenerationIndex::new(
        ballot_publication,
        &elections,
        &contests,
        &candidates,
        &area_contests,
        &areas_tree,
    );
    let election_inputs = election_inputs(&elections, &scheduled_events, &keys_ceremonies);
    let mut voter_election_event = election_event.clone();
    remove_datafix_annotations(voter_election_event.annotations.as_mut());

    let tenant = Uuid::parse_str(tenant_id)?;
    let event = Uuid::parse_str(election_event_id)?;
    let publication = Uuid::parse_str(&ballot_publication.id)?;
    let mut batch = Vec::with_capacity(INSERT_BATCH_SIZE);
    for area in &areas {
        batch.extend(area_ballot_styles(
            area,
            &index,
            &voter_election_event,
            &contests,
            &election_inputs,
        )?);
        if batch.len() >= INSERT_BATCH_SIZE {
            insert_ballot_styles(transaction, tenant, event, publication, &batch).await?;
            batch.clear();
        }
    }
    if !batch.is_empty() {
        insert_ballot_styles(transaction, tenant, event, publication, &batch).await?;
    }
    Ok(())
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

    fn row<T: serde::de::DeserializeOwned>(fields: serde_json::Value) -> T {
        let mut row = serde_json::json!({"tenant_id": "tenant", "election_event_id": "event"});
        row.as_object_mut()
            .unwrap()
            .extend(fields.as_object().unwrap().clone());
        serde_json::from_value(row).unwrap()
    }

    /// Areas `root` > `child` > `leaf` and `other`; the publication holds
    /// elections `e1` and `e2` but not `e3`.
    struct Event {
        publication: BallotPublication,
        elections: Vec<Election>,
        contests: Vec<Contest>,
        candidates: Vec<Candidate>,
        area_contests: Vec<AreaContest>,
        tree: TreeNode,
    }

    fn event() -> Event {
        let mut publication = publication();
        publication.election_ids = Some(vec!["e1".into(), "e2".into()]);
        let areas: Vec<Area> = [
            ("root", None),
            ("child", Some("root")),
            ("leaf", Some("child")),
            ("other", None),
        ]
        .into_iter()
        .map(|(id, parent)| row(serde_json::json!({"id": id, "parent_id": parent})))
        .collect();
        let contest = |id: &str, election: &str| -> Contest {
            row(serde_json::json!({"id": id, "election_id": election}))
        };
        let area_contest = |area: &str, contest: &str| AreaContest {
            id: format!("{area}-{contest}"),
            area_id: area.into(),
            contest_id: contest.into(),
        };
        Event {
            publication,
            elections: ["e1", "e2", "e3"]
                .into_iter()
                .map(|id| row(serde_json::json!({ "id": id })))
                .collect(),
            contests: vec![
                contest("root-e1", "e1"),
                contest("leaf-e1", "e1"),
                contest("child-e2", "e2"),
                contest("leaf-e3", "e3"),
            ],
            candidates: [
                ("a", Some("root-e1")),
                ("b", Some("leaf-e1")),
                ("c", Some("child-e2")),
                ("d", None),
            ]
            .into_iter()
            .map(|(id, contest)| row(serde_json::json!({"id": id, "contest_id": contest})))
            .collect(),
            area_contests: vec![
                area_contest("root", "root-e1"),
                area_contest("leaf", "leaf-e1"),
                area_contest("child", "child-e2"),
                area_contest("leaf", "leaf-e3"),
                area_contest("leaf", "removed"),
            ],
            tree: TreeNode::from_areas(areas.iter().map(|area| area.into()).collect()).unwrap(),
        }
    }

    fn index(event: &Event) -> GenerationIndex<'_> {
        GenerationIndex::new(
            &event.publication,
            &event.elections,
            &event.contests,
            &event.candidates,
            &event.area_contests,
            &event.tree,
        )
    }

    fn sorted<'a>(ids: impl IntoIterator<Item = &'a str>) -> Vec<&'a str> {
        let mut ids: Vec<&str> = ids.into_iter().collect();
        ids.sort();
        ids
    }

    #[test]
    fn an_area_gets_its_own_and_its_ancestors_contests_of_the_publication() {
        let event = event();
        let contests = index(&event).election_contests("leaf").unwrap();
        assert_eq!(sorted(contests.keys().copied()), ["e1", "e2"]);
        assert_eq!(
            sorted(contests["e1"].iter().copied()),
            ["leaf-e1", "root-e1"]
        );
        assert_eq!(sorted(contests["e2"].iter().copied()), ["child-e2"]);

        let contests = index(&event).election_contests("child").unwrap();
        assert_eq!(sorted(contests["e1"].iter().copied()), ["root-e1"]);
        assert!(index(&event).election_contests("other").unwrap().is_empty());
    }

    #[test]
    fn an_area_outside_the_tree_or_a_publication_without_elections_is_an_error() {
        let mut event = event();
        let error = index(&event).election_contests("unknown").unwrap_err();
        assert_eq!(error.to_string(), "area not found in tree");

        event.publication.election_ids = Some(vec![]);
        let error = index(&event).election_contests("leaf").unwrap_err();
        assert_eq!(error.to_string(), "No election ids");
    }

    #[test]
    fn a_ballot_style_gets_the_candidates_of_its_contests_only() {
        let event = event();
        let contests = HashSet::from(["root-e1", "leaf-e1"]);
        let candidates = index(&event).candidates(&contests);
        assert_eq!(
            sorted(candidates.iter().map(|candidate| candidate.id.as_str())),
            ["a", "b"]
        );
    }
}
