// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::tally_validation::{validate_tally_elections, TallyValidationError};
use crate::adapters::system::RandomIds;
use crate::adapters::tally_ceremony::{
    BoardTrusteePrivateKeys, ElectoralLogTallyAudit, EnvSlug, PgElectionEvents, PgElectionsById,
    PgKeysCeremonies, PgTallyCreationReader, PgTallySessions,
};
use crate::domain::tally_ceremony::{
    check_key_restore_status, check_status_change, check_trustee_quorum, is_recount_eligible,
    reaches_key_threshold, recount_elections_status, restore_trustee_key, restored_trustee_count,
    tally_executer, tally_execution_status, waiting_trustee, EXECUTER_USERNAME_ANNOTATION,
    EXECUTER_USER_ID_ANNOTATION,
};
use crate::domain::tally_creation::{
    check_weighted_voting_ballot_styles, check_weighted_voting_policies,
    check_weighted_voting_tally_sheets, WeightedVotingStage,
};
use crate::ports::clock::IdGenerator;
use crate::ports::tally_ceremony::{
    DecryptionSet, ElectionEventReader, ElectionsById, EnvironmentSlug, KeysCeremonyReader,
    NewTallySession, TallyCeremonyAudit, TallyCreationReader, TallyEventSnapshot, TallySessions,
    TrusteePrivateKeys,
};
use crate::postgres::ballot_style::get_ballot_styles_by_elections;
use crate::postgres::tally_session::get_tally_session_by_id;
use crate::postgres::tally_session_contest::get_tally_session_contests;
use crate::postgres::tally_session_execution::get_last_tally_session_execution;
use crate::services::ceremonies::serialize_logs::{
    append_tally_recount_log, append_tally_trustee_log, generate_tally_initial_log,
};
use crate::services::election_event_board::get_election_event_board;
use crate::services::election_event_status::get_election_status;
use crate::services::protocol_manager::get_event_board;
use anyhow::{anyhow, Context, Result};
use b4::messages::newtypes::BatchNumber;
use deadpool_postgres::Transaction;
use sequent_core::ballot::{
    BallotStyle as SequentBallotStyle, ContestEncryptionPolicy, WeightedVotingPolicy,
};
use sequent_core::ballot_codec::multi_ballot::votable_contests;
use sequent_core::serialization::deserialize_with_path::*;
use sequent_core::services::area_tree::ContestsData;
use sequent_core::services::area_tree::TreeNode;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::ceremonies::*;
use sequent_core::types::hasura::core::KeysCeremony;
use sequent_core::types::hasura::core::{AreaContest, TallySessionConfiguration};
use sequent_core::types::hasura::core::{
    BallotStyle, Election, TallySession, TallySessionContest, TallySessionExecution,
};
use sequent_core::types::hasura::core::{Contest, ElectionEvent};
use sequent_core::types::keycloak::VOTE_WEIGHT_BATCHES;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::collections::HashSet;
use std::str::FromStr;
use tracing::{event, instrument, Level};

#[instrument(skip(hasura_transaction), err)]
pub async fn find_last_tally_session_execution_and_all_related_data(
    hasura_transaction: &Transaction<'_>,
    tenant_id: String,
    election_event_id: String,
    tally_session_id: String,
    election_ids: Vec<String>,
) -> Result<
    Option<(
        TallySessionExecution,
        TallySession,
        Vec<TallySessionContest>,
        Vec<BallotStyle>,
    )>,
> {
    // get all data for the execution: the last tally session execution,
    // the list of tally_session_contest, and the ballot styles

    let tally_session_execution = get_last_tally_session_execution(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        &tally_session_id,
    )
    .await?;

    let tally_session_execution = match tally_session_execution {
        Some(tally_session_execution) => tally_session_execution,
        None => {
            return Ok(None);
        }
    };

    let tally_session = get_tally_session_by_id(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        &tally_session_id,
    )
    .await?;

    let tally_session_contest = get_tally_session_contests(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        &tally_session_id,
    )
    .await?;

    let ballot_style = get_ballot_styles_by_elections(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        &election_ids,
    )
    .await?;

    Ok(Some((
        tally_session_execution,
        tally_session,
        tally_session_contest,
        ballot_style,
    )))
}

#[instrument(skip_all, err)]
pub fn get_tally_ceremony_status(input: Option<Value>) -> Result<TallyCeremonyStatus> {
    input
        .map(|value| {
            deserialize_value(value)
                .map_err(|err| anyhow!("Error parsing tally ceremony status: {:?}", err))
        })
        .ok_or(anyhow!("Missing tally ceremony status"))
        .flatten()
}

#[instrument(skip(keys_ceremonies), err)]
pub async fn find_keys_ceremony(
    keys_ceremonies: &impl KeysCeremonyReader,
    tenant_id: &str,
    election_event_id: &str,
    elections: &Vec<Election>,
) -> Result<KeysCeremony> {
    let keys_ceremonies_set: HashSet<String> = elections
        .clone()
        .into_iter()
        .filter_map(|election| election.keys_ceremony_id.clone())
        .collect();

    if 1 != keys_ceremonies_set.len() {
        if 0 == keys_ceremonies_set.len() {
            return Err(
                TallyValidationError::new("The selected elections have no keys ceremony").into(),
            );
        } else {
            return Err(
                TallyValidationError::new("Elections have different keys ceremonies").into(),
            );
        }
    }

    let Some(keys_ceremony_id) = elections[0].keys_ceremony_id.clone() else {
        return Err(TallyValidationError::new("Election has no keys ceremony").into());
    };

    let keys_ceremony = keys_ceremonies
        .get(tenant_id, election_event_id, &keys_ceremony_id)
        .await?;

    let status_str = keys_ceremony.execution_status.clone().unwrap_or_default();
    if KeysCeremonyExecutionStatus::from_str(&status_str).ok()
        != Some(KeysCeremonyExecutionStatus::SUCCESS)
    {
        return Err(TallyValidationError::new("Invalid keys ceremony").into());
    }

    Ok(keys_ceremony)
}

#[instrument]
fn generate_initial_tally_status(
    election_ids: &Vec<String>,
    keys_ceremony_status: &KeysCeremonyStatus,
) -> TallyCeremonyStatus {
    TallyCeremonyStatus {
        stop_date: None,
        logs: generate_tally_initial_log(election_ids),
        trustees: keys_ceremony_status
            .trustees
            .iter()
            .map(|trustee| TallyTrustee {
                name: trustee.name.clone(),
                status: TallyTrusteeStatus::WAITING,
            })
            .collect(),
        elections_status: election_ids
            .iter()
            .map(|election_id| TallyElection {
                election_id: election_id.clone(),
                status: TallyElectionStatus::WAITING,
                progress: 0.0,
            })
            .collect(),
    }
}

#[instrument(err, skip(sessions))]
pub async fn insert_tally_session_contests(
    sessions: &impl TallySessions,
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
    published_ballot_styles: &[SequentBallotStyle],
    configuration: &TallySessionConfiguration,
) -> Result<()> {
    // Each contest area owns `VOTE_WEIGHT_BATCHES` consecutive batches starting
    // at its `session_id`. Only `VOTERS_WEIGHTED_VOTING` fills more than the
    // first, but the stride is unconditional so that a session created under
    // one policy can never allocate a batch inside a run created under another.
    let mut batch: BatchNumber = sessions.next_batch(tenant_id, election_event_id).await?;

    for decryption_set in required_decryption_sets(
        published_ballot_styles,
        configuration.get_contest_encryption_policy(),
    ) {
        sessions
            .insert_contest(
                tenant_id,
                election_event_id,
                tally_session_id,
                &decryption_set,
                batch,
            )
            .await?;
        batch += VOTE_WEIGHT_BATCHES as BatchNumber;
    }
    Ok(())
}

/// Returns the encrypted payloads that must go through the decryption
/// ceremony. Acclaimed contests are present in the published ballot style but
/// deliberately absent from its ciphertext.
fn required_decryption_sets(
    ballot_styles: &[SequentBallotStyle],
    policy: ContestEncryptionPolicy,
) -> HashSet<DecryptionSet> {
    match policy {
        ContestEncryptionPolicy::SINGLE_CONTEST => ballot_styles
            .iter()
            .flat_map(|ballot_style| {
                votable_contests(&ballot_style.contests).map(|contest| {
                    (
                        ballot_style.election_id.clone(),
                        ballot_style.area_id.clone(),
                        Some(contest.id.clone()),
                    )
                })
            })
            .collect(),
        ContestEncryptionPolicy::MULTIPLE_CONTESTS => ballot_styles
            .iter()
            .filter(|ballot_style| votable_contests(&ballot_style.contests).next().is_some())
            .map(|ballot_style| {
                (
                    ballot_style.election_id.clone(),
                    ballot_style.area_id.clone(),
                    None,
                )
            })
            .collect(),
    }
}

fn get_area_contests_for_election_ids(
    contests_map: &HashMap<String, Contest>,
    area_contests_tree: &TreeNode<ContestsData>,
    election_ids: &Vec<String>,
) -> HashSet<AreaContest> {
    let contest_ids: HashSet<String> = contests_map
        .values()
        .filter(|contest| election_ids.contains(&contest.election_id))
        .map(|contest| contest.id.clone())
        .collect();
    area_contests_tree.get_contest_matches(&contest_ids)
}

/// A request to create a tally session for some elections of an event.
pub struct TallyCreation<'a> {
    pub tenant_id: String,
    pub user_id: &'a str,
    pub election_event_id: String,
    pub election_ids: Vec<String>,
    pub configuration: Option<TallySessionConfiguration>,
    pub tally_type: String,
    pub permission_labels: &'a Vec<String>,
    pub username: String,
}

#[instrument(err, skip(transaction))]
pub async fn create_tally_ceremony(
    transaction: &Transaction<'_>,
    tenant_id: String,
    user_id: &str,
    election_event_id: String,
    election_ids: Vec<String>,
    configuration: Option<TallySessionConfiguration>,
    tally_type: String,
    permission_labels: &Vec<String>,
    username: String,
) -> Result<String> {
    create_tally_ceremony_with(
        &PgTallyCreationReader::new(transaction),
        &PgKeysCeremonies::new(transaction),
        &PgTallySessions::new(transaction),
        &PgElectionEvents::new(transaction),
        &ElectoralLogTallyAudit::new(transaction),
        &RandomIds,
        TallyCreation {
            tenant_id,
            user_id,
            election_event_id,
            election_ids,
            configuration,
            tally_type,
            permission_labels,
            username,
        },
    )
    .await
}

/// Returns the id of the new tally session.
pub async fn create_tally_ceremony_with(
    reader: &impl TallyCreationReader,
    keys_ceremonies: &impl KeysCeremonyReader,
    sessions: &impl TallySessions,
    election_events: &impl ElectionEventReader,
    audit: &impl TallyCeremonyAudit,
    ids: &impl IdGenerator,
    request: TallyCreation<'_>,
) -> Result<String> {
    let TallyCreation {
        tenant_id,
        user_id,
        election_event_id,
        election_ids,
        configuration,
        tally_type,
        permission_labels,
        username,
    } = request;
    let TallyEventSnapshot {
        election_event,
        elections: all_elections,
        contests: all_contests,
        areas,
        area_contests: all_area_contests,
    } = reader
        .event_snapshot(&tenant_id, &election_event_id)
        .await?;
    let parsed_tally_type = TallyType::try_from(tally_type.as_str())
        .map_err(|_| TallyValidationError::new("Invalid tally type"))?;
    validate_tally_elections(&all_elections, &election_ids, parsed_tally_type)?;
    let contest_encryption_policy = election_event.get_contest_encryption_policy();
    let decoded_ballots_inclusion_policy = election_event.get_decoded_ballots_inclusion_policy();
    let delegated_voting_policy = election_event.get_delegated_voting_policy();
    let weighted_voting_policy = election_event.get_weighted_voting_policy();
    let published_ballot_style_rows = reader
        .published_ballot_styles(&tenant_id, &election_event_id, &election_ids)
        .await?;
    let published_ballot_styles = published_ballot_style_rows
        .iter()
        .map(|published| {
            let ballot_eml = published.ballot_eml.as_deref().ok_or_else(|| {
                anyhow!("Published ballot style {} has no ballot EML", published.id)
            })?;
            deserialize_str(ballot_eml).map_err(|error| {
                anyhow!(
                    "Could not read published ballot style {}: {error:?}",
                    published.id
                )
            })
        })
        .collect::<Result<Vec<SequentBallotStyle>>>()?;
    if weighted_voting_policy == WeightedVotingPolicy::VOTERS_WEIGHTED_VOTING {
        let stage = WeightedVotingStage::Creation;
        check_weighted_voting_policies(
            stage,
            &delegated_voting_policy,
            &decoded_ballots_inclusion_policy,
        )?;
        let approved_tally_sheets = reader
            .approved_tally_sheets(&tenant_id, &election_event_id)
            .await?;
        check_weighted_voting_tally_sheets(stage, &approved_tally_sheets, &election_ids)?;
        check_weighted_voting_ballot_styles(stage, &published_ballot_styles)?;
    }

    let mut final_configuration = configuration.clone().unwrap_or_default();
    final_configuration.contest_encryption_policy = Some(contest_encryption_policy);
    final_configuration.decoded_ballots_inclusion_policy = Some(decoded_ballots_inclusion_policy);
    final_configuration.delegated_voting_policy = Some(delegated_voting_policy);
    final_configuration.weighted_voting_policy = Some(weighted_voting_policy);
    let contests: Vec<Contest> = all_contests
        .into_iter()
        .filter(|contest| election_ids.contains(&contest.election_id))
        .collect();

    let elections: Vec<Election> = all_elections
        .into_iter()
        .filter(|election| {
            if election_ids.contains(&election.id) {
                let status = get_election_status(election.status.clone()).unwrap_or_default();
                if let Some(is_published) = status.is_published {
                    is_published // Include only if `is_published` is true
                } else {
                    false
                }
            } else {
                false
            }
        })
        .collect();

    let mut selected_elections_permission_labels = HashSet::new();

    let permission_label_filtered_elections: Vec<_> = elections
        .clone()
        .into_iter()
        .filter(|election| {
            if permission_labels.is_empty() {
                return true;
            }

            if let Some(election_perm_label) = &election.permission_label {
                selected_elections_permission_labels.insert(election_perm_label.clone()); // Collect unique labels
                permission_labels.contains(election_perm_label)
            } else {
                true
            }
        })
        .collect();

    if permission_label_filtered_elections.len() != election_ids.len() {
        return Err(TallyValidationError::new(
            "Some elections don't have the required permission label or are not published",
        )
        .into());
    }

    // Convert HashSet to Vec if needed
    let tally_permission_labels: Vec<String> =
        selected_elections_permission_labels.into_iter().collect();

    event!(Level::INFO, "contests {:?}", contests);
    let contest_ids: Vec<String> = contests.clone().into_iter().map(|c| c.id.clone()).collect();
    let area_contests: Vec<AreaContest> = all_area_contests
        .into_iter()
        .filter(|area_contest| contest_ids.contains(&area_contest.contest_id))
        .collect();
    event!(Level::INFO, "area_contests {:?}", area_contests);

    let contests_map: HashMap<String, Contest> = contests
        .into_iter()
        .map(|contest| (contest.id.clone(), contest.clone()))
        .collect();

    let basic_areas = areas.iter().map(|area| area.into()).collect();
    let areas_tree = TreeNode::<()>::from_areas(basic_areas)?;

    event!(Level::INFO, "areas_tree {:?}", area_contests);
    let area_contests_tree = areas_tree.get_contests_data_tree(&area_contests);

    event!(Level::INFO, "area_contests_tree {:?}", area_contests_tree);
    let relevant_area_contests =
        get_area_contests_for_election_ids(&contests_map, &area_contests_tree, &election_ids);
    event!(
        Level::INFO,
        "relevant_area_contests {:?}",
        relevant_area_contests
    );
    let area_ids: Vec<String> = relevant_area_contests
        .iter()
        .map(|area_contest| area_contest.area_id.clone())
        .collect::<HashSet<String>>()
        .iter()
        .map(|val| val.clone())
        .collect();

    let keys_ceremony =
        find_keys_ceremony(keys_ceremonies, &tenant_id, &election_event_id, &elections).await?;
    let keys_ceremony_status = keys_ceremony.status()?;
    let keys_ceremony_id = keys_ceremony.id.clone();
    let initial_status = generate_initial_tally_status(&election_ids, &keys_ceremony_status);
    let tally_session_id: String = ids.new_id().to_string();

    let annotations: Value = json!({
        EXECUTER_USERNAME_ANNOTATION: username,
        EXECUTER_USER_ID_ANNOTATION: user_id,
    });

    let keys_ceremony_policy = keys_ceremony.policy();

    let tally_execution_status = match keys_ceremony_policy {
        CeremoniesPolicy::AUTOMATED_CEREMONIES => TallyExecutionStatus::IN_PROGRESS,
        _ => TallyExecutionStatus::STARTED,
    };

    sessions
        .insert(
            &tenant_id,
            &election_event_id,
            NewTallySession {
                id: tally_session_id.clone(),
                election_ids: election_ids.clone(),
                area_ids: area_ids.clone(),
                keys_ceremony_id: keys_ceremony_id.clone(),
                execution_status: tally_execution_status,
                threshold: keys_ceremony.threshold as i32,
                configuration: Some(final_configuration.clone()),
                tally_type: tally_type.clone(),
                annotations,
                permission_labels: tally_permission_labels,
            },
        )
        .await?;

    sessions
        .append_execution(
            &tenant_id,
            &election_event_id,
            &tally_session_id,
            -1,
            initial_status,
            TallyRunReason::NORMAL,
        )
        .await?;

    insert_tally_session_contests(
        sessions,
        &tenant_id,
        &election_event_id,
        &tally_session_id,
        &published_ballot_styles,
        &final_configuration,
    )
    .await?;

    // get the election event
    let election_event = election_events.get(&tenant_id, &election_event_id).await?;

    // Save this in the electoral log
    let board_name = get_election_event_board(election_event.bulletin_board_reference.clone())
        .with_context(|| "missing bulletin board")?;

    audit
        .key_insertion_started(
            &board_name,
            &tenant_id,
            &election_event_id,
            election_ids,
            user_id,
            &username,
        )
        .await?;

    Ok(tally_session_id.clone())
}

/// A request to move a tally session to another execution status.
pub struct TallyStatusChange {
    pub tenant_id: String,
    pub election_event_id: String,
    pub tally_session: TallySession,
    pub new_execution_status: TallyExecutionStatus,
    pub user_id: String,
    pub username: String,
}

#[instrument(err, skip(hasura_transaction))]
pub async fn update_tally_ceremony(
    hasura_transaction: &Transaction<'_>,
    tenant_id: String,
    election_event_id: String,
    tally_session: TallySession,
    new_execution_status: TallyExecutionStatus,
    user_id: String,
    username: String,
) -> Result<()> {
    update_tally_ceremony_with(
        &PgTallySessions::new(hasura_transaction),
        &PgElectionsById::new(hasura_transaction),
        &EnvSlug,
        &ElectoralLogTallyAudit::new(hasura_transaction),
        TallyStatusChange {
            tenant_id,
            election_event_id,
            tally_session,
            new_execution_status,
            user_id,
            username,
        },
    )
    .await
}

pub async fn update_tally_ceremony_with(
    sessions: &impl TallySessions,
    elections: &impl ElectionsById,
    environment: &impl EnvironmentSlug,
    audit: &impl TallyCeremonyAudit,
    change: TallyStatusChange,
) -> Result<()> {
    let TallyStatusChange {
        tenant_id,
        election_event_id,
        tally_session,
        new_execution_status,
        user_id,
        username,
    } = change;
    let current_status = tally_execution_status(tally_session.execution_status.as_deref());
    check_status_change(&current_status, &new_execution_status)?;

    if new_execution_status == TallyExecutionStatus::IN_PROGRESS {
        let election_ids = tally_session.election_ids.clone().unwrap_or_default();
        let elections = elections
            .get(&tenant_id, &election_event_id, &election_ids)
            .await?;
        let tally_type = tally_session
            .tally_type
            .as_deref()
            .map(TallyType::try_from)
            .transpose()
            .map_err(|_| TallyValidationError::new("Invalid tally type"))?
            .unwrap_or_default();
        validate_tally_elections(&elections, &election_ids, tally_type)?;
    }

    let Some((tally_session_execution, _)) = sessions
        .last_execution_and_session(
            &tenant_id,
            &election_event_id,
            &tally_session.id,
            tally_session.election_ids.clone().unwrap_or_default(),
        )
        .await?
    else {
        event!(Level::INFO, "Can't find last execution status, skipping");
        return Ok(());
    };

    let status = get_tally_ceremony_status(tally_session_execution.status)?;
    check_trustee_quorum(
        tally_session.threshold,
        restored_trustee_count(&status),
        &new_execution_status,
    )?;

    println!(
        "Updating tally session execution status: {:?}",
        &new_execution_status
    );

    println!("new_execution_status:: {:?}", &new_execution_status);

    sessions
        .set_status(
            &tenant_id,
            &election_event_id,
            &tally_session.id,
            new_execution_status.clone(),
            new_execution_status == TallyExecutionStatus::SUCCESS,
        )
        .await?;

    if new_execution_status == TallyExecutionStatus::IN_PROGRESS {
        let slug = environment.env_slug()?;

        // Save this in the electoral log
        let board_name = get_event_board(&tenant_id, &election_event_id, &slug);
        audit
            .tally_opened(
                &board_name,
                &tenant_id,
                &election_event_id,
                tally_session.election_ids.clone(),
                &user_id,
                &username,
            )
            .await?;
    }

    Ok(())
}

/// A trustee's request to restore their private key for a tally session.
pub struct TrusteeKeyRestore<'a> {
    pub claims: &'a JwtClaims,
    pub tenant_id: &'a str,
    pub election_event_id: &'a str,
    pub tally_session_id: &'a str,
    pub private_key_base64: &'a str,
}

#[instrument(err, skip(transaction))]
pub async fn set_private_key(
    transaction: &Transaction<'_>,
    claims: &JwtClaims,
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
    private_key_base64: &str,
) -> Result<bool> {
    set_private_key_with(
        &PgTallySessions::new(transaction),
        &PgKeysCeremonies::new(transaction),
        &BoardTrusteePrivateKeys::new(transaction),
        &PgElectionEvents::new(transaction),
        &ElectoralLogTallyAudit::new(transaction),
        TrusteeKeyRestore {
            claims,
            tenant_id,
            election_event_id,
            tally_session_id,
            private_key_base64,
        },
    )
    .await
}

/// Returns `false`, writing nothing, if the key is not the one the trustee
/// stored on the board in the keys ceremony.
pub async fn set_private_key_with(
    sessions: &impl TallySessions,
    keys_ceremonies: &impl KeysCeremonyReader,
    private_keys: &impl TrusteePrivateKeys,
    election_events: &impl ElectionEventReader,
    audit: &impl TallyCeremonyAudit,
    request: TrusteeKeyRestore<'_>,
) -> Result<bool> {
    let TrusteeKeyRestore {
        claims,
        tenant_id,
        election_event_id,
        tally_session_id,
        private_key_base64,
    } = request;
    let tally_session = sessions
        .get(tenant_id, election_event_id, tally_session_id)
        .await?;

    // The trustee name is simply the username of the user
    let trustee_name = claims
        .trustee
        .clone()
        .ok_or(anyhow!("trustee name not found"))?;

    let Some((tally_session_execution, tally_session)) = sessions
        .last_execution_and_session(
            tenant_id,
            election_event_id,
            tally_session_id,
            tally_session.election_ids.clone().unwrap_or_default(),
        )
        .await?
    else {
        return Err(anyhow!(
            "Can't find tally session or tally session execution"
        ));
    };

    check_key_restore_status(&tally_execution_status(
        tally_session.execution_status.as_deref(),
    ))?;

    // get the keys ceremonies for this election event
    let keys_ceremony = keys_ceremonies
        .get(
            tenant_id,
            election_event_id,
            &tally_session.keys_ceremony_id,
        )
        .await?;

    let tally_ceremony_status = get_tally_ceremony_status(tally_session_execution.status.clone())?;
    let found_trustee = waiting_trustee(&tally_ceremony_status, &trustee_name)?.clone();

    // get the encrypted private key
    let encrypted_private_key = private_keys
        .encrypted_private_key(tenant_id, election_event_id, &trustee_name, &keys_ceremony)
        .await?;

    if encrypted_private_key != private_key_base64 {
        return Ok(false);
    }
    let mut new_status = restore_trustee_key(tally_ceremony_status.clone(), &found_trustee.name);
    new_status.logs = append_tally_trustee_log(&tally_ceremony_status.logs, &trustee_name);
    sessions
        .append_execution(
            tenant_id,
            election_event_id,
            tally_session_id,
            tally_session_execution.current_message_id,
            new_status.clone(),
            TallyRunReason::NORMAL,
        )
        .await?;

    // enough trustees connected, so change tally execution status to connected
    if reaches_key_threshold(&new_status, keys_ceremony.threshold) {
        sessions
            .set_status(
                tenant_id,
                election_event_id,
                tally_session_id,
                TallyExecutionStatus::CONNECTED,
                false,
            )
            .await?;
    }
    println!("after update status");
    // get the election event
    let election_event = election_events.get(tenant_id, election_event_id).await?;

    // Save this in the electoral log
    let board_name = get_election_event_board(election_event.bulletin_board_reference.clone())
        .with_context(|| "missing bulletin board")?;

    audit
        .key_restored(
            &board_name,
            tenant_id,
            election_event_id,
            tally_session.election_ids.clone(),
            &found_trustee.name,
            claims,
        )
        .await?;

    Ok(true)
}

#[instrument(err, skip(hasura_transaction))]
pub async fn set_tally_session_completed(
    hasura_transaction: &Transaction<'_>,
    tenant_id: String,
    election_event_id: String,
    tally_session_id: String,
) -> Result<()> {
    set_tally_session_completed_with(
        &PgTallySessions::new(hasura_transaction),
        &PgElectionEvents::new(hasura_transaction),
        &ElectoralLogTallyAudit::new(hasura_transaction),
        &tenant_id,
        &election_event_id,
        &tally_session_id,
    )
    .await
}

/// An error marking the session completed is swallowed: nothing is posted to
/// the electoral log and `Ok` is returned.
pub async fn set_tally_session_completed_with(
    sessions: &impl TallySessions,
    election_events: &impl ElectionEventReader,
    audit: &impl TallyCeremonyAudit,
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
) -> Result<()> {
    let is_updated = sessions
        .mark_completed(
            tenant_id,
            election_event_id,
            tally_session_id,
            TallyExecutionStatus::SUCCESS,
        )
        .await
        .is_ok();

    if is_updated {
        let election_event = election_events.get(tenant_id, election_event_id).await?;

        let tally_session = sessions
            .get(tenant_id, election_event_id, tally_session_id)
            .await?;

        let executer = tally_executer(tally_session.annotations.as_ref());
        let board_name = get_election_event_board(election_event.bulletin_board_reference.clone())
            .with_context(|| "missing bulletin board")?;

        audit
            .tally_closed(
                &board_name,
                tenant_id,
                election_event_id,
                tally_session.election_ids.clone(),
                executer,
            )
            .await?;
    }

    Ok(())
}

/// Marks a completed tally session as `IN_PROGRESS` for a recount (manual or
/// automatic): inserts a fresh `tally_session_execution` row with
/// `elections_status` reset to `WAITING`/0% and a "recount launched" log
/// entry appended, then flips the session status. Without this, the tally
/// session details keep showing the previous completed run (status, progress,
/// results) until the celery task produces its own first update, which can
/// be a long time (or never, if it bails out early).
///
/// Returns `false` if the session is no longer completed and eligible, or has
/// no `tally_session_execution` row at all.
/// `insert_tally_session` and the first `insert_tally_session_execution`
/// always land in the same transaction (see `create_tally_ceremony` below),
/// so a session created here can't reach `SUCCESS`/completed without one;
/// a session whose status disagrees with its execution history never
/// actually ran, and so has nothing to recount.
///
#[instrument(skip(hasura_transaction), err)]
pub async fn begin_tally_session_recount(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
    election_ids: &[String],
) -> Result<bool> {
    begin_tally_session_recount_with(
        &PgTallySessions::new(hasura_transaction),
        tenant_id,
        election_event_id,
        tally_session_id,
        election_ids,
    )
    .await
}

pub async fn begin_tally_session_recount_with(
    sessions: &impl TallySessions,
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
    election_ids: &[String],
) -> Result<bool> {
    // Serialize the state transition with post-tally finalization. The task
    // itself has a different, long-lived lock; this short row lock protects
    // only writers that can change which execution is considered latest.
    sessions
        .lock_for_update(tenant_id, election_event_id, tally_session_id)
        .await?;

    // Callers inspect the session before opening this transition, but another
    // recount can complete that read and acquire the lock first. Re-check only
    // after the lock is held so a stale caller cannot append a second marker.
    let tally_session = sessions
        .get(tenant_id, election_event_id, tally_session_id)
        .await?;
    if !is_recount_eligible(&tally_session) {
        return Ok(false);
    }

    let Some(last_execution) = sessions
        .last_execution(tenant_id, election_event_id, tally_session_id)
        .await?
    else {
        return Ok(false);
    };

    let mut recount_status = get_tally_ceremony_status(last_execution.status.clone())?;
    recount_status.logs = append_tally_recount_log(&recount_status.logs, &election_ids.to_vec());
    recount_status.elections_status = recount_elections_status(election_ids);

    sessions
        .append_execution(
            tenant_id,
            election_event_id,
            tally_session_id,
            last_execution.current_message_id,
            recount_status,
            // The durable record that a recount was asked for. The celery message
            // this function's callers send afterwards is only a nudge: if it is
            // lost -- expired while no worker was consuming, or dropped because a
            // concurrent run held the lock -- the next process_board tick reads
            // this row and performs the recount anyway.
            TallyRunReason::RECOUNT,
        )
        .await?;

    sessions
        .set_status(
            tenant_id,
            election_event_id,
            tally_session_id,
            TallyExecutionStatus::IN_PROGRESS,
            false,
        )
        .await?;

    Ok(true)
}

#[cfg(test)]
#[path = "tally_ceremony_creation_tests.rs"]
mod creation_tests;

#[cfg(test)]
#[path = "tally_ceremony_state_tests.rs"]
mod state_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use sequent_core::ballot::Contest as SequentContest;

    fn contest(id: &str, is_acclaimed: bool) -> SequentContest {
        SequentContest {
            id: id.to_string(),
            election_id: "election".to_string(),
            is_acclaimed: Some(is_acclaimed),
            ..Default::default()
        }
    }

    fn ballot_style(area_id: &str, contests: Vec<SequentContest>) -> SequentBallotStyle {
        SequentBallotStyle {
            id: format!("style-{area_id}"),
            tenant_id: "tenant".to_string(),
            election_event_id: "event".to_string(),
            election_id: "election".to_string(),
            num_allowed_revotes: None,
            description: None,
            public_key: None,
            area_id: area_id.to_string(),
            area_presentation: None,
            contests,
            election_event_presentation: None,
            election_presentation: None,
            election_dates: None,
            election_event_annotations: None,
            election_annotations: None,
            area_annotations: None,
            multi_contest_encoding_mode: None,
        }
    }

    #[test]
    fn single_contest_decryption_sets_exclude_acclaimed_contests() {
        let styles = vec![
            ballot_style(
                "mixed-area",
                vec![contest("acclaimed", true), contest("votable", false)],
            ),
            ballot_style("acclaimed-area", vec![contest("also-acclaimed", true)]),
        ];

        assert_eq!(
            HashSet::from([(
                "election".to_string(),
                "mixed-area".to_string(),
                Some("votable".to_string()),
            )]),
            required_decryption_sets(&styles, ContestEncryptionPolicy::SINGLE_CONTEST)
        );
    }

    #[test]
    fn multi_contest_decryption_sets_exclude_fully_acclaimed_areas() {
        let styles = vec![
            ballot_style(
                "mixed-area",
                vec![contest("acclaimed", true), contest("votable", false)],
            ),
            ballot_style("acclaimed-area", vec![contest("also-acclaimed", true)]),
        ];

        assert_eq!(
            HashSet::from([("election".to_string(), "mixed-area".to_string(), None,)]),
            required_decryption_sets(&styles, ContestEncryptionPolicy::MULTIPLE_CONTESTS)
        );
    }
}
