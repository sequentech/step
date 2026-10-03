// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Who can sign an action: the users of the tenant realm holding the realm
//! role `sign-<action>`, read from the Keycloak database like the user
//! lists in `services::users`. A user holds it when it is mapped to them, to
//! one of their groups or a parent group, or to a composite role one of
//! those holds. Service accounts and disabled users are not signers. The
//! approve step checks the token's roles instead; this list is what the
//! panel shows and what a rule is checked against.
//!
//! A signer's Posts are the labels their token carries: the
//! `HasuraMultivaluedUserAttributeMapper` maps the `permission_labels`
//! attribute without aggregating, which Keycloak resolves as the user's own
//! values, else those of the first of their groups (by name) that has the
//! attribute, itself or through a parent group.

use anyhow::{Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::signing::SigningAction;
use std::collections::BTreeMap;
use strum::IntoEnumIterator;
use tracing::instrument;

/// The user attribute holding a person's Posts, and the group attribute
/// giving them to its members.
pub const PERMISSION_LABELS_ATTRIBUTE: &str = "permission_labels";
/// The optional user attribute with a signer's title ("Chairperson").
pub const TITLE_ATTRIBUTE: &str = "title";

/// Whether a person with `labels` signs for a Post labelled `post_label`:
/// for a labelled Post, one of their labels is its label; an unlabelled
/// Post is everybody's. These are the Posts Hasura shows them, so a person
/// without labels signs for no labelled Post.
pub fn signs_for(labels: &[String], post_label: Option<&str>) -> bool {
    match post_label {
        None => true,
        Some(label) => labels.iter().any(|own| own == label),
    }
}

/// A person who holds an action's sign permission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signer {
    pub user_id: String,
    pub username: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    /// The `title` attribute, else the name of a group that grants the
    /// permission.
    pub title: Option<String>,
    /// The labels their token carries, sorted.
    pub labels: Vec<String>,
    /// The groups that grant the permission, by name.
    pub groups: Vec<String>,
}

impl Signer {
    /// "First Last", or `None` without either.
    pub fn name(&self) -> Option<String> {
        let parts: Vec<&str> = [&self.first_name, &self.last_name]
            .into_iter()
            .flatten()
            .map(|part| part.trim())
            .filter(|part| !part.is_empty())
            .collect();
        (!parts.is_empty()).then(|| parts.join(" "))
    }

    /// Whether they can sign for a Post with `post_label` ([`signs_for`]).
    pub fn eligible_for(&self, post_label: Option<&str>) -> bool {
        signs_for(&self.labels, post_label)
    }
}

/// Groups (by id) that gain or lose a sign permission: signers are counted
/// under the change before it is made.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GroupChange {
    pub add: Vec<String>,
    pub remove: Vec<String>,
}

/// A group of the realm, and how it holds an action's sign permission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SigningGroup {
    pub id: String,
    pub name: String,
    /// `/parent/child`, as Keycloak shows it.
    pub path: String,
    /// The sign role is mapped to the group itself.
    pub direct: bool,
    /// A composite role mapped to the group holds the sign role.
    pub composite: bool,
}

/// The realm, its sign role, the composite roles holding it, and the
/// groups granting it (with their subgroups, which inherit it). `$1` realm
/// name, `$2` role name, `$3` group ids added, `$4` group ids whose direct
/// mapping is removed, `$5` the labels attribute.
const GRANTS: &str = r#"
    WITH RECURSIVE
    target_realm AS (
        SELECT id FROM realm WHERE name = $1
    ),
    sign_role AS (
        SELECT r.id FROM keycloak_role r
        JOIN target_realm ON r.realm_id = target_realm.id
        WHERE r.name = $2 AND NOT r.client_role
    ),
    sign_roles AS (
        SELECT id FROM sign_role
        UNION
        SELECT c.composite FROM composite_role c
        JOIN sign_roles s ON c.child_role = s.id
    ),
    granting_groups AS (
        SELECT g.id FROM keycloak_group g
        JOIN target_realm ON g.realm_id = target_realm.id
        JOIN group_role_mapping m ON m.group_id = g.id
        WHERE m.role_id IN (SELECT id FROM sign_role) AND NOT (g.id = ANY($4))
        UNION
        SELECT g.id FROM keycloak_group g
        JOIN target_realm ON g.realm_id = target_realm.id
        JOIN group_role_mapping m ON m.group_id = g.id
        WHERE m.role_id IN (SELECT id FROM sign_roles)
            AND m.role_id NOT IN (SELECT id FROM sign_role)
        UNION
        SELECT g.id FROM keycloak_group g
        JOIN target_realm ON g.realm_id = target_realm.id
        WHERE g.id = ANY($3)
    ),
    inheriting_groups AS (
        SELECT id, id AS granted_by FROM granting_groups
        UNION
        SELECT child.id, parent.granted_by FROM keycloak_group child
        JOIN inheriting_groups parent ON child.parent_group = parent.id
    ),
    grants AS (
        SELECT u.id AS user_id, NULL::varchar AS group_id FROM user_entity u
        JOIN target_realm ON u.realm_id = target_realm.id
        JOIN user_role_mapping urm ON urm.user_id = u.id
        WHERE urm.role_id IN (SELECT id FROM sign_roles)
        UNION
        SELECT u.id, ig.granted_by FROM user_entity u
        JOIN target_realm ON u.realm_id = target_realm.id
        JOIN user_group_membership ugm ON ugm.user_id = u.id
        JOIN inheriting_groups ig ON ig.id = ugm.group_id
    ),
    -- Each group with its ancestors, nearest first, up to the first that
    -- has labels.
    group_labels AS (
        SELECT g.id AS group_id, g.id AS holder, g.parent_group, 0 AS depth
        FROM keycloak_group g JOIN target_realm ON g.realm_id = target_realm.id
        UNION ALL
        SELECT gl.group_id, p.id, p.parent_group, gl.depth + 1
        FROM group_labels gl JOIN keycloak_group p ON p.id = gl.parent_group
        WHERE NOT EXISTS (
            SELECT 1 FROM group_attribute ga
            WHERE ga.group_id = gl.holder AND ga.name = $5 AND ga.value IS NOT NULL
        )
    )
"#;

/// The enabled users of `realm`, not service accounts, who hold `action`'s
/// sign permission, by username, as they would after `change`.
#[instrument(skip(keycloak_transaction), err)]
pub async fn list_signers(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    action: SigningAction,
    change: &GroupChange,
) -> Result<Vec<Signer>> {
    let sql = format!(
        r#"{GRANTS}
        SELECT
            u.id,
            u.username,
            u.first_name,
            u.last_name,
            ARRAY(
                SELECT ua.value FROM user_attribute ua
                WHERE ua.user_id = u.id AND ua.name = $5 AND ua.value IS NOT NULL
                ORDER BY 1
            ) AS own_labels,
            ARRAY(
                SELECT ga.value FROM (
                    SELECT gl.holder FROM user_group_membership ugm
                    JOIN keycloak_group g ON g.id = ugm.group_id
                    JOIN group_labels gl ON gl.group_id = g.id
                    WHERE ugm.user_id = u.id AND EXISTS (
                        SELECT 1 FROM group_attribute a
                        WHERE a.group_id = gl.holder AND a.name = $5 AND a.value IS NOT NULL
                    )
                    ORDER BY g.name, g.id
                    LIMIT 1
                ) first_group
                JOIN group_attribute ga ON ga.group_id = first_group.holder
                WHERE ga.name = $5 AND ga.value IS NOT NULL
                ORDER BY 1
            ) AS group_labels,
            (
                SELECT ua.value FROM user_attribute ua
                WHERE ua.user_id = u.id AND ua.name = $6 AND ua.value <> ''
                ORDER BY ua.value LIMIT 1
            ) AS title,
            ARRAY(
                SELECT DISTINCT g.name FROM grants gr
                JOIN keycloak_group g ON g.id = gr.group_id
                WHERE gr.user_id = u.id
                ORDER BY g.name
            ) AS groups
        FROM user_entity u
        WHERE u.id IN (SELECT user_id FROM grants)
            AND u.enabled
            AND u.service_account_client_link IS NULL
        ORDER BY u.username, u.id"#
    );
    let rows = keycloak_transaction
        .query(
            sql.as_str(),
            &[
                &realm,
                &action.sign_permission().to_string(),
                &change.add,
                &change.remove,
                &PERMISSION_LABELS_ATTRIBUTE,
                &TITLE_ATTRIBUTE,
            ],
        )
        .await
        .context("Error listing the signers")?;
    rows.into_iter()
        .map(|row| {
            let groups: Vec<String> = row.try_get("groups")?;
            let title: Option<String> = row.try_get("title")?;
            let own: Vec<String> = row.try_get("own_labels")?;
            let labels = if own.is_empty() {
                row.try_get("group_labels")?
            } else {
                own
            };
            Ok(Signer {
                user_id: row.try_get("id")?,
                username: row.try_get("username")?,
                first_name: row.try_get("first_name")?,
                last_name: row.try_get("last_name")?,
                title: title.or_else(|| groups.first().cloned()),
                labels,
                groups,
            })
        })
        .collect()
}

/// The realm's groups that grant `action`'s sign permission, or the groups
/// with these ids (whether they grant it or not), by path.
async fn groups(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    action: SigningAction,
    ids: Option<&[String]>,
) -> Result<Vec<SigningGroup>> {
    let none: Vec<String> = vec![];
    let sql = format!(
        r#"{GRANTS},
        paths AS (
            SELECT g.id, g.parent_group, ('/' || g.name)::text AS path FROM keycloak_group g
            JOIN target_realm ON g.realm_id = target_realm.id
            UNION ALL
            SELECT p.id, parent.parent_group, ('/' || parent.name || p.path)::text
            FROM paths p JOIN keycloak_group parent ON parent.id = p.parent_group
        )
        SELECT
            g.id,
            g.name,
            (SELECT p.path FROM paths p WHERE p.id = g.id
             ORDER BY length(p.path) DESC LIMIT 1) AS path,
            EXISTS (SELECT 1 FROM group_role_mapping m
                    WHERE m.group_id = g.id AND m.role_id IN (SELECT id FROM sign_role))
                AS direct,
            EXISTS (SELECT 1 FROM group_role_mapping m
                    WHERE m.group_id = g.id AND m.role_id IN (SELECT id FROM sign_roles)
                        AND m.role_id NOT IN (SELECT id FROM sign_role))
                AS composite
        FROM keycloak_group g
        JOIN target_realm ON g.realm_id = target_realm.id
        WHERE CASE WHEN $6::boolean THEN g.id = ANY($7)
                   ELSE g.id IN (SELECT id FROM granting_groups) END
        ORDER BY 3, g.id"#
    );
    let selected = ids.is_some();
    let ids: Vec<String> = ids.map(<[String]>::to_vec).unwrap_or_default();
    let rows = keycloak_transaction
        .query(
            sql.as_str(),
            &[
                &realm,
                &action.sign_permission().to_string(),
                &none,
                &none,
                &PERMISSION_LABELS_ATTRIBUTE,
                &selected,
                &ids,
            ],
        )
        .await
        .context("Error listing the signing groups")?;
    rows.into_iter()
        .map(|row| {
            Ok(SigningGroup {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                path: row.try_get("path")?,
                direct: row.try_get("direct")?,
                composite: row.try_get("composite")?,
            })
        })
        .collect()
}

/// The groups of `realm` that grant `action`'s sign permission, by path.
#[instrument(skip(keycloak_transaction), err)]
pub async fn list_signing_groups(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    action: SigningAction,
) -> Result<Vec<SigningGroup>> {
    groups(keycloak_transaction, realm, action, None).await
}

/// The realm's groups with these ids and how they hold `action`'s sign
/// permission; an unknown id is left out.
#[instrument(skip(keycloak_transaction), err)]
pub async fn groups_by_id(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    action: SigningAction,
    ids: &[String],
) -> Result<Vec<SigningGroup>> {
    groups(keycloak_transaction, realm, action, Some(ids)).await
}

/// Each signer's title as the signing panel shows it, by Keycloak user id:
/// their `title` attribute, else a group that grants them a sign
/// permission (the first action's, in catalog order). People who sign
/// nothing, or have neither, are absent.
#[instrument(skip(keycloak_transaction), err)]
pub async fn signer_titles(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
) -> Result<BTreeMap<String, String>> {
    let mut titles = BTreeMap::new();
    for action in SigningAction::iter() {
        for signer in
            list_signers(keycloak_transaction, realm, action, &GroupChange::default()).await?
        {
            if let Some(title) = signer.title {
                titles.entry(signer.user_id).or_insert(title);
            }
        }
    }
    Ok(titles)
}
