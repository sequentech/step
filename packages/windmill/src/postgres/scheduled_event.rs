// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use sequent_core::services::uuid_validation::parse_uuid_v4;
use sequent_core::types::scheduled_event::*;
use sequent_core::{
    serialization::deserialize_with_path::deserialize_value,
    types::scheduled_event::{EventProcessors, ScheduledEvent},
};
use serde_json::Value;
use std::str::FromStr;
use tokio_postgres::row::Row;
use tracing::{info, instrument};
use uuid::Uuid;

pub struct ScheduledEventWrapper(pub ScheduledEvent);

impl TryFrom<Row> for ScheduledEventWrapper {
    type Error = anyhow::Error;

    #[instrument(skip_all, err)]
    fn try_from(item: Row) -> Result<Self> {
        let event_processors_js: Option<String> = item.try_get("event_processor")?;
        let event_processors: Option<EventProcessors> =
            if let Some(val) = event_processors_js {
                Some(EventProcessors::from_str(&val).map_err(|err| {
                    anyhow!("Error mapping {val:?} into an EventProcessor: {err:?}")
                })?)
            } else {
                None
            };

        let cron_config_js: Option<Value> = item
            .try_get("cron_config")
            .map_err(|err| anyhow!("Error deserializing cron_config: {err}"))?;
        let cron_config: Option<CronConfig> = cron_config_js
            .map(|val| deserialize_value(val))
            .transpose()?;

        Ok(ScheduledEventWrapper(ScheduledEvent {
            id: item
                .try_get::<_, Uuid>("id")
                .map_err(|err| anyhow!("Error deserializing id: {err}"))?
                .to_string(),
            tenant_id: item
                .try_get::<_, Option<Uuid>>("tenant_id")
                .map_err(|err| anyhow!("Error deserializing tenant_id: {err}"))?
                .map(|val| val.to_string()),
            election_event_id: item
                .try_get::<_, Option<Uuid>>("election_event_id")
                .map_err(|err| anyhow!("Error deserializing election_event_id: {err}"))?
                .map(|val| val.to_string()),
            created_at: item.get("created_at"),
            stopped_at: item.get("stopped_at"),
            archived_at: item.get("archived_at"),
            labels: item.get("labels"),
            annotations: item.get("annotations"),
            event_processor: event_processors,
            cron_config: cron_config,
            event_payload: item.get("event_payload"),
            task_id: item.get("task_id"),
        }))
    }
}

#[instrument(skip(hasura_transaction), err)]
pub async fn find_all_active_events(
    hasura_transaction: &Transaction<'_>,
) -> Result<Vec<ScheduledEvent>> {
    let statement = hasura_transaction
        .prepare(
            r#"
            SELECT
                *
            FROM "sequent_backend".scheduled_event
            WHERE
                stopped_at IS NULL AND archived_at IS NULL
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(&statement, &[])
        .await
        .map_err(|err| anyhow!("Error running the find_all_active_events query: {err}"))?;

    let scheduled_events = rows
        .into_iter()
        .map(|row| -> Result<ScheduledEvent> {
            row.try_into()
                .map(|res: ScheduledEventWrapper| -> ScheduledEvent { res.0 })
        })
        .collect::<Result<Vec<ScheduledEvent>>>()
        .with_context(|| "Error converting rows into ScheduledEvent")?;
    Ok(scheduled_events)
}

#[instrument(skip(hasura_transaction), err)]
pub async fn find_scheduled_event_by_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Option<String>,
    election_event_id: Option<String>,
    id: &str,
) -> Result<Option<ScheduledEvent>> {
    let tenant_uuid: Option<uuid::Uuid> = match tenant_id {
        Some(ref tenant_id) => {
            Some(parse_uuid_v4(tenant_id).with_context(|| "Error parsing tenant_id as UUID")?)
        }
        None => None,
    };
    let election_event_uuid: Option<uuid::Uuid> = match election_event_id {
        Some(ref election_event_id) => Some(
            parse_uuid_v4(election_event_id)
                .with_context(|| "Error parsing election_event_id as UUID")?,
        ),
        None => None,
    };
    let id_uuid: uuid::Uuid = parse_uuid_v4(id).with_context(|| "Error parsing id as UUID")?;

    let statement = hasura_transaction
        .prepare(
            r#"
            SELECT
                *
            FROM "sequent_backend".scheduled_event
            WHERE
                (tenant_id = $1 OR $1 IS NULL)
                AND (election_event_id = $2 OR $2 IS NULL)
                AND id = $3
                AND stopped_at IS NULL
                AND archived_at IS NULL
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(&statement, &[&tenant_uuid, &election_event_uuid, &id_uuid])
        .await
        .map_err(|err| anyhow!("Error running the find_scheduled_event_by_id query: {err}"))?;

    let scheduled_events = rows
        .into_iter()
        .map(|row| -> Result<ScheduledEvent> {
            row.try_into()
                .map(|res: ScheduledEventWrapper| -> ScheduledEvent { res.0 })
        })
        .collect::<Result<Vec<ScheduledEvent>>>()
        .with_context(|| "Error converting rows into ScheduledEvent")?;

    Ok(scheduled_events.get(0).cloned())
}

#[instrument(skip(hasura_transaction), err)]
pub async fn find_scheduled_event_by_task_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    task_id: &str,
) -> Result<Option<ScheduledEvent>> {
    let tenant_uuid: uuid::Uuid =
        parse_uuid_v4(tenant_id).with_context(|| "Error parsing tenant_id as UUID")?;
    let election_event_uuid: uuid::Uuid = parse_uuid_v4(election_event_id)
        .with_context(|| "Error parsing election_event_id as UUID")?;
    let statement = hasura_transaction
        .prepare(
            r#"
            SELECT
                *
            FROM "sequent_backend".scheduled_event
            WHERE
                tenant_id = $1
                AND election_event_id = $2
                AND task_id = $3
                AND archived_at IS NULL
            "#,
        )
        .await
        .map_err(|err| anyhow!("Error running the find_scheduled_event_by_task_id query: {err}"))?;

    let rows: Vec<Row> = hasura_transaction
        .query(&statement, &[&tenant_uuid, &election_event_uuid, &task_id])
        .await
        .map_err(|err| anyhow!("Error running the find_scheduled_event_by_task_id query: {err}"))?;

    let scheduled_events = rows
        .into_iter()
        .map(|row| -> Result<ScheduledEvent> {
            row.try_into()
                .map(|res: ScheduledEventWrapper| -> ScheduledEvent { res.0 })
        })
        .collect::<Result<Vec<ScheduledEvent>>>()
        .with_context(|| "Error converting rows into ScheduledEvent")?;

    Ok(scheduled_events.get(0).cloned())
}

#[instrument(skip(hasura_transaction), err)]
pub async fn stop_scheduled_event(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    id: &str,
) -> Result<()> {
    let tenant_uuid: uuid::Uuid =
        parse_uuid_v4(tenant_id).with_context(|| "Error parsing tenant_id as UUID")?;
    let id_uuid: uuid::Uuid =
        parse_uuid_v4(id).with_context(|| "Error parsing election_event_id as UUID")?;

    let statement = hasura_transaction
        .prepare(
            r#"
            UPDATE
                "sequent_backend".scheduled_event
            SET
                stopped_at = NOW(),
                -- A row that ran has no pending tz database recompute.
                annotations = CASE WHEN jsonb_typeof(annotations) = 'object'
                    THEN annotations - 'schedule_recompute' ELSE annotations END
            WHERE
                tenant_id = $1
                AND id = $2
                AND stopped_at IS NULL
            "#,
        )
        .await?;

    let _rows: Vec<Row> = hasura_transaction
        .query(&statement, &[&tenant_uuid, &id_uuid])
        .await
        .map_err(|err| anyhow!("Error running the stop_scheduled_event query: {err}"))?;

    Ok(())
}

#[instrument(skip(hasura_transaction), err)]
pub async fn archive_scheduled_event(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    id: &str,
) -> Result<()> {
    let tenant_uuid: uuid::Uuid =
        parse_uuid_v4(tenant_id).with_context(|| "Error parsing tenant_id as UUID")?;
    let id_uuid: uuid::Uuid =
        parse_uuid_v4(id).with_context(|| "Error parsing election_event_id as UUID")?;

    let statement = hasura_transaction
        .prepare(
            r#"
            UPDATE
                "sequent_backend".scheduled_event
            SET
                stopped_at = COALESCE(stopped_at, NOW()),
                archived_at = NOW()
            WHERE
                tenant_id = $1
                AND id = $2
            "#,
        )
        .await?;

    let _rows: Vec<Row> = hasura_transaction
        .query(&statement, &[&tenant_uuid, &id_uuid])
        .await
        .map_err(|err| anyhow!("Error running the archive_scheduled_event query: {err}"))?;

    Ok(())
}

/// Replaces the cron config (and the channels, when given) of an active
/// scheduled event, or re-arms a stopped one for a future instant. Returns
/// 0 when archived, missing, or stopped without a future instant.
#[instrument(skip(hasura_transaction), err)]
pub async fn update_scheduled_event(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    id: &str,
    cron_config: CronConfig,
    voting_channels: Option<&Vec<sequent_core::ballot::VotingStatusChannel>>,
    task_id: Option<&str>,
) -> Result<u64> {
    let tenant_uuid: uuid::Uuid =
        parse_uuid_v4(tenant_id).with_context(|| "Error parsing tenant_id as UUID")?;
    let id_uuid: uuid::Uuid =
        parse_uuid_v4(id).with_context(|| "Error parsing election_event_id as UUID")?;

    // An edit to a future time re-arms an event that already ran; the
    // instant is the one the scheduler reads (RFC 3339 with an offset).
    let instant: Option<DateTime<Utc>> = cron_config
        .scheduled_date
        .as_deref()
        .and_then(|date| DateTime::parse_from_rfc3339(date).ok())
        .map(|date| date.with_timezone(&Utc));
    let cron_config_js: Value = serde_json::to_value(cron_config)?;

    // A pending tz database recompute no longer applies once the row is
    // saved again: the new instant comes from the current database.
    let statement = hasura_transaction
        .prepare(
            r#"
            UPDATE
                "sequent_backend".scheduled_event
            SET
                cron_config = $3,
                task_id = COALESCE($6, task_id),
                event_payload = CASE WHEN $4::jsonb IS NULL THEN event_payload
                    ELSE COALESCE(event_payload, '{}'::jsonb) || jsonb_build_object('voting_channels', $4::jsonb) END,
                stopped_at = CASE WHEN $5::timestamptz > NOW() THEN NULL ELSE stopped_at END,
                annotations = CASE WHEN jsonb_typeof(annotations) = 'object'
                    THEN annotations - 'schedule_recompute' ELSE annotations END
            WHERE
                tenant_id = $1
                AND id = $2
                AND archived_at IS NULL
                AND (stopped_at IS NULL OR $5::timestamptz > NOW())
            "#,
        )
        .await?;

    let updated = hasura_transaction
        .execute(
            &statement,
            &[
                &tenant_uuid,
                &id_uuid,
                &cron_config_js,
                &voting_channels.map(serde_json::to_value).transpose()?,
                &instant,
                &task_id,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error running the update_scheduled_event query: {err}"))?;

    Ok(updated)
}

#[instrument(skip(hasura_transaction), err)]
pub async fn insert_scheduled_event(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    event_processor: EventProcessors,
    task_id: &str,
    cron_config: CronConfig,
    event_payload: Value,
) -> Result<ScheduledEvent> {
    let tenant_uuid: uuid::Uuid =
        parse_uuid_v4(tenant_id).with_context(|| "Error parsing tenant_id as UUID")?;
    let election_event_uuid: uuid::Uuid = parse_uuid_v4(election_event_id)
        .with_context(|| "Error parsing election_event_id as UUID")?;
    let cron_config_js: Value = serde_json::to_value(cron_config)?;
    let event_processor_s = event_processor.to_string();
    let statement = hasura_transaction
        .prepare(
            r#"
                INSERT INTO
                    "sequent_backend".scheduled_event
                (
                    tenant_id,
                    election_event_id,
                    created_at,
                    event_processor,
                    cron_config,
                    task_id,
                    event_payload
                )
                VALUES (
                    $1,
                    $2,
                    NOW(),
                    $3,
                    $4,
                    $5,
                    $6
                )
                RETURNING
                    id,
                    tenant_id,
                    election_event_id,
                    created_at,
                    stopped_at,
                    archived_at,
                    labels,
                    annotations,
                    event_processor,
                    cron_config,
                    event_payload,
                    task_id;
            "#,
        )
        .await
        .map_err(|err| anyhow!("Error preparing scheduled event statement: {}", err))?;
    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &tenant_uuid,
                &election_event_uuid,
                &event_processor_s,
                &cron_config_js,
                &task_id,
                &event_payload,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error inserting scheduled event: {}", err))?;

    let rows: Vec<ScheduledEvent> = rows
        .into_iter()
        .map(|row| -> Result<ScheduledEvent> {
            row.try_into()
                .map(|res: ScheduledEventWrapper| -> ScheduledEvent { res.0 })
        })
        .collect::<Result<Vec<ScheduledEvent>>>()
        .map_err(|err| anyhow!("Error deserializing scheduled event: {}", err))?;

    if 1 == rows.len() {
        Ok(rows[0].clone())
    } else {
        Err(anyhow!("Unexpected rows affected {}", rows.len()))
    }
}

#[instrument(skip(hasura_transaction), err)]
pub async fn find_scheduled_event_by_election_event_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Vec<ScheduledEvent>> {
    let tenant_uuid: uuid::Uuid =
        parse_uuid_v4(tenant_id).with_context(|| "Error parsing tenant_id as UUID")?;
    let election_event_uuid: uuid::Uuid = parse_uuid_v4(election_event_id)
        .with_context(|| "Error parsing election_event_id as UUID")?;
    let statement = hasura_transaction
        .prepare(
            r#"
            SELECT
                *
            FROM "sequent_backend".scheduled_event
            WHERE
                tenant_id = $1
                AND election_event_id = $2
                AND archived_at IS NULL
            "#,
        )
        .await
        .map_err(|err| anyhow!("Error running the find_scheduled_event_by_task_id query: {err}"))?;

    let rows: Vec<Row> = hasura_transaction
        .query(&statement, &[&tenant_uuid, &election_event_uuid])
        .await
        .map_err(|err| anyhow!("Error running the find_scheduled_event_by_task_id query: {err}"))?;

    info!("rows: {:?}", rows);

    let scheduled_events = rows
        .into_iter()
        .map(|row| -> Result<ScheduledEvent> {
            row.try_into()
                .map(|res: ScheduledEventWrapper| -> ScheduledEvent { res.0 })
        })
        .collect::<Result<Vec<ScheduledEvent>>>()
        .with_context(|| "Error converting rows into ScheduledEvent")?;

    Ok(scheduled_events)
}

#[instrument(skip(hasura_transaction), err)]
pub async fn find_scheduled_event_by_election_event_id_and_event_processor(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    event_processor: &str,
) -> Result<Vec<ScheduledEvent>> {
    let tenant_uuid: uuid::Uuid =
        parse_uuid_v4(tenant_id).with_context(|| "Error parsing tenant_id as UUID")?;
    let election_event_uuid: uuid::Uuid = parse_uuid_v4(election_event_id)
        .with_context(|| "Error parsing election_event_id as UUID")?;
    let statement = hasura_transaction
        .prepare(
            r#"
            SELECT
                *
            FROM "sequent_backend".scheduled_event
            WHERE
                tenant_id = $1
                AND election_event_id = $2
                AND event_processor = $3
                AND archived_at IS NULL
            "#,
        )
        .await
        .map_err(|err| anyhow!("Error running the find_scheduled_event_by_task_id query: {err}"))?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[&tenant_uuid, &election_event_uuid, &event_processor],
        )
        .await
        .map_err(|err| anyhow!("Error running the find_scheduled_event_by_task_id query: {err}"))?;

    info!("rows: {:?}", rows);

    let scheduled_events = rows
        .into_iter()
        .map(|row| -> Result<ScheduledEvent> {
            row.try_into()
                .map(|res: ScheduledEventWrapper| -> ScheduledEvent { res.0 })
        })
        .collect::<Result<Vec<ScheduledEvent>>>()
        .with_context(|| "Error converting rows into ScheduledEvent")?;

    Ok(scheduled_events)
}

#[instrument(skip(hasura_transaction), err)]
pub async fn insert_new_scheduled_event(
    hasura_transaction: &Transaction<'_>,
    new_event: ScheduledEvent,
) -> Result<ScheduledEvent> {
    let tenant_uuid: Option<uuid::Uuid> = match new_event.tenant_id {
        Some(ref tenant_id) => {
            Some(parse_uuid_v4(tenant_id).with_context(|| "Error parsing tenant_id as UUID")?)
        }
        None => None,
    };
    let election_event_uuid: Option<uuid::Uuid> = match new_event.election_event_id {
        Some(ref election_event_id) => Some(
            parse_uuid_v4(election_event_id)
                .with_context(|| "Error parsing election_event_id as UUID")?,
        ),
        None => None,
    };
    let cron_config_js: Option<Value> = new_event
        .cron_config
        .map(|config| serde_json::to_value(config))
        .transpose()?;
    let event_processor_s: Option<String> = new_event
        .event_processor
        .map(|processor| processor.to_string());

    let statement = hasura_transaction
        .prepare(
            r#"
                INSERT INTO
                    "sequent_backend".scheduled_event
                (
                    id,
                    tenant_id,
                    election_event_id,
                    created_at,
                    stopped_at,
                    archived_at,
                    labels,
                    annotations,
                    event_processor,
                    cron_config,
                    event_payload,
                    task_id
                )
                VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12
                )
                RETURNING
                    *
            "#,
        )
        .await
        .map_err(|err| {
            anyhow!(
                "Error preparing insert_new_scheduled_event statement: {}",
                err
            )
        })?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &parse_uuid_v4(&new_event.id).with_context(|| "Error parsing id as UUID")?,
                &tenant_uuid,
                &election_event_uuid,
                &new_event.created_at,
                &new_event.stopped_at,
                &new_event.archived_at,
                &new_event.labels,
                &new_event.annotations,
                &event_processor_s,
                &cron_config_js,
                &new_event.event_payload,
                &new_event.task_id,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error inserting new scheduled event: {}", err))?;

    let rows: Vec<ScheduledEvent> = rows
        .into_iter()
        .map(|row| -> Result<ScheduledEvent> {
            row.try_into()
                .map(|res: ScheduledEventWrapper| -> ScheduledEvent { res.0 })
        })
        .collect::<Result<Vec<ScheduledEvent>>>()
        .map_err(|err| anyhow!("Error deserializing new scheduled event: {}", err))?;

    if rows.len() == 1 {
        Ok(rows[0].clone())
    } else {
        Err(anyhow!(
            "Unexpected number of rows affected: {}",
            rows.len()
        ))
    }
}

/// Sets (`Some`) or removes (`None`) one key of a scheduled event's
/// annotations. Annotations that aren't a JSON object are left alone.
#[instrument(skip(hasura_transaction, value), err)]
pub async fn set_scheduled_event_annotation(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    id: &str,
    key: &str,
    value: Option<Value>,
) -> Result<()> {
    let tenant_uuid: uuid::Uuid =
        parse_uuid_v4(tenant_id).with_context(|| "Error parsing tenant_id as UUID")?;
    let id_uuid: uuid::Uuid =
        parse_uuid_v4(id).with_context(|| "Error parsing scheduled event id as UUID")?;
    hasura_transaction
        .execute(
            r#"
            UPDATE "sequent_backend".scheduled_event
            SET annotations = CASE
                WHEN $4::jsonb IS NULL THEN annotations - $3::text
                ELSE COALESCE(annotations, '{}'::jsonb) || jsonb_build_object($3::text, $4::jsonb)
            END
            WHERE tenant_id = $1
              AND id = $2
              AND (annotations IS NULL OR jsonb_typeof(annotations) = 'object')
              AND ($4::jsonb IS NOT NULL OR annotations ? $3::text)
            "#,
            &[&tenant_uuid, &id_uuid, &key, &value],
        )
        .await
        .map_err(|err| anyhow!("Error setting a scheduled event annotation: {err}"))?;
    Ok(())
}

/// Serializes scheduling/signing before locking mutable rows. The parent
/// row stays locked too, so archiving cannot race an executed transition.
#[instrument(skip(hasura_transaction), err)]
pub async fn lock_scheduling_event(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<()> {
    let tenant_uuid = parse_uuid_v4(tenant_id)?;
    let event_uuid = parse_uuid_v4(election_event_id)?;
    crate::postgres::signing::lock_signing_event(hasura_transaction, tenant_uuid, event_uuid)
        .await?;
    hasura_transaction
        .query(
            "SELECT id FROM sequent_backend.election_event
             WHERE tenant_id = $1 AND id = $2 FOR UPDATE",
            &[&tenant_uuid, &event_uuid],
        )
        .await
        .context("Error locking the scheduling event")?;
    Ok(())
}

/// Keeps the queued task's schedule unchanged between authorization and
/// its effect, including changes sent directly through Hasura.
#[instrument(skip(hasura_transaction), err)]
pub async fn lock_scheduled_event(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    scheduled_event_id: &str,
) -> Result<()> {
    lock_scheduling_event(hasura_transaction, tenant_id, election_event_id).await?;
    hasura_transaction
        .query(
            "SELECT id FROM sequent_backend.scheduled_event
             WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3 FOR UPDATE",
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
                &parse_uuid_v4(scheduled_event_id)?,
            ],
        )
        .await
        .context("Error locking the scheduled row")?;
    Ok(())
}

/// Locks the election rows of the event (or one election) until the end of
/// the transaction, so a read-modify-write of their status or presentation
/// doesn't lose a concurrent one. Statements after it see the latest
/// committed rows.
#[instrument(skip(hasura_transaction), err)]
pub async fn lock_elections(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: Option<&str>,
) -> Result<()> {
    let election_uuid = election_id.map(parse_uuid_v4).transpose()?;
    lock_scheduling_event(hasura_transaction, tenant_id, election_event_id).await?;
    hasura_transaction
        .query(
            r#"
            SELECT id FROM "sequent_backend".election
            WHERE tenant_id = $1 AND election_event_id = $2 AND ($3::uuid IS NULL OR id = $3)
            ORDER BY id
            FOR UPDATE
            "#,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
                &election_uuid,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error locking elections: {err}"))?;
    Ok(())
}

#[instrument(skip(hasura_transaction), err)]
pub async fn rename_scheduled_event_task(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    id: &str,
    task_id: &str,
) -> Result<()> {
    hasura_transaction.execute(
        "UPDATE sequent_backend.scheduled_event SET task_id = $3 WHERE tenant_id = $1 AND id = $2 AND archived_at IS NULL AND task_id IS DISTINCT FROM $3",
        &[&parse_uuid_v4(tenant_id)?, &parse_uuid_v4(id)?, &task_id],
    ).await?;
    Ok(())
}
