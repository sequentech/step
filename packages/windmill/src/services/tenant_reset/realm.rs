// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The tenant realm's part of a reset: what the realm was before the first
//! configuration package (its texts, groups and users), and putting it back
//! in place.
//!
//! The realm itself is never recreated: windmill's own admin client, the
//! installer's accounts and the admin doing the reset live in it. Clients,
//! flows, identity providers and credentials are not touched; a user that
//! was deleted since is recreated with its id and no credentials.

use crate::tasks::migrate_realm_permissions::grant_template_roles;
use anyhow::{anyhow, Context, Result};
use keycloak::KeycloakTokenSupplier;
use reqwest::Method;
use sequent_core::services::keycloak::{parse_realm, PubKeycloakAdmin};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use tracing::{info, instrument, warn};

const PAGE: usize = 100;
const SERVICE_ACCOUNT_PREFIX: &str = "service-account-";

/// The user fields a reset puts back.
const USER_FIELDS: [&str; 8] = [
    "username",
    "enabled",
    "email",
    "emailVerified",
    "firstName",
    "lastName",
    "attributes",
    "requiredActions",
];

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaselineGroup {
    pub path: String,
    pub realm_roles: Vec<String>,
    /// Client roles by clientId. None in a baseline recorded before they
    /// were: the group's client roles are then left as they are.
    #[serde(default)]
    pub client_roles: Option<BTreeMap<String, Vec<String>>>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaselineUser {
    pub id: String,
    /// The `USER_FIELDS` of the user's representation.
    pub fields: Map<String, Value>,
    pub groups: Vec<String>,
    pub realm_roles: Vec<String>,
    /// Client roles by clientId.
    pub client_roles: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RealmBaseline {
    pub display_name: Option<String>,
    pub display_name_html: Option<String>,
    /// Realm texts by locale.
    pub localization: BTreeMap<String, BTreeMap<String, String>>,
    pub groups: Vec<BaselineGroup>,
    pub users: Vec<BaselineUser>,
}

/// What a realm restore did, for the task's result and the audit entry.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RealmRestoreSummary {
    pub users_deleted: Vec<String>,
    pub users_restored: usize,
    pub users_recreated: Vec<String>,
    pub groups_deleted: Vec<String>,
    pub groups_created: Vec<String>,
    pub sessions_ended: usize,
}

struct Admin<'a> {
    client: &'a PubKeycloakAdmin,
    realm: &'a str,
}

impl Admin<'_> {
    async fn send(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, String)],
        body: Option<&Value>,
    ) -> Result<reqwest::Response> {
        let url = if path.is_empty() {
            format!("{}/admin/realms/{}", self.client.url, self.realm)
        } else {
            format!("{}/admin/realms/{}/{path}", self.client.url, self.realm)
        };
        let mut request = self
            .client
            .client
            .request(method.clone(), &url)
            .query(query)
            .bearer_auth(self.client.token_supplier.get(&self.client.url).await?);
        if let Some(body) = body {
            request = request.json(body);
        }
        let response = request.send().await?;
        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow!("{method} {path}: HTTP {status} {text}"));
        }
        Ok(response)
    }

    async fn get(&self, path: &str, query: &[(&str, String)]) -> Result<Value> {
        let response = self.send(Method::GET, path, query, None).await?;
        let text = response.text().await?;
        if text.is_empty() {
            return Ok(Value::Null);
        }
        Ok(serde_json::from_str(&text)?)
    }

    async fn call(&self, method: Method, path: &str, body: Option<&Value>) -> Result<()> {
        self.send(method, path, &[], body).await.map(|_| ())
    }

    async fn users(&self) -> Result<Vec<Value>> {
        let mut users = vec![];
        loop {
            let page = self
                .get(
                    "users",
                    &[
                        ("first", users.len().to_string()),
                        ("max", PAGE.to_string()),
                        ("briefRepresentation", "false".into()),
                    ],
                )
                .await?;
            let page = page.as_array().cloned().unwrap_or_default();
            let done = page.len() < PAGE;
            users.extend(page);
            if done {
                return Ok(users);
            }
        }
    }

    /// Every item of a paged listing.
    async fn paged(&self, path: &str) -> Result<Vec<Value>> {
        let mut items = vec![];
        loop {
            let page = self
                .get(
                    path,
                    &[
                        ("briefRepresentation", "true".into()),
                        ("first", items.len().to_string()),
                        ("max", PAGE.to_string()),
                    ],
                )
                .await?;
            let page = page.as_array().cloned().unwrap_or_default();
            let done = page.len() < PAGE;
            items.extend(page);
            if done {
                return Ok(items);
            }
        }
    }

    /// Every group, subgroups included, by path, with its id.
    async fn groups(&self) -> Result<BTreeMap<String, String>> {
        let mut found = BTreeMap::new();
        let mut pending: Vec<Value> = self.paged("groups").await?;
        while let Some(group) = pending.pop() {
            let (Some(id), Some(path)) = (str_field(&group, "id"), str_field(&group, "path"))
            else {
                continue;
            };
            found.insert(path.clone(), id.clone());
            let inline: Vec<Value> = group
                .get("subGroups")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let has_children = group
                .get("subGroupCount")
                .and_then(Value::as_u64)
                .map(|count| count > 0)
                .unwrap_or(false);
            // Inline subgroups may be a partial list: read them all.
            if has_children || !inline.is_empty() {
                pending.extend(self.paged(&format!("groups/{id}/children")).await?);
            }
        }
        Ok(found)
    }

    async fn role_names(&self, path: &str) -> Result<Vec<String>> {
        let mut names: Vec<String> = self
            .get(path, &[])
            .await?
            .as_array()
            .map(|roles| roles.iter().filter_map(|r| str_field(r, "name")).collect())
            .unwrap_or_default();
        names.sort();
        Ok(names)
    }

    /// Client roles mapped directly to `base` (`users/{id}` or
    /// `groups/{id}`), by clientId.
    async fn client_roles(&self, base: &str) -> Result<BTreeMap<String, Vec<String>>> {
        let mappings = self.get(&format!("{base}/role-mappings"), &[]).await?;
        let mut roles = BTreeMap::new();
        if let Some(clients) = mappings.get("clientMappings").and_then(Value::as_object) {
            for (client_id, mapping) in clients {
                let mut names: Vec<String> = mapping
                    .get("mappings")
                    .and_then(Value::as_array)
                    .map(|r| r.iter().filter_map(|r| str_field(r, "name")).collect())
                    .unwrap_or_default();
                names.sort();
                if !names.is_empty() {
                    roles.insert(client_id.clone(), names);
                }
            }
        }
        Ok(roles)
    }

    async fn realm_role(&self, name: &str) -> Result<Option<Value>> {
        match self.get(&format!("roles/{}", urlencode(name)), &[]).await {
            Ok(role) => Ok(Some(json!({"id": role["id"], "name": role["name"]}))),
            Err(error) if error.to_string().contains("HTTP 404") => Ok(None),
            Err(error) => Err(error),
        }
    }

    async fn client_uuid(&self, client_id: &str) -> Result<Option<String>> {
        let found = self
            .get("clients", &[("clientId", client_id.to_string())])
            .await?;
        Ok(found
            .as_array()
            .and_then(|clients| clients.first())
            .and_then(|client| str_field(client, "id")))
    }

    async fn client_role(&self, client: &str, name: &str) -> Result<Option<Value>> {
        match self
            .get(&format!("clients/{client}/roles/{}", urlencode(name)), &[])
            .await
        {
            Ok(role) => Ok(Some(json!({"id": role["id"], "name": role["name"]}))),
            Err(error) if error.to_string().contains("HTTP 404") => Ok(None),
            Err(error) => Err(error),
        }
    }

    /// Makes the realm roles mapped at `path` (a user's or a group's
    /// `role-mappings/realm`) exactly `wanted`, skipping roles the realm no
    /// longer has.
    async fn set_realm_roles(&self, path: &str, wanted: &[String]) -> Result<()> {
        let current = self.role_names(path).await?;
        let (add, remove) = difference(&current, wanted);
        let mut adding = vec![];
        for name in add {
            match self.realm_role(&name).await? {
                Some(role) => adding.push(role),
                None => warn!(realm = self.realm, role = %name, "Recorded role no longer exists"),
            }
        }
        if !adding.is_empty() {
            self.call(Method::POST, path, Some(&Value::Array(adding)))
                .await?;
        }
        let mut removing = vec![];
        for name in remove {
            if let Some(role) = self.realm_role(&name).await? {
                removing.push(role);
            }
        }
        if !removing.is_empty() {
            self.call(Method::DELETE, path, Some(&Value::Array(removing)))
                .await?;
        }
        Ok(())
    }

    /// Makes the client roles mapped to `base` (`users/{id}` or
    /// `groups/{id}`) exactly `wanted`, skipping clients and roles the
    /// realm no longer has.
    async fn set_client_roles(
        &self,
        base: &str,
        wanted: &BTreeMap<String, Vec<String>>,
    ) -> Result<()> {
        let current = self.client_roles(base).await?;
        let client_ids: BTreeSet<&String> = current.keys().chain(wanted.keys()).collect();
        for client_id in client_ids {
            let empty = vec![];
            let (add, remove) = difference(
                current.get(client_id).unwrap_or(&empty),
                wanted.get(client_id).unwrap_or(&empty),
            );
            if add.is_empty() && remove.is_empty() {
                continue;
            }
            let Some(client) = self.client_uuid(client_id).await? else {
                warn!(realm = self.realm, client = %client_id, "Recorded client no longer exists");
                continue;
            };
            let path = format!("{base}/role-mappings/clients/{client}");
            for (method, names) in [(Method::POST, add), (Method::DELETE, remove)] {
                let mut roles = vec![];
                for name in names {
                    if let Some(role) = self.client_role(&client, &name).await? {
                        roles.push(role);
                    }
                }
                if !roles.is_empty() {
                    self.call(method, &path, Some(&Value::Array(roles))).await?;
                }
            }
        }
        Ok(())
    }
}

fn str_field(value: &Value, field: &str) -> Option<String> {
    value.get(field).and_then(Value::as_str).map(str::to_string)
}

/// Percent-encodes a role name for a path segment.
fn urlencode(segment: &str) -> String {
    segment
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// What to add to `current` and remove from it to get `wanted`.
pub fn difference(current: &[String], wanted: &[String]) -> (Vec<String>, Vec<String>) {
    let current: BTreeSet<&String> = current.iter().collect();
    let wanted: BTreeSet<&String> = wanted.iter().collect();
    (
        wanted.difference(&current).map(|s| s.to_string()).collect(),
        current.difference(&wanted).map(|s| s.to_string()).collect(),
    )
}

/// Groups to delete: those not recorded, without the ones inside a group
/// that is deleted anyway.
pub fn groups_to_delete(current: &[String], recorded: &[String]) -> Vec<String> {
    let recorded: BTreeSet<&String> = recorded.iter().collect();
    let mut gone: Vec<String> = vec![];
    let mut sorted: Vec<&String> = current.iter().collect();
    sorted.sort();
    for path in sorted {
        if recorded.contains(path) {
            continue;
        }
        if gone
            .iter()
            .any(|parent| path.starts_with(&format!("{parent}/")))
        {
            continue;
        }
        gone.push(path.clone());
    }
    gone
}

/// Recorded groups missing now, parents before their subgroups.
pub fn groups_to_create(current: &[String], recorded: &[String]) -> Vec<String> {
    let current: BTreeSet<&String> = current.iter().collect();
    let mut missing: Vec<String> = recorded
        .iter()
        .filter(|path| !current.contains(path))
        .cloned()
        .collect();
    missing.sort_by_key(|path| (path.matches('/').count(), path.clone()));
    missing
}

/// Who a reset deletes from the realm: every user not recorded (or, with no
/// record, created after `created_after_ms`), except the admin doing it and
/// service accounts.
pub fn users_to_delete(
    current: &[Value],
    recorded: Option<&BTreeSet<String>>,
    keep_user: &str,
    created_after_ms: Option<i64>,
) -> Vec<(String, String)> {
    current
        .iter()
        .filter_map(|user| {
            let id = str_field(user, "id")?;
            let username = str_field(user, "username").unwrap_or_default();
            if id == keep_user || username.starts_with(SERVICE_ACCOUNT_PREFIX) {
                return None;
            }
            let delete = match recorded {
                Some(recorded) => !recorded.contains(&id),
                None => match (created_after_ms, user.get("createdTimestamp")) {
                    (Some(after), Some(created)) => created.as_i64().unwrap_or(0) >= after,
                    _ => false,
                },
            };
            delete.then_some((id, username))
        })
        .collect()
}

fn email_of(fields: &Map<String, Value>) -> Option<String> {
    fields
        .get("email")
        .and_then(Value::as_str)
        .filter(|e| !e.is_empty())
        .map(str::to_lowercase)
}

/// Users other than the recorded owner that hold a recorded user's email
/// now, as (id, email). Fails when it is `keep_user`, whose account a reset
/// doesn't change.
pub fn email_conflicts(
    current: &[Value],
    recorded: &[BaselineUser],
    keep_user: &str,
) -> Result<Vec<(String, String)>> {
    let owners: BTreeMap<String, &str> = recorded
        .iter()
        .filter(|u| u.id != keep_user)
        .filter_map(|u| email_of(&u.fields).map(|e| (e, u.id.as_str())))
        .collect();
    let mut conflicts = vec![];
    for user in current {
        let (Some(id), Some(email)) = (str_field(user, "id"), user.as_object().and_then(email_of))
        else {
            continue;
        };
        match owners.get(&email) {
            Some(owner) if *owner != id => {
                if id == keep_user {
                    return Err(anyhow!(
                        "Your account uses the email {email}, which belonged to another user before the first import: change it, or have another admin reset the tenant"
                    ));
                }
                conflicts.push((id, email));
            }
            _ => {}
        }
    }
    Ok(conflicts)
}

/// Groups the reset deletes that `keep_user` belongs to, or is inside of:
/// the access they give the admin doing the reset would be lost.
pub fn caller_groups_deleted(
    current: &[String],
    recorded: &[String],
    caller_groups: &[String],
) -> Vec<String> {
    let deleted = groups_to_delete(current, recorded);
    let mut lost: Vec<String> = caller_groups
        .iter()
        .filter(|path| {
            deleted
                .iter()
                .any(|gone| *path == gone || path.starts_with(&format!("{gone}/")))
        })
        .cloned()
        .collect();
    lost.sort();
    lost
}

/// Checks, before anything is changed, that restoring `baseline` keeps the
/// account and access of `keep_user`, the admin doing the reset.
#[instrument(skip(client, baseline), err)]
pub async fn check_caller(
    client: &PubKeycloakAdmin,
    realm: &str,
    baseline: &RealmBaseline,
    keep_user: &str,
) -> Result<()> {
    let admin = Admin { client, realm };
    let current: Vec<String> = admin.groups().await?.into_keys().collect();
    let recorded: Vec<String> = baseline.groups.iter().map(|g| g.path.clone()).collect();
    let caller: Vec<String> = admin
        .paged(&format!("users/{keep_user}/groups"))
        .await?
        .iter()
        .filter_map(|g| str_field(g, "path"))
        .collect();
    let lost = caller_groups_deleted(&current, &recorded, &caller);
    if !lost.is_empty() {
        return Err(anyhow!(
            "Your account gets access from groups the reset deletes ({}): have an admin whose access comes from groups that existed before the first import reset the tenant",
            lost.join(", ")
        ));
    }
    let me = admin.get(&format!("users/{keep_user}"), &[]).await?;
    email_conflicts(&[me], &baseline.users, keep_user)?;
    Ok(())
}

fn user_fields(user: &Value) -> Map<String, Value> {
    USER_FIELDS
        .iter()
        .filter_map(|field| user.get(*field).map(|v| (field.to_string(), v.clone())))
        .collect()
}

/// Reads what the realm is now.
#[instrument(skip(client), err)]
pub async fn capture_realm(client: &PubKeycloakAdmin, realm: &str) -> Result<RealmBaseline> {
    let admin = Admin { client, realm };
    let representation = admin.get("", &[]).await?;
    let mut localization = BTreeMap::new();
    for locale in admin
        .get("localization", &[])
        .await?
        .as_array()
        .cloned()
        .unwrap_or_default()
    {
        let Some(locale) = locale.as_str() else {
            continue;
        };
        let texts: BTreeMap<String, String> =
            serde_json::from_value(admin.get(&format!("localization/{locale}"), &[]).await?)
                .unwrap_or_default();
        localization.insert(locale.to_string(), texts);
    }

    let mut groups = vec![];
    for (path, id) in admin.groups().await? {
        groups.push(BaselineGroup {
            path,
            realm_roles: admin
                .role_names(&format!("groups/{id}/role-mappings/realm"))
                .await?,
            client_roles: Some(admin.client_roles(&format!("groups/{id}")).await?),
        });
    }

    let mut users = vec![];
    for user in admin.users().await? {
        let Some(id) = str_field(&user, "id") else {
            continue;
        };
        let mut user_groups: Vec<String> = admin
            .paged(&format!("users/{id}/groups"))
            .await?
            .iter()
            .filter_map(|g| str_field(g, "path"))
            .collect();
        user_groups.sort();
        users.push(BaselineUser {
            fields: user_fields(&user),
            groups: user_groups,
            realm_roles: admin
                .role_names(&format!("users/{id}/role-mappings/realm"))
                .await?,
            client_roles: admin.client_roles(&format!("users/{id}")).await?,
            id,
        });
    }

    Ok(RealmBaseline {
        display_name: str_field(&representation, "displayName"),
        display_name_html: str_field(&representation, "displayNameHtml"),
        localization,
        groups,
        users,
    })
}

/// Puts the realm back as `baseline` recorded it, keeping `keep_user` (the
/// admin doing the reset) as it is. Without a baseline, only the users
/// created after `created_after_ms` are deleted. Idempotent.
#[instrument(skip(client, baseline), err)]
pub async fn restore_realm(
    client: &PubKeycloakAdmin,
    realm: &str,
    baseline: Option<&RealmBaseline>,
    keep_user: &str,
    created_after_ms: Option<i64>,
) -> Result<RealmRestoreSummary> {
    let admin = Admin { client, realm };
    let mut summary = RealmRestoreSummary::default();

    // Users first: deleting them also drops their group memberships.
    let current_users = admin.users().await?;
    let recorded_ids: Option<BTreeSet<String>> =
        baseline.map(|b| b.users.iter().map(|u| u.id.clone()).collect());
    for (id, username) in users_to_delete(
        &current_users,
        recorded_ids.as_ref(),
        keep_user,
        created_after_ms,
    ) {
        admin
            .call(Method::DELETE, &format!("users/{id}"), None)
            .await
            .with_context(|| format!("Error deleting user {username}"))?;
        summary.users_deleted.push(username);
    }

    let Some(baseline) = baseline else {
        summary.sessions_ended = end_sessions(&admin, &current_users, keep_user).await?;
        return Ok(summary);
    };

    // Groups.
    let current_groups: Vec<String> = admin.groups().await?.into_keys().collect();
    let recorded_groups: Vec<String> = baseline.groups.iter().map(|g| g.path.clone()).collect();
    let ids = admin.groups().await?;
    for path in groups_to_delete(&current_groups, &recorded_groups) {
        if let Some(id) = ids.get(&path) {
            admin
                .call(Method::DELETE, &format!("groups/{id}"), None)
                .await?;
            summary.groups_deleted.push(path);
        }
    }
    for path in groups_to_create(&current_groups, &recorded_groups) {
        let (parent, name) = path.rsplit_once('/').unwrap_or(("", &path));
        let body = json!({ "name": name });
        if parent.is_empty() {
            match admin.call(Method::POST, "groups", Some(&body)).await {
                Ok(()) => {}
                // Created by someone else since the listing: it is there.
                Err(error)
                    if error.to_string().contains("HTTP 409")
                        && admin.groups().await?.contains_key(&path) =>
                {
                    continue;
                }
                Err(error) => return Err(error),
            }
        } else {
            let ids = admin.groups().await?;
            let parent_id = ids
                .get(parent)
                .ok_or_else(|| anyhow!("parent group {parent} of {path} is missing"))?;
            admin
                .call(
                    Method::POST,
                    &format!("groups/{parent_id}/children"),
                    Some(&body),
                )
                .await?;
        }
        summary.groups_created.push(path);
    }
    let ids = admin.groups().await?;
    for group in &baseline.groups {
        if let Some(id) = ids.get(&group.path) {
            admin
                .set_realm_roles(
                    &format!("groups/{id}/role-mappings/realm"),
                    &group.realm_roles,
                )
                .await?;
            if let Some(client_roles) = &group.client_roles {
                admin
                    .set_client_roles(&format!("groups/{id}"), client_roles)
                    .await?;
            }
        }
    }
    // Roles a release added to the template groups after the record.
    grant_template_roles(client, realm).await?;

    // Recorded users.
    let current_ids: BTreeSet<String> = admin
        .users()
        .await?
        .iter()
        .filter_map(|u| str_field(u, "id"))
        .collect();
    // Emails are unique: free every recorded email another user holds now,
    // so that recreating and restoring the recorded users can't collide,
    // even when two of them swapped addresses. Each surviving recorded user
    // gets its own back below.
    for (id, email) in email_conflicts(&admin.users().await?, &baseline.users, keep_user)? {
        admin
            .call(
                Method::PUT,
                &format!("users/{id}"),
                Some(&json!({"email": ""})),
            )
            .await
            .with_context(|| format!("Error freeing the email {email}"))?;
    }
    let missing: Vec<&BaselineUser> = baseline
        .users
        .iter()
        .filter(|u| u.id != keep_user && !current_ids.contains(&u.id))
        .collect();
    if !missing.is_empty() {
        let users: Vec<Value> = missing
            .iter()
            .map(|u| {
                let mut user = u.fields.clone();
                user.insert("id".into(), json!(u.id));
                Value::Object(user)
            })
            .collect();
        admin
            .call(
                Method::POST,
                "partialImport",
                Some(&json!({"ifResourceExists": "SKIP", "users": users})),
            )
            .await
            .context("Error recreating recorded users")?;
        // A skipped user is not reported as an error: check each one.
        let now: BTreeSet<String> = admin
            .users()
            .await?
            .iter()
            .filter_map(|u| str_field(u, "id"))
            .collect();
        let skipped: Vec<String> = missing
            .iter()
            .filter(|u| !now.contains(&u.id))
            .map(|u| {
                format!(
                    "{} ({})",
                    str_field(&Value::Object(u.fields.clone()), "username").unwrap_or_default(),
                    str_field(&Value::Object(u.fields.clone()), "email").unwrap_or_default()
                )
            })
            .collect();
        if !skipped.is_empty() {
            return Err(anyhow!(
                "Keycloak did not recreate the recorded users {}: another user holds their username or email",
                skipped.join(", ")
            ));
        }
        summary.users_recreated = missing
            .iter()
            .filter_map(|u| u.fields.get("username").and_then(Value::as_str))
            .map(str::to_string)
            .collect();
    }
    for user in baseline.users.iter().filter(|u| u.id != keep_user) {
        let id = &user.id;
        admin
            .call(
                Method::PUT,
                &format!("users/{id}"),
                Some(&Value::Object(user.fields.clone())),
            )
            .await
            .with_context(|| format!("Error restoring user {id}"))?;
        let current: Vec<String> = admin
            .paged(&format!("users/{id}/groups"))
            .await?
            .iter()
            .filter_map(|g| str_field(g, "path"))
            .collect();
        let (join, leave) = difference(&current, &user.groups);
        for (method, paths) in [(Method::PUT, join), (Method::DELETE, leave)] {
            for path in paths {
                if let Some(group) = ids.get(&path) {
                    admin
                        .call(method.clone(), &format!("users/{id}/groups/{group}"), None)
                        .await?;
                }
            }
        }
        admin
            .set_realm_roles(
                &format!("users/{id}/role-mappings/realm"),
                &user.realm_roles,
            )
            .await?;
        admin
            .set_client_roles(&format!("users/{id}"), &user.client_roles)
            .await?;
        summary.users_restored += 1;
    }

    // Texts.
    admin
        .call(
            Method::PUT,
            "",
            Some(&json!({
                "displayName": baseline.display_name.clone().unwrap_or_default(),
                "displayNameHtml": baseline.display_name_html.clone().unwrap_or_default(),
            })),
        )
        .await
        .context("Error restoring the realm display name")?;
    for locale in admin
        .get("localization", &[])
        .await?
        .as_array()
        .cloned()
        .unwrap_or_default()
    {
        if let Some(locale) = locale.as_str() {
            admin
                .call(Method::DELETE, &format!("localization/{locale}"), None)
                .await?;
        }
    }
    for (locale, texts) in &baseline.localization {
        if texts.is_empty() {
            continue;
        }
        admin
            .call(
                Method::POST,
                &format!("localization/{locale}"),
                Some(&serde_json::to_value(texts)?),
            )
            .await
            .with_context(|| format!("Error restoring the {locale} texts"))?;
    }

    summary.sessions_ended = end_sessions(&admin, &admin.users().await?, keep_user).await?;
    info!(realm, ?summary, "Tenant realm restored");
    Ok(summary)
}

/// Signs out every user but `keep_user` and service accounts.
async fn end_sessions(admin: &Admin<'_>, users: &[Value], keep_user: &str) -> Result<usize> {
    let mut ended = 0;
    for user in users {
        let (Some(id), username) = (str_field(user, "id"), str_field(user, "username")) else {
            continue;
        };
        if id == keep_user
            || username
                .as_deref()
                .is_some_and(|u| u.starts_with(SERVICE_ACCOUNT_PREFIX))
        {
            continue;
        }
        admin
            .call(Method::POST, &format!("users/{id}/logout"), None)
            .await?;
        ended += 1;
    }
    Ok(ended)
}

/// The tenant's election event realms that exist in Keycloak.
pub async fn event_realms(client: &PubKeycloakAdmin, tenant_id: &str) -> Result<Vec<String>> {
    let response = client
        .client
        .get(format!("{}/admin/realms", client.url))
        .query(&[("briefRepresentation", "true")])
        .bearer_auth(client.token_supplier.get(&client.url).await?)
        .send()
        .await?;
    let status = response.status();
    if !status.is_success() {
        return Err(anyhow!("GET realms: HTTP {status}"));
    }
    let realms: Vec<Value> = response.json().await?;
    Ok(realms
        .iter()
        .filter_map(|r| str_field(r, "realm"))
        .filter(|name| matches!(parse_realm(name), Some((tenant, Some(_))) if tenant == tenant_id))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn differences_add_and_remove() {
        let (add, remove) = difference(&s(&["a", "b"]), &s(&["b", "c"]));
        assert_eq!((add, remove), (s(&["c"]), s(&["a"])));
    }

    #[test]
    fn only_the_outermost_new_group_is_deleted() {
        let current = s(&["/admin", "/trustees", "/trustees/a", "/admin/new", "/new2"]);
        let recorded = s(&["/admin"]);
        assert_eq!(
            groups_to_delete(&current, &recorded),
            s(&["/admin/new", "/new2", "/trustees"])
        );
    }

    #[test]
    fn parents_are_created_before_subgroups() {
        let recorded = s(&["/a/b/c", "/a", "/a/b", "/z"]);
        assert_eq!(
            groups_to_create(&s(&["/z"]), &recorded),
            s(&["/a", "/a/b", "/a/b/c"])
        );
    }

    #[test]
    fn the_caller_and_service_accounts_stay() {
        let users = vec![
            json!({"id": "caller", "username": "ada", "createdTimestamp": 50}),
            json!({"id": "svc", "username": "service-account-windmill", "createdTimestamp": 50}),
            json!({"id": "old", "username": "installer", "createdTimestamp": 5}),
            json!({"id": "new", "username": "imported", "createdTimestamp": 50}),
        ];
        let recorded: BTreeSet<String> = ["old".to_string()].into();
        assert_eq!(
            users_to_delete(&users, Some(&recorded), "caller", None),
            vec![("new".to_string(), "imported".to_string())]
        );
        // Without a record, by creation time.
        assert_eq!(
            users_to_delete(&users, None, "caller", Some(10)),
            vec![("new".to_string(), "imported".to_string())]
        );
        assert!(users_to_delete(&users, None, "caller", None).is_empty());
    }

    fn recorded(id: &str, email: &str) -> BaselineUser {
        BaselineUser {
            id: id.into(),
            fields: json!({"username": id, "email": email})
                .as_object()
                .cloned()
                .unwrap(),
            ..Default::default()
        }
    }

    #[test]
    fn swapped_and_taken_emails_are_freed_first() {
        let recorded = [recorded("a", "a@x"), recorded("b", "b@x")];
        // a and b swapped; c took a's address after it was deleted.
        let current = [
            json!({"id": "a", "email": "B@x"}),
            json!({"id": "b", "email": "a@x"}),
            json!({"id": "c", "email": "c@x"}),
        ];
        assert_eq!(
            email_conflicts(&current, &recorded, "caller").unwrap(),
            vec![
                ("a".to_string(), "b@x".to_string()),
                ("b".to_string(), "a@x".to_string())
            ]
        );
        let taken = [json!({"id": "c", "email": "a@x"})];
        assert_eq!(
            email_conflicts(&taken, &recorded, "caller").unwrap(),
            vec![("c".to_string(), "a@x".to_string())]
        );
        // The caller's account is never changed: refused instead.
        let caller = [json!({"id": "caller", "email": "a@x"})];
        assert!(email_conflicts(&caller, &recorded, "caller").is_err());
    }

    #[test]
    fn the_callers_groups_must_survive() {
        let current = s(&["/admin", "/new", "/new/sub"]);
        let recorded = s(&["/admin"]);
        assert_eq!(
            caller_groups_deleted(&current, &recorded, &s(&["/admin", "/new/sub"])),
            s(&["/new/sub"])
        );
        assert!(caller_groups_deleted(&current, &recorded, &s(&["/admin"])).is_empty());
    }

    #[test]
    fn older_baselines_leave_group_client_roles_alone() {
        let group: BaselineGroup =
            serde_json::from_value(json!({"path": "/admin", "realmRoles": []})).unwrap();
        assert_eq!(group.client_roles, None);
    }
}
