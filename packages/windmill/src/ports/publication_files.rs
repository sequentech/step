// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::domain::publication_files::PublishedBallotStyle;
use anyhow::Result;
use futures::Stream;
use serde_json::Value;
use std::future::Future;
use std::time::Duration;
use uuid::Uuid;

/// The rows a publication's objects are prepared from, and the references
/// voters are served from.
pub trait PublicationRows: Sync {
    /// Fails when the publication does not exist.
    fn annotations(
        &self,
        tenant: Uuid,
        event: Uuid,
        publication: Uuid,
    ) -> impl Future<Output = Result<Option<Value>>> + Send;

    /// The event as stored in `event.json`, before its shared presentation is
    /// added. Fails when the event does not exist.
    fn event(&self, tenant: Uuid, event: Uuid) -> impl Future<Output = Result<Value>> + Send;

    fn elections(
        &self,
        tenant: Uuid,
        event: Uuid,
        publication: Uuid,
    ) -> impl Future<Output = Result<Vec<Value>>> + Send;

    /// Full styles, EML included, one at a time: memory must not grow with
    /// the number of areas.
    fn styles(
        &self,
        tenant: Uuid,
        event: Uuid,
        publication: Uuid,
    ) -> impl Future<Output = Result<impl Stream<Item = Result<Value>> + Send>> + Send;

    /// Records `root` in the publication's `FILES_ANNOTATION`, keeping its
    /// other annotations.
    fn set_files_root(
        &self,
        tenant: Uuid,
        event: Uuid,
        publication: Uuid,
        root: &str,
    ) -> impl Future<Output = Result<()>> + Send;

    /// Newest publication first.
    fn published_styles(
        &self,
        tenant: Uuid,
        event: Uuid,
        area: Uuid,
        elections: &[Uuid],
    ) -> impl Future<Output = Result<Vec<PublishedBallotStyle>>> + Send;

    /// `None` when the event does not exist; `Some(None)` when it has no status.
    fn event_status(
        &self,
        tenant: Uuid,
        event: Uuid,
    ) -> impl Future<Output = Result<Option<Option<Value>>>> + Send;
}

/// Opens the private bucket. Services open it only once they need it, after
/// the checks that do not.
pub trait PublicationStorage: Sync {
    type Objects: PublicationObjects + Send;

    fn open(&self) -> impl Future<Output = Result<Self::Objects>> + Send;
}

pub trait PublicationObjects: Sync {
    /// Creates `key` only if it does not exist, then verifies that the stored
    /// bytes are the serialized `value`.
    fn put_immutable_json(
        &self,
        key: &str,
        value: &Value,
    ) -> impl Future<Output = Result<()>> + Send;

    /// A URL that reads `key` until it expires.
    fn presign_get(
        &self,
        key: &str,
        expires_in: Duration,
    ) -> impl Future<Output = Result<String>> + Send;
}
