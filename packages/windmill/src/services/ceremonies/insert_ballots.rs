// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
// use crate::hasura::trustee::get_trustees_by_name;
use crate::postgres::election::get_elections;
use crate::postgres::trustee::get_trustees_by_name;
use crate::services::cast_votes::{find_area_ballots, CastVote};
use crate::services::database::PgConfig;
use crate::services::election::get_election_event_elections;
use crate::services::join::merge_join_csv;
use crate::services::protocol_manager::*;
use crate::services::public_keys::deserialize_public_key;
use crate::services::users::list_keycloak_enabled_users_by_area_id_and_authorized_elections;
use anyhow::{anyhow, Context, Result};
use b3::messages::message::Message;
use b3::messages::newtypes::BatchNumber;
use b3::messages::newtypes::TrusteeSet;
use base64::{
    alphabet,
    engine::{self, general_purpose},
    Engine as _,
};
use chrono::{DateTime, Utc};
use csv::WriterBuilder;
use deadpool_postgres::Transaction;
use sequent_core::ballot::{ContestEncryptionPolicy, ElectionPresentation, HashableBallot};
use sequent_core::multi_ballot::HashableMultiBallot;
use sequent_core::serialization::base64::{Base64Deserialize, Base64Serialize};
use sequent_core::serialization::deserialize_with_path::{deserialize_str, deserialize_value};
use sequent_core::services::date::ISO8601;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::hasura::core::{TallySessionContest, TallySessionContestAnnotations};
use serde_json::json;
use std::collections::HashMap;
use strand::backend::ristretto::RistrettoCtx;
use strand::elgamal::Ciphertext;
use strand::signature::StrandSignaturePk;
use tempfile::NamedTempFile;
use tokio::task::JoinHandle;
use tracing::{event, info, instrument, Level};

#[instrument(skip_all, err)]
pub async fn insert_ballots_messages(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    board_name: &str,
    trustee_names: Vec<String>,
    tally_session_contests: Vec<TallySessionContest>,
    contest_encryption_policy: ContestEncryptionPolicy,
) -> Result<Vec<TallySessionContest>> {
    let trustees = get_trustees_by_name(hasura_transaction, &tenant_id, &trustee_names).await?;

    let mut tally_session_contests_updated: Vec<TallySessionContest> = vec![];

    event!(Level::INFO, "trustees len: {:?}", trustees.len());

    // get trustees keys from input strings
    let deserialized_trustee_pks: Vec<StrandSignaturePk> = trustees
        .clone()
        .into_iter()
        .map(|trustee| {
            let public_key = trustee
                .public_key
                .ok_or(anyhow!("Missing trustee public key"))?;
            deserialize_public_key(public_key)
        })
        .collect::<Result<Vec<_>>>()?;

    event!(
        Level::INFO,
        "deserialized_trustee_pks len: {:?}",
        deserialized_trustee_pks.len()
    );

    let realm = get_event_realm(&tenant_id, &election_event_id);
    let protocol_manager = get_protocol_manager(
        hasura_transaction,
        tenant_id,
        Some(election_event_id),
        board_name,
    )
    .await?;
    let mut board = get_b3_pgsql_client().await?;
    let board_messages: Vec<Message> =
        get_board_messages::<RistrettoCtx>(board_name, &mut board).await?;
    let configuration = get_configuration(&board_messages)?;
    let public_key_hash = get_public_key_hash::<RistrettoCtx>(&board_messages)?;
    let selected_trustees: TrusteeSet =
        generate_trustee_set(&configuration, deserialized_trustee_pks.clone());

    let election_ids_alias: HashMap<String, String> =
        get_election_event_elections(&hasura_transaction, tenant_id, election_event_id)
            .await?
            .into_iter()
            .filter_map(|election| election.alias.map(|x| (election.id.clone(), x)))
            .collect();

    for tally_session_contest in tally_session_contests {
        event!(
            Level::INFO,
            "Inserting Ballots message for election {}, contest {}, area {} and batch num {}",
            tally_session_contest.election_id,
            tally_session_contest.contest_id.clone().unwrap_or_default(),
            tally_session_contest.area_id,
            tally_session_contest.session_id,
        );

        // Create a temporary file (auto-deleted when dropped)
        let ballots_temp_file = NamedTempFile::new()
            .map_err(|error| anyhow!("Failed to create temp file {}", error))?;
        event!(
            Level::INFO,
            "Creating temporary file for ballots with path {:?}",
            ballots_temp_file.path()
        );

        find_area_ballots(
            &hasura_transaction,
            &tenant_id,
            &election_event_id,
            &tally_session_contest.area_id,
            &tally_session_contest.election_id,
            &ballots_temp_file.path().to_path_buf(),
        )
        .await?;

        let ballots_temp_file = ballots_temp_file.reopen()?;

        // Create a temporary file (auto-deleted when dropped)
        let users_temp_file = NamedTempFile::new()
            .map_err(|error| anyhow!("Failed to create temp file: {}", error))?;
        event!(
            Level::INFO,
            "Creating temporary file for users with path {:?}",
            users_temp_file.path()
        );

        let election_alias = match election_ids_alias.get(&tally_session_contest.election_id) {
            Some(alias) => alias,
            None => "",
        }
        .to_string();

        list_keycloak_enabled_users_by_area_id_and_authorized_elections(
            keycloak_transaction,
            &realm,
            &tally_session_contest.area_id,
            &election_alias,
            &users_temp_file.path().to_path_buf(),
        )
        .await?;

        let users_temp_file = users_temp_file.reopen()?;

        // Use a join function to filter and extract the ballot content
        let ballots_output_index = 2;
        let ballots_join_indexes = 0;
        let ballot_election_id_index = 1;
        let users_join_idexes = 0;
        let election_id = tally_session_contest.election_id.clone();
        let contest_id = tally_session_contest.contest_id.clone();

        let contest_encryption_policy = contest_encryption_policy.clone();

        let handle: JoinHandle<Result<_>> = tokio::task::spawn_blocking({
            move || {
                tokio::runtime::Handle::current().block_on(async move {
                    let (ballot_contents, elegible_voters, ballots_without_voter, casted_ballots) =
                        merge_join_csv(
                            &ballots_temp_file,
                            &users_temp_file,
                            ballots_join_indexes,
                            users_join_idexes,
                            ballots_output_index,
                            ballot_election_id_index,
                            &election_id,
                        )?;

                    let ciphertexts = ballot_contents
                        .into_iter()
                        .map(|ballot_str| {
                            info!("ballot_str: {ballot_str}");
                            let ciphertext: Ciphertext<RistrettoCtx> =
                                if ContestEncryptionPolicy::MULTIPLE_CONTESTS
                                    == contest_encryption_policy
                                {
                                    let hashable_multi_ballot: HashableMultiBallot =
                                        deserialize_str(&ballot_str)?;

                                    let hashable_multi_ballot_contests = hashable_multi_ballot
                                        .deserialize_contests()
                                        .map_err(|err| anyhow!("{:?}", err))?;
                                    Some(hashable_multi_ballot_contests.ciphertext)
                                } else {
                                    let hashable_ballot: HashableBallot =
                                        deserialize_str(&ballot_str)?;
                                    let contests = hashable_ballot
                                        .deserialize_contests()
                                        .map_err(|err| anyhow!("{:?}", err))?;
                                    contests
                                        .iter()
                                        .find(|contest| {
                                            contest.contest_id
                                                == contest_id.clone().unwrap_or_default()
                                        })
                                        .map(|contest| contest.ciphertext.clone())
                                }
                                .ok_or(anyhow!("Could not get ciphertext"))?;
                            Ok(ciphertext)
                        })
                        .collect::<Result<Vec<_>>>()?;

                    Ok((
                        ciphertexts,
                        elegible_voters,
                        ballots_without_voter,
                        casted_ballots,
                    ))
                })
            }
        });

        // Await the result and handle JoinError explicitly
        let (insertable_ballots, elegible_voters, ballots_without_voter, casted_ballots): (
            Vec<Ciphertext<RistrettoCtx>>,
            u64,
            u64,
            u64,
        ) = match handle.await {
            Ok(inner_result) => inner_result.map_err(|err| anyhow!(err.context("Task failed"))),
            Err(join_error) => Err(anyhow!("Task panicked: {}", join_error)),
        }?;

        let annotations = TallySessionContestAnnotations {
            elegible_voters,
            ballots_without_voter,
            casted_ballots,
        };

        let annotations = serde_json::to_value(&annotations)?;

        tally_session_contests_updated.push(TallySessionContest {
            id: tally_session_contest.id,
            tenant_id: tally_session_contest.tenant_id,
            election_event_id: tally_session_contest.election_event_id,
            area_id: tally_session_contest.area_id,
            contest_id: tally_session_contest.contest_id.clone(),
            session_id: tally_session_contest.session_id,
            created_at: tally_session_contest.created_at,
            last_updated_at: tally_session_contest.last_updated_at,
            labels: tally_session_contest.labels,
            annotations: Some(annotations),
            tally_session_id: tally_session_contest.tally_session_id,
            election_id: tally_session_contest.election_id,
        });

        event!(
            Level::INFO,
            "insertable_ballots len: {:?}",
            insertable_ballots.len()
        );

        let mut board = get_b3_pgsql_client().await?;
        let batch = tally_session_contest.session_id.clone() as BatchNumber;
        add_ballots_to_board(
            &protocol_manager,
            &mut board,
            board_name,
            &board_messages,
            &configuration,
            public_key_hash,
            selected_trustees.clone(),
            insertable_ballots,
            batch,
        )
        .await?
    }
    Ok(tally_session_contests_updated)
}

#[instrument(skip_all, err)]
pub async fn get_elections_end_dates(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<HashMap<String, Option<DateTime<Utc>>>> {
    // Use ballot publications instead?
    let elections = get_elections(hasura_transaction, tenant_id, election_event_id, None)
        .await
        .map_err(|err| anyhow!("Error getting elections {:?}", err))?;

    let elections_dates: HashMap<String, Option<DateTime<_>>> = elections
        .into_iter()
        .map(|election| {
            let election_presentation: ElectionPresentation = election
                .presentation
                .clone()
                .map(|presentation| deserialize_value(presentation))
                .transpose()
                .map_err(|err| anyhow!("Error parsing election presentation {:?}", err))?
                .unwrap_or(Default::default());
            let current_dates = election_presentation
                .dates
                .clone()
                .unwrap_or(Default::default());
            let end_date = current_dates
                .end_date
                .clone()
                .map(|val| ISO8601::to_date_utc(&val).ok())
                .flatten();
            Ok((election.id, end_date))
        })
        .collect::<Result<HashMap<_, _>>>()
        .map_err(|err| anyhow!("Error parsing election dates {:?}", err))?;
    Ok(elections_dates)
}
