// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::audit;
use crate::services::authorization::authorize;
use crate::services::dependencies::HarvestServices;
use crate::types::error_response::ErrorCode;
use anyhow::{anyhow, Context, Result};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use sequent_core::services::jwt;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};
use uuid::Uuid;
use windmill::services::celery_app::get_celery_app;
use windmill::services::electoral_log::{
    post_voter_secret_attribute_audit, ElectoralLogAdminContext,
    VoterSecretAttributeAction, VoterSecretAttributeAudit,
};
use windmill::services::export::integration_secrets::IntegrationSecretsExport;
use windmill::services::participation_authorization::{
    ParticipationAuthorization, ParticipationCommand,
    PARTICIPATION_AUTHORIZATION,
};
use windmill::services::voter_secret_attributes::get_secret_attribute_config;
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
/// its own, and the protocol manager keys need the ceremony export permission.
fn required_export_permissions(options: &ExportOptions) -> Vec<Permissions> {
    let included = [
        (options.include_voters, Permissions::VOTER_EXPORT),
        (options.applications, Permissions::APPLICATION_EXPORT),
        (options.activity_logs, Permissions::LOGS_EXPORT),
        (options.tally, Permissions::TALLY_RESULTS_READ),
        (options.reports, Permissions::REPORT_READ),
        (options.publications, Permissions::PUBLISH_EXPORT),
        (
            options.includes_protocol_manager_keys(),
            Permissions::EXPORT_CEREMONY,
        ),
        (options.s3_files, Permissions::DOCUMENT_DOWNLOAD),
        (options.include_certificates, Permissions::CA_READ),
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

/// Whether the archive carries the authenticator integration secrets.
///
/// Only on the caller's explicit request (`include_integration_secrets`),
/// never from a caller-set `integration_secrets`, for an export the caller
/// chose to encrypt with a password, and only for a holder of
/// `integration-secret-read`. A request that can't be honoured is refused
/// rather than silently exported without the secrets.
fn integration_secrets_export(
    claims: &JwtClaims,
    export_config: &ExportOptions,
) -> Result<IntegrationSecretsExport, (Status, String)> {
    if !export_config.include_integration_secrets {
        return Ok(IntegrationSecretsExport::OMITTED);
    }
    if !export_config.encrypt_with_password {
        return Err((
            Status::BadRequest,
            "Exporting integration secrets requires encrypting the export with a password"
                .to_string(),
        ));
    }
    authorize(
        claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::INTEGRATION_SECRET_READ],
    )?;
    Ok(IntegrationSecretsExport::INCLUDED)
}

#[instrument(skip(services, claims, input))]
#[post("/export-election-event", format = "json", data = "<input>")]
pub async fn export_election_event_route(
    claims: jwt::JwtClaims,
    input: Json<ExportElectionEventInput>,
    services: &State<HarvestServices>,
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
    export_config.validate_protocol_archive().map_err(|error| {
        (
            Status::BadRequest,
            serde_json::json!({
                "message": error.to_string(),
                "extensions": {"code": ErrorCode::TallyValidation}
            })
            .to_string(),
        )
    })?;
    export_config.password = None;
    export_config.contains_voter_secrets = false;
    export_config.integration_secrets =
        integration_secrets_export(&claims, &export_config)?;
    export_config.is_encrypted = export_config.is_encrypted
        || export_config.encrypt_with_password
        || export_config.bulletin_board
        || export_config.reports
        || export_config.applications;

    if (export_config.include_voters || export_config.s3_files)
        && export_config.encrypt_with_password
    {
        let has_secret_attributes = export_config.s3_files
            || !get_secret_attribute_config(&tenant_id, &election_event_id)
                .await
                .map_err(|error| {
                    (Status::InternalServerError, format!("{error:#}"))
                })?
                .validated_names()
                .map_err(|error| (Status::BadRequest, error.to_string()))?
                .is_empty();

        if has_secret_attributes {
            export_config.contains_voter_secrets = authorize(
                &claims,
                true,
                Some(tenant_id.clone()),
                vec![Permissions::VOTER_SECRET_ATTRIBUTE_READ],
            )
            .is_ok();
        }
    }

    let document_id = Uuid::new_v4().to_string();
    // An archive contains the whole event, so the caller must be allowed to
    // read every current election. Persist that explicit upper bound.
    let mut scope_client = services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(|error| (Status::InternalServerError, error.to_string()))?;
    let scope_tx = scope_client
        .transaction()
        .await
        .map_err(|error| (Status::InternalServerError, error.to_string()))?;
    crate::services::authorization::authorize_election_permission_labels(
        &scope_tx,
        &claims,
        &election_event_id,
        None,
    )
    .await?;
    let election_ids =
        crate::services::authorization::explicit_participation_elections(
            &scope_tx,
            &claims,
            &election_event_id,
        )
        .await?;
    drop(scope_tx);
    let mut task_annotations =
        secret_export_task_annotations_with_integration_secrets(
            &document_id,
            export_config.contains_voter_secrets,
            export_config.integration_secrets,
        );
    task_annotations[PARTICIPATION_AUTHORIZATION] =
        serde_json::to_value(ParticipationAuthorization {
            tenant_id: tenant_id.clone(),
            election_event_id: election_event_id.clone(),
            actor_id: claims.hasura_claims.user_id.clone(),
            command: ParticipationCommand::EventArchive,
            command_id: document_id.clone(),
            election_ids,
        })
        .map_err(|error| (Status::InternalServerError, error.to_string()))?;
    if export_config.contains_voter_secrets {
        post_voter_secret_attribute_audit(
            &tenant_id,
            &election_event_id,
            &ElectoralLogAdminContext::from_claims(&claims),
            VoterSecretAttributeAction::Export,
            VoterSecretAttributeAudit {
                voter_id: None,
                voter_username: None,
                attribute_names: &[],
                document_id: Some(&document_id),
            },
        )
        .await
        .map_err(|error| {
            (
                Status::InternalServerError,
                format!("Failed to record the secret-attribute electoral-log entry: {error:#}"),
            )
        })?;
    }

    // Insert the task execution record
    let task_execution = post_with_participation(
        &tenant_id,
        &election_event_id,
        ETasksExecution::EXPORT_ELECTION_EVENT,
        &executer_name,
        task_annotations,
    )
    .await
    .map_err(|error| {
        (
            Status::InternalServerError,
            format!("Failed to insert task execution record: {error:?}"),
        )
    })?;

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
    let integration_secrets = export_config.integration_secrets;

    let _celery_task = match celery_app
        .send_task(export_election_event::export_election_event::new(
            tenant_id,
            election_event_id,
            export_config,
            document_id.clone(),
            task_execution.clone(),
        ))
        .await
    {
        Ok(task) => task,
        Err(error) => {
            update_fail(
                &task_execution,
                &format!("Failed to enqueue export: {error:?}"),
            )
            .await
            .map_err(|update_error| {
                (
                    Status::InternalServerError,
                    format!("Failed to revoke export authorization: {update_error:?}"),
                )
            })?;
            return Err((
                Status::InternalServerError,
                format!("Error sending export_election_event task: {error:?}"),
            ));
        }
    };

    let entry = audit::exported(
        &claims,
        audit::target(
            &claims.hasura_claims.tenant_id,
            Some(&body.election_event_id),
        )?,
        audit::election_event(&body.election_event_id),
        // The tenant log records that service credentials were requested.
        match integration_secrets {
            IntegrationSecretsExport::INCLUDED => {
                "Requested the export of the election event with its \
                 integration secrets"
            }
            IntegrationSecretsExport::OMITTED => {
                "Requested the export of the election event"
            }
        },
        Some(&task_execution.id),
        Some(&document_id),
    );
    audit::record(services, &entry).await?;

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

    /// Each kind of exported data with the permission it needs.
    fn data_kinds() -> Vec<(ExportOptions, &'static str)> {
        let only = |set: fn(&mut ExportOptions)| {
            let mut options = ExportOptions::default();
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
            (only(|o| o.include_certificates = true), "ca-read"),
        ]
    }

    #[test]
    fn event_read_alone_exports_only_the_event_configuration() {
        let reader = admin_claims(CALLER_TENANT_ID, &["election-event-read"]);
        let configuration = ExportOptions {
            scheduled_events: true,
            ..ExportOptions::default()
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
            let mut granted = vec!["election-event-read", permission];
            if options.includes_protocol_manager_keys() {
                granted.push("export-ceremony");
            }
            let claims = admin_claims(CALLER_TENANT_ID, &granted);
            assert!(
                authorize_export(&claims, &options).is_ok(),
                "{permission}"
            );
        }
    }

    #[test]
    fn logs_and_publications_also_need_export_ceremony_for_their_keys() {
        let logs = ExportOptions {
            activity_logs: true,
            ..ExportOptions::default()
        };
        let publications = ExportOptions {
            publications: true,
            ..ExportOptions::default()
        };
        for (options, permission) in
            [(logs, "logs-export"), (publications, "publish-export")]
        {
            let own = admin_claims(
                CALLER_TENANT_ID,
                &["election-event-read", permission],
            );
            let error = authorize_export(&own, &options).unwrap_err();
            assert_eq!(error.0, Status::Unauthorized, "{permission}");

            let with_ceremony = admin_claims(
                CALLER_TENANT_ID,
                &["election-event-read", permission, "export-ceremony"],
            );
            assert!(
                authorize_export(&with_ceremony, &options).is_ok(),
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
            include_certificates: true,
            scheduled_events: true,
            ..ExportOptions::default()
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

#[cfg(test)]
mod integration_secret_tests {
    use super::*;
    use crate::test_claims::Claims;

    fn requested(encrypt_with_password: bool) -> ExportOptions {
        ExportOptions {
            encrypt_with_password,
            include_integration_secrets: true,
            ..ExportOptions::default()
        }
    }

    #[test]
    fn integration_secrets_need_the_request_the_password_and_the_permission() {
        let reader = Claims::new("tenant", "reader")
            .roles([Permissions::INTEGRATION_SECRET_READ])
            .build();
        assert_eq!(
            integration_secrets_export(&reader, &requested(true)),
            Ok(IntegrationSecretsExport::INCLUDED)
        );

        // An ordinary export stays secret-free, even for a holder of the
        // permission and even when the caller forges the server's field.
        let forged = ExportOptions {
            encrypt_with_password: true,
            integration_secrets: IntegrationSecretsExport::INCLUDED,
            ..ExportOptions::default()
        };
        assert_eq!(
            integration_secrets_export(&reader, &forged),
            Ok(IntegrationSecretsExport::OMITTED)
        );

        // Requested without password encryption: refused, not downgraded.
        let refused = integration_secrets_export(&reader, &requested(false));
        assert_eq!(
            refused.map_err(|(status, _)| status),
            Err(Status::BadRequest)
        );

        // Requested without the permission, including by holders of the
        // event-read, voter-secret or realm-attribute read permissions.
        for claims in [
            Claims::new("tenant", "admin")
                .roles([
                    Permissions::ELECTION_EVENT_READ,
                    Permissions::KEYCLOAK_REALM_ATTRIBUTES_READ,
                ])
                .build(),
            Claims::new("tenant", "voters")
                .roles([Permissions::VOTER_SECRET_ATTRIBUTE_READ])
                .build(),
        ] {
            let denied = integration_secrets_export(&claims, &requested(true));
            assert!(denied.is_err());
        }
    }
}
