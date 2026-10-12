// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The ballots of a tally session's contest areas, as its tally boards carry
//! them: the last valid vote of every eligible voter, split into the weight
//! batches of the area.

use crate::postgres::area::get_event_areas;
use crate::postgres::cast_vote::count_unresolved_cast_votes;
use crate::postgres::election_event::get_election_event_by_id;
use crate::services::cast_votes::find_area_ballots;
use crate::services::celery_app::get_worker_threads;
use crate::services::ceremonies::batch_anonymity::{
    enforce_batch_anonymity, session_divergences, small_batch, PostedBatch,
};
use crate::services::ceremonies::sealed_box_ballots::{
    check_sealed_boxes_tallied, sealed_box_ballots, sealed_elections, SealedBox, SealedBoxLog,
};
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::services::election::{get_election_event_elections, ElectionHead};
use crate::services::join::merge_join_csv;
use crate::services::join::read_valid_ballot_voter_ids;
use crate::services::join::{MergeJoinResult, MultiplicitySource};
use crate::services::users::{
    list_keycloak_enabled_users_by_area_id_and_authorized_elections, VoterMultiplicityColumn,
};
use crate::services::weight_batches::weight_batch_offsets;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use protocol_board::BallotCiphertext;
use sequent_core::ballot::{
    BatchAnonymityPolicy, ContestEncryptionPolicy, DelegatedVotingPolicy, HashableBallot,
    WeightedVotingPolicy,
};
use sequent_core::multi_ballot::HashableMultiBallot;
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::services::uuid_validation::parse_uuid_v4;
use sequent_core::types::ceremonies::TallyType;
use sequent_core::types::hasura::core::TallySession;
use sequent_core::types::hasura::core::{TallySessionContest, TallySessionContestAnnotations};
use sequent_core::types::keycloak::{MAX_TOTAL_VOTE_WEIGHT, VOTE_WEIGHT_BATCHES};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tempfile::NamedTempFile;
use tracing::{event, instrument, Level};

fn election_voter_authorization_aliases(
    elections: impl IntoIterator<Item = (String, Option<String>)>,
) -> HashMap<String, String> {
    elections
        .into_iter()
        .map(|(id, external_id)| {
            // Match Keycloak's authorization mapper for missing external IDs.
            let alias = external_id
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| id.clone());
            (id, alias)
        })
        .collect()
}

/// The annotations of a dumped contest area: its counts, the batches its
/// weights fill and, for a sealed ballot box, the seal it was counted from.
pub fn contest_annotations(
    merge_result: &MergeJoinResult,
    weight_bit_mask: Option<u32>,
    ballot_box_seal_hash: Option<String>,
) -> TallySessionContestAnnotations {
    TallySessionContestAnnotations {
        elegible_voters: merge_result.eligible_voters,
        ballots_without_voter: merge_result.ballots_without_voter,
        casted_ballots: merge_result.casted_ballots,
        votes_by_channel: Some(merge_result.casted_ballots_by_channel.clone()),
        weight_bit_mask,
        ballot_box_seal_hash,
    }
}

/// What counting a sealed ballot box needs, shared by the contest tasks.
pub struct SealedBoxes {
    log: SealedBoxLog,
    /// The elections counted from their seals.
    election_ids: HashSet<String>,
    election_names: HashMap<String, String>,
    area_names: HashMap<String, String>,
}

/// What the execution of a tally session needs to count its sealed ballot
/// boxes, or `None` when it counts none. Electoral results of an event that
/// seals its ballot boxes at close, or whose boxes were sealed, are counted
/// from the seals; everything else selects ballots and voters as it always
/// has. Refuses a session that leaves out a sealed box with ballots: its
/// contests can change between the creation and the execution.
pub async fn sealed_boxes_for_execution(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    tally_session: &TallySession,
    event_elections: &[ElectionHead],
) -> Result<Option<SealedBoxes>> {
    let tally_type = tally_session
        .tally_type
        .as_deref()
        .map(|tally_type| TallyType::try_from(tally_type).unwrap_or_default())
        .unwrap_or_default();
    let election_event =
        get_election_event_by_id(hasura_transaction, tenant_id, election_event_id).await?;
    let session_election_ids: HashSet<String> = tally_session
        .election_ids
        .clone()
        .unwrap_or_default()
        .into_iter()
        .collect();
    let election_ids =
        sealed_elections(&election_event, &tally_type, &session_election_ids).await?;
    if election_ids.is_empty() {
        return Ok(None);
    }
    let sealed_boxes = SealedBoxes {
        log: SealedBoxLog::new(hasura_transaction, tenant_id, &election_event).await?,
        election_ids,
        election_names: event_elections
            .iter()
            .map(|election| (election.id.clone(), election.name.clone()))
            .collect(),
        area_names: get_event_areas(hasura_transaction, tenant_id, election_event_id)
            .await?
            .into_iter()
            .map(|area| {
                let name = area.name.unwrap_or_else(|| area.id.clone());
                (area.id, name)
            })
            .collect(),
    };
    check_sealed_boxes_tallied(
        hasura_transaction,
        tenant_id,
        election_event_id,
        &sealed_boxes.election_ids,
        &tally_session.id,
        &sealed_boxes.election_names,
        &sealed_boxes.area_names,
    )
    .await?;
    Ok(Some(sealed_boxes))
}

/// One contest area's ballots, split into the batches its tally boards carry.
#[derive(Debug, Clone)]
pub struct ExtractedContestArea {
    /// The contest area, with the annotations the extraction recorded.
    pub contest: TallySessionContest,
    /// Every batch the area owns, by batch number, with its ballots.
    pub batches: Vec<(i64, Vec<BallotCiphertext>)>,
}

/// The ballot of one stored cast vote the contest area counts.
fn area_ballot(
    content: &str,
    contest_encryption_policy: &ContestEncryptionPolicy,
    tally_session_contest: &TallySessionContest,
) -> Result<BallotCiphertext> {
    match contest_encryption_policy {
        ContestEncryptionPolicy::MULTIPLE_CONTESTS => {
            let ballot: HashableMultiBallot = deserialize_str(content)?;
            BallotCiphertext::parse(&ballot.contests)
        }
        ContestEncryptionPolicy::SINGLE_CONTEST => {
            let contest_id = tally_session_contest.contest_id.as_deref().ok_or_else(|| {
                anyhow!(
                    "tally session contest {} names no contest",
                    tally_session_contest.id
                )
            })?;
            let ballot: HashableBallot = deserialize_str(content)?;
            let entries = ballot
                .contests
                .iter()
                .map(|entry| BallotCiphertext::parse(entry))
                .collect::<Result<Vec<_>>>()?;
            entries
                .into_iter()
                .find(|entry| entry.contest_ids == [contest_id])
                .ok_or_else(|| anyhow!("Could not get ciphertext for contest {contest_id}"))
        }
    }
}

/// Every contest area's ballots, by batch, with the annotations the area gets.
/// Nothing is written: the caller records the boards and the annotations
/// together.
#[instrument(skip_all, err)]
pub async fn extract_tally_boards(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    tally_session_contests: Vec<TallySessionContest>,
    tally_session: &TallySession,
    contest_encryption_policy: ContestEncryptionPolicy,
    delegated_voting_policy: DelegatedVotingPolicy,
    weighted_voting_policy: WeightedVotingPolicy,
    batch_anonymity_policy: BatchAnonymityPolicy,
) -> Result<Vec<ExtractedContestArea>> {
    // A delegate's ballot has no defined weighted semantics, so refuse rather
    // than silently computing weight * (1 + delegate_count). This is a backstop:
    // `create_tally_ceremony` rejects the same combination when the session is
    // created, which is where an operator can still act on it.
    let is_voter_weighted = weighted_voting_policy == WeightedVotingPolicy::VOTERS_WEIGHTED_VOTING;
    if is_voter_weighted && delegated_voting_policy == DelegatedVotingPolicy::ENABLED {
        return Err(anyhow!(
            "Delegated voting and voter-weighted voting cannot both be enabled \
             on the same election event"
        ));
    }

    let realm = get_event_realm(tenant_id, election_event_id);

    let event_elections =
        get_election_event_elections(hasura_transaction, tenant_id, election_event_id).await?;
    let election_ids_alias = election_voter_authorization_aliases(
        event_elections
            .iter()
            .map(|election| (election.id.clone(), election.external_id.clone())),
    );
    let sealed_boxes = sealed_boxes_for_execution(
        hasura_transaction,
        tenant_id,
        election_event_id,
        tally_session,
        &event_elections,
    )
    .await?
    .map(Arc::new);

    let mut extracted = Vec::with_capacity(tally_session_contests.len());

    for tally_session_contest_batch in tally_session_contests.chunks(get_worker_threads()) {
        let mut tasks = Vec::with_capacity(get_worker_threads());
        let tally_session_contest_batch = tally_session_contest_batch.to_vec();

        for tally_session_contest in tally_session_contest_batch {
            let sealed_boxes_clone = sealed_boxes.clone();
            let weighted_voting_policy_clone = weighted_voting_policy.clone();
            let tenant_id_clone = tenant_id.to_string();
            let election_event_id_clone = election_event_id.to_string();
            let election_ids_alias_clone = election_ids_alias.clone();
            let contest_encryption_policy_clone = contest_encryption_policy.clone();
            let batch_anonymity_policy_clone = batch_anonymity_policy.clone();
            let realm_clone = realm.clone();
            let multiplicity_column = if delegated_voting_policy == DelegatedVotingPolicy::ENABLED {
                VoterMultiplicityColumn::DelegateCount
            } else if is_voter_weighted {
                VoterMultiplicityColumn::VoteWeight
            } else {
                VoterMultiplicityColumn::None
            };

            let task = tokio::task::spawn_blocking(move || {
                tokio::runtime::Handle::current().block_on(async move {
                    event!(
                        Level::INFO,
                        "Extracting ballots for election {}, contest {}, area {} and batch num {}",
                        tally_session_contest.election_id,
                        tally_session_contest.contest_id.clone().unwrap_or_default(),
                        tally_session_contest.area_id,
                        tally_session_contest.session_id,
                    );

                    let mut keycloak_db_client: DbClient = get_keycloak_pool()
                        .await
                        .get()
                        .await
                        .with_context(|| "Error acquiring keycloak connection pool")?;
                    let keycloak_transaction_clone = keycloak_db_client
                        .transaction()
                        .await
                        .with_context(|| "Error acquiring keycloak transaction")?;
                    let mut hasura_db_client: DbClient = get_hasura_pool()
                        .await
                        .get()
                        .await
                        .with_context(|| "Error acquiring hasura connection pool")?;
                    let hasura_transaction_clone = hasura_db_client
                        .transaction()
                        .await
                        .with_context(|| "Error acquiring hasura transaction")?;

                    // Create a temporary file (auto-deleted when dropped)
                    let ballots_temp_file = NamedTempFile::new()
                        .map_err(|error| anyhow!("Failed to create temp file {}", error))?;
                    event!(
                        Level::INFO,
                        "Creating temporary file for ballots with path {:?}",
                        ballots_temp_file.path()
                    );

                    // Backstop for the tally-session guard: never extract
                    // ballots while this contest area has an unresolved vote.
                    let tenant_uuid = parse_uuid_v4(&tenant_id_clone)
                        .with_context(|| "Error parsing tenant_id")?;
                    let election_event_uuid = parse_uuid_v4(&election_event_id_clone)
                        .with_context(|| "Error parsing election_event_id")?;
                    let election_uuid = parse_uuid_v4(&tally_session_contest.election_id)
                        .with_context(|| "Error parsing election_id")?;
                    let area_uuid = parse_uuid_v4(&tally_session_contest.area_id)
                        .with_context(|| "Error parsing area_id")?;
                    let unresolved_count = count_unresolved_cast_votes(
                        &hasura_transaction_clone,
                        &tenant_uuid,
                        &election_event_uuid,
                        &election_uuid,
                        &area_uuid,
                    )
                    .await?;
                    if unresolved_count > 0 {
                        return Err(anyhow!(
                            "Refusing to extract ballots for election {} area {}: \
                             {unresolved_count} cast vote(s) have an unresolved Datafix outcome",
                            tally_session_contest.election_id,
                            tally_session_contest.area_id,
                        ));
                    }

                    let sealed_boxes = sealed_boxes_clone.as_ref().filter(|sealed_boxes| {
                        sealed_boxes
                            .election_ids
                            .contains(&tally_session_contest.election_id)
                    });
                    let (merge_result, ballot_box_seal_hash) = match sealed_boxes {
                        Some(sealed_boxes) => {
                            let election_name = sealed_boxes
                                .election_names
                                .get(&tally_session_contest.election_id)
                                .unwrap_or(&tally_session_contest.election_id);
                            let area_name = sealed_boxes
                                .area_names
                                .get(&tally_session_contest.area_id)
                                .unwrap_or(&tally_session_contest.area_id);
                            let sealed = sealed_box_ballots(
                                &hasura_transaction_clone,
                                &sealed_boxes.log,
                                &SealedBox {
                                    tenant_id: &tenant_id_clone,
                                    election_event_id: &election_event_id_clone,
                                    election_id: &tally_session_contest.election_id,
                                    area_id: &tally_session_contest.area_id,
                                    election_name,
                                    area_name,
                                    tally_session_id: &tally_session_contest.tally_session_id,
                                },
                                weighted_voting_policy_clone,
                            )
                            .await?;
                            (sealed.merge_result, Some(sealed.seal_hash))
                        }
                        None => {
                            // Create a temporary file (auto-deleted when dropped)
                            let ballots_temp_file = NamedTempFile::new()
                                .map_err(|error| anyhow!("Failed to create temp file {}", error))?;
                            event!(
                                Level::INFO,
                                "Creating temporary file for ballots with path {:?}",
                                ballots_temp_file.path()
                            );

                            find_area_ballots(
                                &hasura_transaction_clone,
                                &tenant_id_clone,
                                &election_event_id_clone,
                                &tally_session_contest.area_id,
                                &tally_session_contest.election_id,
                                &ballots_temp_file.path().to_path_buf(),
                            )
                            .await?;

                            // A delegator's own valid ballot replaces their delegation.
                            let voters_with_valid_ballot = voters_replacing_their_delegation(
                                multiplicity_column,
                                &ballots_temp_file.reopen()?,
                            )?;

                            let ballots_temp_file = ballots_temp_file.reopen()?;

                            // Create a temporary file (auto-deleted when dropped)
                            let users_temp_file = NamedTempFile::new().map_err(|error| {
                                anyhow!("Failed to create temp file: {}", error)
                            })?;
                            event!(
                                Level::INFO,
                                "Creating temporary file for users with path {:?}",
                                users_temp_file.path()
                            );

                            let election_alias = match election_ids_alias_clone
                                .get(&tally_session_contest.election_id)
                            {
                                Some(alias) => alias,
                                None => "",
                            }
                            .to_string();

                            list_keycloak_enabled_users_by_area_id_and_authorized_elections(
                                &keycloak_transaction_clone,
                                &realm_clone,
                                &tally_session_contest.area_id,
                                &election_alias,
                                &users_temp_file.path().to_path_buf(),
                                multiplicity_column,
                                &voters_with_valid_ballot,
                            )
                            .await?;

                            let users_temp_file = users_temp_file.reopen()?;

                            // Use a join function to filter and extract the ballot content
                            let ballots_output_index = 1;
                            let ballots_channel_index = 2;
                            let ballots_join_indexes = 0;
                            let users_join_idexes = 0;

                            // At most one multiplicity column is emitted by the voter
                            // dump, always at index 1.
                            let multiplicity_source = match multiplicity_column {
                                VoterMultiplicityColumn::DelegateCount => {
                                    Some(MultiplicitySource::DelegateCount(1))
                                }
                                VoterMultiplicityColumn::VoteWeight => {
                                    Some(MultiplicitySource::VoteWeight(1))
                                }
                                VoterMultiplicityColumn::None => None,
                            };

                            let merge_result = merge_join_csv(
                                &ballots_temp_file,
                                &users_temp_file,
                                ballots_join_indexes,
                                users_join_idexes,
                                ballots_output_index,
                                Some(ballots_channel_index),
                                multiplicity_source,
                            )?;
                            (merge_result, None)
                        }
                    };

                    // Checked before anything is posted, so a run that would
                    // be refused does not leave batches on an append-only
                    // board. Only weighting can inflate this beyond the ballot
                    // count, so the cap must not constrain an election that is
                    // not using it. Weighting no longer grows the mix batch --
                    // it grows the plaintexts the tally expands after mixing --
                    // so this bounds that expansion rather than the shuffle.
                    let total_weight: u64 = merge_result
                        .ballot_contents
                        .iter()
                        .map(|(_, multiplicity)| *multiplicity)
                        .sum();
                    if is_voter_weighted && total_weight > MAX_TOTAL_VOTE_WEIGHT {
                        return Err(anyhow!(
                            "Refusing to tally a summed vote weight of \
                             {total_weight} for election {} area {}: the maximum \
                             is {MAX_TOTAL_VOTE_WEIGHT}. This bounds the votes \
                             the tally expands after mixing, not the mix itself. \
                             It cannot be resolved without rescaling the weights \
                             of the voters in this area, which changes the result",
                            tally_session_contest.election_id,
                            tally_session_contest.area_id,
                        ));
                    }

                    // Which of this area's batches will exist. A weight sets
                    // the bits of the batches its voter's ciphertext goes into,
                    // so the union over voters is exactly the set of non-empty
                    // batches.
                    let weight_bit_mask: Option<u32> = if is_voter_weighted {
                        let union = merge_result.ballot_contents.iter().try_fold(
                            0u64,
                            |acc, (_, weight)| -> Result<u64> {
                                // Same refusal the split makes, so the mask can
                                // never claim a batch the split would not fill.
                                let bits = weight_batch_offsets(*weight)?
                                    .map(|bit| 1u64 << bit)
                                    .sum::<u64>();
                                Ok(acc | bits)
                            },
                        )?;
                        // No ballots at all: keep the single empty batch this
                        // area would have had without weighting, so the tally
                        // still has something to wait for.
                        let computed = if union == 0 { 1 } else { union as u32 };
                        Some(computed)
                    } else {
                        // Absent for every other policy, which leaves the stored
                        // annotations byte-identical to what they were before
                        // weighting existed.
                        None
                    };

                    let annotations =
                        contest_annotations(&merge_result, weight_bit_mask, ballot_box_seal_hash);

                    let annotations = serde_json::to_value(&annotations)?;

                    let updated_tally_session_contest = TallySessionContest {
                        annotations: Some(annotations),
                        ..tally_session_contest.clone()
                    };

                    // One vector per batch this area owns. Under weighting a
                    // voter contributes at most one ciphertext to each, so no
                    // batch is larger than the electorate however large the
                    // weights are; every other policy fills only the first.
                    let mut batches: Vec<Vec<BallotCiphertext>> =
                        vec![Vec::new(); VOTE_WEIGHT_BATCHES as usize];
                    let mut batch_voters = vec![0usize; VOTE_WEIGHT_BATCHES as usize];
                    for (ballot_str, multiplicity) in merge_result.ballot_contents {
                        let ballot = area_ballot(
                            &ballot_str,
                            &contest_encryption_policy_clone,
                            &tally_session_contest,
                        )
                        .with_context(|| {
                            format!(
                                "a cast vote of election {} area {}",
                                tally_session_contest.election_id, tally_session_contest.area_id
                            )
                        })?;
                        if is_voter_weighted {
                            // The ciphertext appears once in the batch for
                            // each bit the weight sets, and the tally
                            // multiplies batch `bit` by 2^bit. Summing those
                            // multipliers over the set bits reconstructs the
                            // weight, so the count is exact -- and because no
                            // batch ever holds the same ciphertext twice, the
                            // board shows no repetition to read a weight off.
                            for bit in weight_batch_offsets(multiplicity)? {
                                batches[bit as usize].push(ballot.clone());
                                batch_voters[bit as usize] += 1;
                            }
                        } else {
                            // Delegated voting still repeats within the one
                            // batch, exactly as it did before weighting.
                            batches[0].extend(std::iter::repeat_n(ballot, multiplicity as usize));
                            batch_voters[0] += 1;
                        }
                    }

                    let mut anonymity_issues: Vec<_> = batch_voters
                        .iter()
                        .enumerate()
                        .filter_map(|(bit, voters)| small_batch(bit as u32, *voters))
                        .collect();
                    let posted =
                        posted_by_other_sessions(&hasura_transaction_clone, &tally_session_contest)
                            .await?;
                    let ciphertexts: Vec<Vec<_>> = batches
                        .iter()
                        .map(|batch| {
                            batch
                                .iter()
                                .map(|ballot| ballot.ciphertext.clone())
                                .collect()
                        })
                        .collect();
                    anonymity_issues.extend(session_divergences(&ciphertexts, &posted));
                    enforce_batch_anonymity(
                        &batch_anonymity_policy_clone,
                        &tally_session_contest.election_id,
                        &tally_session_contest.area_id,
                        &anonymity_issues,
                    )?;

                    let base_batch = i64::from(tally_session_contest.session_id);
                    let mut area_batches = Vec::new();
                    for (bit, ballots) in batches.into_iter().enumerate() {
                        // Exactly the batches the mask names, so the tally
                        // waits for exactly what gets posted. Every masked
                        // batch is non-empty except in the one case the mask
                        // is forced to 1: an area with no ballots at all,
                        // which posts a single empty batch just as it did
                        // before weighting existed. Without weighting the mask
                        // is absent and batch 0 is always posted.
                        let is_expected = weight_bit_mask
                            .map(|mask| mask & (1u32 << bit) != 0)
                            .unwrap_or(bit == 0);
                        if !is_expected {
                            continue;
                        }
                        event!(
                            Level::INFO,
                            "insertable_ballots len: {:?} for batch offset {}",
                            ballots.len(),
                            bit
                        );
                        area_batches.push((base_batch + bit as i64, ballots));
                    }

                    Ok(ExtractedContestArea {
                        contest: updated_tally_session_contest,
                        batches: area_batches,
                    })
                })
            });
            tasks.push(task);
        }
        let mut extracted_batch: Vec<ExtractedContestArea> = futures::future::try_join_all(tasks)
            .await?
            .into_iter()
            .collect::<Result<Vec<_>>>()?;

        extracted.append(&mut extracted_batch);
    }

    Ok(extracted)
}

fn voters_replacing_their_delegation(
    column: VoterMultiplicityColumn,
    ballots_file: &std::fs::File,
) -> Result<Vec<String>> {
    match column {
        VoterMultiplicityColumn::DelegateCount => read_valid_ballot_voter_ids(ballots_file, 0, 3),
        VoterMultiplicityColumn::VoteWeight | VoterMultiplicityColumn::None => Ok(Vec::new()),
    }
}

async fn posted_by_other_sessions(
    transaction: &Transaction<'_>,
    contest: &TallySessionContest,
) -> Result<Vec<PostedBatch>> {
    use crate::postgres::protocol_board::{
        get_dkg_board_by_keys_ceremony, get_tally_boards_by_session,
    };
    use crate::postgres::tally_session_contest::get_other_sessions_tally_session_contests;
    use crate::services::weight_batches::contest_weight_batches;
    let mut posted = Vec::new();
    for other in get_other_sessions_tally_session_contests(transaction, contest).await? {
        let boards = get_tally_boards_by_session(
            transaction,
            &other.tenant_id,
            &other.election_event_id,
            &other.tally_session_id,
        )
        .await?;
        for (batch, _) in contest_weight_batches(&other)? {
            let Some(row) = boards.iter().find(|row| row.batch == Some(batch)) else {
                continue;
            };
            let parent = get_dkg_board_by_keys_ceremony(
                transaction,
                &other.tenant_id,
                &other.election_event_id,
                &row.keys_ceremony_id,
            )
            .await?
            .context("tally board has no DKG parent")?;
            let configuration =
                protocol_board::SignedConfiguration::parse(&parent.manager_message)?;
            let ballots =
                protocol_board::SignedBallots::parse(&row.manager_message, &configuration)?;
            let board_name = protocol_board::BoardName::parse(&row.name)?;
            let messages = crate::services::protocol_board::board_handle(&board_name)?
                .export_messages()
                .await?;
            if !messages.contains(&ballots.to_bytes()) {
                continue;
            }
            posted.push(PostedBatch {
                tally_session_id: other.tally_session_id.clone(),
                offset: u32::try_from(batch - i64::from(other.session_id))?,
                ciphertexts: ballots.ciphertexts()?,
            });
        }
    }
    Ok(posted)
}

#[cfg(test)]
mod tests {
    use super::{election_voter_authorization_aliases, voters_replacing_their_delegation};
    use crate::services::users::VoterMultiplicityColumn;
    use std::collections::HashMap;
    use std::io::Write;

    fn ballots_file() -> tempfile::NamedTempFile {
        let mut ballots = tempfile::NamedTempFile::new().unwrap();
        writeln!(
            ballots,
            "alice,content-a,ONLINE,valid,10000000-0000-4000-8000-000000000001\n\
             bob,content-b,ONLINE,discarded,10000000-0000-4000-8000-000000000002"
        )
        .unwrap();
        ballots.flush().unwrap();
        ballots
    }

    #[test]
    fn only_the_delegate_count_reads_the_voters_with_a_valid_ballot() {
        let ballots = ballots_file();
        let voters =
            |column| voters_replacing_their_delegation(column, &ballots.reopen().unwrap()).unwrap();

        assert_eq!(
            voters(VoterMultiplicityColumn::DelegateCount),
            vec!["alice".to_string()]
        );
        assert!(voters(VoterMultiplicityColumn::VoteWeight).is_empty());
        assert!(voters(VoterMultiplicityColumn::None).is_empty());
    }

    #[test]
    fn a_ballots_file_without_the_status_column_is_an_error() {
        let mut ballots = tempfile::NamedTempFile::new().unwrap();
        writeln!(ballots, "alice,content-a").unwrap();
        ballots.flush().unwrap();

        assert!(voters_replacing_their_delegation(
            VoterMultiplicityColumn::DelegateCount,
            &ballots.reopen().unwrap()
        )
        .is_err());
    }

    #[test]
    fn election_authorization_alias_uses_the_election_id_without_an_external_id() {
        let aliases = election_voter_authorization_aliases([("election-uuid".to_string(), None)]);
        assert_eq!(
            aliases,
            HashMap::from([("election-uuid".to_string(), "election-uuid".to_string()),])
        );
    }

    #[test]
    fn election_authorization_alias_preserves_explicit_external_ids() {
        let aliases = election_voter_authorization_aliases([
            ("first-uuid".to_string(), Some("external-alias".to_string())),
            ("second-uuid".to_string(), Some(" ".to_string())),
        ]);
        assert_eq!(
            aliases,
            HashMap::from([
                ("first-uuid".to_string(), "external-alias".to_string()),
                ("second-uuid".to_string(), " ".to_string()),
            ])
        );
    }

    #[test]
    fn election_authorization_alias_uses_the_election_id_for_an_empty_external_id() {
        let aliases = election_voter_authorization_aliases([(
            "election-uuid".to_string(),
            Some(String::new()),
        )]);
        assert_eq!(
            aliases,
            HashMap::from([("election-uuid".to_string(), "election-uuid".to_string()),])
        );
    }
}
