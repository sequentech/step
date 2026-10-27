// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Durable, distinct email notifications to the election board and administrators.

use super::database::{get_hasura_pool, get_keycloak_pool};
use super::providers::email_sender::{EmailSender, TransportFailure};
use anyhow::{ensure, Context, Result};
use deadpool_postgres::Transaction;
use std::time::Duration;
use uuid::Uuid;

/// Queue an email in the caller's state-change transaction. Never contact an
/// external transport while holding the transaction that protects voting.
pub async fn queue_notice(
    tx: &Transaction<'_>,
    tenant: &str,
    event: &str,
    kind: &str,
    details: &str,
) -> Result<Uuid> {
    let id = Uuid::new_v4();
    tx.execute(
        "INSERT INTO sequent_backend.security_notification(id,tenant_id,election_event_id,kind,details) VALUES($1,$2,$3,$4,$5)",
        &[&id, &Uuid::parse_str(tenant)?, &Uuid::parse_str(event)?, &kind, &details],
    ).await?;
    Ok(id)
}

/// Resolve effective staff permissions, including role composites and inherited
/// groups. Disabled people, voters and service accounts cannot become recipients.
async fn recipients(tenant: Uuid) -> Result<Vec<String>> {
    let db = get_keycloak_pool().await.get().await?;
    let realm = format!("tenant-{tenant}");
    let rows = db.query(
        "WITH RECURSIVE target_realm AS (SELECT id FROM realm WHERE name=$1),
         eligible_roles AS (
           SELECT id FROM keycloak_role WHERE realm_id IN (SELECT id FROM target_realm)
             AND NOT client_role AND name IN ('tenant-write','election-state-write')
           UNION SELECT c.composite FROM composite_role c JOIN eligible_roles r ON c.child_role=r.id
         ), eligible_groups AS (
           SELECT g.id FROM keycloak_group g JOIN group_role_mapping m ON m.group_id=g.id
             WHERE g.realm_id IN (SELECT id FROM target_realm) AND m.role_id IN (SELECT id FROM eligible_roles)
           UNION SELECT g.id FROM keycloak_group g JOIN eligible_groups parent ON g.parent_group=parent.id
         ) SELECT DISTINCT u.email FROM user_entity u
         WHERE u.realm_id IN (SELECT id FROM target_realm) AND u.enabled
           AND u.service_account_client_link IS NULL AND u.email IS NOT NULL AND trim(u.email)<>''
           AND (EXISTS(SELECT 1 FROM user_role_mapping m WHERE m.user_id=u.id AND m.role_id IN (SELECT id FROM eligible_roles))
             OR EXISTS(SELECT 1 FROM user_group_membership m WHERE m.user_id=u.id AND m.group_id IN (SELECT id FROM eligible_groups)))",
        &[&realm],
    ).await?;
    let addresses = rows
        .into_iter()
        .map(|row| row.try_get(0))
        .collect::<std::result::Result<Vec<String>, _>>()?;
    ensure!(
        !addresses.is_empty(),
        "No election-board or administrator notification email is configured"
    );
    Ok(addresses)
}

/// Deliver a small bounded batch. A crashed or unknown send is never silently
/// retried: it is marked uncertain for operator reconciliation, avoiding duplicates.
pub async fn dispatch_pending() -> Result<()> {
    let pool = get_hasura_pool().await;
    let db = pool.get().await?;
    db.execute("UPDATE sequent_backend.security_notification SET state='uncertain',failure='Delivery interrupted; reconcile with the mail server before retrying' WHERE state='sending' AND attempted_at<clock_timestamp()-interval '2 minutes'", &[]).await?;
    drop(db);
    for _ in 0..1 {
        let mut db = pool.get().await?;
        let tx = db.transaction().await?;
        let row = tx.query_opt(
            "SELECT id,tenant_id,election_event_id,kind,details,attempts FROM sequent_backend.security_notification WHERE state='pending' AND next_attempt_at<=clock_timestamp() ORDER BY created_at LIMIT 1 FOR UPDATE SKIP LOCKED", &[],
        ).await?;
        let Some(row) = row else {
            tx.commit().await?;
            break;
        };
        let id: Uuid = row.try_get(0)?;
        let tenant: Uuid = row.try_get(1)?;
        let event: Uuid = row.try_get(2)?;
        let kind: String = row.try_get(3)?;
        let details: String = row.try_get(4)?;
        let attempts: i32 = row.try_get::<_, i32>(5)? + 1;
        tx.execute("UPDATE sequent_backend.security_notification SET state='sending',attempted_at=clock_timestamp(),attempts=attempts+1 WHERE id=$1", &[&id]).await?;
        tx.commit().await?;
        drop(db);
        let prepared = tokio::time::timeout(Duration::from_secs(10), async {
            ensure!(
                std::env::var("EMAIL_TRANSPORT_NAME").as_deref() != Ok("Console"),
                "Security notifications require an actual email transport"
            );
            let to = recipients(tenant).await?;
            let sender = EmailSender::new()
                .await
                .context("Security notification transport unavailable")?;
            Ok::<_, anyhow::Error>((to, sender))
        })
        .await;
        let (state, failure) = match prepared {
            Ok(Ok((to, sender))) => {
                let body = format!("Election event: {event}\nSecurity event: {kind}\nNotification: {id}\n\n{details}\n\nReview the event's security status. Follow the administrator and election board guidance before resuming election operations.");
                match tokio::time::timeout(
                    Duration::from_secs(20),
                    sender.send_message(
                        to,
                        format!("Election security notification: {kind}"),
                        body,
                        None,
                        vec![],
                    ),
                )
                .await
                {
                    Ok(Ok(())) => ("sent", None),
                    Ok(Err(error)) => (
                        delivery_failure_state(&error, attempts),
                        Some(error.to_string()),
                    ),
                    Err(_) => (
                        "uncertain",
                        Some("Email transport timed out; reconcile with the mail server".into()),
                    ),
                }
            }
            Ok(Err(error)) => (retry_state(attempts), Some(error.to_string())),
            Err(_) => (
                retry_state(attempts),
                Some("Notification recipient/transport preparation timed out".into()),
            ),
        };
        let db = pool.get().await?;
        db.execute("UPDATE sequent_backend.security_notification SET state=$2,failure=$3,next_attempt_at=clock_timestamp()+interval '60 seconds',delivered_at=CASE WHEN $2='sent' THEN clock_timestamp() ELSE NULL END WHERE id=$1 AND state='sending'", &[&id,&state,&failure]).await?;
    }
    Ok(())
}

fn retry_state(attempts: i32) -> &'static str {
    if attempts < 6 {
        "pending"
    } else {
        "failed"
    }
}

fn delivery_failure_state(error: &TransportFailure, attempts: i32) -> &'static str {
    match error {
        TransportFailure::Refused {
            transient: true, ..
        } => retry_state(attempts),
        TransportFailure::Refused {
            transient: false, ..
        } => "failed",
        TransportFailure::Unknown { .. } => "uncertain",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_definitely_refused_transient_deliveries_are_retried() {
        let temporary = TransportFailure::Refused {
            transient: true,
            reason: "deferred".into(),
        };
        assert_eq!(delivery_failure_state(&temporary, 1), "pending");
        assert_eq!(delivery_failure_state(&temporary, 6), "failed");
        assert_eq!(
            delivery_failure_state(
                &TransportFailure::Refused {
                    transient: false,
                    reason: "rejected".into()
                },
                1
            ),
            "failed"
        );
        assert_eq!(
            delivery_failure_state(
                &TransportFailure::Unknown {
                    reason: "lost answer".into()
                },
                1
            ),
            "uncertain"
        );
    }
}
