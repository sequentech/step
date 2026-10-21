// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::{anyhow, Context, Result};
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};
use uuid::Uuid;
use windmill::services::celery_app::get_celery_app;
use windmill::services::{password, tasks_execution::*};
use windmill::tasks::export_election_event::{self, ExportOptions};
use windmill::types::tasks::ETasksExecution;

#[derive(Serialize, Deserialize, Debug)]
pub struct ExportElectionEventInput {
    election_event_id: String,
    export_configurations: ExportOptions,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ExportElectionEventOutput {
    document_id: String,
    password: Option<String>,
    task_execution: TasksExecution,
}

/// Reading the event covers its configuration, including scheduled events.
/// Every other kind of data needs the permission that exports or reads it on
/// its own.
fn required_export_permissions(options: &ExportOptions) -> Vec<Permissions> {
    let included = [
        (options.include_voters, Permissions::VOTER_EXPORT),
        (options.applications, Permissions::APPLICATION_EXPORT),
        (options.activity_logs, Permissions::LOGS_EXPORT),
        (options.tally, Permissions::TALLY_RESULTS_READ),
        (options.reports, Permissions::REPORT_READ),
        (options.publications, Permissions::PUBLISH_EXPORT),
        (options.bulletin_board, Permissions::EXPORT_CEREMONY),
        (options.s3_files, Permissions::DOCUMENT_DOWNLOAD),
    ];
    std::iter::once(Permissions::ELECTION_EVENT_READ)
        .chain(included.into_iter().filter_map(|(included, permission)| {
            included.then_some(permission)
        }))
        .collect()
}

fn authorize_export(
    claims: &JwtClaims,
    options: &ExportOptions,
) -> Result<(), (Status, String)> {
    authorize(
        claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        required_export_permissions(options),
    )
}

#[instrument(skip(claims))]
#[post("/export-election-event", format = "json", data = "<input>")]
pub async fn export_election_event_route(
    claims: jwt::JwtClaims,
    input: Json<ExportElectionEventInput>,
) -> Result<Json<ExportElectionEventOutput>, (Status, String)> {
    let body = input.into_inner();
    authorize_export(&claims, &body.export_configurations)?;

    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let election_event_id = body.election_event_id.clone();
    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());
    let mut export_config = body.export_configurations.clone();

    // Insert the task execution record
    let task_execution = post(
        &tenant_id,
        Some(&election_event_id),
        ETasksExecution::EXPORT_ELECTION_EVENT,
        &executer_name,
    )
    .await
    .map_err(|error| {
        (
            Status::InternalServerError,
            format!("Failed to insert task execution record: {error:?}"),
        )
    })?;

    let document_id = Uuid::new_v4().to_string();
    let celery_app = get_celery_app().await;

    // todo: generarate only if encrypted
    let charset: String =
        "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_."
            .into();
    let password: Option<String> = if export_config.is_encrypted
        || export_config.bulletin_board
        || export_config.reports
        || export_config.applications
    {
        Some(password::generate_random_string_with_charset(64, &charset))
    } else {
        None
    };
    export_config.password = password.clone();

    let celery_task = celery_app
        .send_task(export_election_event::export_election_event::new(
            tenant_id,
            election_event_id,
            export_config,
            document_id.clone(),
            task_execution.clone(),
        ))
        .await
        .map_err(|error| {
            (
                Status::InternalServerError,
                format!("Error sending export_election_event task: {error:?}"),
            )
        })?;

    let output = ExportElectionEventOutput {
        document_id,
        password,
        task_execution: task_execution.clone(),
    };

    info!("Sent EXPORT_ELECTION_EVENT task  {:?}", &task_execution);

    Ok(Json(output))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::authorization::test_claims::{
        admin_claims, CALLER_TENANT_ID,
    };

    fn nothing() -> ExportOptions {
        ExportOptions {
            password: None,
            is_encrypted: false,
            include_voters: false,
            activity_logs: false,
            bulletin_board: false,
            publications: false,
            s3_files: false,
            scheduled_events: false,
            reports: false,
            applications: false,
            tally: false,
        }
    }

    /// Each kind of exported data with the permission it needs.
    fn data_kinds() -> Vec<(ExportOptions, &'static str)> {
        let only = |set: fn(&mut ExportOptions)| {
            let mut options = nothing();
            set(&mut options);
            options
        };
        vec![
            (only(|o| o.include_voters = true), "voter-export"),
            (only(|o| o.applications = true), "application-export"),
            (only(|o| o.activity_logs = true), "logs-export"),
            (only(|o| o.tally = true), "tally-results-read"),
            (only(|o| o.reports = true), "report-read"),
            (only(|o| o.publications = true), "publish-export"),
            (only(|o| o.bulletin_board = true), "export-ceremony"),
            (only(|o| o.s3_files = true), "document-download"),
        ]
    }

    #[test]
    fn event_read_alone_exports_only_the_event_configuration() {
        let reader = admin_claims(CALLER_TENANT_ID, &["election-event-read"]);
        let configuration = ExportOptions {
            scheduled_events: true,
            ..nothing()
        };
        assert!(authorize_export(&reader, &configuration).is_ok());
        for (options, permission) in data_kinds() {
            let error = authorize_export(&reader, &options).unwrap_err();
            assert_eq!(error.0, Status::Unauthorized, "{permission}");
        }
    }

    #[test]
    fn each_data_kind_is_exported_with_its_permission() {
        for (options, permission) in data_kinds() {
            let claims = admin_claims(
                CALLER_TENANT_ID,
                &["election-event-read", permission],
            );
            assert!(
                authorize_export(&claims, &options).is_ok(),
                "{permission}"
            );
        }
    }

    #[test]
    fn a_full_export_needs_event_read_and_every_data_permission() {
        let everything = ExportOptions {
            include_voters: true,
            applications: true,
            activity_logs: true,
            tally: true,
            reports: true,
            publications: true,
            bulletin_board: true,
            s3_files: true,
            scheduled_events: true,
            ..nothing()
        };
        let mut granted: Vec<_> = data_kinds()
            .into_iter()
            .map(|(_, permission)| permission)
            .collect();
        granted.push("election-event-read");
        let exporter = admin_claims(CALLER_TENANT_ID, &granted);
        assert!(authorize_export(&exporter, &everything).is_ok());
        for missing in &granted {
            let without: Vec<_> =
                granted.iter().copied().filter(|p| p != missing).collect();
            let claims = admin_claims(CALLER_TENANT_ID, &without);
            let error = authorize_export(&claims, &everything).unwrap_err();
            assert_eq!(error.0, Status::Unauthorized, "{missing}");
        }
    }
}
