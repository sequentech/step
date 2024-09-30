use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio_postgres::{Row, Transaction};
use tracing::instrument;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct Schedule {
    pub id: String,
    pub date: String,
    pub name: String,
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_tenant_settings(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
) -> Result<Vec<Schedule>> {
    let tenant_uuid: Uuid =
        Uuid::parse_str(tenant_id).with_context(|| "Error parsing tenant_id as UUID")?;

    let statement = hasura_transaction
        .prepare(
            r#"
            SELECT
                settings
            FROM "sequent_backend".tenant
            WHERE
                id = $1
            "#,
        )
        .await
        .map_err(|err| anyhow!("Error preparing the query: {err}"))?;

    let rows: Vec<Row> = hasura_transaction
        .query(&statement, &[&tenant_uuid])
        .await
        .map_err(|err| anyhow!("Error executing the query: {err}"))?;

    let mut schedules: Vec<Schedule> = Vec::new();

    if rows.is_empty() {
        return Ok(schedules);
    }

    for row in rows {
        let json_value: Value = row.get("settings");

        if let Some(schedules_array) = json_value.get("schedules") {
            if let Ok(schedule_list) =
                serde_json::from_value::<Vec<Schedule>>(schedules_array.clone())
            {
                schedules.extend(schedule_list);
            } else {
                tracing::error!("Failed to deserialize schedules: {:?}", schedules_array);
            }
        }
    }

    Ok(schedules)
}
