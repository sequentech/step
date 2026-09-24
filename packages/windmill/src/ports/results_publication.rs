// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::tally_results_publication::TallyResultsPublication;
use crate::types::results_publication::ResultsRouteScope;
use anyhow::Result;
use sequent_core::ballot::ElectionEventPresentation;
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
