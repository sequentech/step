use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc, Local};
use deadpool_postgres::Transaction;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio_postgres::row::Row;
use uuid::Uuid;
use tracing::{info, instrument};
use strum_macros::{Display, EnumString};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportCronConfig {
    pub is_active: bool,
    pub last_document_produced: Option<DateTime<Local>>,
    pub cron_expression: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub id: String,
    pub election_event_id: String,
    pub tenant_id: String,
    pub election_id: Option<String>,
    pub report_type: String,
    pub template_id: Option<String>,
    pub cron_config: Option<ReportCronConfig>,
    pub created_at: DateTime<Local>,
}

#[allow(non_camel_case_types)]
#[derive(
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    EnumString,
)]
pub enum ReportType {
    MANUAL_VERIFICATION, 
    BALLOT_RECEIPT,
    ELECTORAL_RESULTS,
}

pub struct ReportWrapper(pub Report);

impl TryFrom<Row> for ReportWrapper {
    type Error = anyhow::Error;

    fn try_from(item: Row) -> Result<Self> {
        let cron_config_js: Option<Value> = item.try_get("cron_config")?;
        let cron_config: Option<ReportCronConfig> = cron_config_js.map(|val| serde_json::from_value(val).unwrap());

        Ok(ReportWrapper(Report {
            id: item
                .try_get::<_, Uuid>("id")?
                .to_string(),
            election_event_id: item
                .try_get::<_, Uuid>("election_event_id")?
                .to_string(),
            tenant_id: item
                .try_get::<_, Uuid>("tenant_id")?
                .to_string(),
            election_id: item
                .try_get::<_, Option<Uuid>>("election_id")?
                .map(|val| val.to_string()),
            report_type: item.get("report_type"),
            template_id: item.get("template_id"),
            cron_config: cron_config,
            created_at: item.get("created_at"),
        }))
    }
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_all_active_reports(
    hasura_transaction: &Transaction<'_>,
) -> Result<Vec<Report>> {
    let statement = hasura_transaction
        .prepare(
            r#"
            SELECT *
            FROM "sequent_backend".report
            WHERE (cron_config->>'is_active')::boolean = true
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(&statement, &[])
        .await
        .map_err(|err| anyhow!("Error running get_all_active_reports query: {err}"))?;

    let reports = rows
        .into_iter()
        .map(|row| -> Result<Report> {
            row.try_into()
                .map(|res: ReportWrapper| -> Report { res.0 })
        })
        .collect::<Result<Vec<Report>>>()
        .with_context(|| "Error converting rows into Report")?;
    Ok(reports)
}

#[instrument(skip(hasura_transaction), err)]
pub async fn update_report_cron_config(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    id: &str,
    cron_config: ReportCronConfig,
) -> Result<()> {
    let tenant_uuid: Uuid =
        Uuid::parse_str(tenant_id).with_context(|| "Error parsing tenant_id as UUID")?;
    let id_uuid: Uuid =
        Uuid::parse_str(id).with_context(|| "Error parsing id as UUID")?;

    let cron_config_js: Value = serde_json::to_value(cron_config)?;

    let statement = hasura_transaction
        .prepare(
            r#"
            UPDATE "sequent_backend".sequent_backend_report
            SET cron_config = $3
            WHERE tenant_id = $1
            AND id = $2
            "#,
        )
        .await?;

    let _rows: Vec<Row> = hasura_transaction
        .query(&statement, &[&tenant_uuid, &id_uuid, &cron_config_js])
        .await
        .map_err(|err| anyhow!("Error updating report: {err}"))?;

    Ok(())
}

#[instrument(skip(hasura_transaction), err)]
pub async fn find_by_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    id: &str,
) -> Result<Option<Report>> {
    let tenant_uuid: Uuid =
        Uuid::parse_str(tenant_id).with_context(|| "Error parsing tenant_id as UUID")?;
    let id_uuid: Uuid =
        Uuid::parse_str(id).with_context(|| "Error parsing id as UUID")?;

    let statement = hasura_transaction
        .prepare(
            r#"
            SELECT *
            FROM "sequent_backend".sequent_backend_report
            WHERE tenant_id = $1
            AND id = $2
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(&statement, &[&tenant_uuid, &id_uuid])
        .await
        .map_err(|err| anyhow!("Error running find_by_id query: {err}"))?;

    let reports = rows
        .into_iter()
        .map(|row| -> Result<Report> {
            row.try_into()
                .map(|res: ReportWrapper| -> Report { res.0 })
        })
        .collect::<Result<Vec<Report>>>()
        .with_context(|| "Error converting rows into Report")?;

    Ok(reports.get(0).cloned())
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_template_id_for_report(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    report_type: &ReportType,
    election_id: Option<&str>,
) -> Result<Option<String>> {
    let tenant_uuid = Uuid::parse_str(tenant_id)
        .with_context(|| "Error parsing tenant_id as UUID")?;
    let election_event_uuid = Uuid::parse_str(election_event_id)
        .with_context(|| "Error parsing election_event_id as UUID")?;
    let election_uuid = if let Some(election_id) = election_id {
        Some(Uuid::parse_str(election_id)
            .with_context(|| "Error parsing election_id as UUID")?)
    } else {
        None
    };

    let statement = hasura_transaction
        .prepare(
            r#"
            SELECT template_id
            FROM "sequent_backend".sequent_backend_report
            WHERE tenant_id = $1
              AND election_event_id = $2
              AND report_type = $3
              AND ($4::uuid IS NULL OR election_id = $4::uuid)
            LIMIT 1
            "#,
        )
        .await?;

    let rows = hasura_transaction
        .query(
            &statement,
            &[
                &tenant_uuid,
                &election_event_uuid,
                &report_type.to_string(),
                &election_uuid,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error executing query: {err}"))?;

    if let Some(row) = rows.get(0) {
        let template_id: Option<String> = row.get("template_id");
        Ok(template_id)
    } else {
        Ok(None)
    }
}