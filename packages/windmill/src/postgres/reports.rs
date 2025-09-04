// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::str::FromStr;

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Local, Utc};
use deadpool_postgres::Transaction;
use sequent_core::serialization::deserialize_with_path::{self, deserialize_value};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum_macros::{Display, EnumString};
use tokio_postgres::row::Row;
use tracing::{info, instrument};
use uuid::Uuid;

use crate::services::reports::template_renderer::EReportEncryption;

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone)]
pub struct ReportCronConfig {
    #[serde(default)]
    pub is_active: bool,
    #[serde(default)]
    pub last_document_produced: Option<String>,
    #[serde(default)]
    pub cron_expression: String,
    #[serde(default)]
    pub email_recipients: Vec<String>,
    #[serde(default)]
    pub executer_username: String,
    #[serde(default)]
    pub user_timezone: Option<String>,
}

impl Default for ReportCronConfig {
    fn default() -> Self {
        ReportCronConfig {
            is_active: false,
            last_document_produced: None,
            cron_expression: Default::default(),
            email_recipients: Default::default(),
            executer_username: Default::default(),
            user_timezone: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub id: String,
    pub election_event_id: String,
    pub tenant_id: String,
    pub election_id: Option<String>,
    pub report_type: String,
    pub template_alias: Option<String>,
    pub encryption_policy: EReportEncryption,
    pub cron_config: Option<ReportCronConfig>,
    pub created_at: DateTime<Utc>,
    pub permission_label: Option<Vec<String>>,
}

#[allow(non_camel_case_types)]
#[derive(Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString)]
pub enum ReportType {
    MANUAL_VERIFICATION,
    BALLOT_RECEIPT,
    VOTE_RECEIPT,
    ELECTORAL_RESULTS,
    STATISTICAL_REPORT,
    ACTIVITY_LOGS,
    TRANSMISSION_REPORT,
    STATUS,
    OV_PRE_ENROLLED_APPROVED,
    OV_WHO_VOTED,
    PRE_ENROLLED_OV_SUBJECT_TO_MANUAL_VALIDATION,
    PRE_ENROLLED_OV_BUT_DISAPPROVED,
    LIST_OF_OVERSEAS_VOTERS,
    OVCS_STATISTICS,
    OVCS_INFORMATION,
    OVCS_EVENTS,
    OV_WITH_VOTING_STATUS,
    INITIALIZATION_REPORT,
    AUDIT_LOGS,
    OV_NOT_YET_PRE_ENROLLED_LIST,
    OV_NOT_YET_PRE_ENROLLED_NUMBER,
    OV_TURNOUT_PERCENTAGE,
    OV_TURNOUT_PER_ABOARD_STATUS_SEX,
    OV_TURNOUT_PER_ABOARD_STATUS_SEX_PERCENTAGE,
    BALLOT_IMAGES,
}

pub struct ReportWrapper(pub Report);

impl TryFrom<Row> for ReportWrapper {
    type Error = anyhow::Error;

    fn try_from(item: Row) -> Result<Self> {
        let cron_config_js: Option<Value> = item
            .try_get("cron_config")
            .map_err(|err| anyhow!("Error deserializing cron_config: {err}"))?;
        info!("cron_config wrapper: {:?}", cron_config_js);
        let cron_config: Option<ReportCronConfig> = cron_config_js
            .map(|val| deserialize_with_path::deserialize_value(val).unwrap_or_default());
        info!("cron_config wrapper: {:?}", cron_config);
        Ok(ReportWrapper(Report {
            id: item.try_get::<_, Uuid>("id")?.to_string(),
            election_event_id: item.try_get::<_, Uuid>("election_event_id")?.to_string(),
            tenant_id: item.try_get::<_, Uuid>("tenant_id")?.to_string(),
            election_id: item
                .try_get::<_, Option<Uuid>>("election_id")?
                .map(|val| val.to_string()),
            report_type: item.get("report_type"),
            template_alias: item.get("template_alias"),
            cron_config: cron_config,
            created_at: item.get("created_at"),
            encryption_policy: EReportEncryption::from_str(
                item.get::<_, String>("encryption_policy").as_str(),
            )
            .map_err(|err| {
                anyhow!(
                    "error deserializing encryption_policy: {err:?} {value:?}",
                    value = item.get::<_, String>("encryption_policy").as_str()
                )
            })?,
            permission_label: item.get::<_, Option<Vec<String>>>("permission_label"),
        }))
    }
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_all_active_reports(hasura_transaction: &Transaction<'_>) -> Result<Vec<Report>> {
    let statement = hasura_transaction
        .prepare(
            r#"
            SELECT
                *
            FROM
                "sequent_backend".report
            WHERE
                (cron_config->>'is_active')::boolean = true
            "#,
        )
        .await
        .map_err(|err| anyhow!("Error preparing query: {err}"))?;

    let rows: Vec<Row> = hasura_transaction
        .query(&statement, &[])
        .await
        .map_err(|err| anyhow!("Error running get_all_active_reports query: {err}"))?;

    let reports = rows
        .into_iter()
        .map(|row| -> Result<Report> {
            row.try_into().map(|res: ReportWrapper| -> Report { res.0 })
        })
        .collect::<Result<Vec<Report>>>()
        .with_context(|| "Error converting rows into Report")?;
    Ok(reports)
}

#[instrument(skip(hasura_transaction), err)]
pub async fn update_report_last_document_time(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    id: &str,
) -> Result<()> {
    let tenant_uuid: Uuid =
        Uuid::parse_str(tenant_id).with_context(|| "Error parsing tenant_id as UUID")?;
    let id_uuid: Uuid = Uuid::parse_str(id).with_context(|| "Error parsing id as UUID")?;

    let statement = hasura_transaction
        .prepare(
            r#"
            UPDATE
                "sequent_backend".report
            SET 
                cron_config = 
                    COALESCE(cron_config, '{}'::jsonb),
                    '{last_document_produced}',
                    to_jsonb(
                        to_char(NOW() at time zone 'utc',
                        'YYYY-MM-DD"T"HH24:MI:SS.US'
                    )
                ),
                true
            )
            WHERE
                tenant_id = $1
                AND id = $2
            "#,
        )
        .await
        .map_err(|err| anyhow!("Error preparing query: {err}"))?;

    let affected_rows = hasura_transaction
        .execute(&statement, &[&tenant_uuid, &id_uuid])
        .await
        .map_err(|err| anyhow!("Error updating report: {err}"))?;

    if affected_rows == 0 {
        return Err(anyhow!("No report found with the given tenant_id and id"));
    }

    Ok(())
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_report_by_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    id: &str,
) -> Result<Option<Report>> {
    let tenant_uuid: Uuid =
        Uuid::parse_str(tenant_id).with_context(|| "Error parsing tenant_id as UUID")?;
    let id_uuid: Uuid = Uuid::parse_str(id).with_context(|| "Error parsing id as UUID")?;

    let statement = hasura_transaction
        .prepare(
            r#"
            SELECT
                *
            FROM
                "sequent_backend".report
            WHERE
                tenant_id = $1
                AND id = $2
            "#,
        )
        .await
        .map_err(|err| anyhow!("Error preparing query: {err}"))?;

    let rows: Vec<Row> = hasura_transaction
        .query(&statement, &[&tenant_uuid, &id_uuid])
        .await
        .map_err(|err| anyhow!("Error running find_by_id query: {err}"))?;

    let reports = rows
        .into_iter()
        .map(|row| -> Result<Report> {
            row.try_into().map(|res: ReportWrapper| -> Report { res.0 })
        })
        .collect::<Result<Vec<Report>>>()
        .map_err(|err| anyhow!("Error converting rows into Report: {err:?}"))?;

    Ok(reports.get(0).cloned())
}

/// Returns ONLY THE FIRST the template_alias which matches these arguments,
/// If there are multiple matches, the rest are ignored.
#[instrument(skip(hasura_transaction), err)]
pub async fn get_template_alias_for_report(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    report_type: &ReportType,
    election_id: Option<&str>,
) -> Result<Option<String>> {
    let tenant_uuid =
        Uuid::parse_str(tenant_id).with_context(|| "Error parsing tenant_id as UUID")?;
    let election_event_uuid = Uuid::parse_str(election_event_id)
        .with_context(|| "Error parsing election_event_id as UUID")?;
    let election_uuid = if let Some(election_id) = election_id {
        Some(Uuid::parse_str(election_id).with_context(|| "Error parsing election_id as UUID")?)
    } else {
        None
    };

    let statement = hasura_transaction
        .prepare(
            r#"
            SELECT template_alias
            FROM "sequent_backend".report
            WHERE tenant_id = $1
              AND election_event_id = $2
              AND report_type = $3
              AND ($4::uuid IS NULL OR election_id = $4::uuid)
            LIMIT 1
            "#,
        )
        .await
        .map_err(|err| anyhow!("Error preparing query: {err}"))?;

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

    // If found report is found, return the associated template_alias
    if let Some(row) = rows.get(0) {
        let template_alias: Option<String> = row.get("template_alias");
        return Ok(template_alias);
    }

    // Not found. If election_id was not set we finish
    if election_id.is_none() {
        return Ok(None);
    }

    // Election Id was set, but maybe we find the report if we don't set it,
    // at the election event level as a fallback
    let statement = hasura_transaction
        .prepare(
            r#"
            SELECT
                template_alias
            FROM
                "sequent_backend".report
            WHERE
                tenant_id = $1
                AND election_event_id = $2
                AND report_type = $3
            LIMIT 1
            "#,
        )
        .await
        .map_err(|err| anyhow!("Error preparing query: {err}"))?;

    let rows = hasura_transaction
        .query(
            &statement,
            &[&tenant_uuid, &election_event_uuid, &report_type.to_string()],
        )
        .await
        .map_err(|err| anyhow!("Error executing query: {err}"))?;

    // If found, return
    if let Some(row) = rows.get(0) {
        let template_alias: Option<String> = row.get("template_alias");
        return Ok(template_alias);
    } else {
        return Ok(None);
    }
}

#[instrument(skip(hasura_transaction), err)]
async fn get_reports_by_condition(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    condition_value: &str,
    condition_column: &str,
) -> Result<Vec<Report>> {
    let tenant_uuid =
        Uuid::parse_str(tenant_id).with_context(|| "Error parsing tenant_id as UUID")?;
    let condition_uuid = Uuid::parse_str(condition_value)
        .with_context(|| format!("Error parsing {condition_column} as UUID"))?;

    let statement = hasura_transaction
        .prepare(&format!(
            r#"
                SELECT
                    *
                FROM
                    "sequent_backend".report
                WHERE
                    tenant_id = $1
                    AND {condition_column} = $2
                "#
        ))
        .await
        .map_err(|err| anyhow!("Error preparing query: {err}"))?;

    let rows: Vec<Row> = hasura_transaction
        .query(&statement, &[&tenant_uuid, &condition_uuid])
        .await
        .map_err(|err| anyhow!("Error running get_reports_by_condition query: {err}"))?;

    let reports = rows
        .into_iter()
        .map(|row| -> Result<Report> {
            row.try_into().map(|res: ReportWrapper| -> Report { res.0 })
        })
        .collect::<Result<Vec<Report>>>()
        .with_context(|| "Error converting rows into Report")?;

    Ok(reports)
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_reports_by_election_event_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Vec<Report>> {
    get_reports_by_condition(
        hasura_transaction,
        tenant_id,
        election_event_id,
        "election_event_id",
    )
    .await
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_reports_by_election_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_id: &str,
) -> Result<Vec<Report>> {
    get_reports_by_condition(hasura_transaction, tenant_id, election_id, "election_id").await
}

#[instrument(skip(hasura_transaction), err)]
pub async fn insert_reports(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    reports: &[Report],
) -> Result<()> {
    let tenant_uuid =
        Uuid::parse_str(tenant_id).with_context(|| "Error parsing tenant_id as UUID")?;
    let election_event_uuid = Uuid::parse_str(election_event_id)
        .with_context(|| "Error parsing election_event_id as UUID")?;

    let statement = hasura_transaction
        .prepare(
            r#"
            INSERT INTO "sequent_backend".report (
                id, election_event_id, tenant_id, election_id, report_type, template_alias, cron_config, created_at, encryption_policy, permission_label
            ) VALUES (
                $1,
                $2,
                $3,
                $4,
                $5,
                $6,
                $7,
                $8,
                $9,
                $10
            )
            "#,
        )
        .await
        .map_err(|err| anyhow!("Error preparing query: {err}"))?;

    for report in reports {
        hasura_transaction
            .execute(
                &statement,
                &[
                    &Uuid::parse_str(&report.id)?,
                    &election_event_uuid,
                    &tenant_uuid,
                    &report
                        .election_id
                        .as_ref()
                        .map(|id| Uuid::parse_str(id))
                        .transpose()?,
                    &report.report_type,
                    &report.template_alias,
                    &serde_json::to_value(&report.cron_config)
                        .map_err(|err| anyhow!("Error parsing cron config to value: {err}, cron_config={cron_config:?}", cron_config=report.cron_config))?,
                    &report.created_at,
                    &report.encryption_policy.to_string(),
                    &report.permission_label
                ],
            )
            .await
            .map_err(|err| anyhow!("Error inserting report: {err}"))?;
    }

    Ok(())
}

#[instrument(skip_all, err)]
pub async fn get_report_by_type(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    report_type: &str,
    election_id: &Option<String>,
) -> Result<Option<Report>> {
    let tenant_uuid: Uuid =
        Uuid::parse_str(tenant_id).with_context(|| "Error parsing tenant_id as UUID")?;
    let election_event_uuid = Uuid::parse_str(election_event_id)
        .with_context(|| "Error parsing election_event_id as UUID")?;
    let election_uuid = election_id
        .as_ref()
        .and_then(|id| Uuid::parse_str(&id).ok());

    let statement = hasura_transaction
        .prepare(
            r#"
            SELECT
                *
            FROM
                "sequent_backend".report
            WHERE
                tenant_id = $1
                AND election_event_id = $2
                AND report_type = $3
              AND ($4::uuid IS NULL OR election_id = $4::uuid)
            "#,
        )
        .await
        .map_err(|err| anyhow!("Error preparing query: {err}"))?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &tenant_uuid,
                &election_event_uuid,
                &report_type,
                &election_uuid,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error running find_by_id query: {err}"))?;

    let reports = rows
        .into_iter()
        .map(|row| -> Result<Report> {
            row.try_into().map(|res: ReportWrapper| -> Report { res.0 })
        })
        .collect::<Result<Vec<Report>>>()
        .map_err(|err| anyhow!("Error converting rows into Report: {err:?}"))?;

    Ok(reports.get(0).cloned())
}
