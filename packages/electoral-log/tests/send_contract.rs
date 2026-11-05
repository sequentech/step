// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Audit operations must be usable by the backend's Send task executors.
use electoral_log::{BoardClient, LogQuery};
fn require_send<T: Send>(_: T) {}
fn require_send_type<T: Send>() {}
#[test]
fn postgres_audit_futures_can_run_in_worker_tasks() {
    require_send_type::<BoardClient>();
    let _operations = |client: BoardClient| {
        require_send(async move {
            client.append("test", &[]).await?;
            client.query("test", &LogQuery::default()).await?;
            client.count("test", &LogQuery::default()).await?;
            anyhow::Ok(())
        });
    };
}
