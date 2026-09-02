// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::ballot_publication::{
    get_ballot_publication_by_id, get_previous_publication, get_previous_publication_election,
    insert_ballot_publication, soft_delete_other_ballot_publications, update_ballot_publication,
};
use crate::postgres::ballot_style::get_publication_ballot_styles;
use crate::postgres::election::{get_election_by_id, get_elections_ids, update_election_status};
use crate::postgres::election_event::{get_election_event_by_id, update_election_event_status};
use crate::services::celery_app::get_celery_app;
use crate::services::election_event_board::get_election_event_board;
use crate::services::election_event_status::{get_election_event_status, get_election_status};
use crate::services::electoral_log::*;
use crate::services::tasks_execution::{
    post as post_task_execution, update_fail as update_task_execution_fail,
};
use crate::tasks::update_election_event_ballot_styles::update_election_event_ballot_styles;
use crate::types::tasks::ETasksExecution;
use anyhow::{anyhow, Context, Result};
use chrono::{Local, Utc};
use deadpool_postgres::Transaction;
use sequent_core::ballot::{ElectionEventStatus, ElectionStatus};
use sequent_core::serialization::deserialize_with_path::*;
use sequent_core::services::connection;
use sequent_core::services::date::ISO8601;
use sequent_core::types::hasura::core::{BallotPublication, TasksExecution};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use thiserror::Error;
use tracing::{event, instrument, Level};

use super::ballot_style;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ContestAcclamationState {
    name: Option<String>,
    is_acclaimed: bool,
}

#[derive(Debug, Error)]
#[error("{message}")]
pub struct BallotPublicationValidationError {
    pub reasons: Vec<String>,
    message: String,
}

impl BallotPublicationValidationError {
    fn new(reasons: Vec<String>) -> Self {
        let message = format!(
            "Ballot publication validation failed:\n- {}",
            reasons.join("\n- ")
        );
        Self { reasons, message }
    }
}

fn collect_acclamation_states(
    publication: &Value,
) -> Result<BTreeMap<String, ContestAcclamationState>> {
    let ballot_styles = publication
        .as_array()
        .context("Ballot publication must be an array of ballot styles")?;
    let mut states: BTreeMap<String, ContestAcclamationState> = BTreeMap::new();

    for ballot_style in ballot_styles {
        let contests = ballot_style
            .get("contests")
            .and_then(Value::as_array)
            .context("Ballot style must contain a contests array")?;

        for contest in contests {
            let contest_id = contest
                .get("id")
                .and_then(Value::as_str)
                .context("Ballot contest must contain a string id")?;
            let is_acclaimed = match contest.get("is_acclaimed") {
                None | Some(Value::Null) => false,
                Some(Value::Bool(value)) => *value,
                Some(_) => {
                    return Err(anyhow!(
                        "Ballot contest {contest_id} has a non-boolean is_acclaimed value"
                    ))
                }
            };
            let state = ContestAcclamationState {
                name: contest
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                is_acclaimed,
            };

            if let Some(existing) = states.get(contest_id) {
                if existing.is_acclaimed != state.is_acclaimed {
                    return Err(anyhow!(
                        "Contest {contest_id} has inconsistent is_acclaimed values in one ballot publication"
                    ));
                }
            } else {
                states.insert(contest_id.to_owned(), state);
            }
        }
    }

    Ok(states)
}

fn merge_acclamation_states(
    target: &mut BTreeMap<String, ContestAcclamationState>,
    source: BTreeMap<String, ContestAcclamationState>,
) -> Result<()> {
    for (contest_id, state) in source {
        if let Some(existing) = target.get(&contest_id) {
            if existing.is_acclaimed != state.is_acclaimed {
                return Err(anyhow!(
                    "Contest {contest_id} has inconsistent is_acclaimed values across published ballot styles"
                ));
            }
        } else {
            target.insert(contest_id, state);
        }
    }
    Ok(())
}

fn validate_acclamation_states(
    previous: &BTreeMap<String, ContestAcclamationState>,
    current: &BTreeMap<String, ContestAcclamationState>,
) -> Result<()> {
    let reasons = current
        .iter()
        .filter_map(|(contest_id, current_state)| {
            let previous_state = previous.get(contest_id)?;
            (previous_state.is_acclaimed != current_state.is_acclaimed).then(|| {
                let contest_name = current_state.name.as_deref().unwrap_or(contest_id);
                format!(
                    "Contest \"{contest_name}\" ({contest_id}) changed is_acclaimed from {} to {} after voting started for its election. Restore the published value before publishing again.",
                    previous_state.is_acclaimed, current_state.is_acclaimed
                )
            })
        })
        .collect::<Vec<_>>();

    if reasons.is_empty() {
        Ok(())
    } else {
        Err(BallotPublicationValidationError::new(reasons).into())
    }
}

fn election_has_started(status: &ElectionStatus) -> bool {
    [
        status.voting_period_dates.first_started_at.as_ref(),
        status.kiosk_voting_period_dates.first_started_at.as_ref(),
        status.early_voting_period_dates.first_started_at.as_ref(),
        status
            .telephone_voting_period_dates
            .first_started_at
            .as_ref(),
    ]
    .into_iter()
    .any(|started_at| started_at.is_some())
}

async fn validate_published_acclamation_status(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    ballot_publication: &BallotPublication,
) -> Result<()> {
    let current = collect_acclamation_states(
        &get_publication_json(
            hasura_transaction,
            tenant_id.to_owned(),
            election_event_id.to_owned(),
            ballot_publication.id.clone(),
            None,
            None,
        )
        .await?,
    )?;
    let mut previous = BTreeMap::new();
    let election_ids = ballot_publication
        .election_ids
        .as_ref()
        .context("Ballot publication is missing its election ids")?;
    if election_ids.is_empty() {
        return Err(anyhow!("Ballot publication has no election ids"));
    }

    for election_id in election_ids {
        let election = get_election_by_id(
            hasura_transaction,
            tenant_id,
            election_event_id,
            election_id,
        )
        .await?
        .with_context(|| format!("Can't find election {election_id}"))?;
        let election_status = get_election_status(election.status).unwrap_or_default();
        if !election_has_started(&election_status) {
            continue;
        }

        // Use the latest publication at final-publish time. A draft may have
        // been generated before another draft was published.
        let Some(previous_publication) = get_previous_publication_election(
            hasura_transaction,
            tenant_id,
            election_event_id,
            Some(Utc::now().with_timezone(&Local)),
            election_id,
        )
        .await?
        else {
            continue;
        };

        let publication = get_publication_json(
            hasura_transaction,
            tenant_id.to_owned(),
            election_event_id.to_owned(),
            previous_publication.id,
            Some(election_id.clone()),
            None,
        )
        .await?;
        merge_acclamation_states(&mut previous, collect_acclamation_states(&publication)?)?;
    }

    validate_acclamation_states(&previous, &current)
}

#[instrument(skip(hasura_transaction), err)]
async fn get_election_ids_for_publication(
    hasura_transaction: &Transaction<'_>,
    tenant_id: String,
    election_event_id: String,
    election_id_opt: Option<String>,
) -> Result<Vec<String>> {
    if let Some(election_id) = election_id_opt {
        return Ok(vec![election_id]);
    }
    let elections_ids =
        get_elections_ids(hasura_transaction, &tenant_id, &election_event_id).await?;

    Ok(elections_ids)
}

#[instrument(err)]
pub async fn add_ballot_publication(
    hasura_transaction: &Transaction<'_>,
    tenant_id: String,
    election_event_id: String,
    election_id: Option<String>,
    user_id: String,
    executer_name: &str,
) -> Result<(String, TasksExecution)> {
    let celery_app = get_celery_app().await;

    let election_ids = get_election_ids_for_publication(
        hasura_transaction,
        tenant_id.clone(),
        election_event_id.clone(),
        election_id.clone(),
    )
    .await?;

    let ballot_publication = insert_ballot_publication(
        hasura_transaction,
        &tenant_id.clone(),
        &election_event_id.clone(),
        election_ids.clone(),
        user_id.clone(),
        election_id.clone(),
    )
    .await?
    .with_context(|| "can't find inserted ballot publication")?;

    let task_execution = post_task_execution(
        &tenant_id,
        Some(&election_event_id),
        ETasksExecution::GENERATE_BALLOT_PUBLICATION,
        executer_name,
    )
    .await
    .context("Failed to insert task execution record")?;

    let task = match celery_app
        .send_task(update_election_event_ballot_styles::new(
            tenant_id.clone(),
            election_event_id.clone(),
            ballot_publication.id.clone(),
            task_execution.clone(),
        ))
        .await
    {
        Ok(task) => task,
        Err(err) => {
            let message = format!("Failed to enqueue ballot style generation: {err}");
            update_task_execution_fail(&task_execution, &message)
                .await
                .ok();
            return Err(anyhow!(message));
        }
    };
    event!(
        Level::INFO,
        "Sent CREATE_ELECTION_EVENT_BALLOT_STYLES task {}",
        task.task_id
    );

    Ok((ballot_publication.id.clone(), task_execution))
}

#[instrument(err)]
pub async fn update_publish_ballot(
    hasura_transaction: &Transaction<'_>,
    user_id: String,
    username: String,
    tenant_id: String,
    election_event_id: String,
    ballot_publication_id: String,
) -> Result<()> {
    let ballot_publication = get_ballot_publication_by_id(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &ballot_publication_id,
    )
    .await?
    .with_context(|| "Can't find ballot publication")?;

    if ballot_publication.is_generated.unwrap_or(false) == false {
        return Err(anyhow!(
            "Ballot publication not generated yet, can't publish."
        ));
    }

    if ballot_publication.published_at.is_some() {
        return Ok(());
    }

    validate_published_acclamation_status(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        &ballot_publication,
    )
    .await?;

    let _result = soft_delete_other_ballot_publications(
        &hasura_transaction,
        &ballot_publication_id,
        &election_event_id,
        &tenant_id,
        ballot_publication.election_id.clone(),
    )
    .await?;

    update_ballot_publication(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        &ballot_publication_id,
        true,
        Some(ISO8601::now()),
    )
    .await?;

    let election_event = get_election_event_by_id(
        hasura_transaction,
        &tenant_id.clone(),
        &election_event_id.clone(),
    )
    .await?;

    let mut new_status: ElectionEventStatus =
        get_election_event_status(election_event.status.clone()).unwrap_or(Default::default());
    new_status.is_published = Some(true);
    let new_status_js = serde_json::to_value(new_status)?;

    update_election_event_status(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        new_status_js,
    )
    .await?;

    // Update elections status
    let election_ids = ballot_publication.election_ids.clone().unwrap_or(vec![]);
    for election_id in election_ids.clone() {
        update_election_status(
            &hasura_transaction,
            &election_id,
            &tenant_id.clone(),
            &election_event_id.clone(),
            true,
        )
        .await
        .with_context(|| "error updating election status")?;
    }

    let board_name = get_election_event_board(election_event.bulletin_board_reference.clone())
        .with_context(|| "missing bulletin board")?;

    // let electoral_log = ElectoralLog::new(board_name.as_str()).await?;
    let electoral_log = ElectoralLog::for_admin_user(
        hasura_transaction,
        &board_name,
        &tenant_id,
        &election_event.id,
        &user_id,
        Some(username.clone()),
        Some(election_ids.clone()),
        None,
    )
    .await?;
    electoral_log
        .post_election_published(
            election_event_id.clone(),
            Some(election_ids.clone()),
            ballot_publication_id.clone(),
            Some(user_id),
            Some(username),
        )
        .await
        .map_err(|e| anyhow!("error posting to the electoral log: {e}"))?;
    Ok(())
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_publication_json(
    hasura_transaction: &Transaction<'_>,
    tenant_id: String,
    election_event_id: String,
    ballot_publication_id: String,
    election_id: Option<String>,
    limit: Option<usize>,
) -> Result<Value> {
    let ballot_style = get_publication_ballot_styles(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &ballot_publication_id,
        limit,
    )
    .await?;

    let ballot_style_strings: Vec<Option<String>> = ballot_style
        .into_iter()
        .filter(|ballot_style| {
            election_id
                .clone()
                .map(|id| ballot_style.election_id == id)
                .unwrap_or(true)
        })
        .map(|style| style.ballot_eml.clone())
        .collect();

    let val_arr: Vec<Value> = ballot_style_strings
        .iter()
        .map(|el| el.clone().map(|val| deserialize_str(&val).ok()).flatten())
        .filter(|el| el.is_some())
        .map(|el| el.ok_or(anyhow!("Empty ballot style!")))
        .collect::<Result<Vec<_>>>()?;

    Ok(serde_json::Value::Array(val_arr))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PublicationStyles {
    ballot_publication_id: String,
    ballot_styles: Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PublicationDiff {
    current: PublicationStyles,
    previous: Option<PublicationStyles>,
}

#[instrument(err)]
pub async fn get_ballot_publication_diff(
    hasura_transaction: &Transaction<'_>,
    tenant_id: String,
    election_event_id: String,
    ballot_publication_id: String,
    limit: Option<usize>,
) -> Result<PublicationDiff> {
    let ballot_publication = get_ballot_publication_by_id(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &ballot_publication_id,
    )
    .await?
    .with_context(|| "Can't find ballot publication")?;

    let previous_publication_id = if let Some(election_id) = ballot_publication.election_id.clone()
    {
        get_previous_publication_election(
            &hasura_transaction,
            &tenant_id,
            &election_event_id,
            ballot_publication.created_at.clone(),
            &election_id,
        )
        .await?
        .map(|pub_data| pub_data.id)
        .ok_or_else(|| {
            anyhow!(
                "Can't find ballot publication for election id {}",
                election_id
            )
        })
        .with_context(|| "Error retrieving previous ballot publication for election")
        .ok()
    } else {
        get_previous_publication(
            &hasura_transaction,
            &tenant_id,
            &election_event_id,
            ballot_publication.created_at.clone(),
        )
        .await?
        .map(|pub_data| pub_data.id)
        .ok_or_else(|| anyhow!("Can't find ballot publication"))
        .with_context(|| "Error retrieving previous ballot publication")
        .ok()
    };

    let current_json = get_publication_json(
        &hasura_transaction,
        tenant_id.clone(),
        election_event_id.clone(),
        ballot_publication.id.clone(),
        ballot_publication.election_id.clone(),
        limit,
    )
    .await?;

    let current = PublicationStyles {
        ballot_publication_id: ballot_publication_id.clone(),
        ballot_styles: current_json,
    };

    let previous = if let Some(previous_publication_id) = previous_publication_id {
        let previous_json = get_publication_json(
            &hasura_transaction,
            tenant_id.clone(),
            election_event_id.clone(),
            previous_publication_id.clone(),
            ballot_publication.election_id.clone(),
            limit,
        )
        .await?;

        Some(PublicationStyles {
            ballot_publication_id: previous_publication_id.clone(),
            ballot_styles: previous_json,
        })
    } else {
        None
    };

    Ok(PublicationDiff { current, previous })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn publication(contests: Value) -> Value {
        json!([{ "contests": contests }])
    }

    #[test]
    fn absent_and_false_acclamation_values_are_equivalent() {
        let previous = collect_acclamation_states(&publication(json!([
            {"id": "contest-1", "name": "Mayor"}
        ])))
        .unwrap();
        let current = collect_acclamation_states(&publication(json!([
            {"id": "contest-1", "name": "Mayor", "is_acclaimed": false}
        ])))
        .unwrap();

        assert!(validate_acclamation_states(&previous, &current).is_ok());
    }

    #[test]
    fn first_publication_can_set_acclamation_values() {
        let current = collect_acclamation_states(&publication(json!([
            {"id": "contest-1", "name": "Mayor", "is_acclaimed": true}
        ])))
        .unwrap();

        assert!(validate_acclamation_states(&BTreeMap::new(), &current).is_ok());
    }

    #[test]
    fn detects_when_any_voting_channel_has_started() {
        let mut status = ElectionStatus::default();
        assert!(!election_has_started(&status));

        status.telephone_voting_period_dates.first_started_at =
            Some("2026-08-28T11:00:00Z".parse().unwrap());

        assert!(election_has_started(&status));
    }

    #[test]
    fn reports_every_changed_acclamation_value_in_contest_id_order() {
        let previous = collect_acclamation_states(&publication(json!([
            {"id": "contest-b", "name": "Council", "is_acclaimed": true},
            {"id": "contest-a", "name": "Mayor", "is_acclaimed": false}
        ])))
        .unwrap();
        let current = collect_acclamation_states(&publication(json!([
            {"id": "contest-b", "name": "Council", "is_acclaimed": false},
            {"id": "contest-a", "name": "Mayor", "is_acclaimed": true}
        ])))
        .unwrap();

        let error = validate_acclamation_states(&previous, &current).unwrap_err();
        let validation_error = error
            .downcast_ref::<BallotPublicationValidationError>()
            .unwrap();

        assert_eq!(validation_error.reasons.len(), 2);
        assert!(validation_error.reasons[0].contains("Mayor"));
        assert!(validation_error.reasons[0].contains("false to true"));
        assert!(validation_error.reasons[1].contains("Council"));
        assert!(validation_error.reasons[1].contains("true to false"));
    }

    #[test]
    fn rejects_inconsistent_values_within_one_publication() {
        let result = collect_acclamation_states(&json!([
            {"contests": [{"id": "contest-1", "is_acclaimed": false}]},
            {"contests": [{"id": "contest-1", "is_acclaimed": true}]}
        ]));

        assert!(result.is_err());
    }
}
