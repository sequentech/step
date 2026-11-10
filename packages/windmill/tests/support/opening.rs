// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! What opening voting needs besides the event's own configuration, written
//! as the services write it: a current observation of the external request
//! log (attack heuristics), a live lease of both services (their heartbeats),
//! a voting period around now (the event's start and end rows) and a passed
//! self-test of the event's current generation. Include it with
//! `#[path = "support/opening.rs"] mod opening;`.

#![allow(dead_code)]

use chrono::{Duration, SecondsFormat, Utc};
use deadpool_postgres::Transaction;
use sequent_core::self_tests::{
    SelfTestCheck, SelfTestCheckResult, SelfTestOutcome, SelfTestRun, SelfTestTrigger,
};
use sequent_core::types::scheduled_event::{generate_manage_date_task_name, EventProcessors};
use serde_json::json;
use uuid::Uuid;

/// How long a heartbeat keeps a service's lease.
const LEASE_SECONDS: i64 = 45;

/// Lets an opening of `event` through the lifecycle gate, the self-test being
/// the one `trigger` names. Call it right before the opening: the observation
/// and the leases go stale within a minute. An event that already has its
/// start and end rows keeps them.
pub async fn ready_to_open(
    tx: &Transaction<'_>,
    tenant: Uuid,
    event: Uuid,
    trigger: SelfTestTrigger,
) {
    tx.execute(
        "INSERT INTO sequent_backend.attack_activity_health(source,available,checked_at,reason)
         VALUES('external-nginx',true,clock_timestamp(),'External request log is current')
         ON CONFLICT(source) DO UPDATE
         SET available=true,checked_at=clock_timestamp(),reason=EXCLUDED.reason",
        &[],
    )
    .await
    .unwrap();
    for component in ["harvest", "windmill"] {
        tx.execute(
            "INSERT INTO sequent_backend.lifecycle_service_lease(component,expires_at)
             VALUES($1,clock_timestamp()+($2::bigint*interval '1 second'))
             ON CONFLICT(component) DO UPDATE SET expires_at=EXCLUDED.expires_at",
            &[&component, &LEASE_SECONDS],
        )
        .await
        .unwrap();
    }
    let (tenant_id, event_id) = (tenant.to_string(), event.to_string());
    for (processor, date) in [
        (
            EventProcessors::START_VOTING_PERIOD,
            Utc::now() - Duration::hours(1),
        ),
        (
            EventProcessors::END_VOTING_PERIOD,
            Utc::now() + Duration::hours(1),
        ),
    ] {
        let task = generate_manage_date_task_name(&tenant_id, &event_id, None, &processor);
        tx.execute(
            "INSERT INTO sequent_backend.scheduled_event
                 (tenant_id, election_event_id, event_processor, task_id, cron_config,
                  event_payload)
             SELECT $1::uuid, $2::uuid, $3::text, $4::text, $5::jsonb, $6::jsonb
             WHERE NOT EXISTS (
                 SELECT 1 FROM sequent_backend.scheduled_event
                 WHERE tenant_id = $1 AND election_event_id = $2 AND task_id = $4
                     AND archived_at IS NULL
             )",
            &[
                &tenant,
                &event,
                &processor.to_string(),
                &task,
                &json!({ "scheduled_date": date.to_rfc3339_opts(SecondsFormat::Secs, true) }),
                &json!({ "election_id": null }),
            ],
        )
        .await
        .unwrap();
    }
    let generation: i64 = tx
        .query_one(
            "SELECT generation FROM sequent_backend.election_lifecycle
             WHERE tenant_id = $1 AND election_event_id = $2",
            &[&tenant, &event],
        )
        .await
        .unwrap()
        .get(0);
    let id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();
    let run = SelfTestRun {
        run_id: id.to_string(),
        generation,
        trigger,
        started_at: now.clone(),
        finished_at: Some(now),
        outcome: SelfTestOutcome::Passed,
        checks: SelfTestCheck::ALL
            .into_iter()
            .map(|check| SelfTestCheckResult {
                check,
                outcome: SelfTestOutcome::Passed,
                detail: "Passed".into(),
            })
            .collect(),
    };
    let trigger = serde_json::to_value(trigger).unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.self_test_run
             (id, tenant_id, election_event_id, generation, trigger, result, finished_at,
              outcome)
         VALUES ($1, $2, $3, $4, $5, $6, clock_timestamp(), 'passed')",
        &[
            &id,
            &tenant,
            &event,
            &generation,
            &trigger.as_str().unwrap(),
            &serde_json::to_value(&run).unwrap(),
        ],
    )
    .await
    .unwrap();
}
