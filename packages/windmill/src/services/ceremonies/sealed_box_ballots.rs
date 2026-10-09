// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The tally of a sealed ballot box (VOTE-FREEZE). When the event seals its
//! ballot boxes at close, the tally counts the box from its seal instead of
//! selecting ballots and voters itself: it checks the signed
//! `BallotBoxSealed` entry on the electoral log, the stored manifest, and
//! that the stored ballots are exactly the sealed ones, then counts the
//! manifest's entries. The manifest is authoritative, so a voter disabled
//! after the seal does not change the counts.

use crate::ports::tally_ceremony::BallotBoxSealState;
use crate::postgres::ballot_box_seal::{get_for_box, list_for_elections, BallotBoxSealStatus};
use crate::postgres::tally_session_contest::get_tally_session_contests;
use crate::services::ballot_box_seal::publish::{deliver, sealed_entries, sha256_hex};
use crate::services::ceremonies::tally_validation::{
    validate_sealed_boxes_tallied, ExpectedBallotBox,
};
use crate::services::election_event_board::get_election_event_board;
use crate::services::join::MergeJoinResult;
use crate::services::protocol_manager::{get_board_client, get_protocol_manager};
use anyhow::{anyhow, Context, Result};
use b4::messages::message::Signer;
use deadpool_postgres::Transaction;
use electoral_log::messages::message::{Message, SigningData};
use electoral_log::messages::newtypes::EventIdString;
use electoral_log::messages::statement::StatementType;
use electoral_log::seal::{
    ballot_hash, check_statement, compare_stored, BallotBoxSealManifest, SealDisposition,
};
use electoral_log::{
    ElectoralLogMessage, ElectoralLogVarCharColumn, SqlCompOperators, WhereClauseBTreeMap,
};
use futures::{pin_mut, TryStreamExt};
use sequent_core::ballot::{BallotBoxSealPolicy, WeightedVotingPolicy};
use sequent_core::types::ceremonies::TallyType;
use sequent_core::types::hasura::core::ElectionEvent;
use sequent_core::types::participation::{ParticipationChannel, VotesByChannel};
use sequent_core::util::retry::retry_with_exponential_backoff;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::time::Duration;
use strand::backend::ristretto::RistrettoCtx;
use strand::hash::Hash;
use strand::serialization::StrandDeserialize;
use strand::signature::{StrandSignaturePk, StrandSignatureSk};
use tracing::{error, info, instrument, warn};
use uuid::Uuid;

/// The event presentation key of the ballot box seal policy.
const BALLOT_BOX_SEAL_POLICY_KEY: &str = "ballot_box_seal_policy";
/// Prefix of the delivery id of a `TallyBallotBoxVerified` entry.
const VERIFIED_DELIVERY_PREFIX: &str = "tally-box-verified";
/// Prefix of the delivery id of a `TallyBallotBoxRejected` entry.
const REJECTED_DELIVERY_PREFIX: &str = "tally-box-rejected";
/// Whether an election has any `BallotBoxSealed` entry needs one row.
const SEAL_PRESENCE_READ_LIMIT: i64 = 1;
const BOARD_RETRIES: usize = 5;
const BOARD_INITIAL_BACKOFF: Duration = Duration::from_millis(100);

/// The event's ballot box seal policy: `SEAL_AT_CLOSE` only when the
/// presentation sets it, `DO_NOT_SEAL` otherwise. It reads the one field, the
/// same one the policy-lock trigger compares, so another presentation field
/// that does not parse can't turn the seal check off.
pub fn event_ballot_box_seal_policy(election_event: &ElectionEvent) -> BallotBoxSealPolicy {
    match election_event
        .presentation
        .as_ref()
        .and_then(|presentation| presentation.get(BALLOT_BOX_SEAL_POLICY_KEY))
        .cloned()
        .map(serde_json::from_value::<BallotBoxSealPolicy>)
    {
        Some(Ok(BallotBoxSealPolicy::SEAL_AT_CLOSE)) => BallotBoxSealPolicy::SEAL_AT_CLOSE,
        _ => BallotBoxSealPolicy::DO_NOT_SEAL,
    }
}

/// The event's electoral log: where the seals are and where the tally posts
/// what it found, with the key that signs both.
pub struct SealedBoxLog {
    board: String,
    system_pk: StrandSignaturePk,
    signing_data: SigningData,
}

impl SealedBoxLog {
    #[instrument(skip(hasura_transaction, election_event), err)]
    pub async fn new(
        hasura_transaction: &Transaction<'_>,
        tenant_id: &str,
        election_event: &ElectionEvent,
    ) -> Result<Self> {
        let board = get_election_event_board(election_event.bulletin_board_reference.clone())
            .with_context(|| "missing bulletin board")?;
        let protocol_manager = get_protocol_manager::<RistrettoCtx>(
            hasura_transaction,
            tenant_id,
            Some(&election_event.id),
            &board,
        )
        .await?;
        Self::with_signing_key(board, protocol_manager.get_signing_key().clone())
    }

    /// The electoral log `board`, signed with the event's key `system_sk`.
    pub fn with_signing_key(board: String, system_sk: StrandSignatureSk) -> Result<Self> {
        Ok(SealedBoxLog {
            board,
            system_pk: StrandSignaturePk::from_sk(&system_sk)?,
            signing_data: SigningData::new(system_sk.clone(), "", system_sk),
        })
    }
}

/// The ballot box a tally session counts, with the names its log entries
/// and errors show.
pub struct SealedBox<'a> {
    pub tenant_id: &'a str,
    pub election_event_id: &'a str,
    pub election_id: &'a str,
    pub area_id: &'a str,
    pub election_name: &'a str,
    pub area_name: &'a str,
    pub tally_session_id: &'a str,
}

/// A verified sealed ballot box, counted.
#[derive(Debug)]
pub struct SealedBoxBallots {
    pub merge_result: MergeJoinResult,
    /// Hex SHA-512 of the manifest.
    pub seal_hash: String,
}

/// A box that doesn't match its seal, and how.
struct Rejection(String);

/// What the checks established about a box that matches its seal.
struct VerifiedBox {
    manifest: BallotBoxSealManifest,
    seal_hash: Hash,
    /// The stored content of each counted (ballot hash, Ballot ID).
    contents: HashMap<(Hash, String), String>,
}

/// Checks the ballot box against its seal and counts it from the manifest.
/// A box that doesn't match gets a `TallyBallotBoxRejected` entry and an
/// error; one that matches gets one `TallyBallotBoxVerified` entry per tally
/// session, however many contests read it.
#[instrument(skip(hasura_transaction, log, ballot_box), fields(election_id = ballot_box.election_id, area_id = ballot_box.area_id), err)]
pub async fn sealed_box_ballots(
    hasura_transaction: &Transaction<'_>,
    log: &SealedBoxLog,
    ballot_box: &SealedBox<'_>,
    weighted_voting_policy: WeightedVotingPolicy,
) -> Result<SealedBoxBallots> {
    let verified = match verify_box(hasura_transaction, log, ballot_box).await? {
        Ok(verified) => verified,
        Err(Rejection(what_differs)) => {
            error!(
                election_id = ballot_box.election_id,
                area_id = ballot_box.area_id,
                "The ballot box does not match its seal: {what_differs}"
            );
            post_rejected(log, ballot_box, what_differs).await?;
            return Err(anyhow!(
                "The ballot box of {}, {} does not match its seal",
                ballot_box.election_name,
                ballot_box.area_name
            ));
        }
    };
    let VerifiedBox {
        manifest,
        seal_hash,
        contents,
    } = verified;
    let merge_result = count_manifest(&manifest, contents, weighted_voting_policy)?;
    post_verified(log, ballot_box, seal_hash, manifest.ballots_counted()).await?;
    info!(
        election_id = ballot_box.election_id,
        area_id = ballot_box.area_id,
        "The ballot box matches its seal"
    );
    Ok(SealedBoxBallots {
        merge_result,
        seal_hash: hex::encode(seal_hash),
    })
}

/// The checks of §2.8: one signed `BallotBoxSealed` entry, the stored
/// manifest is the signed one, and the stored ballots are the sealed ones.
/// The outer error is an operational failure (retry); the inner one, a box
/// that doesn't match its seal.
async fn verify_box(
    hasura_transaction: &Transaction<'_>,
    log: &SealedBoxLog,
    ballot_box: &SealedBox<'_>,
) -> Result<std::result::Result<VerifiedBox, Rejection>> {
    let tenant_uuid = Uuid::parse_str(ballot_box.tenant_id)?;
    let event_uuid = Uuid::parse_str(ballot_box.election_event_id)?;
    let election_uuid = Uuid::parse_str(ballot_box.election_id)?;
    let area_uuid = Uuid::parse_str(ballot_box.area_id)?;

    let seal = get_for_box(
        hasura_transaction,
        &tenant_uuid,
        &event_uuid,
        &election_uuid,
        &area_uuid,
    )
    .await?;
    let entries = with_board_retries(|| {
        sealed_entries(&log.board, ballot_box.election_id, ballot_box.area_id)
    })
    .await?;
    let signed = signed_seal_entries(&entries, &log.system_pk)?;
    // Without a published seal row the box isn't ready yet, unless the
    // bulletin board already holds its seal: then the row was deleted or
    // changed after the seal, which is a mismatch.
    let seal = match seal {
        Some(seal) if seal.status == BallotBoxSealStatus::Published => seal,
        seal => {
            let stored = match &seal {
                Some(seal) => format!("is {}", seal.status),
                None => "is missing".to_string(),
            };
            if !signed.is_empty() {
                return Ok(Err(Rejection(format!(
                    "the bulletin board holds the seal of this ballot box, but the stored seal {stored}"
                ))));
            }
            return Err(match seal {
                Some(seal) => anyhow!(
                    "The seal of the ballot box of {} is {}, not on the bulletin board yet",
                    ballot_box.area_name,
                    seal.status
                ),
                None => anyhow!("The ballot box of {} has no seal", ballot_box.area_name),
            });
        }
    };
    let Some(manifest_bytes) = seal.manifest else {
        return Ok(Err(Rejection("the stored seal has no manifest".into())));
    };

    let message = match signed.as_slice() {
        [message] => message,
        [] => {
            return Ok(Err(Rejection(
                "the bulletin board has no BallotBoxSealed entry signed by the event's key \
                 for this ballot box"
                    .into(),
            )))
        }
        several => {
            return Ok(Err(Rejection(format!(
                "the bulletin board has {} different BallotBoxSealed entries signed by the \
                 event's key for this ballot box",
                several.len()
            ))))
        }
    };

    let manifest = match BallotBoxSealManifest::from_bytes(&manifest_bytes) {
        Ok(manifest) => manifest,
        Err(error) => {
            return Ok(Err(Rejection(format!(
                "the stored manifest does not decode: {error}"
            ))))
        }
    };
    if manifest.tenant_id != ballot_box.tenant_id
        || manifest.election_event_id != ballot_box.election_event_id
        || manifest.election_id != ballot_box.election_id
        || manifest.area_id != ballot_box.area_id
    {
        return Ok(Err(Rejection(
            "the stored manifest is for another ballot box".into(),
        )));
    }
    let seal_hash = match check_statement(message, &manifest, &manifest_bytes) {
        Ok(seal_hash) => seal_hash,
        Err(error) => return Ok(Err(Rejection(error.to_string()))),
    };
    if let Err(rejection) = check_counted_weights(&manifest) {
        return Ok(Err(rejection));
    }

    let counted: HashSet<(Hash, String)> = manifest
        .entries
        .iter()
        .filter(|entry| entry.disposition == SealDisposition::Counted)
        .map(|entry| (entry.ballot_hash, entry.ballot_id.clone()))
        .collect();
    let stored = match read_stored_ballots(
        hasura_transaction,
        [&tenant_uuid, &event_uuid, &election_uuid, &area_uuid],
        &counted,
    )
    .await?
    {
        Ok(stored) => stored,
        Err(rejection) => return Ok(Err(rejection)),
    };
    let diff = compare_stored(&manifest, &stored.keys);
    if !diff.is_empty() {
        return Ok(Err(Rejection(diff.describe())));
    }

    Ok(Ok(VerifiedBox {
        manifest,
        seal_hash,
        contents: stored.contents,
    }))
}

/// The distinct `BallotBoxSealed` messages among a box's log entries that
/// the event's key signed, as sender and system. Copies of the same bytes
/// count once; an entry that doesn't decode or isn't signed by the key can't
/// be a seal, so it is reported and ignored (anyone who can write the log
/// could add one, and it must not block the box's tally).
fn signed_seal_entries(
    entries: &[ElectoralLogMessage],
    system_pk: &StrandSignaturePk,
) -> Result<Vec<Message>> {
    let system_der = system_pk.to_der()?;
    let mut seen: HashSet<&[u8]> = HashSet::new();
    let mut signed = Vec::new();
    for entry in entries {
        if !seen.insert(entry.message.as_slice()) {
            continue;
        }
        let message = match Message::strand_deserialize(&entry.message) {
            Ok(message) => message,
            Err(error) => {
                warn!(
                    entry_id = entry.id,
                    "Ignoring a BallotBoxSealed entry that does not decode: {error}"
                );
                continue;
            }
        };
        if let Err(error) = message.verify(system_pk) {
            warn!(
                entry_id = entry.id,
                "Ignoring a BallotBoxSealed entry not signed by the event's key: {error}"
            );
            continue;
        }
        if message.sender.pk.to_der()? != system_der {
            warn!(
                entry_id = entry.id,
                "Ignoring a BallotBoxSealed entry whose sender is not the event's key"
            );
            continue;
        }
        signed.push(message);
    }
    Ok(signed)
}

/// A counted ballot enters the count `weight` times, so a weight of 0
/// would drop it while the seal says it counts.
fn check_counted_weights(manifest: &BallotBoxSealManifest) -> std::result::Result<(), Rejection> {
    match manifest
        .entries
        .iter()
        .find(|entry| entry.disposition == SealDisposition::Counted && entry.weight == 0)
    {
        Some(entry) => Err(Rejection(format!(
            "Ballot ID {} is counted with a weight of 0",
            entry.ballot_id
        ))),
        None => Ok(()),
    }
}

/// Runs a bulletin board call, retrying a transient failure.
async fn with_board_retries<T, F, Fut>(call: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    retry_with_exponential_backoff(call, BOARD_RETRIES, BOARD_INITIAL_BACKOFF).await
}

/// Whether the event's electoral log holds a `BallotBoxSealed` entry for
/// the election.
async fn election_has_seal_entries(board: &str, election_id: &str) -> Result<bool> {
    let columns_matcher: WhereClauseBTreeMap = [
        (
            ElectoralLogVarCharColumn::StatementKind,
            (
                SqlCompOperators::Equal,
                StatementType::BallotBoxSealed.to_string(),
            ),
        ),
        (
            ElectoralLogVarCharColumn::ElectionId,
            (SqlCompOperators::Equal, election_id.to_string()),
        ),
    ]
    .into_iter()
    .collect();
    let entries = with_board_retries(|| async {
        get_board_client()
            .await?
            .get_electoral_log_messages_filtered::<String, String>(
                board,
                Some(columns_matcher.clone()),
                None,
                None,
                Some(SEAL_PRESENCE_READ_LIMIT),
                None,
                None,
            )
            .await
    })
    .await
    .with_context(|| "Error reading the election's seals from the electoral log")?;
    Ok(!entries.is_empty())
}

/// The elections of an electoral results tally that are counted from their
/// seals: every one when the event seals at close, and otherwise each whose
/// ballot boxes were sealed anyway (a policy turned off in the database after
/// the seal can't skip the check, since the board keeps the seal).
/// Initialization reports count no ballots and never use the seals.
pub async fn sealed_elections(
    election_event: &ElectionEvent,
    tally_type: &TallyType,
    election_ids: &HashSet<String>,
) -> Result<HashSet<String>> {
    if *tally_type != TallyType::ELECTORAL_RESULTS {
        return Ok(HashSet::new());
    }
    if event_ballot_box_seal_policy(election_event) == BallotBoxSealPolicy::SEAL_AT_CLOSE {
        return Ok(election_ids.clone());
    }
    // Without an electoral log there is no seal to find.
    let Some(board) = get_election_event_board(election_event.bulletin_board_reference.clone())
    else {
        return Ok(HashSet::new());
    };
    let mut sealed = HashSet::new();
    for election_id in election_ids {
        if election_has_seal_entries(&board, election_id).await? {
            sealed.insert(election_id.clone());
        }
    }
    Ok(sealed)
}

/// Refuses to count a tally session that leaves out a sealed ballot box with
/// ballots of `election_ids`: its area has no contest of the session, so its
/// ballots would silently be missing from the results. The names map ids to
/// the names the refusal shows.
pub async fn check_sealed_boxes_tallied(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_ids: &HashSet<String>,
    tally_session_id: &str,
    election_names: &HashMap<String, String>,
    area_names: &HashMap<String, String>,
) -> Result<()> {
    let election_uuids = election_ids
        .iter()
        .map(|id| Uuid::parse_str(id))
        .collect::<Result<Vec<_>, _>>()?;
    let seals: Vec<BallotBoxSealState> = list_for_elections(
        hasura_transaction,
        &Uuid::parse_str(tenant_id)?,
        &Uuid::parse_str(election_event_id)?,
        &election_uuids,
    )
    .await?
    .into_iter()
    .map(|seal| BallotBoxSealState {
        election_id: seal.election_id.to_string(),
        area_id: seal.area_id.to_string(),
        status: seal.status,
        ballots_in_box: seal.ballots_in_box,
    })
    .collect();
    let tallied: BTreeSet<(String, String)> = get_tally_session_contests(
        hasura_transaction,
        tenant_id,
        election_event_id,
        tally_session_id,
    )
    .await?
    .into_iter()
    .map(|contest| (contest.election_id, contest.area_id))
    .collect();
    let name = |names: &HashMap<String, String>, id: &str| {
        names.get(id).cloned().unwrap_or_else(|| id.to_string())
    };
    let boxes: Vec<ExpectedBallotBox> = seals
        .iter()
        .map(|seal| ExpectedBallotBox {
            election_id: seal.election_id.clone(),
            election_name: name(election_names, &seal.election_id),
            area_id: seal.area_id.clone(),
            area_name: name(area_names, &seal.area_id),
        })
        .collect();
    validate_sealed_boxes_tallied(&boxes, &seals, &tallied)?;
    Ok(())
}

/// Every stored ballot of a box, as the manifest identifies them.
struct StoredBallots {
    /// (ballot hash, Ballot ID) of every row, in any status.
    keys: Vec<(Hash, String)>,
    /// The content of the rows the manifest counts, once per key.
    contents: HashMap<(Hash, String), String>,
}

/// Streams the box's `cast_vote` rows (every status), keeping the content
/// only of the ballots the manifest counts.
async fn read_stored_ballots(
    hasura_transaction: &Transaction<'_>,
    ballot_box: [&Uuid; 4],
    counted: &HashSet<(Hash, String)>,
) -> Result<std::result::Result<StoredBallots, Rejection>> {
    let rows = hasura_transaction
        .query_raw(
            "SELECT content, ballot_id FROM sequent_backend.cast_vote
             WHERE tenant_id = $1 AND election_event_id = $2
               AND election_id = $3 AND area_id = $4",
            ballot_box,
        )
        .await
        .with_context(|| "Error reading the ballot box's cast votes")?;
    pin_mut!(rows);
    let mut stored = StoredBallots {
        keys: Vec::new(),
        contents: HashMap::new(),
    };
    while let Some(row) = rows.try_next().await? {
        let content: Option<String> = row.try_get("content")?;
        let ballot_id: Option<String> = row.try_get("ballot_id")?;
        let (Some(content), Some(ballot_id)) = (content, ballot_id) else {
            return Ok(Err(Rejection(
                "a stored ballot has no content or no Ballot ID".into(),
            )));
        };
        let key = (ballot_hash(&content)?, ballot_id);
        if counted.contains(&key) && !stored.contents.contains_key(&key) {
            stored.contents.insert(key.clone(), content);
        }
        stored.keys.push(key);
    }
    Ok(Ok(stored))
}

/// What the ballot dump reports, from the manifest, with the semantics of
/// `merge_join_csv`: a counted ballot enters the mix `weight` times and
/// counts `weight` towards its channel and towards the cast ballots (one,
/// under voter-weighted voting, where the weight isn't a number of voters);
/// a ballot of a voter who is not eligible counts once as cast, once towards
/// its channel and once as without voter; replaced and discarded ballots
/// don't count.
fn count_manifest(
    manifest: &BallotBoxSealManifest,
    mut contents: HashMap<(Hash, String), String>,
    weighted_voting_policy: WeightedVotingPolicy,
) -> Result<MergeJoinResult> {
    // How many counted entries still need each content: the last one takes
    // it, the others (identical entries, never in practice) copy it.
    let mut uses: HashMap<(Hash, String), usize> = HashMap::new();
    for entry in &manifest.entries {
        if entry.disposition == SealDisposition::Counted {
            *uses
                .entry((entry.ballot_hash, entry.ballot_id.clone()))
                .or_default() += 1;
        }
    }
    let mut ballot_contents = Vec::with_capacity(uses.values().sum());
    let mut ballots_without_voter: u64 = 0;
    let mut casted_ballots: u64 = 0;
    let mut casted_ballots_by_channel = VotesByChannel::new();
    for entry in &manifest.entries {
        match entry.disposition {
            SealDisposition::Counted => {
                let key = (entry.ballot_hash, entry.ballot_id.clone());
                let not_stored = || {
                    anyhow!(
                        "The counted ballot {} of the seal is not stored",
                        entry.ballot_id
                    )
                };
                let remaining = uses.get_mut(&key).ok_or_else(not_stored)?;
                *remaining -= 1;
                let content = if *remaining == 0 {
                    contents.remove(&key)
                } else {
                    contents.get(&key).cloned()
                }
                .ok_or_else(not_stored)?;
                ballot_contents.push((content, entry.weight));
                casted_ballots += match weighted_voting_policy {
                    WeightedVotingPolicy::VOTERS_WEIGHTED_VOTING => 1,
                    _ => entry.weight,
                };
                *casted_ballots_by_channel
                    .entry(ParticipationChannel::from(entry.channel.as_str()))
                    .or_default() += entry.weight;
            }
            SealDisposition::NotEligible => {
                ballots_without_voter += 1;
                casted_ballots += 1;
                *casted_ballots_by_channel
                    .entry(ParticipationChannel::from(entry.channel.as_str()))
                    .or_default() += 1;
            }
            SealDisposition::Replaced | SealDisposition::Discarded => {}
        }
    }
    Ok(MergeJoinResult {
        ballot_contents,
        eligible_voters: manifest.eligible_voters,
        ballots_without_voter,
        casted_ballots,
        casted_ballots_by_channel,
    })
}

async fn post_verified(
    log: &SealedBoxLog,
    ballot_box: &SealedBox<'_>,
    seal_hash: Hash,
    counted: u64,
) -> Result<()> {
    let message = Message::tally_ballot_box_verified_message(
        EventIdString(ballot_box.election_event_id.to_string()),
        ballot_box.election_id,
        ballot_box.area_id,
        ballot_box.election_name,
        ballot_box.area_name,
        seal_hash,
        counted,
        ballot_box.tally_session_id.to_string(),
        &log.signing_data,
    )?;
    let delivery = format!(
        "{VERIFIED_DELIVERY_PREFIX}:{}:{}:{}",
        ballot_box.tally_session_id, ballot_box.election_id, ballot_box.area_id
    );
    let payload = format!("{}:{counted}", hex::encode(seal_hash));
    deliver_once(&log.board, &delivery, &payload, &message).await
}

async fn post_rejected(
    log: &SealedBoxLog,
    ballot_box: &SealedBox<'_>,
    what_differs: String,
) -> Result<()> {
    // The difference is part of the delivery id: a rerun that finds the
    // same difference posts nothing new, one that finds another posts it.
    let delivery = format!(
        "{REJECTED_DELIVERY_PREFIX}:{}:{}:{}:{what_differs}",
        ballot_box.tally_session_id, ballot_box.election_id, ballot_box.area_id
    );
    let payload = what_differs.clone();
    let message = Message::tally_ballot_box_rejected_message(
        EventIdString(ballot_box.election_event_id.to_string()),
        ballot_box.election_id,
        ballot_box.area_id,
        ballot_box.election_name,
        ballot_box.area_name,
        what_differs,
        ballot_box.tally_session_id.to_string(),
        &log.signing_data,
    )?;
    deliver_once(&log.board, &delivery, &payload, &message).await
}

/// Posts `message` once per `delivery`: a repeat with the same `payload` is
/// a no-op.
async fn deliver_once(board: &str, delivery: &str, payload: &str, message: &Message) -> Result<()> {
    let delivery_id = sha256_hex(delivery.as_bytes());
    let payload_hash = sha256_hex(payload.as_bytes());
    with_board_retries(|| deliver(board, &delivery_id, &payload_hash, message))
        .await
        .with_context(|| format!("Error posting to the electoral log of board {board}"))?;
    Ok(())
}

#[cfg(test)]
#[path = "sealed_box_ballots_tests.rs"]
mod tests;
