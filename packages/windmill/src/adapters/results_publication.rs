// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::domain::results_publication::results_publication_log_details;
use crate::ports::results_publication::{
    ResultsDocumentUrls, ResultsEventPresentation, ResultsPublicationAudit,
    ResultsPublicationReader, ResultsPublicationTasks, ResultsPublicationWriter,
};
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::tally_results_publication::{
    get_active_publication_for_route, get_publication_by_id, get_publication_source_facts,
    insert_publishing_publication, mark_publication_failed, revoke_publication,
    NewTallyResultsPublication, PublicationSource, PublicationSourceFacts, TallyResultsPublication,
};
use crate::services::celery_app::get_celery_app;
use crate::services::documents::get_document_url;
use crate::services::election_event_board::get_election_event_board;
use crate::services::electoral_log::ElectoralLog;
use crate::services::tasks_execution::{post, update_fail};
use crate::tasks::publish_results_website::publish_results_website_task;
use crate::types::results_publication::ResultsRouteScope;
use crate::types::tasks::ETasksExecution;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use electoral_log::messages::newtypes::ResultsPublicationAction;
use sequent_core::ballot::ElectionEventPresentation;
use sequent_core::types::hasura::core::TasksExecution;

/// Results publications in the Hasura database.
pub struct PgResultsPublications<'a> {
    pub transaction: &'a Transaction<'a>,
}

impl ResultsPublicationReader for PgResultsPublications<'_> {
    async fn get(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        publication_id: &str,
    ) -> Result<TallyResultsPublication> {
        get_publication_by_id(
            self.transaction,
            tenant_id,
            election_event_id,
            publication_id,
        )
        .await
    }

    async fn active_for_route(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        route_scope: ResultsRouteScope,
        route_election_id: Option<&str>,
    ) -> Result<Option<TallyResultsPublication>> {
        get_active_publication_for_route(
            self.transaction,
            tenant_id,
            election_event_id,
            route_scope,
            route_election_id,
        )
        .await
    }
}

impl ResultsPublicationWriter for PgResultsPublications<'_> {
    async fn source_facts(&self, source: &PublicationSource) -> Result<PublicationSourceFacts> {
        get_publication_source_facts(self.transaction, source).await
    }

    async fn insert_publishing(
        &self,
        publication: NewTallyResultsPublication<'_>,
    ) -> Result<TallyResultsPublication> {
        insert_publishing_publication(self.transaction, publication).await
    }

    async fn mark_failed(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        publication_id: &str,
        error_message: &str,
    ) -> Result<()> {
        mark_publication_failed(
            self.transaction,
            tenant_id,
            election_event_id,
            publication_id,
            error_message,
        )
        .await
    }

    async fn revoke(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        publication_id: &str,
    ) -> Result<()> {
        revoke_publication(
            self.transaction,
            tenant_id,
            election_event_id,
            publication_id,
        )
        .await
    }
}

/// Election event presentations in the Hasura database.
pub struct PgResultsEventPresentation<'a> {
    pub transaction: &'a Transaction<'a>,
}

impl ResultsEventPresentation for PgResultsEventPresentation<'_> {
    async fn get(
        &self,
        tenant_id: &str,
        election_event_id: &str,
    ) -> Result<ElectionEventPresentation> {
        let election_event =
            get_election_event_by_id(self.transaction, tenant_id, election_event_id).await?;
        Ok(election_event.get_presentation()?.unwrap_or_default())
    }
}

/// Presigned S3 URLs for documents registered in the Hasura database.
pub struct S3ResultsDocumentUrls<'a> {
    pub transaction: &'a Transaction<'a>,
}

impl ResultsDocumentUrls for S3ResultsDocumentUrls<'_> {
    async fn url(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        document_id: &str,
    ) -> Result<Option<String>> {
        get_document_url(
            self.transaction,
            tenant_id,
            Some(election_event_id),
            document_id,
        )
        .await
    }
}

/// `PUBLISH_RESULTS_WEBSITE` task executions, sent through Celery.
pub struct CeleryResultsPublicationTasks;

impl ResultsPublicationTasks for CeleryResultsPublicationTasks {
    async fn new_task_execution(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        executed_by_user: &str,
    ) -> Result<TasksExecution> {
        post(
            tenant_id,
            Some(election_event_id),
            ETasksExecution::PUBLISH_RESULTS_WEBSITE,
            executed_by_user,
        )
        .await
    }

    async fn mark_failed(
        &self,
        task_execution: &TasksExecution,
        error_message: &str,
    ) -> Result<()> {
        update_fail(task_execution, error_message).await
    }

    async fn enqueue_publish(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        publication_id: &str,
        user_id: &str,
        username: Option<String>,
        task_execution: &TasksExecution,
    ) -> Result<()> {
        let celery_app = get_celery_app().await;
        celery_app
            .send_task(publish_results_website_task::new(
                tenant_id.to_string(),
                election_event_id.to_string(),
                publication_id.to_string(),
                user_id.to_string(),
                username,
                task_execution.clone(),
            ))
            .await
            .map(|_| ())
            .map_err(|err| anyhow!("{err:?}"))
    }
}

/// Results publication actions in the election event's electoral log.
pub struct ElectoralLogResultsPublicationAudit<'a> {
    pub transaction: &'a Transaction<'a>,
}

impl ResultsPublicationAudit for ElectoralLogResultsPublicationAudit<'_> {
    async fn action(
        &self,
        publication: &TallyResultsPublication,
        action: ResultsPublicationAction,
        user_id: &str,
        username: Option<String>,
    ) -> Result<()> {
        let election_event = get_election_event_by_id(
            self.transaction,
            &publication.tenant_id,
            &publication.election_event_id,
        )
        .await?;
        let board_name = get_election_event_board(election_event.bulletin_board_reference)
            .context("Missing electoral log board for results publication")?;
        let electoral_log = ElectoralLog::for_admin_user(
            self.transaction,
            &board_name,
            &publication.tenant_id,
            &publication.election_event_id,
            user_id,
            username.clone(),
            Some(publication.election_ids.clone()),
            None,
        )
        .await?;

        electoral_log
            .post_results_publication_action(
                publication.election_event_id.clone(),
                results_publication_log_details(publication, action),
                Some(user_id.to_string()),
                username,
            )
            .await
            .context("Failed to post results publication action to the electoral log")
    }
}
