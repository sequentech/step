// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::types::hasura::core::Template;
use tokio_postgres::row::Row;
use tracing::{event, instrument, Level};
use uuid::Uuid;

pub struct TemplateWrapper(pub Template);

impl TryFrom<Row> for TemplateWrapper {
    type Error = anyhow::Error;
    fn try_from(item: Row) -> Result<Self> {
        Ok(TemplateWrapper(Template {
            alias: item.try_get("alias")?,
            tenant_id: item.try_get::<_, Uuid>("tenant_id")?.to_string(),
            template: item.get("template"),
            created_by: item.try_get("created_by")?,
            labels: item.try_get("labels")?,
            annotations: item.try_get("annotations")?,
            created_at: item.get("created_at"),
            updated_at: item.get("updated_at"),
            communication_method: item.try_get("communication_method")?,
            r#type: item.try_get("type")?,
        }))
    }
}

/* Returns election */

#[instrument(skip(hasura_transaction), err)]
pub async fn get_template_by_alias(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    template_alias: &str,
) -> Result<Option<Template>> {
    let statement = hasura_transaction
        .prepare(
            r#"
            SELECT
                alias,
                tenant_id,
                template,
                created_by,
                labels,
                annotations,
                created_at,
                updated_at,
                communication_method,
                type
            FROM
                sequent_backend.template
            WHERE
                tenant_id = $1 AND
                alias = $2;
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(&statement, &[&Uuid::parse_str(tenant_id)?, &template_alias])
        .await?;

    let elections: Vec<Template> = rows
        .into_iter()
        .map(|row| -> Result<Template> {
            row.try_into()
                .map(|res: TemplateWrapper| -> Template { res.0 })
        })
        .collect::<Result<Vec<Template>>>()?;

    Ok(elections.get(0).map(|election| election.clone()))
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_templates_by_tenant_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
) -> Result<Vec<Template>> {
    // Check if tenant_id is a valid UUID string
    if tenant_id.is_empty() {
        return Err(anyhow!("Tenant ID is empty"));
    }

    let tenant_uuid =
        Uuid::parse_str(tenant_id).map_err(|err| anyhow!("Error parsing tenant UUID: {}", err))?;

    let statement = hasura_transaction
        .prepare(
            r#"
            SELECT
                alias,
                tenant_id,
                template,
                created_by,
                labels,
                annotations,
                created_at,
                updated_at,
                communication_method,
                type
            FROM
                sequent_backend.template
            WHERE
                tenant_id = $1;
            "#,
        )
        .await?;

    let rows = hasura_transaction
        .query(&statement, &[&tenant_uuid])
        .await
        .map_err(|err| anyhow!("Error fetching templates: {}", err))?;

    let templates: Vec<Template> = rows
        .into_iter()
        .map(|row| row.try_into().map(|wrapper: TemplateWrapper| wrapper.0))
        .collect::<Result<Vec<_>, _>>()
        .context("Error converting database rows to Template")?;

    Ok(templates)
}

#[instrument(err, skip_all)]
pub async fn insert_templates(
    hasura_transaction: &Transaction<'_>,
    templates: &Vec<Template>,
) -> Result<()> {
    let statement = hasura_transaction
        .prepare(
            r#"
            INSERT INTO sequent_backend.template
            (
                alias,
                tenant_id,
                template,
                created_by,
                labels,
                annotations,
                created_at,
                updated_at,
                communication_method,
                type
            )
            VALUES (
                $1,
                $2,
                $3,
                $4,
                $5,
                $6,
                NOW(),
                NOW(),
                $7,
                $8
            );
            "#,
        )
        .await
        .map_err(|err| anyhow!("Error preparing the insert template query: {err}"))?;

    for template in templates {
        hasura_transaction
            .execute(
                &statement,
                &[
                    &template.alias,
                    &Uuid::parse_str(&template.tenant_id)?,
                    &template.template,
                    &template.created_by,
                    &template.labels,
                    &template.annotations,
                    &template.communication_method,
                    &template.r#type,
                ],
            )
            .await
            .map_err(|err| anyhow!("Error inserting template: {err}"))?;
    }

    Ok(())
}
