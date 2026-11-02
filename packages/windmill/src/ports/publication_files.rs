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

/// The rows behind a prepared publication's objects, and the references
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

/// The styles a generation task committed, read for upload after its
/// transaction ended, while the task keeps its lease.
pub trait GeneratedStyles: Sync {
    /// Fails when the task's lease expired or another task took it over.
    fn keep_alive(&self) -> impl Future<Output = Result<()>> + Send;

    /// Up to `limit` full styles after `after`, by id, EML included: memory
    /// must not grow with the number of areas.
    fn page(
        &self,
        after: Option<Uuid>,
        limit: i64,
    ) -> impl Future<Output = Result<Vec<(Uuid, Value)>>> + Send;
}

/// Opens the private bucket. Services open it only once they need it, after
/// the checks that do not.
pub trait PublicationStorage: Sync {
    type Objects: PublicationObjects + Send;

    fn open(&self) -> impl Future<Output = Result<Self::Objects>> + Send;
}

pub trait PublicationObjects: Sync {
    /// Creates `key` only if it does not exist, then verifies that the stored
    /// bytes are the serialized `value`: a retried upload finds its own object.
    fn put_immutable_json(
        &self,
        key: &str,
        value: &Value,
    ) -> impl Future<Output = Result<()>> + Send;

    /// The stored object. Fails when `key` does not exist.
    fn get_json(&self, key: &str) -> impl Future<Output = Result<Value>> + Send;

    /// A URL that reads `key` until it expires.
    fn presign_get(
        &self,
        key: &str,
        expires_in: Duration,
    ) -> impl Future<Output = Result<String>> + Send;
}
