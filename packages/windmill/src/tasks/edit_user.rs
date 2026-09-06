// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::cast_vote::{
    discard_voter_cast_votes, get_voter_cast_vote_state, VoterCastVoteState,
};
use crate::postgres::election_event::{get_election_event_by_id, ElectionEventDatafix};
use crate::services::database::get_hasura_pool;
use crate::services::electoral_log::{
    post_voter_password_change, ElectoralLogAdminContext, VoterPasswordChangeSource,
};
use crate::services::external;
use crate::services::external::datafix_types::{
    SoapRequest, SoapRequestResponse, SoapRequestResult,
};
use crate::services::external::utils::{
    external_voter_lock_key, post_operation_result_to_electoral_log, voted_via_internet,
    voted_via_not_internet_channel, DATAFIX_VOTER_LOCK_SECS,
};
use crate::services::external::voterview_requests::SoapSendError;
use crate::services::pg_lock::PgLock;
use crate::services::tasks_execution::{update_complete, update_fail};
use crate::types::error::{Error, Result};
use anyhow::Context;
use celery::error::TaskError;
use chrono::Duration;
use deadpool_postgres::Client as DbClient;
use electoral_log::messages::newtypes::ExtApiRequestDirection;
use sequent_core::services::date::ISO8601;
use sequent_core::services::keycloak::{get_event_realm, KeycloakAdminClient};
use sequent_core::services::uuid_validation::parse_uuid_v4;
use sequent_core::types::hasura::core::{ElectionEvent, TasksExecution};
use sequent_core::types::keycloak::{User, ATTR_RESET_VALUE, VOTED_CHANNEL};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{error, info, instrument};
use uuid::Uuid;

/// Input for the `edit_user` task. Mirrors the fields the `/edit-user` route
/// forwards to Keycloak. Only launched for a Datafix election event, so
/// `election_event_id` is always present.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EditUserTaskBody {
    pub tenant_id: String,
    pub user_id: String,
    pub election_event_id: String,
    pub enabled: Option<bool>,
    pub attributes: HashMap<String, Vec<String>>,
    pub email: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub temporary: Option<bool>,
    #[serde(default)]
    pub password_change_initiator: Option<ElectoralLogAdminContext>,
}

/// Response of the `/edit-user` route. For Datafix election events the edit is
/// deferred to the `edit_user` task and `task_execution` is returned (`user` is
/// `None`) so the admin portal can track the VoterView-coupled release in a task
/// widget; otherwise the voter is edited synchronously and returned in `user`
/// with `task_execution` set to `None`.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EditUserOutput {
    pub user: Option<User>,
    pub task_execution: Option<TasksExecution>,
}

/// Edits a Datafix voter asynchronously so the admin's Save is not blocked by
/// the (potentially slow, retried) VoterView round-trip. The release logic runs
/// in [`apply_datafix_voter_edit`]; its outcome is recorded on `task_execution`,
/// which backs the operator's task widget. A Datafix voter's ballots are
/// discarded and its voted-channel attribute reset after the Keycloak
/// disable. A retry also resumes a partial disable when Keycloak is already
/// disabled but active ballots remain. Whether `SetNotVoted` converges is
/// still handled by the separate manual reconciliation process.
#[instrument(
    skip_all,
    fields(
        tenant_id = %body.tenant_id,
        election_event_id = %body.election_event_id,
        user_id = %body.user_id
    ),
    err
)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn edit_user(body: EditUserTaskBody, task_execution: TasksExecution) -> Result<()> {
    match apply_datafix_voter_edit(&body).await {
        Ok(()) => {
            update_complete(&task_execution, None)
                .await
                .context("Failed to update the edit-user task execution status to COMPLETED")?;
            Ok(())
        }
        Err(message) => {
            update_fail(&task_execution, &message).await.ok();
            Err(Error::String(message))
        }
    }
}

/// A save that flips an enabled voter to disabled, which triggers the
/// discard → `SetNotVoted` release path.
#[instrument]
fn is_disable_transition(current: Option<bool>, requested: Option<bool>) -> bool {
    current == Some(true) && requested == Some(false)
}

/// A save that flips a disabled voter back to enabled; only allowed once the
/// voter's Datafix voting state is resolved.
#[instrument]
fn is_reenable_transition(current: Option<bool>, requested: Option<bool>) -> bool {
    current == Some(false) && requested == Some(true)
}

/// Context shared by every phase of a Datafix voter edit: the request body plus
/// the realm and per-voter lock the whole flow operates under.
struct DatafixEditCtx<'a> {
    body: &'a EditUserTaskBody,
    realm: String,
    lock: &'a PgLock,
}

/// The release actions a Datafix voter edit must take, derived from the
/// enabled-flag transition and the voter's current Keycloak attributes.
#[derive(Debug)]
struct VoterReleasePlan {
    /// The edit disables the voter, so its non-discarded ballots must be
    /// discarded.
    release_attempt: bool,
    /// The voter voted online, so the release owes VoterView a `SetNotVoted`.
    owes_set_not_voted: bool,
}

/// Derives the [`VoterReleasePlan`] for an edit and enforces the state guards:
/// a re-enable is refused while the voting state is unresolved, and a release
/// is refused for a voter who voted through another channel.
#[instrument(skip(current_attributes), err, ret)]
fn plan_voter_release(
    current_enabled: Option<bool>,
    requested_enabled: Option<bool>,
    cast_vote_state: &VoterCastVoteState,
    current_attributes: &HashMap<String, Vec<String>>,
) -> std::result::Result<VoterReleasePlan, String> {
    let disable_transition = is_disable_transition(current_enabled, requested_enabled);
    let reenable_transition = is_reenable_transition(current_enabled, requested_enabled);
    let disable_requested = requested_enabled == Some(false);
    let release_attempt = disable_transition
        || (disable_requested
            && (cast_vote_state.has_unresolved_vote
                || cast_vote_state.has_valid_vote
                || voted_via_internet(current_attributes)));

    if reenable_transition
        && (cast_vote_state.has_unresolved_vote
            || cast_vote_state.has_valid_vote
            || voted_via_not_internet_channel(current_attributes))
    {
        return Err(
            "Cannot re-enable a voter while its Datafix voting state is unresolved".to_string(),
        );
    }
    if release_attempt && voted_via_not_internet_channel(current_attributes) {
        return Err(
            "Cannot release a voter recorded as having voted through another channel".to_string(),
        );
    }

    Ok(VoterReleasePlan {
        release_attempt,
        owes_set_not_voted: release_attempt && voted_via_internet(current_attributes),
    })
}

/// Rejects edits to the fields an admin may not change on a Datafix voter: the
/// username (the VoterView identifier) and the voted channel.
#[instrument(skip_all, err)]
fn validate_datafix_immutable_fields(
    body: &EditUserTaskBody,
    current_user: &User,
    current_attributes: &HashMap<String, Vec<String>>,
) -> std::result::Result<(), String> {
    if body.username.is_some() && body.username.as_ref() != current_user.username.as_ref() {
        return Err("A Datafix voter identifier cannot be changed in the admin portal".to_string());
    }
    if let Some(requested_channel) = body.attributes.get(VOTED_CHANNEL) {
        if current_attributes.get(VOTED_CHANNEL) != Some(requested_channel) {
            return Err("The Datafix voting channel cannot be edited directly".to_string());
        }
    }
    Ok(())
}

/// Loads the election event the edit targets, needed for its Datafix
/// configuration when rendering `SetNotVoted`.
#[instrument(err)]
async fn load_election_event(
    tenant_id: &str,
    election_event_id: &str,
) -> anyhow::Result<ElectionEvent> {
    let mut client: DbClient = get_hasura_pool().await.get().await?;
    let transaction = client.transaction().await?;
    get_election_event_by_id(&transaction, tenant_id, election_event_id).await
}

/// Single-round snapshot of the voter's cast-vote states, used to decide whether
/// a disable needs a release and whether a re-enable is safe.
#[instrument(err, ret)]
async fn voter_cast_vote_state(
    tenant_id: &str,
    election_event_id: &str,
    voter_id: &str,
) -> anyhow::Result<VoterCastVoteState> {
    let tenant_id = parse_uuid_v4(tenant_id)?;
    let election_event_id = parse_uuid_v4(election_event_id)?;
    let mut client: DbClient = get_hasura_pool().await.get().await?;
    let transaction = client.transaction().await?;
    get_voter_cast_vote_state(&transaction, &tenant_id, &election_event_id, voter_id).await
}

/// Applies the requested Keycloak fields.
#[instrument(skip(ctx, client), err)]
async fn edit_keycloak_voter(
    ctx: &DatafixEditCtx<'_>,
    client: KeycloakAdminClient,
) -> std::result::Result<(), String> {
    let body = ctx.body;
    client
        .edit_user(
            &ctx.realm,
            &body.user_id,
            body.enabled,
            Some(body.attributes.clone()),
            body.email.clone(),
            body.first_name.clone(),
            body.last_name.clone(),
            body.username.clone(),
            body.password.clone(),
            body.temporary,
        )
        .await
        .map(|_| ())
        .map_err(|err| format!("Error editing Datafix voter in Keycloak: {err:?}"))
}

/// Discards the voter's active ballots in its own Hasura transaction. Keycloak
/// and Hasura are updated sequentially; failures are traced by the caller and
/// left for the existing reconciliation process.
#[instrument(err)]
async fn discard_voter_ballots(
    tenant_id: &str,
    election_event_id: &str,
    voter_id: &str,
) -> anyhow::Result<()> {
    let tenant_id = parse_uuid_v4(tenant_id)?;
    let election_event_id = parse_uuid_v4(election_event_id)?;
    let mut client: DbClient = get_hasura_pool().await.get().await?;
    let transaction = client.transaction().await?;
    let discarded =
        discard_voter_cast_votes(&transaction, &tenant_id, &election_event_id, voter_id).await?;
    transaction.commit().await?;
    info!(discarded, "Discarded active Datafix cast votes");
    Ok(())
}

/// Resets `VOTED_CHANNEL` back to `NONE` after a release discards the voter's
/// ballots, mirroring the reset `unmark_voter_as_voted` already does for the
/// inbound `/unmark-voted` call. Without this the attribute — set once, when a
/// vote first resolves to `Valid`, and otherwise never touched — stays stale
/// after the ballot it described is gone, wrongly blocking a later re-enable
/// and feeding a stale channel into the reconciliation patch for a voter
/// Datafix has no record of. Only ever runs after `plan_voter_release` has
/// already confirmed the voter isn't recorded as voted through another
/// channel, so this can only be clearing a stale `INTERNET` value or a no-op.
#[instrument(skip(ctx))]
async fn clear_voted_channel(ctx: &DatafixEditCtx<'_>) -> anyhow::Result<()> {
    let client = KeycloakAdminClient::new().await?;
    let attributes = HashMap::from([(
        VOTED_CHANNEL.to_string(),
        vec![ATTR_RESET_VALUE.to_string()],
    )]);
    client
        .edit_user(
            &ctx.realm,
            &ctx.body.user_id,
            None,
            Some(attributes),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .map(|_| ())
}

/// Records the outcome of a disabled-voter release in the electoral log.
/// Failures are logged and swallowed so auditing never fails the user edit.
#[instrument(skip(ctx))]
async fn audit_datafix_user_operation(ctx: &DatafixEditCtx<'_>, username: &str, operation: String) {
    let Ok(mut client) = get_hasura_pool().await.get().await else {
        error!("Unable to get a DB connection for the Datafix audit entry");
        return;
    };
    let Ok(transaction) = client.transaction().await else {
        error!("Unable to start a transaction for the Datafix audit entry");
        return;
    };
    if let Err(err) = post_operation_result_to_electoral_log(
        &transaction,
        &ctx.body.tenant_id,
        &ctx.body.election_event_id,
        Some(ctx.body.user_id.as_str()),
        username,
        ExtApiRequestDirection::Outbound,
        operation,
    )
    .await
    {
        error!("Unable to record the Datafix audit entry: {err}");
    }
}

/// Whether a `SetNotVoted` response means VoterView agrees the voter has not
/// voted; used only to pick the audited outcome label.
#[instrument]
fn set_not_voted_converged(response: &SoapRequestResponse) -> bool {
    matches!(
        response,
        SoapRequestResponse::Ok | SoapRequestResponse::AlreadyNotVoted
    )
}

/// Sends `SetNotVoted` to VoterView for the already-disabled, already-released
/// voter and records the outcome in the electoral log. Every outcome —
/// success, an application rejection, a SOAP fault, or a transport failure —
/// is only logged: the ballots are already discarded, so nothing here can or
/// should be rolled back. A divergence between the platform and VoterView is
/// left for the manual reconciliation process to resolve.
#[instrument(skip(ctx, election_event))]
async fn send_set_not_voted(
    ctx: &DatafixEditCtx<'_>,
    election_event: ElectionEvent,
    username: &str,
) {
    let prepared = match external::voterview_requests::prepare(
        SoapRequest::SetNotVoted,
        ElectionEventDatafix(election_event),
        &Some(username.to_string()),
    )
    .await
    {
        Ok(prepared) => prepared,
        Err(err) => {
            error!("Unable to prepare SetNotVoted: {err}");
            audit_datafix_user_operation(
                ctx,
                username,
                "SetNotVoted NotDispatched: pre-dispatch-error".to_string(),
            )
            .await;
            return;
        }
    };
    let template_sha256 = prepared.template_sha256().to_string();
    let operation = match external::voterview_requests::send_prepared(prepared).await {
        Ok(SoapRequestResult {
            response,
            template_sha256,
        }) if set_not_voted_converged(&response) => {
            format!("SetNotVoted Succeeded (template_sha256={template_sha256})")
        }
        Ok(SoapRequestResult {
            response,
            template_sha256,
        }) => {
            format!(
                "SetNotVoted Failed: {} (template_sha256={template_sha256})",
                response.classification()
            )
        }
        Err(SoapSendError::NotDispatched(err)) => {
            error!("SetNotVoted could not be dispatched: {err}");
            format!(
                "SetNotVoted NotDispatched: connection-error (template_sha256={template_sha256})"
            )
        }
        Err(SoapSendError::Ambiguous(err)) => {
            error!("SetNotVoted transport or response error: {err}");
            format!(
                "SetNotVoted Failed: transport-or-response-error (template_sha256={template_sha256})"
            )
        }
    };
    audit_datafix_user_operation(ctx, username, operation).await;
}

/// Runs the voter edit while the per-voter lock is held: validates the edit,
/// plans the release and, on a disable, edits Keycloak before discarding the
/// voter's ballots and sending `SetNotVoted` when required.
#[instrument(skip(ctx, election_event), err)]
async fn run_datafix_voter_edit(
    ctx: &DatafixEditCtx<'_>,
    election_event: ElectionEvent,
) -> std::result::Result<(), String> {
    let body = ctx.body;
    let client = KeycloakAdminClient::new()
        .await
        .map_err(|err| format!("{err:?}"))?;
    let current_user = client
        .get_user(&ctx.realm, &body.user_id)
        .await
        .map_err(|err| format!("{err:?}"))?;
    let current_attributes = current_user.attributes.clone().unwrap_or_default();
    validate_datafix_immutable_fields(body, &current_user, &current_attributes)?;

    let needs_cast_vote_state =
        body.enabled == Some(false) || is_reenable_transition(current_user.enabled, body.enabled);
    let cast_vote_state = if needs_cast_vote_state {
        voter_cast_vote_state(&body.tenant_id, &body.election_event_id, &body.user_id)
            .await
            .map_err(|err| format!("Error checking unresolved cast votes: {err:?}"))?
    } else {
        VoterCastVoteState {
            has_unresolved_vote: false,
            has_valid_vote: false,
        }
    };
    let plan = plan_voter_release(
        current_user.enabled,
        body.enabled,
        &cast_vote_state,
        &current_attributes,
    )?;
    info!("Voter edit plan: {plan:?}, cast-vote state: {cast_vote_state:?}");

    let password_change_initiator = if body.password.is_some() {
        Some(
            body.password_change_initiator
                .as_ref()
                .ok_or("Missing initiating admin for voter password-change audit")?,
        )
    } else {
        None
    };

    edit_keycloak_voter(ctx, client).await?;

    if let Some(admin) = password_change_initiator {
        post_voter_password_change(
            &body.tenant_id,
            &body.election_event_id,
            &body.user_id,
            current_user.username.clone(),
            admin,
            VoterPasswordChangeSource::AdminPortal,
        )
        .await
        .map_err(|err| {
            format!("Voter password changed, but its electoral-log entry failed: {err:#}")
        })?;
    }

    if !plan.release_attempt {
        return Ok(());
    }

    ctx.lock
        .update_expiry_for(DATAFIX_VOTER_LOCK_SECS)
        .await
        .map_err(|err| {
            format!("The Datafix voter lock was lost before discarding ballots: {err}")
        })?;
    discard_voter_ballots(&body.tenant_id, &body.election_event_id, &body.user_id)
        .await
        .map_err(|err| format!("Error discarding Datafix cast votes: {err:?}"))?;
    clear_voted_channel(ctx).await.map_err(|err| {
        format!("Could not reset the voter's voted-channel attribute after discard: {err}")
    })?;

    if !plan.owes_set_not_voted {
        return Ok(());
    }
    let username = current_user
        .username
        .clone()
        .ok_or("Datafix voter has no username")?;
    send_set_not_voted(ctx, election_event, &username).await;
    Ok(())
}

/// Applies an admin edit to a Datafix voter under the per-voter lock. On a
/// disable of a voter who voted online, it discards the voter's ballots and
/// sends `SetNotVoted`, logging whatever VoterView answers; a re-enable is
/// refused while the voter's Datafix voting state is unresolved. Returns
/// `Ok(())` on success or a human-readable failure reason recorded on the task
/// widget.
#[instrument(skip(body), err)]
async fn apply_datafix_voter_edit(body: &EditUserTaskBody) -> std::result::Result<(), String> {
    let realm = get_event_realm(&body.tenant_id, &body.election_event_id);
    let election_event = load_election_event(&body.tenant_id, &body.election_event_id)
        .await
        .map_err(|err| format!("Error loading election event: {err:?}"))?;

    let user_id_uuid =
        parse_uuid_v4(&body.user_id).map_err(|err| format!("Invalid voter id: {err}"))?;
    let lock = PgLock::acquire(
        external_voter_lock_key(&body.tenant_id, &body.election_event_id, &user_id_uuid),
        Uuid::new_v4().to_string(),
        ISO8601::now() + Duration::seconds(DATAFIX_VOTER_LOCK_SECS),
    )
    .await
    .map_err(|err| format!("Another operation is updating this voter: {err}"))?;

    let ctx = DatafixEditCtx {
        body,
        realm,
        lock: &lock,
    };
    let result = run_datafix_voter_edit(&ctx, election_event).await;

    if let Err(err) = lock.release().await {
        if result.is_ok() {
            return Err(format!("Error releasing the Datafix voter lock: {err}"));
        }
        error!("Error releasing the Datafix voter lock: {err}");
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequent_core::types::keycloak::VOTED_CHANNEL_INTERNET_VALUE;

    #[test]
    fn disable_requires_an_enabled_to_disabled_transition() {
        assert!(is_disable_transition(Some(true), Some(false)));
        assert!(!is_disable_transition(Some(false), Some(false)));
        assert!(!is_disable_transition(Some(true), None));
        assert!(!is_disable_transition(None, Some(false)));
    }

    #[test]
    fn reenable_requires_a_disabled_to_enabled_transition() {
        assert!(is_reenable_transition(Some(false), Some(true)));
        assert!(!is_reenable_transition(Some(true), Some(true)));
        assert!(!is_reenable_transition(Some(false), None));
        assert!(!is_reenable_transition(None, Some(true)));
    }

    #[test]
    fn already_not_voted_is_a_converged_release() {
        assert!(set_not_voted_converged(&SoapRequestResponse::Ok));
        assert!(set_not_voted_converged(
            &SoapRequestResponse::AlreadyNotVoted
        ));
        assert!(!set_not_voted_converged(&SoapRequestResponse::Rejected(
            "rejected".to_string()
        )));
    }

    fn no_cast_votes() -> VoterCastVoteState {
        VoterCastVoteState {
            has_unresolved_vote: false,
            has_valid_vote: false,
        }
    }

    fn internet_voter() -> HashMap<String, Vec<String>> {
        HashMap::from([(
            VOTED_CHANNEL.to_string(),
            vec![VOTED_CHANNEL_INTERNET_VALUE.to_string()],
        )])
    }

    #[test]
    fn disabling_a_clean_voter_releases_without_owing_set_not_voted() {
        let plan =
            plan_voter_release(Some(true), Some(false), &no_cast_votes(), &HashMap::new()).unwrap();
        assert!(plan.release_attempt);
        assert!(!plan.owes_set_not_voted);
    }

    #[test]
    fn disabling_an_internet_voter_owes_set_not_voted() {
        let plan = plan_voter_release(Some(true), Some(false), &no_cast_votes(), &internet_voter())
            .unwrap();
        assert!(plan.release_attempt);
        assert!(plan.owes_set_not_voted);
    }

    #[test]
    fn a_repeated_disable_plans_no_release() {
        let plan = plan_voter_release(Some(false), Some(false), &no_cast_votes(), &HashMap::new())
            .unwrap();
        assert!(!plan.release_attempt);
    }

    #[test]
    fn a_partial_disable_with_active_votes_retries_the_release() {
        let state = VoterCastVoteState {
            has_unresolved_vote: true,
            has_valid_vote: false,
        };
        let plan = plan_voter_release(Some(false), Some(false), &state, &HashMap::new()).unwrap();
        assert!(plan.release_attempt);
        assert!(!plan.owes_set_not_voted);
    }

    #[test]
    fn a_partial_disable_with_an_internet_channel_retries_set_not_voted() {
        let plan = plan_voter_release(
            Some(false),
            Some(false),
            &no_cast_votes(),
            &internet_voter(),
        )
        .unwrap();
        assert!(plan.release_attempt);
        assert!(plan.owes_set_not_voted);
    }

    #[test]
    fn reenabling_is_refused_while_valid_votes_exist() {
        let state = VoterCastVoteState {
            has_valid_vote: true,
            ..no_cast_votes()
        };
        assert!(plan_voter_release(Some(false), Some(true), &state, &HashMap::new()).is_err());
    }

    #[test]
    fn reenabling_a_voter_with_only_discarded_internet_ballots_is_allowed() {
        // The voted-channel attribute is never cleared by a discard, so once a
        // voter has ever cast an internet ballot it stays "Internet" forever —
        // re-enable must key off the live `VoterCastVoteState`, not this stale
        // attribute, or a fully-resolved (discarded) voter could never be
        // re-enabled.
        let plan = plan_voter_release(Some(false), Some(true), &no_cast_votes(), &internet_voter())
            .unwrap();
        assert!(!plan.release_attempt);
    }

    #[test]
    fn reenabling_is_refused_while_marked_voted_via_another_channel() {
        // Unlike an internet ballot, a non-internet channel has no
        // corresponding `cast_vote` row — the attribute is the only record of
        // it, and only Datafix's own `/unmark-voted` call may reverse it.
        let attributes = HashMap::from([(VOTED_CHANNEL.to_string(), vec!["PAPER".to_string()])]);
        assert!(
            plan_voter_release(Some(false), Some(true), &no_cast_votes(), &attributes).is_err()
        );
    }

    #[test]
    fn reenabling_a_clean_voter_plans_no_release() {
        let plan =
            plan_voter_release(Some(false), Some(true), &no_cast_votes(), &HashMap::new()).unwrap();
        assert!(!plan.release_attempt);
    }

    #[test]
    fn releasing_an_other_channel_voter_is_refused() {
        let attributes = HashMap::from([(VOTED_CHANNEL.to_string(), vec!["PAPER".to_string()])]);
        assert!(
            plan_voter_release(Some(true), Some(false), &no_cast_votes(), &attributes).is_err()
        );
    }

    fn edit_body(
        username: Option<&str>,
        attributes: HashMap<String, Vec<String>>,
    ) -> EditUserTaskBody {
        EditUserTaskBody {
            tenant_id: String::new(),
            user_id: String::new(),
            election_event_id: String::new(),
            enabled: None,
            attributes,
            email: None,
            first_name: None,
            last_name: None,
            username: username.map(str::to_string),
            password: None,
            temporary: None,
            password_change_initiator: None,
        }
    }

    #[test]
    fn the_voter_identifier_is_immutable() {
        let current_user = User {
            username: Some("voter1".to_string()),
            ..User::default()
        };
        let renamed = edit_body(Some("voter2"), HashMap::new());
        assert!(
            validate_datafix_immutable_fields(&renamed, &current_user, &HashMap::new()).is_err()
        );
        let unchanged = edit_body(Some("voter1"), HashMap::new());
        assert!(
            validate_datafix_immutable_fields(&unchanged, &current_user, &HashMap::new()).is_ok()
        );
    }

    #[test]
    fn the_voted_channel_is_immutable() {
        let current_user = User::default();
        let channel_edit = edit_body(None, internet_voter());
        assert!(
            validate_datafix_immutable_fields(&channel_edit, &current_user, &HashMap::new())
                .is_err()
        );
        let echoed = edit_body(None, internet_voter());
        assert!(
            validate_datafix_immutable_fields(&echoed, &current_user, &internet_voter()).is_ok()
        );
    }
}
