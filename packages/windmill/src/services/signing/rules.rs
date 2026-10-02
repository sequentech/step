// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The signing rules of an event: reading them, the signers each Post has
//! for an action, and saving a rule.
//!
//! A save is refused while the event is locked down, when it asks for more
//! signatures than any Post can give, and when another save came first.
//! It cancels every request of the action still waiting, since they were
//! started under the old rule, and logs the change. It may also change which
//! groups hold the action's sign permission (Keycloak realm roles, which
//! need `role-read` and `role-write`); that is logged in every event with a
//! rule for the action, because the permission is the tenant's.
//!
//! [`save_rule`] makes every database change of the save; Keycloak changes
//! only in [`commit_rule`], last, right before the commit, and is put back
//! when it or the commit fails.

use super::guard::effective_rule;
use super::log::{stage, LogStep, SystemOutcome};
use super::requests::cancel_request;
use super::signers::{groups_by_id, list_signers, list_signing_groups, GroupChange, SigningGroup};
use super::{action_title, log_scope, InvalidReason, SigningCaller, SigningError, SigningResult};
use crate::postgres::signing::*;
use crate::services::election::is_election_event_locked_down_in;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use electoral_log::messages::newtypes::SigningStatementKind;
use sequent_core::services::keycloak::{get_tenant_realm, KeycloakAdminClient};
use sequent_core::signing::{
    CancelReason, RequesterSigning, SigningAction, SigningRequirement, SigningRule,
    SigningRuleError, SigningScope, MAX_EXPIRES_MINUTES, MAX_SIGNATURES,
};
use sequent_core::types::permissions::Permissions;
use serde::Serialize;
use serde_json::json;
use std::time::Duration;
use strum::IntoEnumIterator;
use strum_macros::Display;
use tracing::{instrument, warn};
use uuid::Uuid;

/// How long Keycloak gets for a save's role changes.
pub const ROLE_CHANGE_TIMEOUT: Duration = Duration::from_secs(15);

/// A rule as the Protected actions table shows it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RuleView {
    #[serde(flatten)]
    pub rule: SigningRule,
    /// `None` for an action nobody configured.
    pub updated_by: Option<String>,
    pub updated_by_name: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// Every action's rule, in catalog order; the default where none is saved.
#[instrument(skip(hasura_transaction), err)]
pub async fn list_rules(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
) -> Result<Vec<RuleView>> {
    let saved = list_signing_rules(hasura_transaction, tenant_id, election_event_id).await?;
    Ok(SigningAction::iter()
        .map(
            |action| match saved.iter().find(|row| row.rule.action == action) {
                Some(row) => RuleView {
                    rule: row.rule.clone(),
                    updated_by: Some(row.updated_by.clone()),
                    updated_by_name: row.updated_by_name.clone(),
                    updated_at: Some(row.updated_at),
                },
                None => RuleView {
                    rule: SigningRule::default_for(action),
                    updated_by: None,
                    updated_by_name: None,
                    updated_at: None,
                },
            },
        )
        .collect())
}

/// How many people can sign an action for one Post.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PostCapacity {
    pub election_id: Uuid,
    pub name: String,
    /// The people who can sign for it.
    pub count: usize,
}

/// A group holding an action's sign permission ("Who can sign").
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RoleView {
    pub id: String,
    pub name: String,
    pub path: String,
}

impl From<&SigningGroup> for RoleView {
    fn from(group: &SigningGroup) -> Self {
        RoleView {
            id: group.id.clone(),
            name: group.name.clone(),
            path: group.path.clone(),
        }
    }
}

/// The signers an action has: per Post for Post actions, and the most any
/// Post (or, for event and trustee actions, the event) has.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SigningCapacity {
    pub max: usize,
    pub posts: Vec<PostCapacity>,
    /// The Posts with fewer people who can sign than the number asked for.
    pub posts_short: Vec<PostCapacity>,
    /// When the person who starts a request can't sign it: the Posts where
    /// the others are fewer than the number asked for.
    pub posts_short_without_requester: Vec<PostCapacity>,
    /// The groups that grant the sign permission.
    pub roles: Vec<RoleView>,
    /// The action's requests waiting for signatures, which a rule save
    /// cancels.
    pub waiting: usize,
    /// The event's configuration version: its published event-level
    /// ballot publications.
    pub config_version: i64,
}

/// The capacity of `action` in the event, as it would be after `change`;
/// short Posts are those with fewer than `signatures` signers (and one more
/// when `requester_signing` keeps the requester out), by default what the
/// saved rule asks.
#[instrument(skip(hasura_transaction, keycloak_transaction), err)]
pub async fn capacity(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    action: SigningAction,
    change: &GroupChange,
    signatures: Option<u16>,
    requester_signing: Option<RequesterSigning>,
) -> Result<SigningCapacity> {
    let realm = get_tenant_realm(&tenant_id.to_string());
    let signers = list_signers(keycloak_transaction, &realm, action, change).await?;
    let roles = list_signing_groups(keycloak_transaction, &realm, action)
        .await?
        .iter()
        .map(RoleView::from)
        .collect();
    let (max, posts) = match action.scope() {
        SigningScope::Post | SigningScope::PostAndCountry => {
            let posts: Vec<PostCapacity> =
                list_signing_posts(hasura_transaction, tenant_id, election_event_id)
                    .await?
                    .into_iter()
                    .map(|post| PostCapacity {
                        election_id: post.id,
                        count: signers
                            .iter()
                            .filter(|signer| signer.eligible_for(post.permission_label.as_deref()))
                            .count(),
                        name: post.name,
                    })
                    .collect();
            (
                posts.iter().map(|post| post.count).max().unwrap_or(0),
                posts,
            )
        }
        SigningScope::Event | SigningScope::Trustee => (signers.len(), vec![]),
    };
    let saved = effective_rule(hasura_transaction, tenant_id, election_event_id, action).await?;
    let signatures = usize::from(signatures.unwrap_or_else(|| saved.required()));
    let requester_out = requester_signing.unwrap_or(saved.requester_signing_effective())
        == RequesterSigning::NotAllowed
        && !action.is_trustee();
    let short = |needed: usize| -> Vec<PostCapacity> {
        posts
            .iter()
            .filter(|post| post.count < needed)
            .cloned()
            .collect()
    };
    let posts_short = short(signatures);
    let posts_short_without_requester = if requester_out {
        short(signatures + 1)
    } else {
        vec![]
    };
    let waiting =
        list_waiting_signing_requests(hasura_transaction, tenant_id, election_event_id, action)
            .await?
            .len();
    let config_version =
        count_published_event_publications(hasura_transaction, tenant_id, election_event_id)
            .await?;
    Ok(SigningCapacity {
        max,
        posts,
        posts_short,
        posts_short_without_requester,
        roles,
        waiting,
        config_version,
    })
}

/// Changes which groups hold a realm role.
#[async_trait]
pub trait SigningRoleAdmin: Send + Sync {
    /// Gets ready before any lock is taken (the Keycloak client signs in).
    async fn prepare(&self) -> Result<()> {
        Ok(())
    }
    async fn grant(&self, realm: &str, group_id: &str, permission: &str) -> Result<()>;
    async fn revoke(&self, realm: &str, group_id: &str, permission: &str) -> Result<()>;
}

/// Through the Keycloak admin API, as `/set-role-permission` does. Its
/// token is cached process-wide, so [`SigningRoleAdmin::prepare`] signs in
/// once and the changes reuse it.
#[derive(Debug, Clone, Copy, Default)]
pub struct KeycloakSigningRoleAdmin;

#[async_trait]
impl SigningRoleAdmin for KeycloakSigningRoleAdmin {
    async fn prepare(&self) -> Result<()> {
        KeycloakAdminClient::new().await.map(|_| ())
    }

    async fn grant(&self, realm: &str, group_id: &str, permission: &str) -> Result<()> {
        KeycloakAdminClient::new()
            .await?
            .set_role_permission(realm, group_id, permission)
            .await
    }

    async fn revoke(&self, realm: &str, group_id: &str, permission: &str) -> Result<()> {
        KeycloakAdminClient::new()
            .await?
            .delete_role_permission(realm, group_id, permission)
            .await
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveRuleInput {
    pub action: SigningAction,
    pub requirement: SigningRequirement,
    pub signatures: u16,
    pub requester_signing: RequesterSigning,
    pub expires_minutes: Option<u32>,
    /// Groups (by id) gaining or losing the sign permission.
    pub roles: Option<GroupChange>,
    /// The revision the editor read; 0 for an action never saved.
    pub expected_revision: i64,
}

/// What a saved rule should know about the Posts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Display)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum RuleWarning {
    /// The event has no Posts yet, so nobody can sign it for one.
    NoPosts,
    /// Some Posts have fewer people who can sign than the rule asks.
    ShortPosts,
    /// Without the requester, some Posts can't reach the number.
    RequesterExcluded,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SaveRuleOutcome {
    /// The rule's new revision.
    pub revision: i64,
    pub rule: SigningRule,
    /// The Posts that have fewer people who can sign than the rule asks.
    pub short_posts: Vec<PostCapacity>,
    pub warnings: Vec<RuleWarning>,
    /// The waiting requests the save cancelled.
    pub cancelled: Vec<Uuid>,
}

/// The Keycloak side of a save, made by [`commit_rule`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RoleChanges {
    pub realm: String,
    pub permission: String,
    /// Group ids gaining the permission.
    pub add: Vec<String>,
    /// Group ids losing it.
    pub remove: Vec<String>,
}

impl RoleChanges {
    pub fn is_empty(&self) -> bool {
        self.add.is_empty() && self.remove.is_empty()
    }
}

/// A save whose database changes are made and not committed.
#[derive(Debug, Clone, PartialEq)]
pub struct SavedRule {
    pub outcome: SaveRuleOutcome,
    pub roles: RoleChanges,
}

fn requirement_text(rule: &SigningRule) -> String {
    match rule.requirement {
        SigningRequirement::NotRequired => "no signatures needed".into(),
        SigningRequirement::Required => format!("{} signatures needed", rule.required()),
    }
}

fn group_list(groups: &[SigningGroup]) -> String {
    if groups.is_empty() {
        return "none".into();
    }
    groups
        .iter()
        .map(|group| group.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// The realm's groups with these ids, in the same order; refused when one
/// is unknown, or, among `removing`, holds the permission only through a
/// composite role.
async fn resolve_groups(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    action: SigningAction,
    ids: &[String],
    removing: bool,
) -> SigningResult<Vec<SigningGroup>> {
    let found = groups_by_id(keycloak_transaction, realm, action, ids).await?;
    ids.iter()
        .map(|id| {
            let group = found
                .iter()
                .find(|group| group.id == *id)
                .cloned()
                .ok_or_else(|| {
                    SigningError::invalid(
                        InvalidReason::UnknownGroup,
                        format!("There is no group {id}."),
                    )
                })?;
            if removing && group.composite && !group.direct {
                return Err(SigningError::invalid(
                    InvalidReason::Composite,
                    format!(
                        "{} can sign through a composite role; change it in Users and Roles.",
                        group.name
                    ),
                ));
            }
            Ok(group)
        })
        .collect()
}

fn validate(rule: &SigningRule) -> SigningResult<()> {
    if rule.signatures > MAX_SIGNATURES {
        return Err(SigningError::invalid(
            InvalidReason::OutOfRange,
            format!("A rule asks for at most {MAX_SIGNATURES} signatures."),
        ));
    }
    if rule
        .expires_minutes
        .is_some_and(|minutes| minutes > MAX_EXPIRES_MINUTES)
    {
        return Err(SigningError::invalid(
            InvalidReason::OutOfRange,
            format!("A request waits at most {MAX_EXPIRES_MINUTES} minutes."),
        ));
    }
    rule.validate().map_err(|error| {
        let reason = match error {
            SigningRuleError::NoSignatures => InvalidReason::NoSignatures,
            SigningRuleError::ZeroExpiry => InvalidReason::ZeroExpiry,
        };
        SigningError::invalid(reason, error.to_string())
    })
}

/// Makes every database change of a rule save; see the module
/// documentation. The caller holds `signing-rules-write`, called
/// [`SigningRoleAdmin::prepare`] before when the roles change, and ends
/// with [`commit_rule`].
#[instrument(skip(hasura_transaction, keycloak_transaction), err)]
pub async fn save_rule(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    caller: &SigningCaller,
    tenant_id: Uuid,
    election_event_id: Uuid,
    input: &SaveRuleInput,
) -> SigningResult<SavedRule> {
    let requested = input.roles.clone().unwrap_or_default();
    let changes_roles = !requested.add.is_empty() || !requested.remove.is_empty();
    if changes_roles && !(caller.has(Permissions::ROLE_READ) && caller.has(Permissions::ROLE_WRITE))
    {
        return Err(SigningError::Forbidden(
            "Changing who can sign needs the permissions to read and edit roles.".into(),
        ));
    }
    let action = input.action;
    let rule = SigningRule {
        action,
        requirement: input.requirement,
        signatures: input.signatures,
        requester_signing: input.requester_signing,
        expires_minutes: input.expires_minutes,
        revision: input.expected_revision,
    };
    validate(&rule)?;
    let realm = get_tenant_realm(&tenant_id.to_string());
    let added = resolve_groups(keycloak_transaction, &realm, action, &requested.add, false).await?;
    let removed = resolve_groups(
        keycloak_transaction,
        &realm,
        action,
        &requested.remove,
        true,
    )
    .await?;
    let change = GroupChange {
        add: added.iter().map(|group| group.id.clone()).collect(),
        remove: removed.iter().map(|group| group.id.clone()).collect(),
    };

    // The events whose log the step writes to, locked in one order.
    let mut events = vec![election_event_id];
    if changes_roles {
        events.extend(list_events_with_signing_rule(hasura_transaction, tenant_id, action).await?);
    }
    events.sort();
    events.dedup();
    for event in &events {
        lock_signing_event(hasura_transaction, tenant_id, *event).await?;
    }
    if is_election_event_locked_down_in(
        hasura_transaction,
        &tenant_id.to_string(),
        &election_event_id.to_string(),
    )
    .await?
    {
        return Err(SigningError::invalid(
            InvalidReason::LockedDown,
            "The election event is locked down; its signing rules can't change.",
        ));
    }

    let capacity = capacity(
        hasura_transaction,
        keycloak_transaction,
        tenant_id,
        election_event_id,
        action,
        &change,
        Some(rule.required()),
        Some(rule.requester_signing_effective()),
    )
    .await?;
    let mut warnings = vec![];
    let mut short_posts = vec![];
    if rule.is_required() && !action.is_trustee() {
        let post_scoped = matches!(
            action.scope(),
            SigningScope::Post | SigningScope::PostAndCountry
        );
        if post_scoped && capacity.posts.is_empty() {
            warnings.push(RuleWarning::NoPosts);
        } else if usize::from(rule.required()) > capacity.max {
            return Err(SigningError::invalid(
                InvalidReason::OverCapacity,
                format!(
                    "{} signatures is more than any Post can give: at most {}.",
                    rule.required(),
                    capacity.max
                ),
            ));
        }
        if !capacity.posts_short.is_empty() {
            warnings.push(RuleWarning::ShortPosts);
        }
        if !capacity.posts_short_without_requester.is_empty() {
            warnings.push(RuleWarning::RequesterExcluded);
        }
        short_posts = capacity.posts_short;
    }

    let current =
        get_signing_rule(hasura_transaction, tenant_id, election_event_id, action).await?;
    let current_revision = current.as_ref().map(|row| row.rule.revision).unwrap_or(0);
    if current_revision != input.expected_revision {
        return Err(SigningError::Conflict(format!(
            "The rule changed since it was read (revision {current_revision})."
        )));
    }

    let permission = action.sign_permission().to_string();
    if changes_roles {
        let details = json!({
            "action": action.to_string(),
            "permission": permission,
            "added": added.iter().map(|g| json!({"id": g.id, "name": g.name, "path": g.path})).collect::<Vec<_>>(),
            "removed": removed.iter().map(|g| json!({"id": g.id, "name": g.name, "path": g.path})).collect::<Vec<_>>(),
        });
        let description = format!(
            "Changed who can sign {}: added {}; removed {}",
            action_title(action).to_lowercase(),
            group_list(&added),
            group_list(&removed),
        );
        for event in &events {
            stage(
                hasura_transaction,
                &LogStep {
                    kind: SigningStatementKind::SigningPermissionChanged,
                    user: caller.actor(),
                    system: SystemOutcome::Info,
                    scope: log_scope(tenant_id, *event, None, None),
                    description: description.clone(),
                    details: details.clone(),
                },
            )
            .await?;
        }
    }

    let saved = upsert_signing_rule(
        hasura_transaction,
        tenant_id,
        election_event_id,
        &rule,
        input.expected_revision,
        &caller.user_id,
        Some(&caller.display_name),
    )
    .await?
    .ok_or_else(|| SigningError::Conflict("Another save came first.".into()))?;

    let mut cancelled = vec![];
    for waiting in
        list_waiting_signing_requests(hasura_transaction, tenant_id, election_event_id, action)
            .await?
    {
        let Some(locked) =
            lock_signing_request(hasura_transaction, tenant_id, election_event_id, waiting.id)
                .await?
        else {
            continue;
        };
        cancel_request(
            hasura_transaction,
            &locked,
            CancelReason::RuleChanged,
            caller.actor(),
            None,
        )
        .await?;
        cancelled.push(locked.id);
    }

    let old = current
        .map(|row| row.rule)
        .unwrap_or_else(|| SigningRule::default_for(action));
    stage(
        hasura_transaction,
        &LogStep {
            kind: SigningStatementKind::SigningRuleChanged,
            user: caller.actor(),
            system: SystemOutcome::Info,
            scope: log_scope(tenant_id, election_event_id, None, None),
            description: format!(
                "Changed the signing rule of {}: {}",
                action_title(action).to_lowercase(),
                requirement_text(&saved.rule)
            ),
            details: json!({
                "action": action.to_string(),
                "old": old,
                "new": saved.rule,
                "cancelled": cancelled,
            }),
        },
    )
    .await
    .context("Error logging the rule change")?;
    Ok(SavedRule {
        outcome: SaveRuleOutcome {
            revision: saved.rule.revision,
            rule: saved.rule,
            short_posts,
            warnings,
            cancelled,
        },
        roles: RoleChanges {
            realm,
            permission,
            add: change.add,
            remove: change.remove,
        },
    })
}

/// One role change sent to Keycloak.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sent<'a> {
    Granted(&'a str),
    Revoked(&'a str),
}

async fn send<'a>(
    roles: &dyn SigningRoleAdmin,
    changes: &'a RoleChanges,
    sent: &mut Vec<Sent<'a>>,
) -> Result<()> {
    for group_id in &changes.add {
        sent.push(Sent::Granted(group_id));
        roles
            .grant(&changes.realm, group_id, &changes.permission)
            .await?;
    }
    for group_id in &changes.remove {
        sent.push(Sent::Revoked(group_id));
        roles
            .revoke(&changes.realm, group_id, &changes.permission)
            .await?;
    }
    Ok(())
}

/// Puts back what was sent, newest first; a failure is logged.
async fn undo(roles: &dyn SigningRoleAdmin, changes: &RoleChanges, sent: &[Sent<'_>]) {
    for change in sent.iter().rev() {
        let result = match change {
            Sent::Granted(group_id) => {
                roles
                    .revoke(&changes.realm, group_id, &changes.permission)
                    .await
            }
            Sent::Revoked(group_id) => {
                roles
                    .grant(&changes.realm, group_id, &changes.permission)
                    .await
            }
        };
        if let Err(error) = result {
            warn!("Putting back a signing role change failed ({change:?}): {error:?}");
        }
    }
}

/// Makes a save's Keycloak changes, then commits its database changes.
/// When Keycloak fails or takes longer than [`ROLE_CHANGE_TIMEOUT`], or the
/// commit fails, what was sent to Keycloak is put back and nothing is
/// saved.
#[instrument(skip(hasura_transaction, roles), err)]
pub async fn commit_rule(
    hasura_transaction: Transaction<'_>,
    roles: &dyn SigningRoleAdmin,
    changes: &RoleChanges,
) -> SigningResult<()> {
    let mut sent = vec![];
    let applied = tokio::time::timeout(ROLE_CHANGE_TIMEOUT, send(roles, changes, &mut sent)).await;
    let failure = match applied {
        Ok(Ok(())) => None,
        Ok(Err(error)) => Some(error),
        Err(_) => Some(anyhow!("Keycloak didn't answer in time")),
    };
    if let Some(error) = failure {
        undo(roles, changes, &sent).await;
        return Err(SigningError::Internal(
            error.context("Error changing who can sign"),
        ));
    }
    if let Err(error) = hasura_transaction.commit().await {
        undo(roles, changes, &sent).await;
        return Err(SigningError::Internal(
            anyhow!(error).context("Error committing the rule"),
        ));
    }
    Ok(())
}
