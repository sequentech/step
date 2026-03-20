// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::election::get_elections;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::trustee::get_trustees_by_name;
use crate::services::celery_app::get_worker_threads;
use crate::services::ceremonies::insert_ballots::adapters::{
    B3BulletinBoardAdapter, KeycloakVoterRepository, PostgresBallotRepository,
};
use crate::services::ceremonies::insert_ballots::domain::merge_join_csv;
use crate::services::ceremonies::insert_ballots::ports::{
    BallotRepository, BulletinBoardPort, VoterRepository,
};
use crate::services::election::get_election_event_elections;
use crate::services::protocol_manager::*;
use crate::services::public_keys::deserialize_public_key;
use anyhow::{anyhow, Context, Result};
use b3::messages::newtypes::BatchNumber;
use base64::{
    alphabet,
    engine::{self, general_purpose},
    Engine as _,
};
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use sequent_core::ballot::{
    ContestEncryptionPolicy, DelegatedVotingPolicy, ElectionPresentation, HashableBallot,
};
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
use tempfile::NamedTempFile;
use tokio::task::JoinHandle;
use tracing::{event, info, instrument, Level};

use std::sync::Arc;

pub mod adapters;
pub mod domain;
pub mod ports;

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
    delegated_voting_policy: DelegatedVotingPolicy,
) -> Result<Vec<TallySessionContest>> {
    let trustees = get_trustees_by_name(hasura_transaction, &tenant_id, &trustee_names).await?;

    event!(Level::INFO, "trustees len: {:?}", trustees.len());

    let deserialized_trustee_pks = trustees
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

    let bulletin_board = Arc::new(
        B3BulletinBoardAdapter::new(
            hasura_transaction,
            tenant_id,
            election_event_id,
            board_name,
            deserialized_trustee_pks,
        )
        .await?,
    );

    let election_ids_alias: HashMap<String, String> =
        get_election_event_elections(&hasura_transaction, tenant_id, election_event_id)
            .await?
            .into_iter()
            .filter_map(|election| election.external_id.map(|x| (election.id.clone(), x)))
            .collect();

    let mut tally_session_contests_updated = Vec::with_capacity(tally_session_contests.len());

    for tally_session_contest_batch in tally_session_contests.chunks(get_worker_threads()) {
        let mut tasks = Vec::with_capacity(get_worker_threads());
        let tally_session_contest_batch = tally_session_contest_batch.to_vec();

        for tally_session_contest in tally_session_contest_batch {
            let board_name_clone = board_name.to_string();
            let election_ids_alias_clone = election_ids_alias.clone();
            let contest_encryption_policy_clone = contest_encryption_policy.clone();
            let realm_clone = realm.clone();
            let bulletin_board_clone = Arc::clone(&bulletin_board);
            let is_delegated = delegated_voting_policy == DelegatedVotingPolicy::ENABLED;

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

                    let election_alias =
                        match election_ids_alias_clone.get(&tally_session_contest.election_id) {
                            Some(alias) => alias,
                            None => "",
                        }
                        .to_string();

                    let ballot_repo = PostgresBallotRepository;
                    let voter_repo = KeycloakVoterRepository;
                    process_contest(
                        &ballot_repo,
                        &voter_repo,
                        &*bulletin_board_clone,
                        &tally_session_contest,
                        &realm_clone,
                        &election_alias,
                        &board_name_clone,
                        contest_encryption_policy_clone,
                        is_delegated,
                    )
                    .await
                })
            });
            tasks.push(task);
        }

        let mut tally_session_contest_to_add: Vec<TallySessionContest> =
            futures::future::try_join_all(tasks)
                .await?
                .into_iter()
                .collect::<Result<Vec<_>>>()?;

        tally_session_contests_updated.append(&mut tally_session_contest_to_add);
    }

    Ok(tally_session_contests_updated)
}

/// Processes a single tally session contest: fetches ballots and voters via the provided ports,
/// performs a merge join to filter eligible ballots, extracts ciphertexts, submits them to the
/// bulletin board, and returns the contest updated with annotation counts.
pub(crate) async fn process_contest(
    ballot_repo: &dyn BallotRepository,
    voter_repo: &dyn VoterRepository,
    bulletin_board: &dyn BulletinBoardPort,
    tally_session_contest: &TallySessionContest,
    realm: &str,
    election_alias: &str,
    board_name: &str,
    contest_encryption_policy: ContestEncryptionPolicy,
    is_delegated: bool,
) -> Result<TallySessionContest> {
    let ballots_temp_file = NamedTempFile::new()
        .map_err(|error| anyhow!("Failed to create temp file {}", error))?;
    event!(
        Level::INFO,
        "Creating temporary file for ballots with path {:?}",
        ballots_temp_file.path()
    );

    ballot_repo
        .find_area_ballots(
            &tally_session_contest.tenant_id,
            &tally_session_contest.election_event_id,
            &tally_session_contest.area_id,
            &tally_session_contest.election_id,
            &ballots_temp_file.path().to_path_buf(),
        )
        .await?;

    let ballots_temp_file = ballots_temp_file.reopen()?;

    let users_temp_file = NamedTempFile::new()
        .map_err(|error| anyhow!("Failed to create temp file: {}", error))?;
    event!(
        Level::INFO,
        "Creating temporary file for users with path {:?}",
        users_temp_file.path()
    );

    voter_repo
        .find_eligible_voters(
            realm,
            &tally_session_contest.area_id,
            election_alias,
            &users_temp_file.path().to_path_buf(),
            is_delegated,
        )
        .await?;

    let users_temp_file = users_temp_file.reopen()?;

    let ballots_output_index = 1;
    let ballots_join_indexes = 0;
    let users_join_indexes = 0;
    let contest_id = tally_session_contest.contest_id.clone();

    let delegate_count_index = if is_delegated { Some(1) } else { None };

    let (ballot_contents, elegible_voters, ballots_without_voter, casted_ballots) =
        merge_join_csv(
            &ballots_temp_file,
            &users_temp_file,
            ballots_join_indexes,
            users_join_indexes,
            ballots_output_index,
            delegate_count_index,
        )?;

    let ciphertexts = ballot_contents
        .into_iter()
        .map(|ballot_str| {
            info!("ballot_str: {ballot_str}");
            let ciphertext: Ciphertext<RistrettoCtx> =
                if ContestEncryptionPolicy::MULTIPLE_CONTESTS == contest_encryption_policy {
                    let hashable_multi_ballot: HashableMultiBallot =
                        deserialize_str(&ballot_str)?;
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
                        .find(|contest| {
                            contest.contest_id == contest_id.clone().unwrap_or_default()
                        })
                        .map(|contest| contest.ciphertext.clone())
                }
                .ok_or(anyhow!("Could not get ciphertext"))?;
            Ok(ciphertext)
        })
        .collect::<Result<Vec<_>>>()?;

    let annotations = TallySessionContestAnnotations {
        elegible_voters,
        ballots_without_voter,
        casted_ballots,
    };

    let annotations = serde_json::to_value(&annotations)?;

    event!(
        Level::INFO,
        "insertable_ballots len: {:?}",
        ciphertexts.len()
    );

    let batch = tally_session_contest.session_id as BatchNumber;
    bulletin_board
        .add_ballots(board_name, ciphertexts, batch)
        .await?;

    Ok(TallySessionContest {
        id: tally_session_contest.id.clone(),
        tenant_id: tally_session_contest.tenant_id.clone(),
        election_event_id: tally_session_contest.election_event_id.clone(),
        area_id: tally_session_contest.area_id.clone(),
        contest_id: tally_session_contest.contest_id.clone(),
        session_id: tally_session_contest.session_id,
        created_at: tally_session_contest.created_at,
        last_updated_at: tally_session_contest.last_updated_at,
        labels: tally_session_contest.labels.clone(),
        annotations: Some(annotations),
        tally_session_id: tally_session_contest.tally_session_id.clone(),
        election_id: tally_session_contest.election_id.clone(),
    })
}

#[instrument(skip_all, err)]
pub async fn get_elections_end_dates(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<HashMap<String, Option<DateTime<Utc>>>> {
    let elections = get_elections(hasura_transaction, tenant_id, election_event_id)
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
    use super::process_contest;
    use crate::services::ceremonies::insert_ballots::ports::{
        BallotRepository, BulletinBoardPort, VoterRepository,
    };
    use anyhow::Result;
    use async_trait::async_trait;
    use b3::messages::newtypes::BatchNumber;
    use sequent_core::ballot::ContestEncryptionPolicy;
    use sequent_core::types::hasura::core::{TallySessionContest, TallySessionContestAnnotations};
    use std::path::PathBuf;
    use strand::backend::ristretto::RistrettoCtx;
    use strand::elgamal::Ciphertext;

    struct FakeBallotRepository(String);

    #[async_trait]
    impl BallotRepository for FakeBallotRepository {
        async fn find_area_ballots(
            &self,
            _tenant_id: &str,
            _election_event_id: &str,
            _area_id: &str,
            _election_id: &str,
            output_file: &PathBuf,
        ) -> Result<()> {
            tokio::fs::write(output_file, self.0.as_bytes()).await?;
            Ok(())
        }
    }

    struct FakeVoterRepository(String);

    #[async_trait]
    impl VoterRepository for FakeVoterRepository {
        async fn find_eligible_voters(
            &self,
            _realm: &str,
            _area_id: &str,
            _election_alias: &str,
            output_file: &PathBuf,
            _delegated_voting_enabled: bool,
        ) -> Result<()> {
            tokio::fs::write(output_file, self.0.as_bytes()).await?;
            Ok(())
        }
    }

    struct FakeBulletinBoardPort;

    #[async_trait]
    impl BulletinBoardPort for FakeBulletinBoardPort {
        async fn add_ballots(
            &self,
            _board_name: &str,
            _ciphertexts: Vec<Ciphertext<RistrettoCtx>>,
            _batch: BatchNumber,
        ) -> Result<()> {
            Ok(())
        }
    }

    fn make_contest() -> TallySessionContest {
        TallySessionContest {
            id: "test-contest-id".to_string(),
            tenant_id: "test-tenant".to_string(),
            election_event_id: "test-ee".to_string(),
            area_id: "area-1".to_string(),
            contest_id: Some("contest-1".to_string()),
            session_id: 1,
            created_at: None,
            last_updated_at: None,
            labels: None,
            annotations: None,
            tally_session_id: "ts-1".to_string(),
            election_id: "election-1".to_string(),
        }
    }

    fn parse_annotations(contest: &TallySessionContest) -> TallySessionContestAnnotations {
        serde_json::from_value(contest.annotations.clone().expect("annotations should be set"))
            .expect("failed to parse annotations")
    }

    /// Ballot cast by a voter not in the eligible-voters list → counts as ballots_without_voter.
    #[tokio::test]
    async fn test_process_contest_no_eligible_voters() -> Result<()> {
        let ballot_repo = FakeBallotRepository("voter_a,ballot_content\n".to_string());
        let voter_repo = FakeVoterRepository("".to_string());
        let board = FakeBulletinBoardPort;
        let contest = make_contest();

        let updated = process_contest(
            &ballot_repo,
            &voter_repo,
            &board,
            &contest,
            "realm",
            "",
            "board",
            ContestEncryptionPolicy::SINGLE_CONTEST,
            false,
        )
        .await?;

        let ann = parse_annotations(&updated);
        assert_eq!(ann.elegible_voters, 0);
        assert_eq!(ann.casted_ballots, 1);
        assert_eq!(ann.ballots_without_voter, 1);
        Ok(())
    }

    /// Ballot's voter ID does not appear in the eligible-voters file.
    #[tokio::test]
    async fn test_process_contest_voter_ballot_mismatch() -> Result<()> {
        let ballot_repo = FakeBallotRepository("voter_x,ballot_content\n".to_string());
        let voter_repo = FakeVoterRepository("voter_y\n".to_string());
        let board = FakeBulletinBoardPort;
        let contest = make_contest();

        let updated = process_contest(
            &ballot_repo,
            &voter_repo,
            &board,
            &contest,
            "realm",
            "",
            "board",
            ContestEncryptionPolicy::SINGLE_CONTEST,
            false,
        )
        .await?;

        let ann = parse_annotations(&updated);
        assert_eq!(ann.elegible_voters, 1);
        assert_eq!(ann.casted_ballots, 1);
        assert_eq!(ann.ballots_without_voter, 1);
        Ok(())
    }

    /// Delegated voting enabled, eligible voter present, but no ballots cast.
    #[tokio::test]
    async fn test_process_contest_delegated_no_ballots() -> Result<()> {
        let ballot_repo = FakeBallotRepository("".to_string());
        let voter_repo = FakeVoterRepository("voter_a,2\n".to_string());
        let board = FakeBulletinBoardPort;
        let contest = make_contest();

        let updated = process_contest(
            &ballot_repo,
            &voter_repo,
            &board,
            &contest,
            "realm",
            "",
            "board",
            ContestEncryptionPolicy::SINGLE_CONTEST,
            true,
        )
        .await?;

        let ann = parse_annotations(&updated);
        assert_eq!(ann.elegible_voters, 1);
        assert_eq!(ann.casted_ballots, 0);
        assert_eq!(ann.ballots_without_voter, 0);
        Ok(())
    }
}
