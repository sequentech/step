// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::results_publication_lifecycle::{
    ResultsPublicationArtifactStore, ResultsPublicationRenderer,
};
use crate::postgres::tally_results_publication::TallyResultsPublication;
use crate::types::results_publication::ResultsPublicationManifest;
use anyhow::{anyhow, Result};
use sequent_core::types::hasura::core::Document;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, MutexGuard};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ArtifactCall {
    Prepare,
    PublishPublic,
    PublishPrivate,
    Document,
    DeleteDocuments,
    DeletePublic,
    DeletePrivate,
    UploadIndex,
    UploadLatest,
}

#[derive(Clone, Default)]
pub struct ArtifactsState {
    pub documents: HashMap<(String, String, String), Document>,
    pub removed_documents: HashSet<(String, String, String)>,
    pub removed_public_paths: HashSet<String>,
    pub removed_private_paths: HashSet<String>,
    pub indexes: HashMap<String, Value>,
    pub latest_manifests: HashMap<String, Value>,
    pub prepared_contests: Option<Vec<String>>,
    pub public_contests: Option<Vec<String>>,
    pub private_contests: Option<Vec<String>>,
    output: Option<(Value, ResultsPublicationManifest)>,
    failures: HashMap<ArtifactCall, String>,
}

impl ArtifactsState {
    fn check(&self, call: ArtifactCall) -> Result<()> {
        match self.failures.get(&call) {
            Some(message) => Err(anyhow!("{message}")),
            None => Ok(()),
        }
    }
}

#[derive(Default)]
pub struct InMemoryResultsArtifacts(Mutex<ArtifactsState>);

impl InMemoryResultsArtifacts {
    fn state(&self) -> MutexGuard<'_, ArtifactsState> {
        self.0.lock().expect("artifacts lock")
    }
    pub fn snapshot(&self) -> ArtifactsState {
        self.state().clone()
    }
    pub fn fail_on(&self, call: ArtifactCall, message: &str) {
        self.state().failures.insert(call, message.into());
    }
    pub fn set_output(&self, documents: Value, manifest: ResultsPublicationManifest) {
        self.state().output = Some((documents, manifest));
    }
    pub fn insert_document(&self, tenant_id: &str, event_id: &str, document: Document) {
        self.state().documents.insert(
            (tenant_id.into(), event_id.into(), document.id.clone()),
            document,
        );
    }
}

impl ResultsPublicationRenderer for InMemoryResultsArtifacts {
    type Source = String;
    async fn prepare(
        &self,
        publication: &TallyResultsPublication,
        selected_contests: &[String],
    ) -> Result<String> {
        let mut state = self.state();
        state.check(ArtifactCall::Prepare)?;
        state.prepared_contests = Some(selected_contests.to_vec());
        Ok(publication.id.clone())
    }
    async fn publish_public(
        &self,
        publication: &TallyResultsPublication,
        source: &String,
        selected_contests: &[String],
    ) -> Result<(Value, ResultsPublicationManifest)> {
        assert_eq!(source, &publication.id);
        let mut state = self.state();
        state.check(ArtifactCall::PublishPublic)?;
        state.public_contests = Some(selected_contests.to_vec());
        state
            .output
            .clone()
            .ok_or_else(|| anyhow!("No rendered artifacts"))
    }
    async fn publish_private(
        &self,
        publication: &TallyResultsPublication,
        source: &String,
        selected_contests: &[String],
    ) -> Result<(Value, ResultsPublicationManifest)> {
        assert_eq!(source, &publication.id);
        let mut state = self.state();
        state.check(ArtifactCall::PublishPrivate)?;
        state.private_contests = Some(selected_contests.to_vec());
        state
            .output
            .clone()
            .ok_or_else(|| anyhow!("No rendered artifacts"))
    }
}

impl ResultsPublicationArtifactStore for InMemoryResultsArtifacts {
    async fn document(
        &self,
        publication: &TallyResultsPublication,
        document_id: &str,
    ) -> Result<Option<Document>> {
        let state = self.state();
        state.check(ArtifactCall::Document)?;
        Ok(state
            .documents
            .get(&(
                publication.tenant_id.clone(),
                publication.election_event_id.clone(),
                document_id.into(),
            ))
            .cloned())
    }
    async fn delete_documents(
        &self,
        publication: &TallyResultsPublication,
        document_ids: &[String],
    ) -> Result<()> {
        let mut state = self.state();
        state.check(ArtifactCall::DeleteDocuments)?;
        for id in document_ids {
            let key = (
                publication.tenant_id.clone(),
                publication.election_event_id.clone(),
                id.clone(),
            );
            state.documents.remove(&key);
            state.removed_documents.insert(key);
        }
        Ok(())
    }
    async fn delete_public_path(&self, path: String) -> Result<()> {
        let mut state = self.state();
        state.check(ArtifactCall::DeletePublic)?;
        state.removed_public_paths.insert(path);
        Ok(())
    }
    async fn delete_private_path(&self, path: String) -> Result<()> {
        let mut state = self.state();
        state.check(ArtifactCall::DeletePrivate)?;
        state.removed_private_paths.insert(path);
        Ok(())
    }
    async fn upload_index(&self, key: &str, index: &Value) -> Result<()> {
        let mut state = self.state();
        state.check(ArtifactCall::UploadIndex)?;
        state.indexes.insert(key.into(), index.clone());
        Ok(())
    }
    async fn upload_latest_manifest(
        &self,
        publication: &TallyResultsPublication,
        manifest: &Value,
    ) -> Result<()> {
        let mut state = self.state();
        state.check(ArtifactCall::UploadLatest)?;
        state
            .latest_manifests
            .insert(publication.id.clone(), manifest.clone());
        Ok(())
    }
}
