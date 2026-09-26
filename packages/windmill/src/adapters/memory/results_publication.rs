// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::results_publication::{
    ResultsDocumentUrls, ResultsEventPresentation, ResultsPublicationAudit,
    ResultsPublicationReader, ResultsPublicationTasks, ResultsPublicationWriter,
};
use crate::postgres::tally_results_publication::{
    NewTallyResultsPublication, PublicationSource, PublicationSourceFacts, TallyResultsPublication,
};
use crate::types::results_publication::{
    ContestPublicationState, ResultsPublicationStatus, ResultsRouteScope,
};
use crate::types::tasks::ETasksExecution;
use anyhow::{anyhow, Result};
use electoral_log::messages::newtypes::ResultsPublicationAction;
use sequent_core::ballot::ElectionEventPresentation;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::hasura::extra::TasksExecutionStatus;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, MutexGuard};

fn injected(failure: &Option<String>) -> Result<()> {
    match failure {
        Some(message) => Err(anyhow!("{message}")),
        None => Ok(()),
    }
}

/// Publication store calls that tests can make fail on their own.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PublicationCall {
    SourceFacts,
    InsertPublishing,
    MarkFailed,
    Revoke,
    Active,
    Superseded,
    MarkPublished,
    MarkSuperseded,
    ClearFinalizationError,
}

#[derive(Debug, Default)]
struct PublicationsState {
    publications: Vec<TallyResultsPublication>,
    source_facts: Option<PublicationSourceFacts>,
    failure: Option<String>,
    call_failures: HashMap<PublicationCall, String>,
}

impl PublicationsState {
    fn check(&self, call: PublicationCall) -> Result<()> {
        injected(&self.failure)?;
        injected(&self.call_failures.get(&call).cloned())
    }

    fn find_mut(
        &mut self,
        tenant_id: &str,
        election_event_id: &str,
        publication_id: &str,
    ) -> Option<&mut TallyResultsPublication> {
        self.publications.iter_mut().find(|publication| {
            publication.tenant_id == tenant_id
                && publication.election_event_id == election_event_id
                && publication.id == publication_id
        })
    }
}

/// Publications selected and changed the way the PostgreSQL queries do.
#[derive(Debug, Default)]
pub struct InMemoryResultsPublications(Mutex<PublicationsState>);

impl InMemoryResultsPublications {
    pub fn with(publications: impl IntoIterator<Item = TallyResultsPublication>) -> Self {
        Self(Mutex::new(PublicationsState {
            publications: publications.into_iter().collect(),
            ..Default::default()
        }))
    }

    /// Every later call fails with `message`.
    pub fn fail_with(&self, message: &str) {
        self.state().failure = Some(message.to_string());
    }

    /// Later `call`s fail with `message`.
    pub fn fail_on(&self, call: PublicationCall, message: &str) {
        self.state().call_failures.insert(call, message.to_string());
    }

    /// Replaces the facts a consistent source would have.
    pub fn set_source_facts(&self, facts: PublicationSourceFacts) {
        self.state().source_facts = Some(facts);
    }

    pub fn stored(&self) -> Vec<TallyResultsPublication> {
        self.state().publications.clone()
    }

    fn state(&self) -> MutexGuard<'_, PublicationsState> {
        self.0.lock().expect("publications lock")
    }
}

impl ResultsPublicationReader for InMemoryResultsPublications {
    async fn get(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        publication_id: &str,
    ) -> Result<TallyResultsPublication> {
        let state = self.state();
        injected(&state.failure)?;
        state
            .publications
            .iter()
            .find(|publication| {
                publication.tenant_id == tenant_id
                    && publication.election_event_id == election_event_id
                    && publication.id == publication_id
            })
            .cloned()
            .ok_or_else(|| anyhow!("Publication not found"))
    }

    async fn active_for_route(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        route_scope: ResultsRouteScope,
        route_election_id: Option<&str>,
    ) -> Result<Option<TallyResultsPublication>> {
        let state = self.state();
        injected(&state.failure)?;
        Ok(state
            .publications
            .iter()
            .filter(|publication| {
                publication.tenant_id == tenant_id
                    && publication.election_event_id == election_event_id
                    && publication.route_scope == route_scope
                    && publication.publication_status == ResultsPublicationStatus::Published
                    && publication.route_election_id.as_deref() == route_election_id
            })
            .max_by_key(|publication| publication.version)
            .cloned())
    }
}

fn distinct_count<T: Eq + std::hash::Hash>(values: &[T]) -> i64 {
    values.iter().collect::<HashSet<_>>().len() as i64
}

impl ResultsPublicationWriter for InMemoryResultsPublications {
    /// Unless replaced, every source election and contest exists and every
    /// contest has results.
    async fn source_facts(&self, source: &PublicationSource) -> Result<PublicationSourceFacts> {
        let state = self.state();
        state.check(PublicationCall::SourceFacts)?;
        Ok(state
            .source_facts
            .clone()
            .unwrap_or_else(|| PublicationSourceFacts {
                valid_execution: true,
                election_count: distinct_count(&source.election_ids),
                contest_count: distinct_count(&source.contest_ids),
                tallied_contest_count: distinct_count(&source.contest_ids),
            }))
    }

    async fn insert_publishing(
        &self,
        publication: NewTallyResultsPublication<'_>,
    ) -> Result<TallyResultsPublication> {
        let mut state = self.state();
        state.check(PublicationCall::InsertPublishing)?;
        let version = state
            .publications
            .iter()
            .filter(|stored| {
                stored.tenant_id == publication.tenant_id
                    && stored.election_event_id == publication.election_event_id
                    && stored.route_scope == publication.route_scope
                    && stored.route_election_id.as_deref() == publication.route_election_id
            })
            .map(|stored| stored.version)
            .max()
            .unwrap_or(0)
            + 1;
        let stored = TallyResultsPublication {
            id: format!("publication-{}", state.publications.len() + 1),
            tenant_id: publication.tenant_id.to_string(),
            election_event_id: publication.election_event_id.to_string(),
            tally_session_id: publication.tally_session_id.to_string(),
            tally_session_execution_id: publication.tally_session_execution_id.to_string(),
            results_event_id: publication.results_event_id.to_string(),
            task_execution_id: Some(publication.task_execution_id.to_string()),
            route_scope: publication.route_scope,
            route_election_id: publication.route_election_id.map(str::to_string),
            election_ids: publication.election_ids.to_vec(),
            access: publication.access,
            visibility_scope: publication.visibility_scope,
            published_contest_ids: publication.contest_ids.to_vec(),
            contest_publication_state: publication
                .contest_ids
                .iter()
                .map(|id| (id.clone(), ContestPublicationState::Published))
                .collect(),
            documents: json!({}),
            manifest: None,
            publication_status: ResultsPublicationStatus::Publishing,
            version,
            error_message: None,
            published_by_user_id: publication.published_by_user_id.map(str::to_string),
        };
        state.publications.push(stored.clone());
        Ok(stored)
    }

    async fn mark_failed(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        publication_id: &str,
        error_message: &str,
    ) -> Result<()> {
        let mut state = self.state();
        state.check(PublicationCall::MarkFailed)?;
        let publication = state
            .find_mut(tenant_id, election_event_id, publication_id)
            .filter(|publication| {
                matches!(
                    publication.publication_status,
                    ResultsPublicationStatus::Publishing | ResultsPublicationStatus::Failed
                )
            })
            .ok_or_else(|| anyhow!("Publication is not in a state that can be marked failed"))?;
        publication.publication_status = ResultsPublicationStatus::Failed;
        publication.error_message = Some(error_message.to_string());
        Ok(())
    }

    async fn revoke(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        publication_id: &str,
    ) -> Result<()> {
        let mut state = self.state();
        state.check(PublicationCall::Revoke)?;
        let publication = state
            .find_mut(tenant_id, election_event_id, publication_id)
            .filter(|publication| {
                publication.publication_status == ResultsPublicationStatus::Published
            })
            .ok_or_else(|| anyhow!("Publication not found or not currently published"))?;
        publication.publication_status = ResultsPublicationStatus::Revoked;
        Ok(())
    }
}

#[derive(Debug, Default)]
struct PresentationsState {
    presentations: HashMap<(String, String), ElectionEventPresentation>,
    failure: Option<String>,
}

/// Election event presentations by tenant and event.
#[derive(Debug, Default)]
pub struct InMemoryResultsEventPresentation(Mutex<PresentationsState>);

impl InMemoryResultsEventPresentation {
    pub fn with(
        tenant_id: &str,
        election_event_id: &str,
        presentation: ElectionEventPresentation,
    ) -> Self {
        let presentations = HashMap::from([(
            (tenant_id.to_string(), election_event_id.to_string()),
            presentation,
        )]);
        Self(Mutex::new(PresentationsState {
            presentations,
            failure: None,
        }))
    }

    /// Every later call fails with `message`.
    pub fn fail_with(&self, message: &str) {
        self.state().failure = Some(message.to_string());
    }

    fn state(&self) -> MutexGuard<'_, PresentationsState> {
        self.0.lock().expect("presentations lock")
    }
}

impl ResultsEventPresentation for InMemoryResultsEventPresentation {
    async fn get(
        &self,
        tenant_id: &str,
        election_event_id: &str,
    ) -> Result<ElectionEventPresentation> {
        let state = self.state();
        injected(&state.failure)?;
        state
            .presentations
            .get(&(tenant_id.to_string(), election_event_id.to_string()))
            .cloned()
            .ok_or_else(|| anyhow!("Election event {election_event_id} not found"))
    }
}

#[derive(Debug, Default)]
struct DocumentUrlsState {
    urls: HashMap<(String, String, String), String>,
    failure: Option<String>,
}

/// Download URLs by tenant, event and document.
#[derive(Debug, Default)]
pub struct InMemoryResultsDocumentUrls(Mutex<DocumentUrlsState>);

impl InMemoryResultsDocumentUrls {
    pub fn insert(&self, tenant_id: &str, election_event_id: &str, document_id: &str, url: &str) {
        self.state().urls.insert(
            (
                tenant_id.to_string(),
                election_event_id.to_string(),
                document_id.to_string(),
            ),
            url.to_string(),
        );
    }

    /// Every later call fails with `message`.
    pub fn fail_with(&self, message: &str) {
        self.state().failure = Some(message.to_string());
    }

    fn state(&self) -> MutexGuard<'_, DocumentUrlsState> {
        self.0.lock().expect("document URLs lock")
    }
}

impl ResultsDocumentUrls for InMemoryResultsDocumentUrls {
    async fn url(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        document_id: &str,
    ) -> Result<Option<String>> {
        let state = self.state();
        injected(&state.failure)?;
        Ok(state
            .urls
            .get(&(
                tenant_id.to_string(),
                election_event_id.to_string(),
                document_id.to_string(),
            ))
            .cloned())
    }
}

/// A publish task sent to the broker.
#[derive(Clone, Debug, PartialEq)]
pub struct EnqueuedPublish {
    pub tenant_id: String,
    pub election_event_id: String,
    pub publication_id: String,
    pub user_id: String,
    pub username: Option<String>,
    pub task_execution_id: String,
}

#[derive(Debug, Default)]
struct TasksState {
    executions: Vec<TasksExecution>,
    enqueued: Vec<EnqueuedPublish>,
    execution_failure: Option<String>,
    enqueue_failure: Option<String>,
}

/// Task executions and a broker that remember what they were given.
#[derive(Debug, Default)]
pub struct InMemoryResultsPublicationTasks(Mutex<TasksState>);

impl InMemoryResultsPublicationTasks {
    pub fn fail_task_executions_with(&self, message: &str) {
        self.state().execution_failure = Some(message.to_string());
    }

    pub fn fail_enqueues_with(&self, message: &str) {
        self.state().enqueue_failure = Some(message.to_string());
    }

    pub fn executions(&self) -> Vec<TasksExecution> {
        self.state().executions.clone()
    }

    pub fn enqueued(&self) -> Vec<EnqueuedPublish> {
        self.state().enqueued.clone()
    }

    fn state(&self) -> MutexGuard<'_, TasksState> {
        self.0.lock().expect("tasks lock")
    }
}

impl ResultsPublicationTasks for InMemoryResultsPublicationTasks {
    async fn new_task_execution(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        executed_by_user: &str,
    ) -> Result<TasksExecution> {
        let mut state = self.state();
        injected(&state.execution_failure)?;
        let task_type = ETasksExecution::PUBLISH_RESULTS_WEBSITE;
        let execution = TasksExecution {
            id: format!("task-{}", state.executions.len() + 1),
            tenant_id: tenant_id.to_string(),
            election_event_id: Some(election_event_id.to_string()),
            name: task_type.to_name().to_string(),
            task_type: task_type.to_string(),
            execution_status: TasksExecutionStatus::IN_PROGRESS.to_string(),
            created_at: Default::default(),
            start_at: None,
            end_at: None,
            annotations: None,
            labels: None,
            logs: None,
            executed_by_user: executed_by_user.to_string(),
        };
        state.executions.push(execution.clone());
        Ok(execution)
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
        let mut state = self.state();
        injected(&state.enqueue_failure)?;
        state.enqueued.push(EnqueuedPublish {
            tenant_id: tenant_id.to_string(),
            election_event_id: election_event_id.to_string(),
            publication_id: publication_id.to_string(),
            user_id: user_id.to_string(),
            username,
            task_execution_id: task_execution.id.clone(),
        });
        Ok(())
    }
}

/// An electoral log entry about a publication.
#[derive(Clone, Debug, PartialEq)]
pub struct AuditEntry {
    pub publication_id: String,
    pub action: ResultsPublicationAction,
    pub user_id: String,
    pub username: Option<String>,
}

#[derive(Debug, Default)]
struct AuditState {
    entries: Vec<AuditEntry>,
    failure: Option<String>,
}

/// An electoral log that keeps its entries.
#[derive(Debug, Default)]
pub struct InMemoryResultsPublicationAudit(Mutex<AuditState>);

impl InMemoryResultsPublicationAudit {
    pub fn fail_with(&self, message: &str) {
        self.state().failure = Some(message.to_string());
    }

    pub fn entries(&self) -> Vec<AuditEntry> {
        self.state().entries.clone()
    }

    fn state(&self) -> MutexGuard<'_, AuditState> {
        self.0.lock().expect("audit lock")
    }
}

impl ResultsPublicationAudit for InMemoryResultsPublicationAudit {
    async fn action(
        &self,
        publication: &TallyResultsPublication,
        action: ResultsPublicationAction,
        user_id: &str,
        username: Option<String>,
    ) -> Result<()> {
        let mut state = self.state();
        injected(&state.failure)?;
        state.entries.push(AuditEntry {
            publication_id: publication.id.clone(),
            action,
            user_id: user_id.to_string(),
            username,
        });
        Ok(())
    }
}

impl crate::ports::results_publication_lifecycle::ResultsPublicationLifecycle
    for InMemoryResultsPublications
{
    async fn active(
        &self,
        tenant_id: &str,
        event_id: &str,
    ) -> Result<Vec<TallyResultsPublication>> {
        let state = self.state();
        state.check(PublicationCall::Active)?;
        let mut publications: Vec<_> = state
            .publications
            .iter()
            .filter(|p| {
                p.tenant_id == tenant_id
                    && p.election_event_id == event_id
                    && p.publication_status == ResultsPublicationStatus::Published
            })
            .cloned()
            .collect();
        publications.sort_by(|a, b| {
            a.route_scope
                .as_ref()
                .cmp(b.route_scope.as_ref())
                .then(a.route_election_id.cmp(&b.route_election_id))
                .then(b.version.cmp(&a.version))
        });
        Ok(publications)
    }
    async fn superseded(
        &self,
        tenant_id: &str,
        event_id: &str,
    ) -> Result<Vec<TallyResultsPublication>> {
        let state = self.state();
        state.check(PublicationCall::Superseded)?;
        Ok(state
            .publications
            .iter()
            .filter(|p| {
                p.tenant_id == tenant_id
                    && p.election_event_id == event_id
                    && p.publication_status == ResultsPublicationStatus::Superseded
            })
            .cloned()
            .collect())
    }
    async fn mark_published(
        &self,
        publication: &TallyResultsPublication,
        documents: serde_json::Value,
        manifest: serde_json::Value,
    ) -> Result<()> {
        let mut state = self.state();
        state.check(PublicationCall::MarkPublished)?;
        let target_index = state
            .publications
            .iter()
            .position(|stored| {
                stored.tenant_id == publication.tenant_id
                    && stored.election_event_id == publication.election_event_id
                    && stored.id == publication.id
            })
            .ok_or_else(|| anyhow!("Publication not found"))?;
        if !matches!(
            state.publications[target_index].publication_status,
            ResultsPublicationStatus::Publishing | ResultsPublicationStatus::Failed
        ) {
            return Err(anyhow!(
                "Publication is not in a state that can be activated"
            ));
        }
        for stored in &mut state.publications {
            if stored.tenant_id == publication.tenant_id
                && stored.election_event_id == publication.election_event_id
                && stored.id != publication.id
                && stored.route_scope == publication.route_scope
                && stored.route_election_id == publication.route_election_id
                && stored.publication_status == ResultsPublicationStatus::Published
            {
                stored.publication_status = ResultsPublicationStatus::Superseded;
            }
        }
        let stored = &mut state.publications[target_index];
        stored.publication_status = ResultsPublicationStatus::Published;
        stored.documents = documents;
        stored.manifest = Some(manifest);
        stored.error_message = None;
        Ok(())
    }
    async fn mark_superseded(&self, publication: &TallyResultsPublication) -> Result<()> {
        let mut state = self.state();
        state.check(PublicationCall::MarkSuperseded)?;
        if let Some(stored) = state.find_mut(
            &publication.tenant_id,
            &publication.election_event_id,
            &publication.id,
        ) {
            if stored.publication_status == ResultsPublicationStatus::Published {
                stored.publication_status = ResultsPublicationStatus::Superseded;
            }
        }
        Ok(())
    }
    async fn clear_finalization_error(
        &self,
        tenant_id: &str,
        event_id: &str,
        publication_id: &str,
    ) -> Result<()> {
        let mut state = self.state();
        state.check(PublicationCall::ClearFinalizationError)?;
        let stored = state
            .find_mut(tenant_id, event_id, publication_id)
            .filter(|publication| {
                publication.publication_status == ResultsPublicationStatus::Published
            })
            .ok_or_else(|| {
                anyhow!("Published publication was not available to update its finalization error")
            })?;
        stored.error_message = None;
        Ok(())
    }
}

#[cfg(test)]
mod activation_tests {
    use super::*;
    use crate::domain::results_publication::fixtures::publication;
    use crate::ports::results_publication_lifecycle::ResultsPublicationLifecycle;

    fn activation_store(
        status: ResultsPublicationStatus,
    ) -> (InMemoryResultsPublications, TallyResultsPublication) {
        let target = TallyResultsPublication {
            publication_status: status,
            version: 2,
            ..publication()
        };
        let active = TallyResultsPublication {
            id: "previous-publication".into(),
            ..publication()
        };
        (
            InMemoryResultsPublications::with([active, target.clone()]),
            target,
        )
    }

    async fn assert_activation_succeeds() {
        for status in [
            ResultsPublicationStatus::Publishing,
            ResultsPublicationStatus::Failed,
        ] {
            let (store, target) = activation_store(status);
            store
                .mark_published(&target, json!({"document": "new"}), json!({"version": 2}))
                .await
                .unwrap();
            let stored = store.stored();
            assert_eq!(
                stored[0].publication_status,
                ResultsPublicationStatus::Superseded
            );
            assert_eq!(
                stored[1].publication_status,
                ResultsPublicationStatus::Published
            );
            assert_eq!(stored[1].documents, json!({"document": "new"}));
            assert_eq!(stored[1].manifest, Some(json!({"version": 2})));
        }
    }

    #[tokio::test]
    async fn rejected_activation_status_preserves_every_stored_publication() {
        assert_activation_succeeds().await;
        for status in [
            ResultsPublicationStatus::Published,
            ResultsPublicationStatus::Revoked,
            ResultsPublicationStatus::Superseded,
        ] {
            let (store, mut target) = activation_store(status);
            let before = serde_json::to_value(store.stored()).unwrap();
            // A stale caller snapshot must not bypass the stored status.
            target.publication_status = ResultsPublicationStatus::Publishing;
            let error = store
                .mark_published(&target, json!({"document": "new"}), json!({"version": 2}))
                .await
                .unwrap_err();
            assert_eq!(
                error.to_string(),
                "Publication is not in a state that can be activated"
            );
            assert_eq!(serde_json::to_value(store.stored()).unwrap(), before);
        }
    }

    #[tokio::test]
    async fn missing_activation_target_preserves_every_stored_publication() {
        assert_activation_succeeds().await;
        let (store, mut target) = activation_store(ResultsPublicationStatus::Publishing);
        let before = serde_json::to_value(store.stored()).unwrap();
        target.id = "missing-publication".into();
        let error = store
            .mark_published(&target, json!({"document": "new"}), json!({"version": 2}))
            .await
            .unwrap_err();
        assert_eq!(error.to_string(), "Publication not found");
        assert_eq!(serde_json::to_value(store.stored()).unwrap(), before);
    }
}
