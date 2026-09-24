// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::results_publication::{
    ResultsDocumentUrls, ResultsEventPresentation, ResultsPublicationReader,
};
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::tally_results_publication::{
    get_active_publication_for_route, get_publication_by_id, TallyResultsPublication,
};
use crate::services::documents::get_document_url;
use crate::types::results_publication::ResultsRouteScope;
use anyhow::Result;
use deadpool_postgres::Transaction;
use sequent_core::ballot::ElectionEventPresentation;

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
