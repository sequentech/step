// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::domain::publication_files::{PublishedBallotStyle, FILES_ANNOTATION};
use crate::ports::publication_files::{PublicationObjects, PublicationRows, PublicationStorage};
use anyhow::{anyhow, bail, Result};
use futures::Stream;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;
use uuid::Uuid;

/// The rows of one ballot publication.
#[derive(Clone, Debug, Default)]
pub struct PublicationRecord {
    pub annotations: Option<Value>,
    pub elections: Vec<Value>,
    pub styles: Vec<Value>,
    /// Streaming the styles fails after this many rows.
    pub styles_fail_after: Option<usize>,
}

#[derive(Debug, Default)]
struct Tables {
    events: HashMap<(Uuid, Uuid), (Value, Option<Value>)>,
    publications: HashMap<(Uuid, Uuid, Uuid), PublicationRecord>,
    published_styles: HashMap<(Uuid, Uuid, Uuid), Vec<PublishedBallotStyle>>,
}

/// Rows keyed by the identifiers the queries filter on, so a query with a
/// wrong identifier finds nothing.
#[derive(Debug, Default)]
pub struct MemoryPublicationRows(Mutex<Tables>);

impl MemoryPublicationRows {
    /// `object` is the event as stored in `event.json`, before its shared
    /// presentation is added.
    pub fn with_event(
        self,
        tenant: Uuid,
        event: Uuid,
        object: Value,
        status: Option<Value>,
    ) -> Self {
        self.tables()
            .events
            .insert((tenant, event), (object, status));
        self
    }

    pub fn with_publication(
        self,
        tenant: Uuid,
        event: Uuid,
        publication: Uuid,
        record: PublicationRecord,
    ) -> Self {
        self.tables()
            .publications
            .insert((tenant, event, publication), record);
        self
    }

    /// The published styles of an area, in the order the voter query returns them.
    pub fn with_published_styles(
        self,
        tenant: Uuid,
        event: Uuid,
        area: Uuid,
        styles: Vec<PublishedBallotStyle>,
    ) -> Self {
        self.tables()
            .published_styles
            .insert((tenant, event, area), styles);
        self
    }

    pub fn stored_annotations(
        &self,
        tenant: Uuid,
        event: Uuid,
        publication: Uuid,
    ) -> Option<Value> {
        self.tables()
            .publications
            .get(&(tenant, event, publication))
            .and_then(|record| record.annotations.clone())
    }

    fn tables(&self) -> MutexGuard<'_, Tables> {
        self.0.lock().expect("publication rows lock")
    }
}

impl PublicationRows for MemoryPublicationRows {
    async fn annotations(
        &self,
        tenant: Uuid,
        event: Uuid,
        publication: Uuid,
    ) -> Result<Option<Value>> {
        self.tables()
            .publications
            .get(&(tenant, event, publication))
            .map(|record| record.annotations.clone())
            .ok_or_else(|| anyhow!("no ballot publication row"))
    }

    async fn event(&self, tenant: Uuid, event: Uuid) -> Result<Value> {
        self.tables()
            .events
            .get(&(tenant, event))
            .map(|(object, _)| object.clone())
            .ok_or_else(|| anyhow!("no election event row"))
    }

    async fn elections(&self, tenant: Uuid, event: Uuid, publication: Uuid) -> Result<Vec<Value>> {
        Ok(self
            .tables()
            .publications
            .get(&(tenant, event, publication))
            .map(|record| record.elections.clone())
            .unwrap_or_default())
    }

    async fn styles(
        &self,
        tenant: Uuid,
        event: Uuid,
        publication: Uuid,
    ) -> Result<impl Stream<Item = Result<Value>> + Send> {
        let record = self
            .tables()
            .publications
            .get(&(tenant, event, publication))
            .cloned()
            .unwrap_or_default();
        let mut rows: Vec<Result<Value>> = record.styles.into_iter().map(Ok).collect();
        if let Some(count) = record.styles_fail_after {
            rows.truncate(count);
            rows.push(Err(anyhow!("style row stream failed")));
        }
        Ok(futures::stream::iter(rows))
    }

    async fn set_files_root(
        &self,
        tenant: Uuid,
        event: Uuid,
        publication: Uuid,
        root: &str,
    ) -> Result<()> {
        if let Some(record) = self
            .tables()
            .publications
            .get_mut(&(tenant, event, publication))
        {
            let mut annotations = match record.annotations.take() {
                Some(Value::Object(annotations)) => annotations,
                _ => Map::new(),
            };
            annotations.insert(FILES_ANNOTATION.to_owned(), Value::String(root.to_owned()));
            record.annotations = Some(Value::Object(annotations));
        }
        Ok(())
    }

    async fn published_styles(
        &self,
        tenant: Uuid,
        event: Uuid,
        area: Uuid,
        elections: &[Uuid],
    ) -> Result<Vec<PublishedBallotStyle>> {
        Ok(self
            .tables()
            .published_styles
            .get(&(tenant, event, area))
            .map(|styles| {
                styles
                    .iter()
                    .filter(|style| elections.contains(&style.election_id))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn event_status(&self, tenant: Uuid, event: Uuid) -> Result<Option<Option<Value>>> {
        Ok(self
            .tables()
            .events
            .get(&(tenant, event))
            .map(|(_, status)| status.clone()))
    }
}

#[derive(Debug, Default)]
struct Bucket {
    objects: Vec<(String, Vec<u8>)>,
    unavailable: bool,
    failing_key: Option<String>,
    failing_presign: bool,
    signatures: usize,
}

/// A private bucket that keeps objects in the order they were written.
/// Every presigned URL carries a new signature, as real ones do when signed
/// at different times.
#[derive(Clone, Debug, Default)]
pub struct MemoryPublicationObjects(Arc<Mutex<Bucket>>);

impl MemoryPublicationObjects {
    /// Opening fails, as with missing S3 configuration.
    pub fn unavailable() -> Self {
        Self::with(|bucket| bucket.unavailable = true)
    }

    pub fn failing_at(key: impl Into<String>) -> Self {
        let key = key.into();
        Self::with(|bucket| bucket.failing_key = Some(key))
    }

    pub fn failing_presign() -> Self {
        Self::with(|bucket| bucket.failing_presign = true)
    }

    pub fn keys(&self) -> Vec<String> {
        self.bucket()
            .objects
            .iter()
            .map(|(key, _)| key.clone())
            .collect()
    }

    pub fn json(&self, key: &str) -> Option<Value> {
        self.bucket()
            .objects
            .iter()
            .find(|(stored, _)| stored == key)
            .map(|(_, bytes)| serde_json::from_slice(bytes).expect("stored JSON"))
    }

    fn with(configure: impl FnOnce(&mut Bucket)) -> Self {
        let objects = Self::default();
        configure(&mut objects.bucket());
        objects
    }

    fn bucket(&self) -> MutexGuard<'_, Bucket> {
        self.0.lock().expect("bucket lock")
    }
}

impl PublicationStorage for MemoryPublicationObjects {
    type Objects = Self;

    async fn open(&self) -> Result<Self> {
        if self.bucket().unavailable {
            bail!("private bucket unavailable");
        }
        Ok(self.clone())
    }
}

impl PublicationObjects for MemoryPublicationObjects {
    async fn put_immutable_json(&self, key: &str, value: &Value) -> Result<()> {
        let bytes = serde_json::to_vec(value)?;
        let mut bucket = self.bucket();
        if bucket.failing_key.as_deref() == Some(key) {
            bail!("upload rejected");
        }
        if bucket.objects.iter().any(|(stored, _)| stored == key) {
            bail!("object already exists");
        }
        bucket.objects.push((key.to_owned(), bytes));
        Ok(())
    }

    async fn presign_get(&self, key: &str, expires_in: Duration) -> Result<String> {
        let mut bucket = self.bucket();
        if bucket.failing_presign {
            bail!("presigning failed");
        }
        bucket.signatures += 1;
        Ok(format!(
            "https://bucket.test/{key}?expires={}&signature={}",
            expires_in.as_secs(),
            bucket.signatures
        ))
    }
}
