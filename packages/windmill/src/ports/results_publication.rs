// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::domain::results_publication::ResultsPublicationServiceResult;
use crate::postgres::tally_results_publication::{
    NewTallyResultsPublication, PublicationSource, PublicationSourceFacts, TallyResultsPublication,
};
use crate::types::results_publication::{PublishResultsWebsiteInput, ResultsRouteScope};
use anyhow::Result;
use electoral_log::messages::newtypes::ResultsPublicationAction;
use sequent_core::ballot::ElectionEventPresentation;
use sequent_core::types::hasura::core::TasksExecution;
use std::future::Future;

/// Stored results publications.
pub trait ResultsPublicationReader: Sync {
    /// Fails when the publication does not exist.
    fn get(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        publication_id: &str,
    ) -> impl Future<Output = Result<TallyResultsPublication>> + Send;

    /// The newest published, unrevoked publication for the route.
    fn active_for_route(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        route_scope: ResultsRouteScope,
        route_election_id: Option<&str>,
    ) -> impl Future<Output = Result<Option<TallyResultsPublication>>> + Send;
}

/// Changes to stored results publications.
pub trait ResultsPublicationWriter: ResultsPublicationReader {
    fn source_facts(
        &self,
        source: &PublicationSource,
    ) -> impl Future<Output = Result<PublicationSourceFacts>> + Send;

    /// Stores a `Publishing` publication with the next version of its route.
    fn insert_publishing(
        &self,
        publication: NewTallyResultsPublication<'_>,
    ) -> impl Future<Output = Result<TallyResultsPublication>> + Send;

    /// Fails unless the publication is `Publishing` or `Failed`.
    fn mark_failed(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        publication_id: &str,
        error_message: &str,
    ) -> impl Future<Output = Result<()>> + Send;

    /// Fails unless the publication is `Published`.
    fn revoke(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        publication_id: &str,
    ) -> impl Future<Output = Result<()>> + Send;
}

/// The database steps of a publication request. Each step commits before
/// the next one starts.
pub trait ResultsPublicationRequestSteps: Sync {
    /// Checks the request against the results website policy and its tally
    /// source.
    fn validate(
        &self,
        tenant_id: &str,
        input: &PublishResultsWebsiteInput,
    ) -> impl Future<Output = ResultsPublicationServiceResult<()>> + Send;

    fn insert_publishing(
        &self,
        publication: NewTallyResultsPublication<'_>,
    ) -> impl Future<Output = ResultsPublicationServiceResult<TallyResultsPublication>> + Send;

    fn mark_failed(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        publication_id: &str,
        error_message: &str,
    ) -> impl Future<Output = ResultsPublicationServiceResult<()>> + Send;
}

/// The task that publishes results website artifacts.
pub trait ResultsPublicationTasks: Sync {
    /// Records an in-progress `PUBLISH_RESULTS_WEBSITE` task execution.
    fn new_task_execution(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        executed_by_user: &str,
    ) -> impl Future<Output = Result<TasksExecution>> + Send;

    /// Marks the execution failed when the request cannot start its worker.
    fn mark_failed(
        &self,
        task_execution: &TasksExecution,
        error_message: &str,
    ) -> impl Future<Output = Result<()>> + Send;

    /// Sends the publish task. The error message is the broker error's
    /// debug output.
    fn enqueue_publish(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        publication_id: &str,
        user_id: &str,
        username: Option<String>,
        task_execution: &TasksExecution,
    ) -> impl Future<Output = Result<()>> + Send;
}

/// The electoral log of actions on results publications.
pub trait ResultsPublicationAudit: Sync {
    fn action(
        &self,
        publication: &TallyResultsPublication,
        action: ResultsPublicationAction,
        user_id: &str,
        username: Option<String>,
    ) -> impl Future<Output = Result<()>> + Send;
}

/// The election event presentation, which holds the results website policy.
pub trait ResultsEventPresentation: Sync {
    /// The default presentation when the event has none; fails when the
    /// event does not exist or its presentation is invalid.
    fn get(
        &self,
        tenant_id: &str,
        election_event_id: &str,
    ) -> impl Future<Output = Result<ElectionEventPresentation>> + Send;
}

/// Download URLs for stored documents.
pub trait ResultsDocumentUrls: Sync {
    /// `None` when the document does not exist.
    fn url(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        document_id: &str,
    ) -> impl Future<Output = Result<Option<String>>> + Send;
}
