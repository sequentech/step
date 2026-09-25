// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::results_publication::{
    ResultsDocumentUrls, ResultsEventPresentation, ResultsPublicationReader,
};
use crate::postgres::tally_results_publication::TallyResultsPublication;
use crate::types::results_publication::{ResultsPublicationStatus, ResultsRouteScope};
use anyhow::{anyhow, Result};
use sequent_core::ballot::ElectionEventPresentation;
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

fn injected(failure: &Option<String>) -> Result<()> {
    match failure {
        Some(message) => Err(anyhow!("{message}")),
        None => Ok(()),
    }
}

#[derive(Debug, Default)]
struct PublicationsState {
    publications: Vec<TallyResultsPublication>,
    failure: Option<String>,
}

/// Publications selected the way the PostgreSQL queries select them.
#[derive(Debug, Default)]
pub struct InMemoryResultsPublications(Mutex<PublicationsState>);

impl InMemoryResultsPublications {
    pub fn with(publications: impl IntoIterator<Item = TallyResultsPublication>) -> Self {
        Self(Mutex::new(PublicationsState {
            publications: publications.into_iter().collect(),
            failure: None,
        }))
    }

    /// Every later call fails with `message`.
    pub fn fail_with(&self, message: &str) {
        self.state().failure = Some(message.to_string());
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
