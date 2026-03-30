// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Application services for the insert-ballots ceremony flow.
//!
//! The types in this module coordinate the ports defined in `ports.rs` without
//! owning transactions or infrastructure clients directly. That keeps the
//! orchestration testable while allowing the caller to control transaction
//! propagation and retries.

use super::adapters::{extract_ciphertexts, merge_ballots_and_voters};
use super::{
    BallotBoardPort, ContestBallotUpserter, InsertBallotsBoardContext, PrepareBoardContextRequest,
    TrusteePublicKeyResolver, UpsertBallotsMessagesRequest,
};
use crate::repositories::ballots::BallotRepository;
use crate::repositories::election_event::ElectionEventRepository;
use crate::repositories::trustees::TrusteeRepository;
use crate::repositories::voters::EligibleUserRepository;
use crate::services::public_keys::deserialize_public_key;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use b3::messages::statement::StatementType;
use futures::stream::{self, StreamExt, TryStreamExt};
use sequent_core::ballot::{ContestEncryptionPolicy, DelegatedVotingPolicy};
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::hasura::core::{TallySessionContest, TallySessionContestAnnotations};
use std::collections::HashMap;
use std::collections::HashSet;
use tempfile::NamedTempFile;
use tracing::{event, instrument, Level};

#[derive(Clone)]
pub(super) struct InsertBallotsRequest {
    pub tenant_id: String,
    pub election_event_id: String,
    pub board_name: String,
    pub trustee_names: Vec<String>,
    pub tally_session_contests: Vec<TallySessionContest>,
    pub contest_encryption_policy: ContestEncryptionPolicy,
    pub delegated_voting_policy: DelegatedVotingPolicy,
    pub skip_board_posting: bool,
}

/// Resolves trustee names into deserialized public keys using a trustee
/// repository supplied by the orchestration layer.
///
/// This service is intentionally small: it centralizes key-loading validation
/// so `InsertBallotsService` can focus on orchestration rather than trustee data
/// cleanup.
pub struct TrusteePublicKeysService<'a, R>
where
    R: TrusteeRepository,
{
    trustee_repository: &'a R,
}

impl<'a, R> TrusteePublicKeysService<'a, R>
where
    R: TrusteeRepository,
{
    /// Creates a trustee-key resolver backed by the provided repository.
    ///
    /// The repository is expected to be transaction-bound and owned by the
    /// outer orchestration layer.
    pub fn new(trustee_repository: &'a R) -> Self {
        Self { trustee_repository }
    }
}

#[async_trait]
impl<R> TrusteePublicKeyResolver for TrusteePublicKeysService<'_, R>
where
    R: TrusteeRepository,
{
    async fn resolve_public_keys(
        &self,
        tenant_id: &str,
        trustee_names: &[String],
    ) -> Result<Vec<strand::signature::StrandSignaturePk>> {
        let trustees = self
            .trustee_repository
            .get_trustees_by_name(tenant_id, trustee_names)
            .await?;
        let trustee_keys_by_name = trustees.into_iter().try_fold(
            HashMap::new(),
            |mut keys_by_name,
             trustee|
             -> Result<HashMap<String, strand::signature::StrandSignaturePk>> {
                let trustee_name = trustee
                    .name
                    .clone()
                    .ok_or(anyhow!("Missing trustee name"))?;
                let public_key = trustee.public_key.ok_or(anyhow!(
                    "Missing trustee public key for trustee {trustee_name}"
                ))?;
                let deserialized_public_key = deserialize_public_key(public_key)?;
                keys_by_name.insert(trustee_name, deserialized_public_key);
                Ok(keys_by_name)
            },
        )?;

        trustee_names
            .iter()
            .map(|trustee_name| {
                trustee_keys_by_name
                    .get(trustee_name)
                    .cloned()
                    .ok_or(anyhow!(
                        "Missing trustee public key for trustee {trustee_name}"
                    ))
            })
            .collect()
    }
}

/// Orchestrates ballot export, annotation computation, optional ciphertext
/// decoding, and optional board posting for a set of tally-session contests.
///
/// The service performs one-time setup, such as election-alias loading and
/// board-context preparation, before processing contests concurrently within the
/// lifetime of the caller-owned dependencies.
pub struct InsertBallotsService<'a, T, E, B, U, O>
where
    T: TrusteePublicKeyResolver,
    E: ElectionEventRepository,
    B: BallotRepository,
    U: EligibleUserRepository,
    O: BallotBoardPort,
{
    trustee_public_key_resolver: &'a T,
    election_event_repository: &'a E,
    ballot_repository: &'a B,
    eligible_user_repository: &'a U,
    board_port: &'a O,
    worker_threads: usize,
}

impl<'a, T, E, B, U, O> InsertBallotsService<'a, T, E, B, U, O>
where
    T: TrusteePublicKeyResolver,
    E: ElectionEventRepository,
    B: BallotRepository,
    U: EligibleUserRepository,
    O: BallotBoardPort,
{
    /// Creates the insert-ballots application service.
    ///
    /// All collaborators are borrowed from the outer orchestration layer so the
    /// caller remains responsible for transaction scope and lifecycle.
    pub fn new(
        trustee_public_key_resolver: &'a T,
        election_event_repository: &'a E,
        ballot_repository: &'a B,
        eligible_user_repository: &'a U,
        board_port: &'a O,
        worker_threads: usize,
    ) -> Self {
        Self {
            trustee_public_key_resolver,
            election_event_repository,
            ballot_repository,
            eligible_user_repository,
            board_port,
            worker_threads: worker_threads.max(1),
        }
    }

    /// Reconciles board messages and tally-session contests.
    ///
    /// Missing ballot batches are inserted with board posting enabled, while
    /// contests that already have ballots on the board are reprocessed only to
    /// recover missing annotations.
    #[instrument(skip_all, err)]
    pub async fn execute(
        &self,
        request: UpsertBallotsMessagesRequest<'_>,
    ) -> Result<Vec<TallySessionContest>> {
        let contest_encryption_policy = request
            .tally_session
            .configuration
            .clone()
            .unwrap_or_default()
            .get_contest_encryption_policy();
        let delegated_voting_policy = request
            .tally_session
            .configuration
            .clone()
            .unwrap_or_default()
            .get_delegated_voting_policy();

        let expected_batch_ids: HashSet<i64> = request
            .tally_session_contests
            .iter()
            .map(|contest| contest.session_id as i64)
            .collect();
        let existing_ballots_batches: HashSet<i64> = request
            .messages
            .iter()
            .filter(|message| {
                expected_batch_ids.contains(&(message.statement.get_batch_number() as i64))
                    && StatementType::Ballots == message.statement.get_kind()
            })
            .map(|message| message.statement.get_batch_number() as i64)
            .collect();

        let missing_ballots_batches: Vec<TallySessionContest> = request
            .tally_session_contests
            .iter()
            .filter(|contest| !existing_ballots_batches.contains(&(contest.session_id as i64)))
            .cloned()
            .collect();
        let missing_annotations_batches: Vec<TallySessionContest> = request
            .tally_session_contests
            .iter()
            .filter(|contest| {
                existing_ballots_batches.contains(&(contest.session_id as i64))
                    && contest.annotations.is_none()
            })
            .cloned()
            .collect();

        let mut updated_contests = Vec::new();
        if !missing_ballots_batches.is_empty() {
            updated_contests.extend(
                self.insert_ballots(InsertBallotsRequest {
                    tenant_id: request.tenant_id.clone(),
                    election_event_id: request.election_event_id.clone(),
                    board_name: request.board_name.clone(),
                    trustee_names: request.trustee_names.clone(),
                    tally_session_contests: missing_ballots_batches,
                    contest_encryption_policy: contest_encryption_policy.clone(),
                    delegated_voting_policy: delegated_voting_policy.clone(),
                    skip_board_posting: false,
                })
                .await?,
            );
        }

        if !missing_annotations_batches.is_empty() {
            updated_contests.extend(
                self.insert_ballots(InsertBallotsRequest {
                    tenant_id: request.tenant_id,
                    election_event_id: request.election_event_id,
                    board_name: request.board_name,
                    trustee_names: request.trustee_names,
                    tally_session_contests: missing_annotations_batches,
                    contest_encryption_policy,
                    delegated_voting_policy,
                    skip_board_posting: true,
                })
                .await?,
            );
        }

        Ok(updated_contests)
    }

    /// Executes the insert-ballots use case for the provided contest batches.
    ///
    /// When board posting is enabled, trustee keys and board context are loaded
    /// once and reused across all contests. Contest processing then runs with
    /// bounded concurrency controlled by `worker_threads`.
    #[instrument(skip_all, err)]
    pub(super) async fn insert_ballots(
        &self,
        request: InsertBallotsRequest,
    ) -> Result<Vec<TallySessionContest>> {
        let InsertBallotsRequest {
            tenant_id,
            election_event_id,
            board_name,
            trustee_names,
            tally_session_contests,
            contest_encryption_policy,
            delegated_voting_policy,
            skip_board_posting,
        } = request;

        let realm = get_event_realm(&tenant_id, &election_event_id);
        let delegated_voting_enabled =
            delegated_voting_policy == sequent_core::ballot::DelegatedVotingPolicy::ENABLED;
        let election_ids_alias = self
            .election_event_repository
            .get_election_aliases(&tenant_id, &election_event_id)
            .await?;

        let board_context = if skip_board_posting {
            None
        } else {
            let trustee_public_keys = self
                .trustee_public_key_resolver
                .resolve_public_keys(&tenant_id, &trustee_names)
                .await?;

            Some(
                self.board_port
                    .prepare_board_context(PrepareBoardContextRequest {
                        tenant_id: tenant_id.clone(),
                        election_event_id: election_event_id.clone(),
                        board_name: board_name.clone(),
                        trustee_public_keys,
                    })
                    .await?,
            )
        };

        let service = self;
        stream::iter(
            tally_session_contests
                .into_iter()
                .map(|tally_session_contest| {
                    let tenant_id = tenant_id.clone();
                    let election_event_id = election_event_id.clone();
                    let board_name = board_name.clone();
                    let realm = realm.clone();
                    let election_ids_alias = election_ids_alias.clone();
                    let contest_encryption_policy = contest_encryption_policy.clone();
                    let board_context = board_context.clone();

                    async move {
                        service
                            .process_contest(
                                tally_session_contest,
                                &tenant_id,
                                &election_event_id,
                                &board_name,
                                &realm,
                                delegated_voting_enabled,
                                &election_ids_alias,
                                &contest_encryption_policy,
                                board_context.as_ref(),
                            )
                            .await
                    }
                }),
        )
        .buffer_unordered(self.worker_threads)
        .try_collect()
        .await
    }

    /// Processes one tally-session contest.
    ///
    /// The workflow exports ballots and eligible users, computes annotations,
    /// and optionally posts the decoded ciphertexts to the board using the
    /// previously prepared board context.
    #[instrument(skip_all, err)]
    async fn process_contest(
        &self,
        tally_session_contest: TallySessionContest,
        tenant_id: &str,
        election_event_id: &str,
        board_name: &str,
        realm: &str,
        delegated_voting_enabled: bool,
        election_ids_alias: &HashMap<String, String>,
        contest_encryption_policy: &sequent_core::ballot::ContestEncryptionPolicy,
        board_context: Option<&InsertBallotsBoardContext>,
    ) -> Result<TallySessionContest> {
        event!(
            Level::INFO,
            "Inserting Ballots message for election {}, contest {}, area {} and batch num {}",
            tally_session_contest.election_id,
            tally_session_contest.contest_id.clone().unwrap_or_default(),
            tally_session_contest.area_id,
            tally_session_contest.session_id,
        );

        let ballots_temp_file = NamedTempFile::new()?;
        self.ballot_repository
            .export_area_ballots(
                tenant_id,
                election_event_id,
                &tally_session_contest.area_id,
                &tally_session_contest.election_id,
                ballots_temp_file.path(),
            )
            .await?;
        let ballots_file = ballots_temp_file.reopen()?;

        let users_temp_file = NamedTempFile::new()?;
        let election_alias = election_ids_alias
            .get(&tally_session_contest.election_id)
            .map(String::as_str)
            .unwrap_or_default();
        self.eligible_user_repository
            .export_enabled_users(
                realm,
                &tally_session_contest.area_id,
                election_alias,
                users_temp_file.path(),
                delegated_voting_enabled,
            )
            .await?;
        let users_file = users_temp_file.reopen()?;

        let processing_output =
            merge_ballots_and_voters(&ballots_file, &users_file, delegated_voting_enabled)?;

        let annotations = serde_json::to_value(TallySessionContestAnnotations {
            elegible_voters: processing_output.elegible_voters,
            ballots_without_voter: processing_output.ballots_without_voter,
            casted_ballots: processing_output.casted_ballots,
        })?;

        let updated_tally_session_contest = TallySessionContest {
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
        };

        if let Some(board_context) = board_context {
            let ciphertexts = extract_ciphertexts(
                processing_output.ballot_contents,
                contest_encryption_policy,
                tally_session_contest.contest_id.as_deref(),
            )?;
            self.board_port
                .post_ballots(
                    board_name,
                    board_context,
                    tally_session_contest.session_id as b3::messages::newtypes::BatchNumber,
                    ciphertexts,
                )
                .await?;
        }

        Ok(updated_tally_session_contest)
    }
}

#[async_trait]
impl<T, E, B, U, O> ContestBallotUpserter for InsertBallotsService<'_, T, E, B, U, O>
where
    T: TrusteePublicKeyResolver,
    E: ElectionEventRepository,
    B: BallotRepository,
    U: EligibleUserRepository,
    O: BallotBoardPort,
{
    async fn upsert_ballots(
        &self,
        request: UpsertBallotsMessagesRequest<'_>,
    ) -> Result<Vec<TallySessionContest>> {
        self.execute(request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::ballots::BallotRepository;
    use crate::repositories::election_event::ElectionEventRepository;
    use crate::repositories::trustees::TrusteeRepository;
    use crate::repositories::voters::EligibleUserRepository;
    use crate::services::ceremonies::insert_ballots::{
        BallotBoardPort, InsertBallotsBoardContext, PrepareBoardContextRequest,
        TrusteePublicKeyResolver,
    };
    use anyhow::Result;
    use async_trait::async_trait;
    use b3::messages::artifact::Configuration;
    use b3::messages::message::{Message, Sender};
    use b3::messages::newtypes::{
        BatchNumber, CiphertextsHash, ConfigurationHash, PublicKeyHash, NULL_TRUSTEE,
    };
    use b3::messages::protocol_manager::ProtocolManager;
    use b3::messages::statement::Statement;
    use chrono::Local;
    use mockall::mock;
    use sequent_core::ballot::{
        BallotStyle, ContestEncryptionPolicy, DelegatedVotingPolicy, HashableBallot,
        PublicKeyConfig, SignedHashableBallot,
    };
    use sequent_core::encrypt::{encrypt_decoded_contest, DEFAULT_PUBLIC_KEY_RISTRETTO_STR};
    use sequent_core::fixtures::ballot_codec::get_fixtures;
    use sequent_core::types::hasura::core::{TallySession, Trustee};
    use serde_json::json;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use strand::backend::ristretto::RistrettoCtx;
    use strand::signature::{StrandSignaturePk, StrandSignatureSk};

    const FIXTURE_CONTEST_ID: &str = "1fc963b1-f93b-4151-93d6-bbe0ea5eac46";

    #[derive(Default)]
    struct TrusteeRepositoryFake {
        trustees: Vec<Trustee>,
    }

    #[async_trait]
    impl TrusteeRepository for TrusteeRepositoryFake {
        async fn get_trustees_by_name(
            &self,
            _tenant_id: &str,
            _trustee_names: &[String],
        ) -> Result<Vec<Trustee>> {
            Ok(self.trustees.clone())
        }
    }

    #[derive(Default)]
    struct TrusteePublicKeyResolverFake {
        call_count: Mutex<usize>,
        resolved_keys: Vec<StrandSignaturePk>,
    }

    #[async_trait]
    impl TrusteePublicKeyResolver for TrusteePublicKeyResolverFake {
        async fn resolve_public_keys(
            &self,
            _tenant_id: &str,
            _trustee_names: &[String],
        ) -> Result<Vec<StrandSignaturePk>> {
            *self.call_count.lock().unwrap() += 1;
            Ok(self.resolved_keys.clone())
        }
    }

    mock! {
        ElectionEventRepositoryDouble {}

        #[async_trait]
        impl ElectionEventRepository for ElectionEventRepositoryDouble {
            async fn get_election_aliases(
                &self,
                tenant_id: &str,
                election_event_id: &str,
            ) -> std::result::Result<HashMap<String, String>, anyhow::Error>;
        }
    }

    mock! {
        BallotRepositoryDouble {}

        #[async_trait]
        impl BallotRepository for BallotRepositoryDouble {
            async fn export_area_ballots(
                &self,
                tenant_id: &str,
                election_event_id: &str,
                area_id: &str,
                election_id: &str,
                output_path: &std::path::Path,
            ) -> std::result::Result<(), anyhow::Error>;
        }
    }

    mock! {
        EligibleUserRepositoryDouble {}

        #[async_trait]
        impl EligibleUserRepository for EligibleUserRepositoryDouble {
            async fn export_enabled_users(
                &self,
                realm: &str,
                area_id: &str,
                election_alias: &str,
                output_path: &std::path::Path,
                delegated_voting_enabled: bool,
            ) -> std::result::Result<(), anyhow::Error>;
        }
    }

    mock! {
        BallotBoardPortDouble {}

        #[async_trait]
        impl BallotBoardPort for BallotBoardPortDouble {
            async fn prepare_board_context(
                &self,
                request: PrepareBoardContextRequest,
            ) -> std::result::Result<InsertBallotsBoardContext, anyhow::Error>;

            async fn post_ballots(
                &self,
                board_name: &str,
                board_context: &InsertBallotsBoardContext,
                batch: BatchNumber,
                ciphertexts: Vec<strand::elgamal::Ciphertext<strand::backend::ristretto::RistrettoCtx>>,
            ) -> std::result::Result<(), anyhow::Error>;
        }
    }

    fn sample_contest(
        session_id: i32,
        annotations: Option<serde_json::Value>,
    ) -> TallySessionContest {
        TallySessionContest {
            id: format!("contest-{session_id}"),
            tenant_id: "tenant".to_string(),
            election_event_id: "event".to_string(),
            area_id: "area".to_string(),
            contest_id: Some(FIXTURE_CONTEST_ID.to_string()),
            session_id,
            created_at: Some(Local::now()),
            last_updated_at: Some(Local::now()),
            labels: None,
            annotations,
            tally_session_id: "tally-session".to_string(),
            election_id: "election-1".to_string(),
        }
    }

    fn ballot_csv_row(voter_id: &str, ballot_payload: &str) -> String {
        format!("{voter_id},\"{}\"\n", ballot_payload.replace('"', "\"\""))
    }

    fn serialized_hashable_ballot() -> String {
        let fixture = get_fixtures()
            .into_iter()
            .find(|fixture| fixture.title == "plurality_fixture")
            .unwrap();
        let ballot_style = BallotStyle {
            id: "ballot-style".to_string(),
            tenant_id: "tenant".to_string(),
            election_event_id: "event".to_string(),
            election_id: "election-1".to_string(),
            num_allowed_revotes: Some(1),
            description: Some("fixture".to_string()),
            public_key: Some(PublicKeyConfig {
                public_key: DEFAULT_PUBLIC_KEY_RISTRETTO_STR.to_string(),
                is_demo: false,
            }),
            area_id: "area".to_string(),
            area_presentation: None,
            contests: vec![fixture.contest],
            election_event_presentation: None,
            election_presentation: None,
            election_dates: None,
            election_event_annotations: None,
            election_annotations: None,
            area_annotations: None,
        };
        let auditable_ballot = encrypt_decoded_contest::<RistrettoCtx>(
            &RistrettoCtx,
            &vec![fixture.plaintext],
            &ballot_style,
        )
        .unwrap();
        let signed_hashable_ballot = SignedHashableBallot::try_from(&auditable_ballot).unwrap();
        let hashable_ballot = HashableBallot::try_from(&signed_hashable_ballot).unwrap();

        serde_json::to_string(&hashable_ballot).unwrap()
    }

    fn ballots_message(batch: i32) -> Message {
        let signing_key = StrandSignatureSk::gen().unwrap();

        Message {
            sender: Sender::new(
                "sender".to_string(),
                StrandSignaturePk::from_sk(&signing_key).unwrap(),
            ),
            signature: signing_key.sign(&[]).unwrap(),
            statement: Statement::Ballots(
                0,
                ConfigurationHash([0; 64]),
                batch as usize,
                CiphertextsHash([0; 64]),
                PublicKeyHash([0; 64]),
                [NULL_TRUSTEE; 12],
            ),
            artifact: None,
        }
    }

    #[tokio::test]
    async fn insert_service_computes_annotations_without_board_posting() {
        let trustee_public_key_resolver = TrusteePublicKeyResolverFake::default();
        let mut election_repository = MockElectionEventRepositoryDouble::new();
        election_repository
            .expect_get_election_aliases()
            .once()
            .returning(|_, _| {
                Ok(HashMap::from([(
                    "election-1".to_string(),
                    "alias-1".to_string(),
                )]))
            });
        let mut ballot_repository = MockBallotRepositoryDouble::new();
        ballot_repository
            .expect_export_area_ballots()
            .once()
            .returning(|_, _, _, _, output_path| {
                std::fs::write(output_path, "user-1,ballot")?;
                Ok(())
            });
        let mut eligible_user_repository = MockEligibleUserRepositoryDouble::new();
        eligible_user_repository
            .expect_export_enabled_users()
            .once()
            .returning(|_, _, _, output_path, _| {
                std::fs::write(output_path, "user-1")?;
                Ok(())
            });
        let mut board_port = MockBallotBoardPortDouble::new();
        board_port.expect_prepare_board_context().times(0);
        board_port.expect_post_ballots().times(0);
        let service = InsertBallotsService::new(
            &trustee_public_key_resolver,
            &election_repository,
            &ballot_repository,
            &eligible_user_repository,
            &board_port,
            2,
        );

        let contests = service
            .insert_ballots(InsertBallotsRequest {
                tenant_id: "tenant".to_string(),
                election_event_id: "event".to_string(),
                board_name: "board".to_string(),
                trustee_names: vec![],
                tally_session_contests: vec![sample_contest(1, None)],
                contest_encryption_policy: ContestEncryptionPolicy::SINGLE_CONTEST,
                delegated_voting_policy: DelegatedVotingPolicy::DISABLED,
                skip_board_posting: true,
            })
            .await
            .unwrap();

        let annotations = contests[0].annotations.clone().unwrap();
        assert_eq!(annotations["elegible_voters"], 1);
        assert_eq!(annotations["ballots_without_voter"], 0);
        assert_eq!(annotations["casted_ballots"], 1);
        assert_eq!(*trustee_public_key_resolver.call_count.lock().unwrap(), 0);
    }

    #[tokio::test]
    async fn insert_service_prepares_board_and_posts_batches() {
        let sk = StrandSignatureSk::gen().unwrap();
        let trustee_public_key_resolver = TrusteePublicKeyResolverFake {
            call_count: Mutex::new(0),
            resolved_keys: vec![StrandSignaturePk::from_sk(&sk).unwrap()],
        };
        let mut election_repository = MockElectionEventRepositoryDouble::new();
        election_repository
            .expect_get_election_aliases()
            .once()
            .returning(|_, _| {
                Ok(HashMap::from([(
                    "election-1".to_string(),
                    "alias-1".to_string(),
                )]))
            });
        let ballot_payload = serialized_hashable_ballot();
        let ballot_row = ballot_csv_row("user-1", &ballot_payload);
        let mut ballot_repository = MockBallotRepositoryDouble::new();
        ballot_repository
            .expect_export_area_ballots()
            .once()
            .returning(move |_, _, _, _, output_path| {
                std::fs::write(output_path, ballot_row.clone())?;
                Ok(())
            });
        let mut eligible_user_repository = MockEligibleUserRepositoryDouble::new();
        eligible_user_repository
            .expect_export_enabled_users()
            .once()
            .returning(|_, _, _, output_path, _| {
                std::fs::write(output_path, "user-1")?;
                Ok(())
            });
        let post_calls = Arc::new(Mutex::new(Vec::new()));
        let post_calls_clone = Arc::clone(&post_calls);
        let mut board_port = MockBallotBoardPortDouble::new();
        board_port
            .expect_prepare_board_context()
            .once()
            .returning(|request| {
                let signing_key = StrandSignatureSk::gen()?;
                let trustee_key_1 = StrandSignatureSk::gen()?;
                let trustee_key_2 = StrandSignatureSk::gen()?;
                Ok(InsertBallotsBoardContext {
                    protocol_manager: Arc::new(ProtocolManager::new(signing_key)),
                    messages: Arc::new(Vec::<Message>::new()),
                    configuration: Configuration::new(
                        0,
                        StrandSignaturePk::from_sk(&trustee_key_1)?,
                        vec![
                            StrandSignaturePk::from_sk(&trustee_key_1)?,
                            StrandSignaturePk::from_sk(&trustee_key_2)?,
                        ],
                        2,
                        std::marker::PhantomData,
                    ),
                    public_key_hash: PublicKeyHash([0; 64]),
                    selected_trustees: {
                        let _ = request;
                        [NULL_TRUSTEE; 12]
                    },
                })
            });
        board_port
            .expect_post_ballots()
            .once()
            .returning(move |_, _, batch, _| {
                post_calls_clone.lock().unwrap().push(batch);
                Ok(())
            });
        let service = InsertBallotsService::new(
            &trustee_public_key_resolver,
            &election_repository,
            &ballot_repository,
            &eligible_user_repository,
            &board_port,
            2,
        );

        service
            .insert_ballots(InsertBallotsRequest {
                tenant_id: "tenant".to_string(),
                election_event_id: "event".to_string(),
                board_name: "board".to_string(),
                trustee_names: vec!["Trustee 1".to_string()],
                tally_session_contests: vec![sample_contest(7, None)],
                contest_encryption_policy: ContestEncryptionPolicy::SINGLE_CONTEST,
                delegated_voting_policy: DelegatedVotingPolicy::ENABLED,
                skip_board_posting: false,
            })
            .await
            .unwrap();

        assert_eq!(post_calls.lock().unwrap().as_slice(), &[7]);
        assert_eq!(*trustee_public_key_resolver.call_count.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn trustee_public_keys_service_resolves_keys_in_requested_order() {
        let first_sk = StrandSignatureSk::gen().unwrap();
        let second_sk = StrandSignatureSk::gen().unwrap();
        let expected_first_pk = StrandSignaturePk::from_sk(&first_sk).unwrap();
        let expected_second_pk = StrandSignaturePk::from_sk(&second_sk).unwrap();
        let repository = TrusteeRepositoryFake {
            trustees: vec![
                Trustee {
                    id: "trustee-2".to_string(),
                    public_key: Some(
                        StrandSignaturePk::from_sk(&second_sk)
                            .unwrap()
                            .to_der_b64_string()
                            .unwrap(),
                    ),
                    name: Some("Trustee 2".to_string()),
                    tenant_id: "tenant".to_string(),
                    created_at: None,
                    last_updated_at: None,
                    labels: None,
                    annotations: None,
                },
                Trustee {
                    id: "trustee-1".to_string(),
                    public_key: Some(
                        StrandSignaturePk::from_sk(&first_sk)
                            .unwrap()
                            .to_der_b64_string()
                            .unwrap(),
                    ),
                    name: Some("Trustee 1".to_string()),
                    tenant_id: "tenant".to_string(),
                    created_at: None,
                    last_updated_at: None,
                    labels: None,
                    annotations: None,
                },
            ],
        };
        let service = TrusteePublicKeysService::new(&repository);

        let resolved_keys = service
            .resolve_public_keys(
                "tenant",
                &["Trustee 1".to_string(), "Trustee 2".to_string()],
            )
            .await
            .unwrap();

        assert_eq!(resolved_keys, vec![expected_first_pk, expected_second_pk,]);
    }

    #[tokio::test]
    async fn trustee_public_keys_service_fails_on_missing_public_key() {
        let repository = TrusteeRepositoryFake {
            trustees: vec![Trustee {
                id: "trustee-1".to_string(),
                public_key: None,
                name: Some("Trustee 1".to_string()),
                tenant_id: "tenant".to_string(),
                created_at: None,
                last_updated_at: None,
                labels: None,
                annotations: None,
            }],
        };
        let service = TrusteePublicKeysService::new(&repository);

        let error = service
            .resolve_public_keys("tenant", &["Trustee 1".to_string()])
            .await
            .unwrap_err();

        assert!(error
            .to_string()
            .contains("Missing trustee public key for trustee Trustee 1"));
    }

    #[tokio::test]
    async fn trustee_public_keys_service_fails_on_invalid_public_key() {
        let repository = TrusteeRepositoryFake {
            trustees: vec![Trustee {
                id: "trustee-1".to_string(),
                public_key: Some("invalid-public-key".to_string()),
                name: Some("Trustee 1".to_string()),
                tenant_id: "tenant".to_string(),
                created_at: None,
                last_updated_at: None,
                labels: None,
                annotations: None,
            }],
        };
        let service = TrusteePublicKeysService::new(&repository);

        let error = service
            .resolve_public_keys("tenant", &["Trustee 1".to_string()])
            .await
            .unwrap_err();

        assert!(!error.to_string().is_empty());
    }

    #[tokio::test]
    async fn upsert_service_routes_missing_batches_and_missing_annotations() {
        let tally_session = TallySession {
            id: "tally-session".to_string(),
            tenant_id: "tenant".to_string(),
            election_event_id: "event".to_string(),
            created_at: None,
            last_updated_at: None,
            labels: None,
            annotations: None,
            election_ids: None,
            area_ids: None,
            is_execution_completed: false,
            keys_ceremony_id: "keys".to_string(),
            execution_status: None,
            threshold: 0,
            configuration: Some(
                serde_json::from_value(json!({
                    "contest_encryption_policy": "single-contest",
                    "delegated_voting_policy": "disabled"
                }))
                .unwrap(),
            ),
            tally_type: None,
            permission_label: None,
        };

        let trustee_public_key_resolver = TrusteePublicKeyResolverFake::default();
        let mut election_repository = MockElectionEventRepositoryDouble::new();
        election_repository
            .expect_get_election_aliases()
            .times(2)
            .returning(|_, _| {
                Ok(HashMap::from([(
                    "election-1".to_string(),
                    "alias-1".to_string(),
                )]))
            });
        let ballot_payload = serialized_hashable_ballot();
        let ballot_row = ballot_csv_row("user-1", &ballot_payload);
        let mut ballot_repository = MockBallotRepositoryDouble::new();
        ballot_repository
            .expect_export_area_ballots()
            .times(2)
            .returning(move |_, _, _, _, output_path| {
                std::fs::write(output_path, ballot_row.clone())?;
                Ok(())
            });
        let mut eligible_user_repository = MockEligibleUserRepositoryDouble::new();
        eligible_user_repository
            .expect_export_enabled_users()
            .times(2)
            .returning(|_, _, _, output_path, _| {
                std::fs::write(output_path, "user-1")?;
                Ok(())
            });
        let post_calls = Arc::new(Mutex::new(Vec::new()));
        let post_calls_clone = Arc::clone(&post_calls);
        let mut board_port = MockBallotBoardPortDouble::new();
        board_port
            .expect_prepare_board_context()
            .once()
            .returning(|_| {
                let signing_key = StrandSignatureSk::gen()?;
                let trustee_key_1 = StrandSignatureSk::gen()?;
                let trustee_key_2 = StrandSignatureSk::gen()?;
                Ok(InsertBallotsBoardContext {
                    protocol_manager: Arc::new(ProtocolManager::new(signing_key)),
                    messages: Arc::new(Vec::<Message>::new()),
                    configuration: Configuration::new(
                        0,
                        StrandSignaturePk::from_sk(&trustee_key_1)?,
                        vec![
                            StrandSignaturePk::from_sk(&trustee_key_1)?,
                            StrandSignaturePk::from_sk(&trustee_key_2)?,
                        ],
                        2,
                        std::marker::PhantomData,
                    ),
                    public_key_hash: PublicKeyHash([0; 64]),
                    selected_trustees: [NULL_TRUSTEE; 12],
                })
            });
        board_port
            .expect_post_ballots()
            .once()
            .returning(move |_, _, batch, _| {
                post_calls_clone.lock().unwrap().push(batch);
                Ok(())
            });
        let service = InsertBallotsService::new(
            &trustee_public_key_resolver,
            &election_repository,
            &ballot_repository,
            &eligible_user_repository,
            &board_port,
            2,
        );

        let board_messages = vec![ballots_message(1), ballots_message(3)];

        let updated = service
            .execute(UpsertBallotsMessagesRequest {
                tenant_id: "tenant".to_string(),
                election_event_id: "event".to_string(),
                board_name: "board".to_string(),
                trustee_names: vec!["Trustee 1".to_string()],
                messages: &board_messages,
                tally_session_contests: vec![
                    sample_contest(1, None),
                    sample_contest(2, None),
                    sample_contest(3, Some(json!({ "kept": true }))),
                ],
                tally_session,
            })
            .await
            .unwrap();

        assert_eq!(updated.len(), 2);
        assert_eq!(post_calls.lock().unwrap().as_slice(), &[2]);
        assert_eq!(*trustee_public_key_resolver.call_count.lock().unwrap(), 1);
    }
}
