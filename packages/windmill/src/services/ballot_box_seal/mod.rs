// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Ballot box seals (VOTE-FREEZE). With the event's Ballot Box Seal Policy
//! set to Seal at close, closing an election creates one `pending` seal per
//! ballot box ([`on_close`], in the close transaction). Once the grace
//! period ends, [`seal::seal_box`] locks, hashes and signs each box, and
//! [`publish::publish_box`] posts the signed entry to the event's electoral
//! log and uploads the public seal record. The `cast_vote` guard refuses
//! every write to a box whose seal isn't pending.

pub mod deadline;
pub mod publish;
pub mod record;
pub mod seal;
pub mod sink;

use crate::postgres::ballot_box_seal::{
    insert_pending, ClosedBy, NewPendingSeal, BALLOT_BOX_SEALED_ERROR,
};
use crate::postgres::ballot_style::get_ballot_styles_by_elections;
use crate::postgres::election::get_election_by_id;
use crate::services::documents::upload_and_return_public_event_document;
use crate::services::election_event_status::get_election_status;
use crate::services::protocol_manager::get_protocol_manager;
use crate::services::users::{
    list_keycloak_enabled_users_by_area_id_and_authorized_elections, VoterMultiplicityColumn,
};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use b4::messages::message::Signer;
use chrono::{DateTime, Utc};
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::ballot::{
    BallotBoxSealPolicy, BallotStyle as SequentBallotStyle, DelegatedVotingPolicy,
    ElectionEventPresentation, WeightedVotingPolicy,
};
use sequent_core::ballot_codec::multi_ballot::votable_contests;
use sequent_core::serialization::deserialize_with_path::{deserialize_str, deserialize_value};
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::hasura::core::{ElectionEvent, VotingChannels};
use sequent_core::types::keycloak::{MAX_VOTE_WEIGHT, MIN_VOTE_WEIGHT};
use std::collections::{BTreeSet, HashMap};
use strand::backend::ristretto::RistrettoCtx;
use strand::signature::StrandSignatureSk;
use tempfile::NamedTempFile;
use tracing::{error, info, instrument};
use uuid::Uuid;

/// What a voter or a writer is told when the guard refuses a write to a
/// sealed ballot box.
pub const BALLOT_BOX_SEALED_MESSAGE: &str =
    "The ballot box is sealed: voting is closed and no ballot can be added, changed or deleted";

/// The presentation key of the event's Ballot Box Seal Policy.
const SEAL_POLICY_KEY: &str = "ballot_box_seal_policy";

/// Who or what closed voting, as the close hook records it on new seals. A
/// signed close is recorded by [`sink::PendingSealSink`] in the same
/// transaction, after the hook.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloseProvenance {
    /// A manual Stop Voting (or the effect of a signed request, which the
    /// sink then records as signed).
    User { username: Option<String> },
    /// A scheduled close, signed or not.
    Scheduled,
}

impl CloseProvenance {
    /// The provenance of a close by `user_id`: a person when there is one,
    /// else the scheduler (scheduled closes run without a user).
    pub fn of_actor(user_id: Option<&str>, username: Option<&str>) -> Self {
        match user_id {
            Some(_) => CloseProvenance::User {
                username: username.map(str::to_owned),
            },
            None => CloseProvenance::Scheduled,
        }
    }
}

impl From<CloseProvenance> for ClosedBy {
    fn from(provenance: CloseProvenance) -> Self {
        match provenance {
            CloseProvenance::User { username } => ClosedBy::User { username },
            CloseProvenance::Scheduled => ClosedBy::Scheduled,
        }
    }
}

/// The event's Ballot Box Seal Policy, read from its presentation like the
/// SQL `ballot_box_seal_policy()`: only `seal-at-close` seals; a missing or
/// unknown value, or a presentation that doesn't parse, doesn't.
pub fn seal_policy(election_event: &ElectionEvent) -> BallotBoxSealPolicy {
    election_event
        .presentation
        .as_ref()
        .and_then(|presentation| presentation.get(SEAL_POLICY_KEY))
        .and_then(|value| serde_json::from_value::<BallotBoxSealPolicy>(value.clone()).ok())
        .unwrap_or_default()
}

/// The event's presentation, read strictly: one that doesn't parse is an
/// error (the sealer retries), never the defaults.
pub fn strict_presentation(election_event: &ElectionEvent) -> Result<ElectionEventPresentation> {
    Ok(election_event
        .get_presentation()
        .map_err(|error| anyhow!("The election event presentation doesn't parse: {error}"))?
        .unwrap_or_default())
}

/// How long a call to immudb or the public bucket may take before the
/// attempt fails and the next one retries.
pub const REMOTE_CALL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

/// `call`, failing after [`REMOTE_CALL_TIMEOUT`].
pub async fn with_timeout<T>(
    what: &str,
    call: impl std::future::Future<Output = Result<T>>,
) -> Result<T> {
    tokio::time::timeout(REMOTE_CALL_TIMEOUT, call)
        .await
        .map_err(|_| anyhow!("{what} timed out after {}s", REMOTE_CALL_TIMEOUT.as_secs()))?
}

/// What kind of error left a seal pending: the code its `waiting_reason`
/// shows (`error:<code>`); the details only go to the logs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum_macros::Display)]
#[strum(serialize_all = "lowercase")]
pub enum SealErrorCategory {
    /// Reading or posting to the electoral log.
    Board,
    /// Reading the census.
    Census,
    /// Reading the event's signing key.
    Keystore,
    /// Uploading the public record.
    Storage,
    /// The event's or election's configuration doesn't parse.
    Settings,
    /// A ballot of the box can't be read.
    Ballots,
    /// The database.
    Database,
    Other,
}

/// The category of an error of the sealer: the one it was tagged with,
/// else `database` for a database error, else `other`.
pub fn error_category(error: &anyhow::Error) -> SealErrorCategory {
    if let Some(category) = error.downcast_ref::<SealErrorCategory>() {
        return *category;
    }
    if error
        .chain()
        .any(|cause| cause.downcast_ref::<tokio_postgres::Error>().is_some())
    {
        return SealErrorCategory::Database;
    }
    SealErrorCategory::Other
}

/// Whether `error` is the guard refusing a write to a sealed ballot box
/// (SQLSTATE 42501, message `ballot_box_sealed`).
pub fn is_ballot_box_sealed(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        cause
            .downcast_ref::<tokio_postgres::Error>()
            .and_then(|error| error.as_db_error())
            .is_some_and(|error| {
                error.code() == &tokio_postgres::error::SqlState::INSUFFICIENT_PRIVILEGE
                    && error.message() == BALLOT_BOX_SEALED_ERROR
            })
    })
}

/// Refuses to delete an election event with ballot box seals, in any
/// status: seals are permanent. Run it before any side effect of a delete
/// (the boards, the realm, documents), not only in the database delete.
pub async fn refuse_sealed_event_delete(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<()> {
    if crate::postgres::ballot_box_seal::any_for_event(
        hasura_transaction,
        &Uuid::parse_str(tenant_id)?,
        &Uuid::parse_str(election_event_id)?,
    )
    .await?
    {
        return Err(crate::postgres::election_event::SealedEventDeleteRefusal.into());
    }
    Ok(())
}

/// The sealed ballot boxes (election, area) that hold active (`valid` or
/// `in-progress`) ballots of the voter: a discard leaves those ballots as
/// they were sealed.
pub async fn voter_sealed_boxes(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    voter_id: &str,
) -> Result<Vec<(Uuid, Uuid)>> {
    hasura_transaction
        .query(
            "SELECT DISTINCT cast_vote.election_id, cast_vote.area_id
             FROM sequent_backend.cast_vote
             JOIN sequent_backend.ballot_box_seal AS seal
               ON seal.tenant_id = cast_vote.tenant_id
              AND seal.election_event_id = cast_vote.election_event_id
              AND seal.election_id = cast_vote.election_id
              AND seal.area_id = cast_vote.area_id
             WHERE cast_vote.tenant_id = $1 AND cast_vote.election_event_id = $2
               AND cast_vote.voter_id_string = $3
               AND cast_vote.status IN ('valid', 'in-progress')
               AND seal.status <> 'pending'
             ORDER BY 1, 2",
            &[tenant_id, election_event_id, &voter_id],
        )
        .await
        .context("Error listing the voter's sealed ballot boxes")?
        .into_iter()
        .map(|row| Ok((row.try_get(0)?, row.try_get(1)?)))
        .collect()
}

/// Whether the election has any ballot box seal, in any status.
pub async fn election_has_seals(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
) -> Result<bool> {
    crate::postgres::ballot_box_seal::any_for_election(
        hasura_transaction,
        &Uuid::parse_str(tenant_id)?,
        &Uuid::parse_str(election_event_id)?,
        &Uuid::parse_str(election_id)?,
    )
    .await
}

/// The seal deadline the ballot box's seal row fixed, if it has one.
pub async fn box_grace_deadline(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    election_id: &Uuid,
    area_id: &Uuid,
) -> Result<Option<DateTime<Utc>>> {
    Ok(hasura_transaction
        .query_opt(
            "SELECT grace_deadline FROM sequent_backend.ballot_box_seal
             WHERE tenant_id = $1 AND election_event_id = $2 AND election_id = $3
               AND area_id = $4",
            &[tenant_id, election_event_id, election_id, area_id],
        )
        .await
        .context("Error reading the ballot box's seal deadline")?
        .map(|row| row.try_get("grace_deadline"))
        .transpose()?)
}

/// The census of a ballot box: each enabled voter of the area authorized
/// for the election, with the weight its ballot counts with.
pub type Census = HashMap<String, u64>;

/// What sealing and publishing need besides the Hasura database and the
/// event's electoral log: the key that signs the log, the census, and the
/// public bucket. Production uses [`ProductionSealEnvironment`].
#[async_trait]
pub trait SealEnvironment: Send + Sync {
    /// The event's protocol manager key, which signs its electoral log as
    /// sender and system.
    async fn signing_key(
        &self,
        hasura_transaction: &Transaction<'_>,
        tenant_id: &str,
        election_event_id: &str,
        board: &str,
    ) -> Result<StrandSignatureSk>;

    /// The census of the area for the election (alias `election_alias`).
    async fn census(
        &self,
        election_event: &ElectionEvent,
        area_id: &str,
        election_alias: &str,
    ) -> Result<Census>;

    /// Uploads the public record `name` of the event with the fixed
    /// `document_id`, in `hasura_transaction`; returns the document id.
    async fn upload_record(
        &self,
        hasura_transaction: &Transaction<'_>,
        tenant_id: &str,
        election_event_id: &str,
        name: &str,
        json: &[u8],
        document_id: Uuid,
    ) -> Result<Uuid>;
}

/// The vault's protocol manager key, the Keycloak census and the public
/// bucket.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProductionSealEnvironment;

/// The media type of a public seal record.
const RECORD_MEDIA_TYPE: &str = "application/json";

#[async_trait]
impl SealEnvironment for ProductionSealEnvironment {
    async fn signing_key(
        &self,
        hasura_transaction: &Transaction<'_>,
        tenant_id: &str,
        election_event_id: &str,
        board: &str,
    ) -> Result<StrandSignatureSk> {
        Ok(get_protocol_manager::<RistrettoCtx>(
            hasura_transaction,
            tenant_id,
            Some(election_event_id),
            board,
        )
        .await?
        .get_signing_key()
        .clone())
    }

    async fn census(
        &self,
        election_event: &ElectionEvent,
        area_id: &str,
        election_alias: &str,
    ) -> Result<Census> {
        let multiplicity_column = multiplicity_column(&strict_presentation(election_event)?);
        let realm = get_event_realm(&election_event.tenant_id, &election_event.id);
        let users_file = NamedTempFile::new().context("Error creating the census file")?;
        {
            let mut keycloak_client: DbClient = crate::services::database::get_keycloak_pool()
                .await
                .get()
                .await
                .context("Error acquiring a keycloak connection")?;
            let keycloak_transaction = keycloak_client
                .transaction()
                .await
                .context("Error starting a keycloak transaction")?;
            list_keycloak_enabled_users_by_area_id_and_authorized_elections(
                &keycloak_transaction,
                &realm,
                area_id,
                election_alias,
                &users_file.path().to_path_buf(),
                multiplicity_column,
            )
            .await?;
        }
        census_from_csv(users_file.reopen()?, multiplicity_column)
    }

    async fn upload_record(
        &self,
        hasura_transaction: &Transaction<'_>,
        tenant_id: &str,
        election_event_id: &str,
        name: &str,
        json: &[u8],
        document_id: Uuid,
    ) -> Result<Uuid> {
        use std::io::Write;
        let mut file = NamedTempFile::new().context("Error creating the seal record file")?;
        file.write_all(json)?;
        file.flush()?;
        let file_path = file
            .path()
            .to_str()
            .ok_or_else(|| anyhow!("The seal record file path is not UTF-8"))?;
        let document = with_timeout(
            "Uploading the seal record",
            upload_and_return_public_event_document(
                hasura_transaction,
                file_path,
                json.len() as u64,
                RECORD_MEDIA_TYPE,
                tenant_id,
                election_event_id,
                name,
                Some(document_id.to_string()),
            ),
        )
        .await?;
        Ok(Uuid::parse_str(&document.id)?)
    }
}

/// Which multiplicity column the census reads, as the tally's
/// `insert_ballots` picks it, from the strictly read presentation.
pub fn multiplicity_column(presentation: &ElectionEventPresentation) -> VoterMultiplicityColumn {
    if presentation
        .delegated_voting_policy
        .clone()
        .unwrap_or_default()
        == DelegatedVotingPolicy::ENABLED
    {
        VoterMultiplicityColumn::DelegateCount
    } else if presentation
        .weighted_voting_policy
        .clone()
        .unwrap_or_default()
        == WeightedVotingPolicy::VOTERS_WEIGHTED_VOTING
    {
        VoterMultiplicityColumn::VoteWeight
    } else {
        VoterMultiplicityColumn::None
    }
}

/// The census from the Keycloak voter dump (voter id, then at most one
/// multiplicity column). A counted ballot's weight is the multiplicity the
/// tally's `merge_join_csv` gives it: 1, one plus the delegate count, or
/// the vote weight (within its bounds).
pub fn census_from_csv(
    reader: impl std::io::Read,
    multiplicity_column: VoterMultiplicityColumn,
) -> Result<Census> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(reader);
    let mut census = Census::new();
    for record in reader.records() {
        let record = record?;
        let voter_id = record
            .get(0)
            .ok_or_else(|| anyhow!("A census row has no voter id"))?
            .to_string();
        let multiplicity = || -> Result<u64> {
            let raw = record
                .get(1)
                .ok_or_else(|| anyhow!("A census row has no multiplicity column"))?;
            raw.trim()
                .parse()
                .map_err(|_| anyhow!("Invalid multiplicity {raw:?} in the census"))
        };
        let weight = match multiplicity_column {
            VoterMultiplicityColumn::None => 1,
            VoterMultiplicityColumn::DelegateCount => 1 + multiplicity()?,
            VoterMultiplicityColumn::VoteWeight => {
                let weight = multiplicity()?;
                anyhow::ensure!(
                    (MIN_VOTE_WEIGHT..=MAX_VOTE_WEIGHT).contains(&weight),
                    "Vote weight {weight} is out of range, must be between \
                     {MIN_VOTE_WEIGHT} and {MAX_VOTE_WEIGHT}"
                );
                weight
            }
        };
        census.insert(voter_id, weight);
    }
    Ok(census)
}

/// The event's published ballot styles of the given elections: the styles
/// of a generated or published ballot publication (what the tally reads).
pub async fn published_ballot_styles(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_ids: &[String],
) -> Result<Vec<SequentBallotStyle>> {
    published_style_rows(
        hasura_transaction,
        tenant_id,
        election_event_id,
        election_ids,
    )
    .await?
    .iter()
    .map(parse_style)
    .collect()
}

/// A published ballot style row's ballot style.
fn parse_style(
    style: &sequent_core::types::hasura::core::BallotStyle,
) -> Result<SequentBallotStyle> {
    let ballot_eml = style
        .ballot_eml
        .as_deref()
        .ok_or_else(|| anyhow!("Published ballot style {} has no ballot EML", style.id))?;
    deserialize_str(ballot_eml)
        .map_err(|error| anyhow!("Could not read ballot style {}: {error:?}", style.id))
}

/// The published ballot style rows of the given elections.
async fn published_style_rows(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_ids: &[String],
) -> Result<Vec<sequent_core::types::hasura::core::BallotStyle>> {
    let styles = get_ballot_styles_by_elections(
        hasura_transaction,
        tenant_id,
        election_event_id,
        &election_ids.to_vec(),
    )
    .await?;
    let published: std::collections::HashSet<String> = hasura_transaction
        .query(
            "SELECT id::text FROM sequent_backend.ballot_publication
             WHERE tenant_id = $1 AND election_event_id = $2
             AND (COALESCE(is_generated, false) OR published_at IS NOT NULL)",
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
            ],
        )
        .await?
        .into_iter()
        .map(|row| row.get(0))
        .collect();
    Ok(styles
        .into_iter()
        .filter(|style| published.contains(&style.ballot_publication_id))
        .collect())
}

/// The ballot boxes the published ballot styles define: one (election id,
/// area id) per style with a votable contest. The close hook and the tally
/// both use it.
pub fn expected_ballot_boxes(styles: &[SequentBallotStyle]) -> BTreeSet<(String, String)> {
    styles
        .iter()
        .filter(|style| votable_contests(&style.contests).next().is_some())
        .map(|style| (style.election_id.clone(), style.area_id.clone()))
        .collect()
}

/// The close hook: for each of `election_ids` whose event seals at close
/// and whose voting has finished ([`deadline::seal_deadline`]), one
/// `pending` seal per ballot box, with its close time and seal deadline.
/// Boxes that already have a seal keep it. Runs in the close transaction;
/// returns how many seals it created.
#[instrument(skip(hasura_transaction, election_event), err)]
pub async fn on_close(
    hasura_transaction: &Transaction<'_>,
    election_event: &ElectionEvent,
    election_ids: &[String],
    provenance: CloseProvenance,
) -> Result<u64> {
    if seal_policy(election_event) != BallotBoxSealPolicy::SEAL_AT_CLOSE {
        return Ok(0);
    }
    let tenant_id = election_event.tenant_id.as_str();
    let election_event_id = election_event.id.as_str();
    let tenant_uuid = Uuid::parse_str(tenant_id)?;
    let event_uuid = Uuid::parse_str(election_event_id)?;
    let closed_by = ClosedBy::from(provenance);
    let mut created = 0;
    for election_id in election_ids {
        let Some(election) = get_election_by_id(
            hasura_transaction,
            tenant_id,
            election_event_id,
            election_id,
        )
        .await?
        else {
            continue;
        };
        let status = get_election_status(election.status.clone()).unwrap_or_default();
        let channels: VotingChannels = election
            .voting_channels
            .clone()
            .map(deserialize_value)
            .transpose()
            .context("Failed to deserialize the election's voting channels")?
            .unwrap_or_default();
        let presentation = election.get_presentation().unwrap_or_default();
        let Some((closed_at, grace_deadline)) =
            deadline::seal_deadline(&status, &channels, &presentation)
        else {
            continue;
        };
        let election_uuid = Uuid::parse_str(election_id)?;
        let rows: Vec<NewPendingSeal> = election_areas(
            hasura_transaction,
            tenant_id,
            election_event_id,
            election_id,
        )
        .await?
        .into_iter()
        .map(|area_id| NewPendingSeal {
            tenant_id: tenant_uuid,
            election_event_id: event_uuid,
            election_id: election_uuid,
            area_id,
            closed_at,
            grace_deadline,
            close_request_id: None,
            closed_by: closed_by.clone(),
        })
        .collect();
        let inserted = insert_pending(hasura_transaction, &rows).await?;
        info!(
            %election_id,
            boxes = rows.len(),
            inserted,
            %grace_deadline,
            "Ballot boxes to seal after the close"
        );
        created += inserted;
    }
    Ok(created)
}

/// The areas of the election's ballot boxes: those of
/// [`expected_ballot_boxes`], plus every area that already has ballots of
/// the election. Empty boxes are sealed too. A published style that can't
/// be read never stops the close: its row's area counts as a box, and an
/// `error!` names the style (the tally's readiness reports it too).
async fn election_areas(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
) -> Result<BTreeSet<Uuid>> {
    let rows = published_style_rows(
        hasura_transaction,
        tenant_id,
        election_event_id,
        &[election_id.to_string()],
    )
    .await?;
    let mut styles = vec![];
    let mut areas = BTreeSet::new();
    for row in &rows {
        match parse_style(row) {
            Ok(style) => styles.push(style),
            Err(style_error) => {
                error!(
                    style_id = %row.id,
                    %election_id,
                    "Sealing the area of a ballot style that can't be read: {style_error:#}"
                );
                if let Some(area_id) = row
                    .area_id
                    .as_deref()
                    .and_then(|id| Uuid::parse_str(id).ok())
                {
                    areas.insert(area_id);
                }
            }
        }
    }
    for (_, area_id) in expected_ballot_boxes(&styles)
        .into_iter()
        .filter(|(election, _)| election == election_id)
    {
        match Uuid::parse_str(&area_id) {
            Ok(area_id) => {
                areas.insert(area_id);
            }
            Err(_) => error!(%election_id, %area_id, "A ballot style names an invalid area id"),
        }
    }
    let rows = hasura_transaction
        .query(
            "SELECT DISTINCT area_id FROM sequent_backend.cast_vote
             WHERE tenant_id = $1 AND election_event_id = $2 AND election_id = $3
               AND area_id IS NOT NULL",
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
                &Uuid::parse_str(election_id)?,
            ],
        )
        .await
        .context("Error listing the areas with ballots")?;
    for row in rows {
        areas.insert(row.try_get("area_id")?);
    }
    Ok(areas)
}
