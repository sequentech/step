// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Run with scripts/postgres/demo_signing_installation/test-canonical-reload.sh.
//! Exercises the actual request writer and typed row reader across a commit,
//! not a JSON fixture or a reconstructed in-memory approval.

use super::matches_canonical_request;
use crate::postgres::signing::{get_signing_request, insert_signing_request, NewSigningRequest};
use anyhow::{ensure, Context, Result};
use chrono::{TimeZone, Timelike, Utc};
use deadpool_postgres::{Manager, Pool};
use sequent_core::signing::{sha256_hex, SigningAction, SigningPayload, SigningPayloadFields};
use serde_json::json;
use std::path::PathBuf;
use tokio_postgres::NoTls;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires the dedicated socket-only disposable PostgreSQL fixture"]
async fn committed_canonical_request_is_the_reviewed_request() -> Result<()> {
    let socket = PathBuf::from(std::env::var("SIGNING_CANONICAL_TEST_SOCKET")?);
    ensure!(
        socket.is_absolute()
            && socket.canonicalize()? == socket
            && socket
                .file_name()
                .and_then(|v| v.to_str())
                .is_some_and(|v| v.starts_with("demo-canonical."))
    );
    let mut config = tokio_postgres::Config::new();
    config
        .host_path(&socket)
        .port(46337)
        .user("canonical_fixture")
        .dbname("canonical_request_14990");
    let pool = Pool::builder(Manager::new(config, NoTls))
        .max_size(2)
        .build()?;
    let mut writer = pool.get().await?;
    let mut reader = pool.get().await?;
    let env_ok: bool = writer
        .query_one(
            "SELECT current_database()='canonical_request_14990' AND inet_client_addr() IS NULL",
            &[],
        )
        .await?
        .get(0);
    ensure!(env_ok);
    let id = Uuid::from_u128;
    let (tenant, event, post, country) = (id(1), id(2), id(3), id(4));
    let tx = writer.transaction().await?;
    tx.execute(
        "INSERT INTO sequent_backend.tenant(id,slug) VALUES ($1,'canonical-fixture')",
        &[&tenant],
    )
    .await?;
    tx.execute("INSERT INTO sequent_backend.election_event(id,tenant_id,encryption_protocol) VALUES ($1,$2,'RSA256')",&[&event,&tenant]).await?;
    tx.execute(
        "INSERT INTO sequent_backend.election(id,tenant_id,election_event_id) VALUES ($1,$2,$3)",
        &[&post, &tenant, &event],
    )
    .await?;
    tx.execute(
        "INSERT INTO sequent_backend.area(id,tenant_id,election_event_id) VALUES ($1,$2,$3)",
        &[&country, &tenant, &event],
    )
    .await?;
    tx.commit().await?;

    let created = Utc
        .timestamp_opt(1_900_000_000, 987_654_321)
        .single()
        .unwrap();
    for (n, area, revision) in [
        (10, None, None),
        (11, Some(country), None),
        (12, None, Some("r1")),
        (13, Some(country), Some("r2")),
    ] {
        let req = id(n);
        let tx = writer.transaction().await?;
        let fields = SigningPayloadFields {
            tenant_id: tenant.to_string(),
            election_event_id: event.to_string(),
            request_id: req,
            action: SigningAction::InitializeVoting,
            election_id: Some(post.to_string()),
            area_id: area.map(|v| v.to_string()),
            subject: json!({"publication_id":id(21),"title":"José"}),
            config_revision: revision.map(str::to_owned),
            rule_revision: 7,
            requested_by: id(20).to_string(),
            created_at: created,
            expires_at: Some(created + chrono::Duration::minutes(15)),
        };
        let payload = SigningPayload::new(fields.clone())?;
        let canonical = payload.canonical()?;
        let digest = sha256_hex(canonical.as_bytes());
        let to_store = NewSigningRequest {
            id: req,
            tenant_id: tenant,
            election_event_id: event,
            action: fields.action,
            election_id: Some(post),
            area_id: area,
            trustee_id: None,
            scope_key: format!("post-country-canonical-{n}"),
            subject: fields.subject,
            canonical_payload: canonical.clone(),
            payload_sha256: digest.clone(),
            document_id: None,
            document_sha256: None,
            code: payload.code().to_owned(),
            config_revision: fields.config_revision,
            rule_revision: fields.rule_revision,
            rule_snapshot: json!({}),
            required: 2,
            requested_by: fields.requested_by,
            requested_by_username: "synthetic".to_owned(),
            requested_by_name: None,
            expires_at: fields.expires_at,
            permission_label: None,
            created_at: created,
        };
        ensure!(insert_signing_request(&tx, &to_store).await?.is_some());
        tx.commit().await?;
        let r = reader.transaction().await?;
        let actual = get_signing_request(&r, tenant, event, req)
            .await?
            .context("missing committed request")?;
        ensure!(
            actual.canonical_payload == canonical
                && actual.payload_sha256 == digest
                && actual.created_at.nanosecond() == 987_654_000
                && matches_canonical_request(&actual)
        );
        r.commit().await?;
        // Mutate only execution-time scope in the *database*, retain the
        // originally reviewed bytes and hash. Reload must fail closed.
        let changed = writer.transaction().await?;
        changed
            .execute(
                "UPDATE sequent_backend.signing_request SET config_revision=$2 WHERE id=$1",
                &[&req, &"changed-after-review"],
            )
            .await?;
        changed.commit().await?;
        let r = reader.transaction().await?;
        let altered = get_signing_request(&r, tenant, event, req).await?.unwrap();
        ensure!(
            altered.canonical_payload == canonical
                && altered.payload_sha256 == digest
                && !matches_canonical_request(&altered)
        );
        r.commit().await?;
    }
    Ok(())
}
