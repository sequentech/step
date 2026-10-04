// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The EML side of a transmission's signatures: each approval also signs
//! the package's EML bytes (design §2), which the certificate verifier
//! checks against the request's stored EML.

use super::super::approve::DocumentSigner;
use super::super::certificates::CertificateIdentity;
use super::super::log::Actor;
use super::super::pdf::{PanelDocument, PdfPrepared, RevisionStore};
use super::super::{InvalidReason, SigningCaller, SigningError, SigningResult};
use super::transmission::{sbei_refusal, sha256_hex, SbeiDirectory};
use crate::postgres::signing::SigningRequestRow;
use crate::postgres::signing_document_revision::signing_document_access_restricted;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deadpool_postgres::{Client, Transaction};
use sequent_core::signing::{CertificateCheckId, DocumentKind};
use std::sync::Arc;
use uuid::Uuid;

/// Takes the EML signatures of transmission requests.
pub struct EmlDocumentSigner {
    store: Arc<dyn RevisionStore>,
    sbeis: Arc<dyn SbeiDirectory>,
}

impl EmlDocumentSigner {
    pub fn new(store: Arc<dyn RevisionStore>, sbeis: Arc<dyn SbeiDirectory>) -> Self {
        EmlDocumentSigner { store, sbeis }
    }
}

#[async_trait]
impl DocumentSigner for EmlDocumentSigner {
    fn supports(&self, kind: DocumentKind) -> bool {
        matches!(kind, DocumentKind::Eml | DocumentKind::NoDocument)
    }

    /// The request's EML, as its payload names it (by SHA-256).
    async fn document_bytes(
        &self,
        hasura_transaction: &Transaction<'_>,
        request: &SigningRequestRow,
    ) -> Result<Option<Vec<u8>>> {
        if request.action.document() != DocumentKind::Eml {
            return Ok(None);
        }
        let (Some(document_id), Some(sha256)) = (request.document_id, &request.document_sha256)
        else {
            return Ok(None);
        };
        let bytes = self
            .store
            .load(
                hasura_transaction,
                request.tenant_id,
                request.election_event_id,
                document_id,
            )
            .await?;
        if &sha256_hex(&bytes) != sha256 {
            return Err(anyhow!(
                "The stored document of signing request {} is not the one it signs",
                request.code
            ));
        }
        Ok(Some(bytes))
    }

    /// The request's EML, for the signer to download and sign.
    async fn panel_document(
        &self,
        hasura_transaction: &Transaction<'_>,
        request: &SigningRequestRow,
    ) -> SigningResult<PanelDocument> {
        let Some(document_id) = request.document_id else {
            return Ok(PanelDocument::default());
        };
        let (tenant_id, event_id) = (request.tenant_id, request.election_event_id);
        if signing_document_access_restricted(hasura_transaction, tenant_id, event_id, document_id)
            .await?
        {
            return Ok(PanelDocument::default());
        }
        let link = self
            .store
            .link(hasura_transaction, tenant_id, event_id, document_id)
            .await?;
        Ok(PanelDocument {
            url: link.as_ref().map(|link| link.url.clone()),
            name: link.and_then(|link| link.name),
            revision: None,
        })
    }

    /// The EML signature was verified with the certificate and is kept
    /// with the approval. When the Post has a transmission configuration of
    /// its people, the central servers know only them: anyone else, or a
    /// certificate other than the one a 2025 signature pinned, is refused.
    async fn embed(
        &self,
        hasura_transaction: &Transaction<'_>,
        request: &SigningRequestRow,
        signer: &Actor,
        certificate: &CertificateIdentity,
        pdf_cms: Option<&[u8]>,
        _revision: Option<i32>,
    ) -> SigningResult<()> {
        if pdf_cms.is_some() {
            return Err(SigningError::invalid(
                InvalidReason::Document,
                format!("Signing request {} signs no PDF.", request.code),
            ));
        }
        let sbeis = self.sbeis.post_sbeis(hasura_transaction, request).await?;
        match sbei_refusal(
            sbeis.as_ref(),
            &signer.username,
            &certificate.fingerprint_sha256,
        ) {
            Some(message) => Err(SigningError::Refused {
                check: CertificateCheckId::Registered,
                message: message.into(),
                other_holder: None,
            }),
            None => Ok(()),
        }
    }
}

/// The document side of every action: EML requests go to `eml`, the
/// others to `other`.
pub struct DocumentSigners {
    eml: Arc<dyn DocumentSigner>,
    other: Arc<dyn DocumentSigner>,
}

impl DocumentSigners {
    pub fn new(eml: Arc<dyn DocumentSigner>, other: Arc<dyn DocumentSigner>) -> Self {
        DocumentSigners { eml, other }
    }

    fn of(&self, kind: DocumentKind) -> &Arc<dyn DocumentSigner> {
        match kind {
            DocumentKind::Eml => &self.eml,
            DocumentKind::Pdf | DocumentKind::NoDocument => &self.other,
        }
    }
}

#[async_trait]
impl DocumentSigner for DocumentSigners {
    fn supports(&self, kind: DocumentKind) -> bool {
        self.of(kind).supports(kind)
    }

    async fn document_bytes(
        &self,
        hasura_transaction: &Transaction<'_>,
        request: &SigningRequestRow,
    ) -> Result<Option<Vec<u8>>> {
        self.of(request.action.document())
            .document_bytes(hasura_transaction, request)
            .await
    }

    async fn prepare_pdf(
        &self,
        client: &mut Client,
        caller: &SigningCaller,
        tenant_id: Uuid,
        request_id: Uuid,
        chain_pem: &[String],
        now: DateTime<Utc>,
    ) -> SigningResult<PdfPrepared> {
        self.of(DocumentKind::Pdf)
            .prepare_pdf(client, caller, tenant_id, request_id, chain_pem, now)
            .await
    }

    async fn panel_document(
        &self,
        hasura_transaction: &Transaction<'_>,
        request: &SigningRequestRow,
    ) -> SigningResult<PanelDocument> {
        self.of(request.action.document())
            .panel_document(hasura_transaction, request)
            .await
    }

    async fn embed(
        &self,
        hasura_transaction: &Transaction<'_>,
        request: &SigningRequestRow,
        signer: &Actor,
        certificate: &CertificateIdentity,
        pdf_cms: Option<&[u8]>,
        revision: Option<i32>,
    ) -> SigningResult<()> {
        self.of(request.action.document())
            .embed(
                hasura_transaction,
                request,
                signer,
                certificate,
                pdf_cms,
                revision,
            )
            .await
    }
}
