// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! A package's message templates, applied: each stored under its alias, as
//! the package carries it. A template the tenant doesn't have is created,
//! one it has as the package has it is left alone, and one it has otherwise
//! is replaced, every row with that alias, so applying never adds a second
//! template under one alias, however often the package is applied.

use crate::services::import::configuration_package_apply::SectionPreview;
use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use sequent_core::election_config::package_templates::PackageTemplate;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use uuid::Uuid;

/// Who stored a template a package created.
pub const CREATED_BY: &str = "configuration-package";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TemplateAction {
    Create,
    Replace,
    Unchanged,
}

/// A stored template's content, as compared with the package's.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredTemplate {
    pub template: Value,
    pub communication_method: String,
    pub template_type: String,
}

impl StoredTemplate {
    fn of(template: &PackageTemplate) -> Self {
        StoredTemplate {
            template: template.template.clone(),
            communication_method: template.communication_method.clone(),
            template_type: template.template_type.clone(),
        }
    }
}

/// What applying does to each template, given the tenant's templates under
/// the package's aliases.
pub fn plan(
    templates: &[PackageTemplate],
    stored: &BTreeMap<String, Vec<StoredTemplate>>,
) -> Vec<TemplateAction> {
    templates
        .iter()
        .map(|template| match stored.get(&template.alias) {
            None => TemplateAction::Create,
            Some(rows) if rows.is_empty() => TemplateAction::Create,
            Some(rows) if rows.iter().all(|row| *row == StoredTemplate::of(template)) => {
                TemplateAction::Unchanged
            }
            Some(_) => TemplateAction::Replace,
        })
        .collect()
}

/// The section's preview rows: alias, name, kind and action.
pub fn changes(templates: &[PackageTemplate], actions: &[TemplateAction]) -> Value {
    Value::Array(
        templates
            .iter()
            .zip(actions)
            .map(|(template, action)| {
                json!({
                    "alias": template.alias,
                    "name": template.template.get("name").cloned().unwrap_or(Value::Null),
                    "type": template.template_type,
                    "action": action,
                })
            })
            .collect(),
    )
}

pub fn preview(
    templates: &[PackageTemplate],
    stored: &BTreeMap<String, Vec<StoredTemplate>>,
) -> SectionPreview {
    let actions = plan(templates, stored);
    SectionPreview {
        needs_apply: actions
            .iter()
            .any(|action| *action != TemplateAction::Unchanged),
        changes: changes(templates, &actions),
        blockers: vec![],
    }
}

/// The tenant's templates under the package's aliases.
pub async fn stored(
    tx: &Transaction<'_>,
    tenant: &Uuid,
    templates: &[PackageTemplate],
) -> Result<BTreeMap<String, Vec<StoredTemplate>>> {
    let aliases: Vec<String> = templates.iter().map(|t| t.alias.clone()).collect();
    let rows = tx
        .query(
            "SELECT alias, template, communication_method, type
             FROM sequent_backend.template
             WHERE tenant_id = $1 AND alias = ANY($2)
             ORDER BY alias, id",
            &[tenant, &aliases],
        )
        .await?;
    let mut stored: BTreeMap<String, Vec<StoredTemplate>> = BTreeMap::new();
    for row in rows {
        let alias: Option<String> = row.get("alias");
        let Some(alias) = alias else { continue };
        stored.entry(alias).or_default().push(StoredTemplate {
            template: row
                .get::<_, Option<Value>>("template")
                .unwrap_or(Value::Null),
            communication_method: row
                .get::<_, Option<String>>("communication_method")
                .unwrap_or_default(),
            template_type: row.get::<_, Option<String>>("type").unwrap_or_default(),
        });
    }
    Ok(stored)
}

/// Stores the package's templates in `tx`, one tenant's applications at a
/// time, and returns the preview rows of what it did.
pub async fn apply(
    tx: &Transaction<'_>,
    tenant: &Uuid,
    templates: &[PackageTemplate],
) -> Result<Value> {
    tx.query_one(
        "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
        &[&format!("package-templates:{tenant}")],
    )
    .await?;
    let current = stored(tx, tenant, templates).await?;
    let actions = plan(templates, &current);
    for (template, action) in templates.iter().zip(&actions) {
        match action {
            TemplateAction::Unchanged => {}
            TemplateAction::Create => {
                tx.execute(
                    "INSERT INTO sequent_backend.template
                     (alias, tenant_id, template, created_by, created_at, updated_at,
                      communication_method, type)
                     VALUES ($1, $2, $3, $4, NOW(), NOW(), $5, $6)",
                    &[
                        &template.alias,
                        tenant,
                        &template.template,
                        &CREATED_BY,
                        &template.communication_method,
                        &template.template_type,
                    ],
                )
                .await?;
            }
            TemplateAction::Replace => {
                let changed = tx
                    .execute(
                        "UPDATE sequent_backend.template
                         SET template = $3, communication_method = $4, type = $5,
                             updated_at = NOW()
                         WHERE tenant_id = $1 AND alias = $2",
                        &[
                            tenant,
                            &template.alias,
                            &template.template,
                            &template.communication_method,
                            &template.template_type,
                        ],
                    )
                    .await?;
                if changed == 0 {
                    return Err(anyhow!(
                        "template '{}' changed while it was applied; preview again",
                        template.alias
                    ));
                }
            }
        }
    }
    Ok(changes(templates, &actions))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn template(alias: &str, body: &str) -> PackageTemplate {
        PackageTemplate {
            alias: alias.into(),
            template_type: "CREDENTIALS".into(),
            communication_method: "EMAIL".into(),
            template: json!({"alias":alias,"name":alias,"sms":{"message":body}}),
        }
    }

    #[test]
    fn templates_are_created_left_or_replaced_by_alias() {
        let wanted = [
            template("new", "a"),
            template("same", "b"),
            template("edited", "c"),
            template("twice", "d"),
        ];
        let stored = BTreeMap::from([
            ("same".to_string(), vec![StoredTemplate::of(&wanted[1])]),
            (
                "edited".to_string(),
                vec![StoredTemplate::of(&template("edited", "old"))],
            ),
            // Two rows under one alias, one of them different: both get the
            // package's, never a third.
            (
                "twice".to_string(),
                vec![
                    StoredTemplate::of(&wanted[3]),
                    StoredTemplate::of(&template("twice", "old")),
                ],
            ),
        ]);
        assert_eq!(
            plan(&wanted, &stored),
            [
                TemplateAction::Create,
                TemplateAction::Unchanged,
                TemplateAction::Replace,
                TemplateAction::Replace
            ]
        );
        let preview = preview(&wanted, &stored);
        assert!(preview.needs_apply);
        assert_eq!(
            preview.changes[0],
            json!({"alias":"new","name":"new","type":"CREDENTIALS","action":"create"})
        );
        let unchanged = preview_of_all_unchanged(&wanted);
        assert!(!unchanged.needs_apply);
    }

    fn preview_of_all_unchanged(wanted: &[PackageTemplate]) -> SectionPreview {
        let stored = wanted
            .iter()
            .map(|t| (t.alias.clone(), vec![StoredTemplate::of(t)]))
            .collect();
        preview(wanted, &stored)
    }
}
