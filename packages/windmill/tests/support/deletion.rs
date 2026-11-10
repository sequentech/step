// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! What deleting an election event needs besides the event itself, written
//! as the deletion task holds it: an export of the event's current data and
//! the board's executed approval, bound to the deletion. Include it with
//! `#[path = "support/deletion.rs"] mod deletion;`.

#![allow(dead_code)]

use deadpool_postgres::Transaction;
use serde_json::json;
use uuid::Uuid;

/// Archives `event`, which must still be in Preparation, and gives this
/// transaction what the deletion task holds when it deletes the event: an
/// export of its current data and the board's executed approval, bound to
/// the deletion and named by `sequent.authorized_deletion`. Call it right
/// before the deletion, in the same transaction: a later write changes the
/// data the export covers.
pub async fn authorize_deletion(tx: &Transaction<'_>, tenant: Uuid, event: Uuid) {
    windmill::postgres::trusted_write(tx).await.unwrap();
    tx.execute(
        "UPDATE sequent_backend.election_event SET is_archived = true
         WHERE tenant_id = $1 AND id = $2",
        &[&tenant, &event],
    )
    .await
    .unwrap();
    let current = tx
        .query_one(
            "SELECT l.generation, v.revision
             FROM sequent_backend.election_lifecycle l
             JOIN sequent_backend.election_execution_revision v
                 ON v.tenant_id = l.tenant_id AND v.election_event_id = l.election_event_id
             WHERE l.tenant_id = $1 AND l.election_event_id = $2",
            &[&tenant, &event],
        )
        .await
        .unwrap();
    let (generation, revision): (i64, i64) = (current.get(0), current.get(1));
    let request = Uuid::new_v4();
    tx.execute(
        "INSERT INTO sequent_backend.signing_request
             (id, tenant_id, election_event_id, action, scope_key, subject,
              canonical_payload, payload_sha256, code, rule_revision, rule_snapshot,
              required, status, requested_by, requested_by_username, completed_at,
              executed_at)
         VALUES ($1, $2, $3, 'delete-execution-data', 'deletion', $4, '{}',
                 repeat('0', 64), 'code', 1, '{}', 1, 'executed', 'admin', 'admin',
                 now(), now())",
        &[
            &request,
            &tenant,
            &event,
            &json!({"lifecycle_generation": generation, "source_revision": revision}),
        ],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.execution_export
             (tenant_id, election_event_id, document_id, request_id, source_revision,
              lifecycle_generation, manifest_sha256, archive_sha256, signing_public_key)
         VALUES ($1, $2, $3, $4, $5, $6, repeat('1', 64), repeat('2', 64), 'key')",
        &[
            &tenant,
            &event,
            &Uuid::new_v4(),
            &Uuid::new_v4(),
            &revision,
            &generation,
        ],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.board_operation_binding
             (tenant_id, election_event_id, request_id, binding_kind, binding_id)
         VALUES ($1, $2, $3, 'deletion', $4)",
        &[&tenant, &event, &request, &Uuid::new_v4()],
    )
    .await
    .unwrap();
    tx.execute(
        "SELECT set_config('sequent.authorized_deletion', $1, true)",
        &[&request.to_string()],
    )
    .await
    .unwrap();
}
