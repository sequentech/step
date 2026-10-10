// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::adapters::memory::user_tasks::{InMemoryUserTasks, UserTaskCall};
use crate::test_claims::Claims;
use sequent_core::types::permissions::Permissions::*;
use serde_json::json;

fn claims(roles: &[Permissions]) -> JwtClaims {
    Claims::new("tenant-a", "user-a")
        .username("operator-a")
        .roles(roles.iter().map(ToString::to_string))
        .build()
}

fn import_input(event_id: Option<&str>) -> ImportUsersBody {
    ImportUsersBody {
        tenant_id: "tenant-a".into(),
        document_id: "source-document".into(),
        election_event_id: event_id.map(str::to_owned),
        is_admin: false,
        may_write_secret_attributes: false,
        secret_write_initiator: None,
        sha256: Some("source-sha256".into()),
    }
}

fn export_input(event_id: Option<&str>, secrets: bool) -> ExportUsersBody {
    ExportUsersBody {
        tenant_id: "tenant-a".into(),
        election_event_id: event_id.map(str::to_owned),
        election_id: Some("election-a".into()),
        include_secret_attributes: secrets,
    }
}

async fn import(
    tasks: &InMemoryUserTasks,
    claims: JwtClaims,
    input: ImportUsersBody,
) -> UserTaskResult<ImportUsersOutput> {
    import_users_with(tasks, tasks, tasks, claims, input).await
}

async fn export(
    tasks: &InMemoryUserTasks,
    claims: JwtClaims,
    input: ExportUsersBody,
) -> UserTaskResult<ExportUsersOutput> {
    export_users_with(tasks, tasks, tasks, tasks, claims, input).await
}

#[tokio::test]
async fn imports_keep_the_existing_task_row_when_authorization_fails() {
    let tasks = InMemoryUserTasks::default();
    let error = import(&tasks, claims(&[]), import_input(Some("event-a")))
        .await
        .unwrap_err();
    assert_eq!(error.0, Status::Unauthorized);
    let state = tasks.snapshot();
    assert_eq!(state.tasks.len(), 1);
    assert_eq!(state.tasks[0].tenant_id, "tenant-a");
    assert_eq!(state.tasks[0].election_event_id.as_deref(), Some("event-a"));
    assert_eq!(state.tasks[0].task_type, "IMPORT_USERS");
    assert_eq!(state.tasks[0].execution_status, "IN_PROGRESS");
    assert_eq!(state.connection_count, 0);
    assert!(state.import_attempts.is_empty());
    assert!(state.failed_tasks.is_empty());
}

#[tokio::test]
async fn import_task_rows_use_the_claims_tenant_before_checking_the_body_tenant(
) {
    let tasks = InMemoryUserTasks::default();
    let mut input = import_input(Some("event-a"));
    input.tenant_id = "other-tenant".into();
    assert_eq!(
        import(&tasks, claims(&[VOTER_CREATE]), input)
            .await
            .unwrap_err()
            .0,
        Status::Unauthorized
    );
    assert_eq!(tasks.snapshot().tasks[0].tenant_id, "tenant-a");
    assert!(tasks.snapshot().import_attempts.is_empty());
}

#[tokio::test]
async fn import_ledger_failures_prevent_dispatch_and_keep_the_insertion_error_context(
) {
    let tasks = InMemoryUserTasks::default();
    tasks.fail_on(UserTaskCall::CreateImport, "ledger offline");
    let error = import(
        &tasks,
        claims(&[VOTER_CREATE]),
        import_input(Some("event-a")),
    )
    .await
    .unwrap_err();
    assert_eq!(error.0, Status::InternalServerError);
    assert!(error
        .1
        .starts_with("Failed to insert task execution record: ledger offline"));
    assert!(tasks.snapshot().tasks.is_empty());
    assert_eq!(tasks.snapshot().connection_count, 0);
}

#[tokio::test]
async fn tenant_imports_store_an_empty_event_and_force_the_admin_flag() {
    let tasks = InMemoryUserTasks::default();
    let mut claims = claims(&[USER_CREATE]);
    claims.name = Some("Display Operator".into());
    let output = import(&tasks, claims, import_input(None)).await.unwrap();
    assert_eq!(output.task_execution.id, "task-1");
    let state = tasks.snapshot();
    assert_eq!(state.tasks[0].election_event_id.as_deref(), Some(""));
    assert_eq!(state.tasks[0].executed_by_user, "Display Operator");
    assert_eq!(
        serde_json::to_value(&state.import_attempts[0].0).unwrap(),
        json!({
            "tenant_id":"tenant-a", "document_id":"source-document", "election_event_id":null,
            "is_admin":true, "may_write_secret_attributes":false, "secret_write_initiator":null,
            "sha256":"source-sha256"
        })
    );
    assert_eq!(state.import_attempts[0].1.id, "task-1");
}

#[tokio::test]
async fn an_empty_event_id_keeps_voter_authorization_and_the_existing_admin_flag(
) {
    let tasks = InMemoryUserTasks::default();
    import(&tasks, claims(&[VOTER_CREATE]), import_input(Some("")))
        .await
        .unwrap();
    assert!(tasks.snapshot().import_attempts[0].0.is_admin);
    let denied = InMemoryUserTasks::default();
    assert_eq!(
        import(&denied, claims(&[USER_CREATE]), import_input(Some("")))
            .await
            .unwrap_err()
            .0,
        Status::Unauthorized
    );
}

#[tokio::test]
async fn voter_imports_replace_untrusted_admin_and_secret_write_flags() {
    let tasks = InMemoryUserTasks::default();
    let mut input = import_input(Some("event-a"));
    input.is_admin = true;
    input.may_write_secret_attributes = true;
    input.secret_write_initiator = Some(ElectoralLogAdminContext {
        user_id: "forged-user".into(),
        username: Some("forged-name".into()),
        authorized_election_ids: None,
        area_id: None,
    });
    import(&tasks, claims(&[VOTER_CREATE]), input)
        .await
        .unwrap();
    let state = tasks.snapshot();
    assert!(!state.import_attempts[0].0.is_admin);
    assert!(!state.import_attempts[0].0.may_write_secret_attributes);
    assert!(state.import_attempts[0].0.secret_write_initiator.is_none());
    assert_eq!(state.tasks[0].executed_by_user, "user-a");
}

#[tokio::test]
async fn permitted_voter_imports_bind_secret_writes_to_the_authenticated_actor()
{
    let tasks = InMemoryUserTasks::default();
    import(
        &tasks,
        claims(&[VOTER_CREATE, VOTER_SECRET_ATTRIBUTE_WRITE]),
        import_input(Some("event-a")),
    )
    .await
    .unwrap();
    let state = tasks.snapshot();
    let input = &state.import_attempts[0].0;
    assert!(input.may_write_secret_attributes);
    let actor = input.secret_write_initiator.as_ref().unwrap();
    assert_eq!(actor.user_id, "user-a");
    assert_eq!(actor.username.as_deref(), Some("operator-a"));
}

#[tokio::test]
async fn import_broker_failures_still_return_the_in_progress_task() {
    let tasks = InMemoryUserTasks::default();
    tasks.fail_on(UserTaskCall::SendImport, "broker offline");
    let output = import(
        &tasks,
        claims(&[VOTER_CREATE]),
        import_input(Some("event-a")),
    )
    .await
    .unwrap();
    assert_eq!(output.task_execution.id, "task-1");
    assert_eq!(output.task_execution.execution_status, "IN_PROGRESS");
    assert_eq!(tasks.snapshot().import_attempts.len(), 1);
    assert!(tasks.snapshot().failed_tasks.is_empty());
}

#[tokio::test]
async fn denied_exports_do_not_allocate_documents_or_create_task_rows() {
    let tasks = InMemoryUserTasks::default();
    assert_eq!(
        export(&tasks, claims(&[]), export_input(Some("event-a"), false))
            .await
            .unwrap_err()
            .0,
        Status::Unauthorized
    );
    let state = tasks.snapshot();
    assert_eq!(state.document_count, 0);
    assert_eq!(state.connection_count, 0);
    assert!(state.tasks.is_empty());
}

#[tokio::test]
async fn tenant_exports_cannot_include_secret_attributes() {
    let tasks = InMemoryUserTasks::default();
    assert_eq!(export(&tasks, claims(&[USER_READ, VOTER_SECRET_ATTRIBUTE_READ]), export_input(None, true)).await.unwrap_err(),
        (Status::BadRequest, "Secret attributes can only be included in an election-event voter export".into()));
    assert_eq!(tasks.snapshot().document_count, 0);
    assert!(tasks.snapshot().tasks.is_empty());
}

#[tokio::test]
async fn decrypted_voter_exports_require_the_secret_read_permission_before_allocating_a_document(
) {
    let tasks = InMemoryUserTasks::default();
    assert_eq!(
        export(
            &tasks,
            claims(&[VOTER_READ]),
            export_input(Some("event-a"), true)
        )
        .await
        .unwrap_err()
        .0,
        Status::Unauthorized
    );
    assert_eq!(tasks.snapshot().document_count, 0);
    assert!(tasks.snapshot().audits.is_empty());
    assert!(tasks.snapshot().tasks.is_empty());
}

#[tokio::test]
async fn an_export_that_cannot_be_audited_creates_no_task_or_broker_connection()
{
    let tasks = InMemoryUserTasks::default();
    tasks.fail_on(UserTaskCall::Audit, "audit offline");
    assert_eq!(export(&tasks, claims(&[VOTER_READ, VOTER_SECRET_ATTRIBUTE_READ]), export_input(Some("event-a"), true)).await.unwrap_err(),
        (Status::InternalServerError, "Failed to record the secret-attribute electoral-log entry: audit offline".into()));
    assert_eq!(tasks.snapshot().document_count, 1);
    assert!(tasks.snapshot().tasks.is_empty());
    assert_eq!(tasks.snapshot().connection_count, 0);
}

#[tokio::test]
async fn decrypted_exports_bind_the_audit_grant_and_message_to_the_same_document(
) {
    let tasks = InMemoryUserTasks::default();
    let output = export(
        &tasks,
        claims(&[VOTER_READ, VOTER_SECRET_ATTRIBUTE_READ]),
        export_input(Some("event-a"), true),
    )
    .await
    .unwrap();
    assert_eq!(output.document_id, "document-1");
    assert!(output.error_msg.is_none());
    assert_eq!(output.task_execution.as_ref().unwrap().id, "task-1");
    let state = tasks.snapshot();
    assert_eq!(
        state.audits,
        vec![(
            "tenant-a".into(),
            "event-a".into(),
            "document-1".into(),
            "user-a".into()
        )]
    );
    assert_eq!(state.tasks[0].task_type, "EXPORT_VOTERS");
    assert_eq!(
        state.tasks[0].annotations,
        Some(
            json!({"secret_export_authorization": {"document_id":"document-1", "voter_secret_attributes":true, "expires_at":"2026-01-02T00:00:00Z"}})
        )
    );
    let sent = &state.export_attempts[0];
    assert_eq!(
        serde_json::to_value(&sent.0).unwrap(),
        json!({"Users":{"tenant_id":"tenant-a", "election_event_id":"event-a", "election_id":"election-a", "include_secret_attributes":true}})
    );
    assert_eq!(sent.1, "document-1");
    assert_eq!(sent.2.as_ref().unwrap().id, "task-1");
}

#[tokio::test]
async fn ordinary_voter_exports_bind_a_grant_without_secret_read_access() {
    let tasks = InMemoryUserTasks::default();
    export(
        &tasks,
        claims(&[VOTER_READ]),
        export_input(Some("event-a"), false),
    )
    .await
    .unwrap();
    let state = tasks.snapshot();
    assert!(state.audits.is_empty());
    assert_eq!(
        state.tasks[0].annotations,
        Some(
            json!({"secret_export_authorization": {"document_id":"document-1", "voter_secret_attributes":false, "expires_at":"2026-01-02T00:00:00Z"}})
        )
    );
}

#[tokio::test]
async fn tenant_exports_send_a_document_without_a_task_row() {
    let tasks = InMemoryUserTasks::default();
    let output =
        export(&tasks, claims(&[USER_READ]), export_input(None, false))
            .await
            .unwrap();
    assert_eq!(output.document_id, "document-1");
    assert!(output.task_execution.is_none());
    let state = tasks.snapshot();
    assert!(state.tasks.is_empty());
    assert!(state.audits.is_empty());
    assert_eq!(state.export_attempts.len(), 1);
    assert!(state.export_attempts[0].2.is_none());
}

#[tokio::test]
async fn export_ledger_failures_preserve_the_prior_secret_export_audit() {
    let tasks = InMemoryUserTasks::default();
    tasks.fail_on(UserTaskCall::CreateExport, "ledger offline");
    let error = export(
        &tasks,
        claims(&[VOTER_READ, VOTER_SECRET_ATTRIBUTE_READ]),
        export_input(Some("event-a"), true),
    )
    .await
    .unwrap_err();
    assert_eq!(error.0, Status::InternalServerError);
    assert!(error
        .1
        .starts_with("Failed to insert task execution record: ledger offline"));
    assert_eq!(tasks.snapshot().audits.len(), 1);
    assert!(tasks.snapshot().tasks.is_empty());
    assert_eq!(tasks.snapshot().connection_count, 0);
}

#[tokio::test]
async fn voter_export_broker_failures_revoke_the_task_grant_before_returning_the_error_message(
) {
    let tasks = InMemoryUserTasks::default();
    tasks.fail_on(UserTaskCall::SendExport, "broker offline");
    let output = export(
        &tasks,
        claims(&[VOTER_READ]),
        export_input(Some("event-a"), false),
    )
    .await
    .unwrap();
    assert_eq!(output.document_id, "document-1");
    assert_eq!(
        output.error_msg.as_deref(),
        Some("Error sending Export Users task: $broker offline")
    );
    assert_eq!(
        output.task_execution.as_ref().unwrap().execution_status,
        "IN_PROGRESS"
    );
    let state = tasks.snapshot();
    assert_eq!(state.tasks[0].execution_status, "FAILED");
    assert_eq!(
        state.failed_tasks,
        vec![(
            "task-1".into(),
            "Failed to enqueue voter export: BrokerFailure(broker offline)"
                .into()
        )]
    );
    assert_eq!(state.export_attempts.len(), 1);
}

#[tokio::test]
async fn an_export_grant_revocation_failure_replaces_the_broker_response_with_an_internal_error(
) {
    let tasks = InMemoryUserTasks::default();
    tasks.fail_on(UserTaskCall::SendExport, "broker offline");
    tasks.fail_on(UserTaskCall::Fail, "ledger offline");
    let error = export(
        &tasks,
        claims(&[VOTER_READ]),
        export_input(Some("event-a"), false),
    )
    .await
    .unwrap_err();
    assert_eq!(error.0, Status::InternalServerError);
    assert!(error.1.starts_with(
        "Failed to revoke voter export authorization: ledger offline"
    ));
    assert_eq!(tasks.snapshot().tasks[0].execution_status, "IN_PROGRESS");
}

#[tokio::test]
async fn tenant_export_broker_failures_return_an_error_message_without_updating_a_task(
) {
    let tasks = InMemoryUserTasks::default();
    tasks.fail_on(UserTaskCall::SendExport, "broker offline");
    tasks.fail_on(UserTaskCall::Fail, "no ledger row should be changed");
    let output =
        export(&tasks, claims(&[USER_READ]), export_input(None, false))
            .await
            .unwrap();
    assert_eq!(
        output.error_msg.as_deref(),
        Some("Error sending Export Users task: $broker offline")
    );
    assert!(output.task_execution.is_none());
    assert!(tasks.snapshot().failed_tasks.is_empty());
}

#[tokio::test]
async fn consecutive_exports_keep_each_document_bound_to_its_own_task_grant() {
    let tasks = InMemoryUserTasks::default();
    export(
        &tasks,
        claims(&[VOTER_READ]),
        export_input(Some("event-a"), false),
    )
    .await
    .unwrap();
    let second = export(
        &tasks,
        claims(&[VOTER_READ]),
        export_input(Some("event-a"), false),
    )
    .await
    .unwrap();
    assert_eq!(second.document_id, "document-2");
    assert_eq!(second.task_execution.as_ref().unwrap().id, "task-2");
    let state = tasks.snapshot();
    assert_eq!(
        state.tasks[1].annotations.as_ref().unwrap()
            ["secret_export_authorization"]["document_id"],
        "document-2"
    );
    assert_eq!(state.export_attempts[1].1, "document-2");
    assert_eq!(state.export_attempts[1].2.as_ref().unwrap().id, "task-2");
}
