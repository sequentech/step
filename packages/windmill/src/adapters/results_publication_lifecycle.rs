// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::adapters::results_publication::PgResultsPublications;
use crate::ports::results_publication_lifecycle::{
    ResultsPublicationArtifactStore, ResultsPublicationLifecycle, ResultsPublicationRenderer,
};
use crate::postgres::tally_results_publication::TallyResultsPublication;
use crate::postgres::{document, tally_results_publication as publications};
use crate::services::results_publication::{
    prepare_publication_source, publish_private_artifacts, publish_public_artifacts,
    upload_latest_public_manifest_value, upload_public_json_key, PreparedPublicationSource,
};
use crate::types::results_publication::ResultsPublicationManifest;
use anyhow::Result;
use deadpool_postgres::Transaction;
use sequent_core::{services::s3, types::hasura::core::Document};
use serde_json::Value;

impl ResultsPublicationLifecycle for PgResultsPublications<'_> {
    async fn active(
        &self,
        tenant_id: &str,
        event_id: &str,
    ) -> Result<Vec<TallyResultsPublication>> {
        publications::list_active_public_publications(self.transaction, tenant_id, event_id).await
    }
    async fn superseded(
        &self,
        tenant_id: &str,
        event_id: &str,
    ) -> Result<Vec<TallyResultsPublication>> {
        publications::list_superseded_publications(self.transaction, tenant_id, event_id).await
    }
    async fn mark_published(
        &self,
        publication: &TallyResultsPublication,
        documents: Value,
        manifest: Value,
    ) -> Result<()> {
        publications::mark_publication_published(self.transaction, publication, documents, manifest)
            .await
    }
    async fn mark_superseded(&self, publication: &TallyResultsPublication) -> Result<()> {
        publications::mark_publication_superseded(self.transaction, publication).await
    }
    async fn clear_finalization_error(
        &self,
        tenant_id: &str,
        event_id: &str,
        publication_id: &str,
    ) -> Result<()> {
        publications::set_publication_finalization_error(
            self.transaction,
            tenant_id,
            event_id,
            publication_id,
            None,
        )
        .await
    }
}

pub struct StoredResultsArtifacts<'a> {
    pub transaction: &'a Transaction<'a>,
}

impl ResultsPublicationRenderer for StoredResultsArtifacts<'_> {
    type Source = PreparedPublicationSource;
    async fn prepare(
        &self,
        publication: &TallyResultsPublication,
        selected_contests: &[String],
    ) -> Result<Self::Source> {
        prepare_publication_source(self.transaction, publication, selected_contests).await
    }
    async fn publish_public(
        &self,
        publication: &TallyResultsPublication,
        source: &Self::Source,
        selected_contests: &[String],
    ) -> Result<(Value, ResultsPublicationManifest)> {
        publish_public_artifacts(
            self.transaction,
            publication,
            source.file.path(),
            selected_contests,
            source.contests.clone(),
            source.custom_css.clone(),
            &source.language_config,
        )
        .await
    }
    async fn publish_private(
        &self,
        publication: &TallyResultsPublication,
        source: &Self::Source,
        selected_contests: &[String],
    ) -> Result<(Value, ResultsPublicationManifest)> {
        publish_private_artifacts(
            self.transaction,
            publication,
            source.file.path(),
            selected_contests,
            source.contests.clone(),
            source.custom_css.clone(),
            &source.language_config,
        )
        .await
    }
}

impl ResultsPublicationArtifactStore for StoredResultsArtifacts<'_> {
    async fn document(
        &self,
        publication: &TallyResultsPublication,
        document_id: &str,
    ) -> Result<Option<Document>> {
        document::get_document(
            self.transaction,
            &publication.tenant_id,
            Some(publication.election_event_id.clone()),
            document_id,
        )
        .await
    }
    async fn delete_documents(
        &self,
        publication: &TallyResultsPublication,
        document_ids: &[String],
    ) -> Result<()> {
        document::delete_documents(
            self.transaction,
            &publication.tenant_id,
            &publication.election_event_id,
            document_ids,
        )
        .await?;
        Ok(())
    }
    async fn delete_public_path(&self, path: String) -> Result<()> {
        s3::delete_files_from_s3(s3::get_public_bucket()?, path, s3::S3Endpoint::Server).await
    }
    async fn delete_private_path(&self, path: String) -> Result<()> {
        s3::delete_files_from_s3(s3::get_private_bucket()?, path, s3::S3Endpoint::Server).await
    }
    async fn upload_index(&self, key: &str, index: &Value) -> Result<()> {
        upload_public_json_key(key, index).await
    }
    async fn upload_latest_manifest(
        &self,
        publication: &TallyResultsPublication,
        manifest: &Value,
    ) -> Result<()> {
        upload_latest_public_manifest_value(publication, manifest).await
    }
}
