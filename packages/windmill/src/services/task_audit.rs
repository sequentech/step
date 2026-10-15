// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The audit entries of what tasks do for an admin: an import, an export or
//! a bulk change is one entry, with its counts and its file's SHA-256.
//!
//! The route that queues the task passes the admin as an optional task
//! input, so a message queued before the input existed still runs: its
//! entry has no actor.

use crate::postgres::audit_log_outbox::AuditLogTarget;
use crate::services::audit_log::{
    stage, AdminAction, AdminChange, AuditActor, AuditObject, AuditOrigin,
};
use crate::services::providers::transactions_provider::provide_hasura_transaction;
use crate::tasks::audit_log_outbox::kick_audit_log_outbox;
use anyhow::{Context, Result};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::io::{Read, Seek, SeekFrom};
use tracing::instrument;
use uuid::Uuid;

/// The id of an object that stands for several: the rows of an import or an
/// export.
pub const MANY: &str = "*";

pub const TENANT: &str = "tenant";
pub const TEMPLATE: &str = "template";

/// The election event's log, or the tenant's without one. An empty event id
/// is no event, as task inputs carry it.
pub fn target(tenant_id: &str, election_event_id: Option<&str>) -> Result<AuditLogTarget> {
    Ok(AuditLogTarget {
        tenant_id: Uuid::parse_str(tenant_id).context("Invalid tenant id")?,
        election_event_id: election_event_id
            .filter(|id| !id.is_empty())
            .map(|id| Uuid::parse_str(id).context("Invalid election event id"))
            .transpose()?,
    })
}

/// A change a task makes for `actor`.
pub fn change(
    actor: Option<AuditActor>,
    target: AuditLogTarget,
    object: AuditObject,
    action: AdminAction,
) -> AdminChange {
    AdminChange::new(
        target,
        object,
        action,
        actor.unwrap_or_default(),
        AuditOrigin::Windmill,
    )
}

/// An import, an export or a bulk change of `kind` objects: one entry that
/// stands for all of them, with its facts in `extra`.
pub fn bulk(
    actor: Option<AuditActor>,
    target: AuditLogTarget,
    kind: &str,
    action: AdminAction,
    description: impl Into<String>,
    extra: Value,
) -> AdminChange {
    change(actor, target, AuditObject::new(kind, MANY, None), action)
        .with_extra(extra)
        .with_description(description)
}

/// A tenant's configuration exported to a document or imported from one.
pub fn tenant_config(
    actor: Option<AuditActor>,
    tenant_id: &str,
    action: AdminAction,
    extra: Value,
) -> Result<AdminChange> {
    Ok(change(
        actor,
        target(tenant_id, None)?,
        AuditObject::new(TENANT, tenant_id, None),
        action,
    )
    .with_extra(extra)
    .with_description(match action {
        AdminAction::Import => "Imported the tenant configuration",
        _ => "Exported the tenant configuration",
    }))
}

/// The document an import read, with the checksum its admin declared, when
/// they did.
pub fn source_document(document_id: &str, declared_sha256: Option<&str>) -> Value {
    let mut source = serde_json::json!({"document_id": document_id});
    if let Some(sha256) = declared_sha256.filter(|sha256| !sha256.is_empty()) {
        source["declared_sha256"] = Value::String(sha256.to_lowercase());
    }
    source
}

/// The data rows of a delimited file with a header row. The file is left at
/// its start.
pub fn csv_rows<F: Read + Seek>(file: &mut F, separator: u8) -> Result<usize> {
    file.seek(SeekFrom::Start(0))?;
    let rows = csv::ReaderBuilder::new()
        .delimiter(separator)
        .has_headers(true)
        .flexible(true)
        .from_reader(&mut *file)
        .records()
        .filter(|record| record.is_ok())
        .count();
    file.seek(SeekFrom::Start(0))?;
    Ok(rows)
}

/// Queues the entry of work a task finished outside one transaction, and
/// asks a worker to post it.
#[instrument(skip_all, err)]
pub async fn record(change: AdminChange) -> Result<()> {
    provide_hasura_transaction(|hasura_transaction| {
        Box::pin(async move {
            stage(hasura_transaction, &change).await?;
            Ok(())
        })
    })
    .await?;
    kick_audit_log_outbox();
    Ok(())
}

/// The SHA-256 of a whole file, in hex. The file is left at its start.
pub fn file_sha256<F: Read + Seek>(file: &mut F) -> Result<String> {
    file.seek(SeekFrom::Start(0))?;
    let mut hasher = Sha256::new();
    std::io::copy(file, &mut hasher)?;
    file.seek(SeekFrom::Start(0))?;
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    const TENANT_ID: &str = "4f1c6b38-9f5e-4a59-8d3b-2a7f0c1e5d61";
    const EVENT_ID: &str = "b7e2d915-3c4a-4e8f-9a61-5d0f2c8b7e43";

    #[test]
    fn an_empty_or_absent_event_id_is_the_tenant_log() {
        for event in [None, Some("")] {
            assert_eq!(target(TENANT_ID, event).unwrap().election_event_id, None);
        }
        assert_eq!(
            target(TENANT_ID, Some(EVENT_ID)).unwrap().election_event_id,
            Some(Uuid::parse_str(EVENT_ID).unwrap())
        );
        assert!(target("tenant", None).is_err());
        assert!(target(TENANT_ID, Some("event")).is_err());
    }

    #[test]
    fn a_task_without_its_admin_writes_a_system_entry() {
        let object = AuditObject::new("template", MANY, None);
        let target = target(TENANT_ID, None).unwrap();
        let unknown = change(None, target, object.clone(), AdminAction::Import);
        assert_eq!(unknown.actor, AuditActor::default());
        assert_eq!(unknown.origin, AuditOrigin::Windmill);

        let admin = AuditActor {
            user_id: Some("user".into()),
            username: Some("ada".into()),
            role: None,
        };
        let known = change(Some(admin.clone()), target, object, AdminAction::Import);
        assert_eq!(known.actor, admin);
    }

    #[test]
    fn a_bulk_entry_stands_for_many_objects_and_carries_its_facts() {
        let entry = bulk(
            None,
            target(TENANT_ID, Some(EVENT_ID)).unwrap(),
            "voter",
            AdminAction::Import,
            "Imported voters",
            serde_json::json!({"count": 2, "file_sha256": "ab"}),
        );
        assert_eq!(entry.object, AuditObject::new("voter", "*", None));
        assert_eq!(entry.describe(), "Imported voters");
        assert_eq!(
            entry.details().unwrap()["extra"],
            serde_json::json!({"count": 2, "file_sha256": "ab"})
        );
    }

    #[test]
    fn a_tenant_configuration_transfer_is_an_entry_about_the_tenant() {
        let exported = tenant_config(
            None,
            TENANT_ID,
            AdminAction::Export,
            serde_json::json!({"document_id": "document-1"}),
        )
        .unwrap();
        assert_eq!(exported.target.election_event_id, None);
        assert_eq!(exported.object, AuditObject::new(TENANT, TENANT_ID, None));
        assert_eq!(exported.describe(), "Exported the tenant configuration");

        let imported = tenant_config(
            None,
            TENANT_ID,
            AdminAction::Import,
            source_document("document-2", Some("ABCD")),
        )
        .unwrap();
        assert_eq!(imported.action, AdminAction::Import);
        assert_eq!(imported.describe(), "Imported the tenant configuration");
        assert_eq!(
            imported.extra,
            serde_json::json!({"document_id": "document-2", "declared_sha256": "abcd"})
        );
        assert!(tenant_config(None, "tenant", AdminAction::Export, Value::Null).is_err());
    }

    #[test]
    fn an_undeclared_checksum_is_left_out_of_the_source() {
        for undeclared in [None, Some("")] {
            assert_eq!(
                source_document("document-1", undeclared),
                serde_json::json!({"document_id": "document-1"})
            );
        }
    }

    #[test]
    fn the_rows_of_a_delimited_file_exclude_its_header() {
        let mut csv =
            Cursor::new(b"username,email\nada,a@x.test\n\"multi\nline\",b@x.test\n".to_vec());
        assert_eq!(csv_rows(&mut csv, b',').unwrap(), 2);
        assert_eq!(csv.position(), 0);

        let mut tsv = Cursor::new(b"username\temail\nada\ta@x.test\n".to_vec());
        assert_eq!(csv_rows(&mut tsv, b'\t').unwrap(), 1);
        assert_eq!(csv_rows(&mut Cursor::new(Vec::new()), b',').unwrap(), 0);
    }

    #[test]
    fn a_file_is_hashed_whole_and_left_at_its_start() {
        let mut file = Cursor::new(b"abc".to_vec());
        file.set_position(2);
        assert_eq!(
            file_sha256(&mut file).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(file.position(), 0);
    }
}
