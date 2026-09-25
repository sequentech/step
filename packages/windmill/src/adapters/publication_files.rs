// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::domain::publication_files::{PublishedBallotStyle, FILES_ANNOTATION};
use crate::ports::publication_files::{PublicationObjects, PublicationRows, PublicationStorage};
use crate::postgres::publication_files::{
    get_election_event_status, get_publication_annotations, get_publication_elections,
    get_publication_event, get_published_ballot_styles, merge_ballot_publication_annotation,
    stream_publication_styles,
};
use anyhow::{bail, Context, Result};
use aws_sdk_s3::{presigning::PresigningConfig, primitives::ByteStream, Client};
use deadpool_postgres::Transaction;
use futures::Stream;
use sequent_core::services::s3::{get_private_bucket, get_s3_client};
use sequent_core::util::aws::get_s3_aws_config;
use serde_json::Value;
use std::time::Duration;
use uuid::Uuid;

/// Reads and writes in the caller's transaction.
pub struct PgPublicationRows<'a> {
    pub transaction: &'a Transaction<'a>,
}

impl PublicationRows for PgPublicationRows<'_> {
    async fn annotations(
        &self,
        tenant: Uuid,
        event: Uuid,
        publication: Uuid,
    ) -> Result<Option<Value>> {
        get_publication_annotations(self.transaction, tenant, event, publication).await
    }

    async fn event(&self, tenant: Uuid, event: Uuid) -> Result<Value> {
        get_publication_event(self.transaction, tenant, event).await
    }

    async fn elections(&self, tenant: Uuid, event: Uuid, publication: Uuid) -> Result<Vec<Value>> {
        get_publication_elections(self.transaction, tenant, event, publication).await
    }

    async fn styles(
        &self,
        tenant: Uuid,
        event: Uuid,
        publication: Uuid,
    ) -> Result<impl Stream<Item = Result<Value>> + Send> {
        stream_publication_styles(self.transaction, tenant, event, publication).await
    }

    async fn set_files_root(
        &self,
        tenant: Uuid,
        event: Uuid,
        publication: Uuid,
        root: &str,
    ) -> Result<()> {
        merge_ballot_publication_annotation(
            self.transaction,
            tenant,
            event,
            publication,
            FILES_ANNOTATION,
            root,
        )
        .await
    }

    async fn published_styles(
        &self,
        tenant: Uuid,
        event: Uuid,
        area: Uuid,
        elections: &[Uuid],
    ) -> Result<Vec<PublishedBallotStyle>> {
        get_published_ballot_styles(
            self.transaction,
            tenant,
            event,
            area,
            elections,
            FILES_ANNOTATION,
        )
        .await
    }

    async fn event_status(&self, tenant: Uuid, event: Uuid) -> Result<Option<Option<Value>>> {
        get_election_event_status(self.transaction, tenant, event).await
    }
}

/// The endpoint an S3 client targets. Uploads use the one servers reach;
/// presigned URLs must name the one voters' browsers reach.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum S3Endpoint {
    Server,
    Public,
}

/// The private bucket, configured from the environment when opened.
pub struct S3PublicationStorage {
    pub endpoint: S3Endpoint,
}

impl PublicationStorage for S3PublicationStorage {
    type Objects = S3PublicationObjects;

    async fn open(&self) -> Result<S3PublicationObjects> {
        let bucket = get_private_bucket()?;
        let client =
            get_s3_client(get_s3_aws_config(self.endpoint == S3Endpoint::Server).await?).await?;
        Ok(S3PublicationObjects { client, bucket })
    }
}

pub struct S3PublicationObjects {
    client: Client,
    bucket: String,
}

impl PublicationObjects for S3PublicationObjects {
    async fn put_immutable_json(&self, key: &str, value: &Value) -> Result<()> {
        let bytes = serde_json::to_vec(value)?;
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type("application/json")
            .cache_control("private, max-age=300")
            .if_none_match("*")
            .body(ByteStream::from(bytes.clone()))
            .send()
            .await
            .context("Cannot upload ballot publication object")?;
        let stored = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .context("Cannot validate ballot publication object")?
            .body
            .collect()
            .await?
            .into_bytes();
        if stored.as_ref() != bytes.as_slice() {
            bail!("Ballot publication object verification failed");
        }
        Ok(())
    }

    async fn presign_get(&self, key: &str, expires_in: Duration) -> Result<String> {
        let signed = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .presigned(PresigningConfig::expires_in(expires_in)?)
            .await?;
        Ok(signed.uri().to_string())
    }
}
