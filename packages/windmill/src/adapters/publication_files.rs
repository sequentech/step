// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::domain::publication_files::{PublishedBallotStyle, FILES_ANNOTATION};
use crate::ports::publication_files::{
    GeneratedStyles, PublicationObjects, PublicationRows, PublicationStorage,
};
use crate::postgres::publication_files::{
    get_election_event_status, get_publication_annotations, get_publication_elections,
    get_publication_event, get_publication_style_page, get_published_ballot_styles,
    stream_publication_styles,
};
use crate::services::ballot_styles::publication_files::heartbeat;
use crate::services::pg_lock::PgLock;
use anyhow::{bail, Context, Result};
use aws_sdk_s3::{presigning::PresigningConfig, primitives::ByteStream, Client};
use deadpool_postgres::{Pool, Transaction};
use futures::Stream;
use sequent_core::services::s3::{self, get_private_bucket, get_shared_s3_client};
use serde_json::Value;
use std::time::Duration;
use uuid::Uuid;

/// The longest a single upload or its read-back may take.
const OBJECT_TIMEOUT: Duration = Duration::from_secs(60);

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

/// A generation task's committed styles, each page read on a connection of
/// its own, and its lease renewed through the pool.
pub struct PgGeneratedStyles<'a> {
    pub pool: &'a Pool,
    pub lease: &'a PgLock,
    pub tenant: Uuid,
    pub event: Uuid,
    pub publication: Uuid,
}

impl GeneratedStyles for PgGeneratedStyles<'_> {
    async fn keep_alive(&self) -> Result<()> {
        heartbeat(self.pool, self.lease).await
    }

    async fn page(&self, after: Option<Uuid>, limit: i64) -> Result<Vec<(Uuid, Value)>> {
        let client = self.pool.get().await?;
        get_publication_style_page(
            &client,
            self.tenant,
            self.event,
            self.publication,
            after,
            limit,
        )
        .await
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
        let client = get_shared_s3_client(match self.endpoint {
            S3Endpoint::Server => s3::S3Endpoint::Server,
            S3Endpoint::Public => s3::S3Endpoint::Client,
        })
        .await?;
        Ok(S3PublicationObjects { client, bucket })
    }
}

pub struct S3PublicationObjects {
    client: Client,
    bucket: String,
}

impl PublicationObjects for S3PublicationObjects {
    async fn put_immutable_json(&self, key: &str, value: &Value) -> Result<()> {
        tokio::time::timeout(OBJECT_TIMEOUT, async {
            let bytes = serde_json::to_vec(value)?;
            let result = self
                .client
                .put_object()
                .bucket(&self.bucket)
                .key(key)
                .content_type("application/json")
                .cache_control("private, max-age=300")
                .if_none_match("*")
                .body(ByteStream::from(bytes.clone()))
                .send()
                .await;
            if let Err(error) = result {
                // A retry after a lost PUT response can find the immutable object already there.
                if error
                    .raw_response()
                    .map(|response| response.status().as_u16())
                    != Some(412)
                {
                    return Err(error).context("Cannot upload ballot publication object");
                }
            }
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
        })
        .await
        .context("Ballot publication upload timed out")?
    }

    async fn get_json(&self, key: &str) -> Result<Value> {
        let stored = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .context("Cannot read ballot publication object")?
            .body
            .collect()
            .await?
            .into_bytes();
        serde_json::from_slice(&stored).context("Ballot publication object is not JSON")
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

#[cfg(test)]
mod tests {
    //! The S3 contract against a local stand-in: the SDK builds and sends
    //! real requests, and the stand-in records them.

    use super::*;
    use aws_sdk_s3::config::{BehaviorVersion, Credentials, Region};
    use serde_json::json;
    use std::sync::{Arc, Mutex};
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
    use tokio::net::{TcpListener, TcpStream};

    const KEY: &str = "tenant-a/event-b/publication-c/d/event.json";
    const OBJECT_PATH: &str = "/private/tenant-a/event-b/publication-c/d/event.json";

    fn objects(endpoint: &str) -> S3PublicationObjects {
        let config = aws_sdk_s3::Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new("us-east-1"))
            .credentials_provider(Credentials::new("access", "secret", None, None, "test"))
            .endpoint_url(endpoint)
            .force_path_style(true)
            .build();
        S3PublicationObjects {
            client: Client::from_conf(config),
            bucket: "private".to_owned(),
        }
    }

    #[derive(Clone, Copy)]
    enum Store {
        Keeps,
        RejectsExisting,
        /// Stores the upload but answers that it existed, as a retry after a
        /// lost response finds its own object.
        AlreadyHolds,
        Loses,
        Corrupts,
    }

    #[derive(Debug)]
    struct Request {
        method: String,
        path: String,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    }

    impl Request {
        fn header(&self, name: &str) -> Option<&str> {
            self.headers
                .iter()
                .find(|(header, _)| header == name)
                .map(|(_, value)| value.as_str())
        }

        fn object_path(&self) -> &str {
            self.path.split('?').next().unwrap_or_default()
        }
    }

    async fn read_request(socket: &mut BufReader<TcpStream>) -> Option<Request> {
        let mut line = String::new();
        if socket.read_line(&mut line).await.ok()? == 0 {
            return None;
        }
        let mut words = line.split_whitespace();
        let (method, path) = (words.next()?.to_owned(), words.next()?.to_owned());
        let mut headers = Vec::new();
        loop {
            let mut header = String::new();
            socket.read_line(&mut header).await.ok()?;
            let Some((name, value)) = header.trim_end().split_once(':') else {
                break;
            };
            headers.push((name.to_ascii_lowercase(), value.trim().to_owned()));
        }
        let length = headers
            .iter()
            .find(|(name, _)| name == "content-length")
            .map_or(Some(0), |(_, value)| value.parse().ok())?;
        let mut body = vec![0; length];
        socket.read_exact(&mut body).await.ok()?;
        Some(Request {
            method,
            path,
            headers,
            body,
        })
    }

    /// Serves one object on a local port and records every request.
    async fn fake_s3(store: Store) -> (String, Arc<Mutex<Vec<Request>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let stored = Arc::new(Mutex::new(Vec::new()));
        let log = requests.clone();
        tokio::spawn(async move {
            while let Ok((socket, _)) = listener.accept().await {
                let (log, stored) = (log.clone(), stored.clone());
                tokio::spawn(async move {
                    let mut socket = BufReader::new(socket);
                    while let Some(request) = read_request(&mut socket).await {
                        let (status, body) = match (request.method.as_str(), store) {
                            ("PUT", Store::RejectsExisting) => (412, Vec::new()),
                            ("PUT", Store::AlreadyHolds) => {
                                *stored.lock().unwrap() = request.body.clone();
                                (412, Vec::new())
                            }
                            ("PUT", _) => {
                                *stored.lock().unwrap() = request.body.clone();
                                (200, Vec::new())
                            }
                            (_, Store::Loses) => (404, Vec::new()),
                            (_, Store::Corrupts) => (200, b"{}".to_vec()),
                            _ => (200, stored.lock().unwrap().clone()),
                        };
                        log.lock().unwrap().push(request);
                        let head = format!(
                            "HTTP/1.1 {status} Fake\r\ncontent-length: {}\r\n\r\n",
                            body.len()
                        );
                        let socket = socket.get_mut();
                        if socket.write_all(head.as_bytes()).await.is_err()
                            || socket.write_all(&body).await.is_err()
                        {
                            return;
                        }
                    }
                });
            }
        });
        (endpoint, requests)
    }

    async fn put(endpoint: &str, value: &Value) -> Result<()> {
        tokio::time::timeout(
            Duration::from_secs(30),
            objects(endpoint).put_immutable_json(KEY, value),
        )
        .await
        .expect("the fake S3 answers")
    }

    #[tokio::test]
    async fn objects_are_created_once_as_private_json_and_read_back() {
        let (endpoint, requests) = fake_s3(Store::Keeps).await;
        put(
            &endpoint,
            &json!({"id": "style", "ballot_eml_prefix": "{\"a\":"}),
        )
        .await
        .unwrap();

        let requests = requests.lock().unwrap();
        let [upload, read_back] = requests.as_slice() else {
            panic!("expected an upload and a read-back: {requests:?}");
        };
        assert_eq!(
            (upload.method.as_str(), upload.object_path()),
            ("PUT", OBJECT_PATH)
        );
        assert_eq!(upload.header("content-type"), Some("application/json"));
        assert_eq!(upload.header("cache-control"), Some("private, max-age=300"));
        assert_eq!(upload.header("if-none-match"), Some("*"));
        assert_eq!(
            upload.body,
            br#"{"id":"style","ballot_eml_prefix":"{\"a\":"}"#
        );
        assert_eq!(
            (read_back.method.as_str(), read_back.object_path()),
            ("GET", OBJECT_PATH)
        );
    }

    #[tokio::test]
    async fn an_existing_object_is_not_replaced() {
        let (endpoint, requests) = fake_s3(Store::RejectsExisting).await;
        let error = put(&endpoint, &json!({})).await.unwrap_err();
        assert_eq!(
            error.to_string(),
            "Ballot publication object verification failed"
        );
        // The existing object is only read back, never written over.
        let requests = requests.lock().unwrap();
        let methods: Vec<&str> = requests.iter().map(|r| r.method.as_str()).collect();
        assert_eq!(methods, ["PUT", "GET"]);
    }

    #[tokio::test]
    async fn a_retried_upload_accepts_its_own_existing_object() {
        let (endpoint, _) = fake_s3(Store::AlreadyHolds).await;
        put(&endpoint, &json!({"id": "style"})).await.unwrap();
    }

    #[tokio::test]
    async fn an_object_that_cannot_be_read_back_is_not_verified() {
        let (endpoint, _) = fake_s3(Store::Loses).await;
        let error = put(&endpoint, &json!({})).await.unwrap_err();
        assert_eq!(
            error.to_string(),
            "Cannot validate ballot publication object"
        );
    }

    #[tokio::test]
    async fn an_object_read_back_with_other_bytes_fails_verification() {
        let (endpoint, _) = fake_s3(Store::Corrupts).await;
        let error = put(&endpoint, &json!({"id": "style"})).await.unwrap_err();
        assert_eq!(
            error.to_string(),
            "Ballot publication object verification failed"
        );
    }

    #[tokio::test]
    async fn a_stored_object_is_read_as_json() {
        let (endpoint, requests) = fake_s3(Store::Keeps).await;
        let value = json!({"id": "event", "presentation": {"css": ".a {}"}});
        put(&endpoint, &value).await.unwrap();
        assert_eq!(objects(&endpoint).get_json(KEY).await.unwrap(), value);
        let requests = requests.lock().unwrap();
        let read = requests.last().expect("a read");
        assert_eq!(
            (read.method.as_str(), read.object_path()),
            ("GET", OBJECT_PATH)
        );
    }

    #[tokio::test]
    async fn a_missing_or_malformed_object_cannot_be_read() {
        let (endpoint, _) = fake_s3(Store::Loses).await;
        let error = objects(&endpoint).get_json(KEY).await.unwrap_err();
        assert_eq!(error.to_string(), "Cannot read ballot publication object");

        // Nothing was stored, so the stand-in answers with an empty body.
        let (endpoint, _) = fake_s3(Store::Keeps).await;
        let error = objects(&endpoint).get_json(KEY).await.unwrap_err();
        assert_eq!(error.to_string(), "Ballot publication object is not JSON");
    }

    #[tokio::test]
    async fn presigned_urls_read_the_private_object_until_they_expire() {
        let url = objects("http://127.0.0.1:9000")
            .presign_get(KEY, Duration::from_secs(120))
            .await
            .unwrap();
        assert!(
            url.starts_with(&format!("http://127.0.0.1:9000{OBJECT_PATH}?")),
            "{url}"
        );
        assert!(url.contains("X-Amz-Expires=120"), "{url}");
        assert!(url.contains("X-Amz-Signature="), "{url}");
    }
}
