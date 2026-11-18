// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
// use crate::hasura::trustee::get_trustees_by_name;
use crate::postgres::election::get_elections;
use crate::postgres::trustee::get_trustees_by_name;
use crate::services::cast_votes::{ciphertext_fingerprint, find_area_ballots, CastVote};
use crate::services::celery_app::get_worker_threads;
use crate::services::database::{get_hasura_pool, get_keycloak_pool, PgConfig};
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
use std::collections::{HashMap, HashSet};
use strand::backend::ristretto::RistrettoCtx;
use strand::elgamal::Ciphertext;
use strand::hash::Hash;
use strand::signature::StrandSignaturePk;
use tempfile::NamedTempFile;
use tokio::task::JoinHandle;
use tracing::{event, info, instrument, Level};

use deadpool_postgres::Client as DbClient;

use std::sync::Arc; // Add this import

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
    skip_board_posting: bool,
) -> Result<Vec<TallySessionContest>> {
    let trustees = get_trustees_by_name(hasura_transaction, &tenant_id, &trustee_names).await?;

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
    // Wrap protocol_manager in an Arc
    let protocol_manager = Arc::new(
        get_protocol_manager(
            hasura_transaction,
            tenant_id,
            Some(election_event_id),
            board_name,
        )
        .await?,
    );
    let mut board_client = get_b3_pgsql_client().await?;
    let board_messages =
        Arc::new(get_board_messages::<RistrettoCtx>(board_name, &mut board_client).await?);
    let manager_pk = strand::signature::StrandSignaturePk::from_sk(&protocol_manager.signing_key)?;
    let configuration = get_configuration(&board_messages, &manager_pk)?;
    let public_key_hash = get_public_key_hash::<RistrettoCtx>(&board_messages, &configuration)?;
    let selected_trustees: TrusteeSet =
        generate_trustee_set(&configuration, deserialized_trustee_pks.clone());

    let election_ids_alias: HashMap<String, String> =
        get_election_event_elections(&hasura_transaction, tenant_id, election_event_id)
            .await?
            .into_iter()
            .filter_map(|election| election.external_id.map(|x| (election.id.clone(), x)))
            .collect();

    // Collect all futures for parallel execution
    let mut tally_session_contests_updated = Vec::with_capacity(tally_session_contests.len());

    for tally_session_contest_batch in tally_session_contests.chunks(get_worker_threads()) {
        let mut tasks = Vec::with_capacity(get_worker_threads());
        let tally_session_contest_batch = tally_session_contest_batch.to_vec();

        for tally_session_contest in tally_session_contest_batch {
            // Clone necessary variables for each task. Arc::clone increments the ref count.
            let tenant_id_clone = tenant_id.to_string();
            let election_event_id_clone = election_event_id.to_string();
            let board_name_clone = board_name.to_string();
            let protocol_manager_arc_clone = Arc::clone(&protocol_manager); // Clone the Arc
            let configuration_clone = configuration.clone(); // Assuming Configuration can be cloned
            let public_key_hash_clone = public_key_hash.clone(); // Assuming PublicKeyHash can be cloned
            let selected_trustees_clone = selected_trustees.clone();
            let election_ids_alias_clone = election_ids_alias.clone();
            let contest_encryption_policy_clone = contest_encryption_policy.clone();
            let realm_clone = realm.clone();
            let board_messages_clone = Arc::clone(&board_messages); // board_messages also needs to be cloned if it's not Sync + Send

            let task = tokio::task::spawn_blocking(move || {
                tokio::runtime::Handle::current().block_on(async move {
                    event!(
                Level::INFO,
                "Inserting Ballots message for election {}, contest {}, area {} and batch num {}",
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

                    find_area_ballots(
                        &hasura_transaction_clone,
                        &tenant_id_clone,
                        &election_event_id_clone,
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

                    let election_alias =
                        match election_ids_alias_clone.get(&tally_session_contest.election_id) {
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
                    )
                    .await?;

                    let users_temp_file = users_temp_file.reopen()?;

                    // Use a join function to filter and extract the ballot content
                    let ballots_output_index = 1;
                    let ballots_join_indexes = 0;
                    let users_join_idexes = 0;
                    let contest_id = tally_session_contest.contest_id.clone();

                    let (ballot_contents, elegible_voters, ballots_without_voter, casted_ballots) =
                        merge_join_csv(
                            &ballots_temp_file,
                            &users_temp_file,
                            ballots_join_indexes,
                            users_join_idexes,
                            ballots_output_index,
                        )?;

                    let annotations = TallySessionContestAnnotations {
                        elegible_voters,
                        ballots_without_voter,
                        casted_ballots,
                    };

                    let annotations = serde_json::to_value(&annotations)?;

                    let updated_tally_session_contest = TallySessionContest {
                        id: tally_session_contest.id.clone(),
                        tenant_id: tally_session_contest.tenant_id.clone(),
                        election_event_id: tally_session_contest.election_event_id.clone(),
                        area_id: tally_session_contest.area_id.clone(),
                        contest_id: tally_session_contest.contest_id.clone(),
                        session_id: tally_session_contest.session_id.clone(),
                        created_at: tally_session_contest.created_at.clone(),
                        last_updated_at: tally_session_contest.last_updated_at.clone(),
                        labels: tally_session_contest.labels.clone(),
                        annotations: Some(annotations),
                        tally_session_id: tally_session_contest.tally_session_id.clone(),
                        election_id: tally_session_contest.election_id.clone(),
                    };

                    if !skip_board_posting {
                        let ballot_count = ballot_contents.len();
                        let ciphertexts = contest_ciphertexts(
                            ballot_contents,
                            &contest_encryption_policy_clone,
                            &contest_id.unwrap_or_default(),
                        )?;
                        let repeated_ciphertexts = ballot_count - ciphertexts.len();
                        if repeated_ciphertexts > 0 {
                            event!(
                                Level::WARN,
                                "{repeated_ciphertexts} ballot(s) for election {}, contest {}, area {} \
                                 repeat a ciphertext of another ballot; each ciphertext is counted once",
                                tally_session_contest.election_id,
                                tally_session_contest.contest_id.clone().unwrap_or_default(),
                                tally_session_contest.area_id,
                            );
                        }

                        event!(
                            Level::INFO,
                            "insertable_ballots len: {:?}",
                            ciphertexts.len()
                        );

                        let mut board = get_b3_pgsql_client().await?;
                        let batch = tally_session_contest.session_id.clone() as BatchNumber;
                        add_ballots_to_board(
                            &protocol_manager_arc_clone, // Use the Arc clone here
                            &mut board,
                            &board_name_clone,
                            &board_messages_clone, // Use the cloned board_messages
                            &configuration_clone,
                            public_key_hash_clone,
                            selected_trustees_clone,
                            ciphertexts,
                            batch,
                        )
                        .await?;
                    }

                    Ok(updated_tally_session_contest)
                })
            });
            tasks.push(task);
        }
        // Await all tasks and collect results
        let mut tally_session_contest_to_add: Vec<TallySessionContest> =
            futures::future::try_join_all(tasks)
                .await?
                .into_iter()
                .collect::<Result<Vec<_>>>()?;

        tally_session_contests_updated.append(&mut tally_session_contest_to_add);
    }

    Ok(tally_session_contests_updated)
}

/// Reads the ciphertext each ballot holds for the tallied contest. A ciphertext
/// held by more than one ballot is kept once, at its first position, so
/// repeating it does not count it again.
fn contest_ciphertexts(
    ballot_contents: Vec<String>,
    contest_encryption_policy: &ContestEncryptionPolicy,
    contest_id: &str,
) -> Result<Vec<Ciphertext<RistrettoCtx>>> {
    let mut ciphertexts: Vec<Ciphertext<RistrettoCtx>> = Vec::new();
    let mut seen: HashSet<Hash> = HashSet::new();
    for ballot_str in ballot_contents {
        let ciphertext: Ciphertext<RistrettoCtx> =
            if ContestEncryptionPolicy::MULTIPLE_CONTESTS == *contest_encryption_policy {
                let hashable_multi_ballot: HashableMultiBallot = deserialize_str(&ballot_str)?;

                let hashable_multi_ballot_contests = hashable_multi_ballot
                    .deserialize_contests()
                    .map_err(|err| anyhow!("{:?}", err))?;
                Some(hashable_multi_ballot_contests.ciphertext)
            } else {
                let hashable_ballot: HashableBallot = deserialize_str(&ballot_str)?;
                let contests = hashable_ballot
                    .deserialize_contests()
                    .map_err(|err| anyhow!("{:?}", err))?;
                contests
                    .iter()
                    .find(|contest| contest.contest_id == contest_id)
                    .map(|contest| contest.ciphertext.clone())
            }
            .ok_or_else(|| {
                anyhow!(
                    "Could not get the ciphertext of contest {contest_id} from a ballot \
                     ({contest_encryption_policy:?})"
                )
            })?;
        let fingerprint = ciphertext_fingerprint(&ciphertext)
            .map_err(|err| anyhow!("Could not fingerprint ciphertext: {err:?}"))?;
        if seen.insert(fingerprint) {
            ciphertexts.push(ciphertext);
        }
    }
    Ok(ciphertexts)
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

#[cfg(test)]
mod tests {
    use super::*;
    use sequent_core::ballot::{HashableBallotContest, TYPES_VERSION};
    use sequent_core::encrypt::DEFAULT_PLAINTEXT_LABEL;
    use strand::context::Ctx;
    use strand::elgamal::{PrivateKey, PublicKey};

    const CONTEST_ID: &str = "contest";

    fn encrypted_contest(contest_id: &str) -> HashableBallotContest<RistrettoCtx> {
        let ctx = RistrettoCtx;
        let mut rng = ctx.get_rng();
        let secret_key = PrivateKey::gen(&ctx);
        let public_key = PublicKey::from_element(secret_key.pk_element(), &ctx);
        let (ciphertext, proof, _) = public_key
            .encrypt_and_pok(&ctx.rnd(&mut rng), &DEFAULT_PLAINTEXT_LABEL)
            .unwrap();
        HashableBallotContest {
            contest_id: contest_id.to_string(),
            ciphertext,
            proof,
        }
    }

    fn ballot(issue_date: &str, contests: Vec<HashableBallotContest<RistrettoCtx>>) -> String {
        serde_json::to_string(&HashableBallot {
            version: TYPES_VERSION,
            issue_date: issue_date.to_string(),
            contests: HashableBallot::serialize_contests(&contests).unwrap(),
            config: "style".to_string(),
            ballot_style_hash: "style-hash".to_string(),
        })
        .unwrap()
    }

    #[test]
    fn ciphertext_cast_by_several_voters_enters_the_batch_once() {
        let original = encrypted_contest(CONTEST_ID);
        let other = encrypted_contest(CONTEST_ID);
        let ballots = vec![
            ballot("2026-01-01", vec![original.clone()]),
            ballot("2026-01-02", vec![original.clone(), encrypted_contest("x")]),
            ballot("2026-01-03", vec![other.clone()]),
        ];

        let ciphertexts = contest_ciphertexts(
            ballots,
            &ContestEncryptionPolicy::SINGLE_CONTEST,
            CONTEST_ID,
        )
        .unwrap();

        assert_eq!(ciphertexts, vec![original.ciphertext, other.ciphertext]);
    }

    #[test]
    fn distinct_ciphertexts_keep_their_order() {
        let contests: Vec<_> = (0..3).map(|_| encrypted_contest(CONTEST_ID)).collect();
        let ballots = contests
            .iter()
            .map(|contest| ballot("2026-01-01", vec![contest.clone()]))
            .collect();

        let ciphertexts = contest_ciphertexts(
            ballots,
            &ContestEncryptionPolicy::SINGLE_CONTEST,
            CONTEST_ID,
        )
        .unwrap();

        assert_eq!(
            ciphertexts,
            contests
                .into_iter()
                .map(|contest| contest.ciphertext)
                .collect::<Vec<_>>()
        );
    }
}
