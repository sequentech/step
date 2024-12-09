// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::hasura::election::get_all_elections_for_event;
use crate::hasura::tally_session::set_tally_session_completed;
use crate::hasura::tally_session_execution::get_last_tally_session_execution::ResponseData;
use crate::hasura::tally_session_execution::{
    get_last_tally_session_execution, insert_tally_session_execution,
};
use crate::hasura::trustee::get_trustees_by_name;
use crate::services::cast_votes::{find_area_ballots, CastVote};
use crate::services::ceremonies::insert_ballots::get_last_tally_session_execution::GetLastTallySessionExecutionSequentBackendTallySessionContest;
use crate::services::protocol_manager::*;
use crate::services::public_keys::deserialize_public_key;
use crate::services::users::list_keycloak_enabled_users_by_area_id;
use anyhow::{anyhow, Context, Result};
use b3::messages::message::Message;
use b3::messages::newtypes::BatchNumber;
use b3::messages::newtypes::TrusteeSet;
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use sequent_core::ballot::{ContestEncryptionPolicy, ElectionPresentation, HashableBallot};
use sequent_core::multi_ballot::HashableMultiBallot;
use sequent_core::serialization::deserialize_with_path::{deserialize_str, deserialize_value};
use sequent_core::services::connection::AuthHeaders;
use sequent_core::services::date::ISO8601;
use sequent_core::services::keycloak::get_event_realm;
use std::collections::HashMap;
use strand::backend::ristretto::RistrettoCtx;
use strand::elgamal::Ciphertext;
use strand::signature::StrandSignaturePk;
use tracing::{event, instrument, Level};

#[instrument(skip_all, err)]
pub async fn insert_ballots_messages(
    auth_headers: &AuthHeaders,
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    board_name: &str,
    trustee_names: Vec<String>,
    tally_session_contests: Vec<GetLastTallySessionExecutionSequentBackendTallySessionContest>,
    contest_encryption_policy: ContestEncryptionPolicy
) -> Result<()> {
    let trustees = get_trustees_by_name(&auth_headers, &tenant_id, &trustee_names)
        .await?
        .data
        .with_context(|| "can't find trustees")?
        .sequent_backend_trustee;

    event!(Level::INFO, "trustees len: {:?}", trustees.len());

    // get trustees keys from input strings
    let deserialized_trustee_pks: Vec<StrandSignaturePk> = trustees
        .clone()
        .into_iter()
        .map(|trustee| deserialize_public_key(trustee.public_key.unwrap()))
        .collect();

    event!(
        Level::INFO,
        "deserialized_trustee_pks len: {:?}",
        deserialized_trustee_pks.len()
    );

    let realm = get_event_realm(&tenant_id, &election_event_id);
    let protocol_manager = get_protocol_manager(board_name).await?;
    let mut board = get_b3_pgsql_client().await?;
    let board_messages: Vec<Message> =
        get_board_messages::<RistrettoCtx>(board_name, &mut board).await?;
    let configuration = get_configuration(&board_messages)?;
    let public_key_hash = get_public_key_hash::<RistrettoCtx>(&board_messages)?;
    let selected_trustees: TrusteeSet =
        generate_trustee_set(&configuration, deserialized_trustee_pks.clone());

    for tally_session_contest in tally_session_contests {
        event!(
            Level::INFO,
            "Inserting Ballots message for election {}, contest {}, area {} and batch num {}",
            tally_session_contest.election_id,
            tally_session_contest.contest_id.clone().unwrap_or_default(),
            tally_session_contest.area_id,
            tally_session_contest.session_id,
        );
        let ballots_list = find_area_ballots(
            &hasura_transaction,
            &tenant_id,
            &election_event_id,
            &tally_session_contest.area_id,
        )
        .await?;

        event!(Level::INFO, "ballots_list len: {:?}", ballots_list.len());

        let users_map = list_keycloak_enabled_users_by_area_id(
            keycloak_transaction,
            &realm,
            &tally_session_contest.area_id,
        )
        .await?;

        /*
        let elections_end_dates =
            get_elections_end_dates(auth_headers, tenant_id, election_event_id)
                .await
                .with_context(|| "error getting elections' end_date")?;
        */

        let insertable_ballots: Vec<Ciphertext<RistrettoCtx>> = ballots_list
            .iter()
            .filter(|ballot| {
                let Some(voter_id) = ballot.voter_id_string.clone() else {
                    return false;
                };

                let Some(election_id) = ballot.election_id.clone() else {
                    return false;
                };

                if tally_session_contest.election_id != election_id {
                    return false;
                }

                let Some(ballot_created_at) = ballot.created_at else {
                    return false;
                };

                /*let valid = match elections_end_dates.get(&election_id) {
                    Some(Some(election_end_date)) => ballot_created_at <= *election_end_date,
                    _ => true,
                };*/
                let valid = true;

                users_map.contains(&voter_id) && valid
            })
            .map(|ballot| -> Result<Option<Ciphertext<RistrettoCtx>>> {
                Ok(ballot
                    .content
                    .clone()
                    .map(|ballot_str| -> Result<Option<Ciphertext<RistrettoCtx>>> {
                        if ContestEncryptionPolicy::MULTIPLE_CONTESTS  == contest_encryption_policy {
                            let hashable_multi_ballot: HashableMultiBallot = deserialize_str(&ballot_str)?;

                            let hashable_multi_ballot_contests = hashable_multi_ballot
                                .deserialize_contests()
                                .map_err(|err| anyhow!("{:?}", err))?;
                            Ok(Some(hashable_multi_ballot_contests.ciphertext))
                        } else {
                            let hashable_ballot: HashableBallot = deserialize_str(&ballot_str)?;
                            let contests = hashable_ballot
                                .deserialize_contests()
                                .map_err(|err| anyhow!("{:?}", err))?;
                            Ok(contests
                                .iter()
                                .find(|contest| contest.contest_id == tally_session_contest.contest_id.clone().unwrap_or_default())
                                .map(|contest| contest.ciphertext.clone()))
                        }
                    })
                    .transpose()?
                    .flatten())
            })
            .collect::<Result<Vec<_>>>()?
            .iter()
            .filter_map(|ballot_opt| ballot_opt.clone())
            .collect();

        event!(
            Level::INFO,
            "insertable_ballots len: {:?}",
            ballots_list.len()
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
        .await?;
    }
    Ok(())
}

#[instrument(skip_all, err)]
pub async fn get_elections_end_dates(
    auth_headers: &AuthHeaders,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<HashMap<String, Option<DateTime<Utc>>>> {
    // TODO: use ballot publications instead?
    let elections_dates: HashMap<String, Option<DateTime<_>>> = get_all_elections_for_event(
        auth_headers.clone(),
        tenant_id.to_string(),
        election_event_id.to_string(),
    )
    .await?
    .data
    .expect("expected data")
    .sequent_backend_election
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

#[instrument(skip_all, err, ret)]
pub async fn count_auditable_ballots(
    elections_end_dates: &HashMap<String, Option<DateTime<Utc>>>,
    auth_headers: &AuthHeaders,
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    contest_id: &str,
    area_id: &str,
) -> Result<usize> {
    event!(
        Level::INFO,
        "Counting Auditable Ballots for election {election_id}, contest {contest_id} area {area_id}"
    );

    let realm = get_event_realm(tenant_id, election_event_id);
    let ballots_list =
        find_area_ballots(hasura_transaction, tenant_id, election_event_id, area_id).await?;

    event!(Level::INFO, "ballots_list len: {:?}", ballots_list.len());

    let users_map =
        list_keycloak_enabled_users_by_area_id(keycloak_transaction, &realm, area_id).await?;

    /*
    let elections_end_dates = get_elections_end_dates(auth_headers, tenant_id, election_event_id)
        .await
        .with_context(|| "error getting elections' end_date")?;
    */

    let auditable_ballots: Vec<&CastVote> = ballots_list
        .iter()
        .filter(|ballot| {
            let Some(voter_id) = ballot.voter_id_string.clone() else {
                return true;
            };

            let Some(election_id) = ballot.election_id.clone() else {
                return true;
            };

            let Some(ballot_created_at) = ballot.created_at else {
                return true;
            };
            /*
            let valid = match elections_end_dates.get(&election_id) {
                Some(Some(election_end_date)) => ballot_created_at <= *election_end_date,
                _ => true,
            };
            */
            let valid: bool = true;

            !users_map.contains(&voter_id) || !valid
        })
        .collect();
    Ok(auditable_ballots.len())
}
