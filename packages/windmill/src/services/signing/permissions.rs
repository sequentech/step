// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Who can sign an action is its `sign-<action>` permission on roles. A
//! change to it is logged, as SigningPermissionChanged, in every election
//! event of the tenant that has a rule for the action.

use crate::postgres::signing::{list_events_with_signing_rule, lock_signing_event};
use crate::services::signing::log::{stage, Actor, LogScope, LogStep, SystemOutcome};
use anyhow::{anyhow, ensure, Result};
use deadpool_postgres::Transaction;
use electoral_log::messages::newtypes::SigningStatementKind;
use sequent_core::signing::SigningAction;
use sequent_core::types::permissions::Permissions;
use serde_json::json;
use std::collections::BTreeSet;
use strum::IntoEnumIterator;
use strum_macros::Display;
use tracing::instrument;
use uuid::Uuid;

/// Whether a role gained or lost a permission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
#[strum(serialize_all = "kebab-case")]
pub enum RolePermissionChange {
    Added,
    Removed,
}

/// The action a permission lets its holders sign, if it is one of the
/// `sign-<action>` permissions.
pub fn signing_action_of(permission: &str) -> Option<SigningAction> {
    SigningAction::iter().find(|action| action.sign_permission().to_string() == permission)
}

/// A role (a Keycloak group) gained or lost an action's sign permission.
#[derive(Debug, Clone)]
pub struct SigningPermissionChange {
    pub tenant_id: Uuid,
    pub action: SigningAction,
    pub role_id: String,
    pub role_name: String,
    pub change: RolePermissionChange,
    /// Who changed it.
    pub editor: Actor,
    /// The permissions that allowed the change.
    pub allowed_by: Vec<Permissions>,
}

/// The election events a change to sign permissions is logged in, whose
/// signing locks the transaction holds: those of the tenant with a rule for
/// each action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockedEvents {
    tenant_id: Uuid,
    by_action: Vec<(SigningAction, Vec<Uuid>)>,
}

impl LockedEvents {
    /// Every locked event, in order.
    pub fn events(&self) -> Vec<Uuid> {
        let events: BTreeSet<Uuid> = self
            .by_action
            .iter()
            .flat_map(|(_, events)| events.iter().copied())
            .collect();
        events.into_iter().collect()
    }

    fn of(&self, action: SigningAction) -> Option<&[Uuid]> {
        self.by_action
            .iter()
            .find(|(locked, _)| *locked == action)
            .map(|(_, events)| events.as_slice())
    }
}

/// Takes, in one order, the signing lock of every election event of the
/// tenant that has a rule for one of `actions`. Call it before reading what
/// a role holds and before changing Keycloak, then stage the changes against
/// the result.
#[instrument(skip(hasura_transaction), err)]
pub async fn lock_permission_change_events(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    actions: &[SigningAction],
) -> Result<LockedEvents> {
    let mut by_action = vec![];
    for action in actions {
        let events = list_events_with_signing_rule(hasura_transaction, tenant_id, *action).await?;
        by_action.push((*action, events));
    }
    let locked = LockedEvents {
        tenant_id,
        by_action,
    };
    // Events in id order, so two changes lock them in the same order.
    for election_event_id in locked.events() {
        lock_signing_event(hasura_transaction, tenant_id, election_event_id).await?;
    }
    Ok(locked)
}

/// Locks the events of one change and stages it (see
/// [`stage_permission_changes`]). Returns the events logged in.
#[instrument(skip(hasura_transaction), err)]
pub async fn stage_permission_change(
    hasura_transaction: &Transaction<'_>,
    change: &SigningPermissionChange,
) -> Result<Vec<Uuid>> {
    let locked =
        lock_permission_change_events(hasura_transaction, change.tenant_id, &[change.action])
            .await?;
    stage_permission_changes(hasura_transaction, &locked, std::slice::from_ref(change)).await?;
    Ok(locked.events())
}

/// Stages SigningPermissionChanged for each change in the locked events
/// with a rule for its action, in the caller's transaction, in the order
/// given. A change to an action whose events aren't locked is refused.
#[instrument(skip(hasura_transaction), err)]
pub async fn stage_permission_changes(
    hasura_transaction: &Transaction<'_>,
    locked: &LockedEvents,
    changes: &[SigningPermissionChange],
) -> Result<()> {
    for change in changes {
        ensure!(
            change.tenant_id == locked.tenant_id,
            "the change is for another tenant than the locked events"
        );
        let events = locked
            .of(change.action)
            .ok_or_else(|| anyhow!("the events of {} are not locked", change.action))?;
        let permission = change.action.sign_permission().to_string();
        let description = format!(
            "Role {} updated: {permission} {}",
            change.role_name, change.change
        );
        let details = json!({
            "action": change.action.to_string(),
            "permission": permission,
            "role_id": change.role_id,
            "role": change.role_name,
            "change": change.change.to_string(),
            "allowed_by": change.allowed_by.iter().map(ToString::to_string).collect::<Vec<_>>(),
        });
        for election_event_id in events {
            stage(
                hasura_transaction,
                &LogStep {
                    kind: SigningStatementKind::SigningPermissionChanged,
                    user: change.editor.clone(),
                    system: SystemOutcome::Info,
                    scope: LogScope {
                        tenant_id: change.tenant_id,
                        election_event_id: *election_event_id,
                        election_id: None,
                        area_id: None,
                    },
                    description: description.clone(),
                    details: details.clone(),
                },
            )
            .await?;
        }
    }
    Ok(())
}

/// The actions whose sign permission is among `permissions`, once each, in
/// the order they first appear.
pub fn signing_actions_in<'a>(
    permissions: impl IntoIterator<Item = &'a str>,
) -> Vec<SigningAction> {
    let mut actions = vec![];
    for action in permissions.into_iter().filter_map(signing_action_of) {
        if !actions.contains(&action) {
            actions.push(action);
        }
    }
    actions
}
