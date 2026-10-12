// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Audit operations must be usable by the backend's Send task executors.
use electoral_log::{AuditQuery, BoardClient};
fn require_send<T: Send>(_: T) {}
#[test]
fn immudb_audit_futures_can_run_in_worker_tasks() {
    require_send(BoardClient::new(
        "http://unused:3322",
        "synthetic",
        "synthetic",
    ));
    let _operations = |mut client: BoardClient| {
        require_send(async move {
            client.append_deliveries("test", &[]).await?;
            client
                .query_messages("test", &AuditQuery::default())
                .await?;
            client.count_query("test", None).await?;
            anyhow::Ok(())
        });
    };
}
