// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use deadpool_postgres::Transaction;
use sequent_core::types::hasura::core::CommunicationTemplate;
use tokio_postgres::row::Row;
use tracing::{event, instrument, Level};
use uuid::Uuid;

pub struct CommunicationTemplateWrapper(pub CommunicationTemplate);

impl TryFrom<Row> for CommunicationTemplateWrapper {
    type Error = anyhow::Error;
    fn try_from(item: Row) -> Result<Self> {
        Ok(CommunicationTemplateWrapper(CommunicationTemplate {
            id: item.try_get::<_, Uuid>("id")?.to_string(),
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
pub async fn get_template_by_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    template_id: &str,
) -> Result<Option<CommunicationTemplate>> {
    let statement = hasura_transaction
        .prepare(
            r#"
            SELECT
                id,
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
                id = $2;
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(template_id)?,
            ],
        )
        .await?;

    let elections: Vec<CommunicationTemplate> = rows
        .into_iter()
        .map(|row| -> Result<CommunicationTemplate> {
            row.try_into()
                .map(|res: CommunicationTemplateWrapper| -> CommunicationTemplate { res.0 })
        })
        .collect::<Result<Vec<CommunicationTemplate>>>()?;

    Ok(elections.get(0).map(|election| election.clone()))
}
