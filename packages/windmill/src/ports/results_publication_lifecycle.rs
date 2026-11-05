// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::results_publication::ResultsPublicationReader;
use crate::postgres::tally_results_publication::TallyResultsPublication;
use crate::types::results_publication::ResultsPublicationManifest;
use anyhow::Result;
use sequent_core::types::hasura::core::Document;
use serde_json::Value;
use std::future::Future;

pub trait ResultsPublicationLifecycle: ResultsPublicationReader {
    fn active(
        &self,
        tenant_id: &str,
        event_id: &str,
    ) -> impl Future<Output = Result<Vec<TallyResultsPublication>>> + Send;
    fn superseded(
        &self,
        tenant_id: &str,
        event_id: &str,
    ) -> impl Future<Output = Result<Vec<TallyResultsPublication>>> + Send;
    fn mark_published(
        &self,
        publication: &TallyResultsPublication,
        documents: Value,
        manifest: Value,
    ) -> impl Future<Output = Result<()>> + Send;
    fn mark_superseded(
        &self,
        publication: &TallyResultsPublication,
    ) -> impl Future<Output = Result<()>> + Send;
    fn clear_finalization_error(
        &self,
        tenant_id: &str,
        event_id: &str,
        publication_id: &str,
    ) -> impl Future<Output = Result<()>> + Send;
}

/// SQLite source preparation and artifact uploads retain their temporary
/// files until the publication has been stored.
pub trait ResultsPublicationRenderer: Sync {
    type Source: Send + Sync;
    fn prepare(
        &self,
        publication: &TallyResultsPublication,
        selected_contests: &[String],
    ) -> impl Future<Output = Result<Self::Source>> + Send;
    fn publish_public(
        &self,
        publication: &TallyResultsPublication,
        source: &Self::Source,
        selected_contests: &[String],
    ) -> impl Future<Output = Result<(Value, ResultsPublicationManifest)>> + Send;
    fn publish_private(
        &self,
        publication: &TallyResultsPublication,
        source: &Self::Source,
        selected_contests: &[String],
    ) -> impl Future<Output = Result<(Value, ResultsPublicationManifest)>> + Send;
}

/// Stored document metadata and the object storage used by results artifacts.
pub trait ResultsPublicationArtifactStore: Sync {
    fn document(
        &self,
        publication: &TallyResultsPublication,
        document_id: &str,
    ) -> impl Future<Output = Result<Option<Document>>> + Send;
    fn delete_documents(
        &self,
        publication: &TallyResultsPublication,
        document_ids: &[String],
    ) -> impl Future<Output = Result<()>> + Send;
    fn delete_public_path(&self, path: String) -> impl Future<Output = Result<()>> + Send;
    fn delete_private_path(&self, path: String) -> impl Future<Output = Result<()>> + Send;
    fn upload_index(&self, key: &str, index: &Value) -> impl Future<Output = Result<()>> + Send;
    fn upload_latest_manifest(
        &self,
        publication: &TallyResultsPublication,
        manifest: &Value,
    ) -> impl Future<Output = Result<()>> + Send;
}
