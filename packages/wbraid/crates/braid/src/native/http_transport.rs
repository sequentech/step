// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! HTTP+S3 transport (§8, M2): the braid-side client that talks to a live b4 over
//! HTTP, implementing [`Transport`].
//!
//! Posting uses b4's two-step flow (`initiate` → S3 `PUT` or inline → `confirm`);
//! fetching pulls all of a board's messages and, for S3-backed bodies, downloads
//! them via the presigned URL b4 returns. b4 is a dumb opaque store (§8), so this
//! client (de)serializes `ProtocolMessage<C>` itself and enforces the version
//! exact-match at the boundary (§10.1). Verification happens later in the board
//! client — the transport only moves bytes.

use std::sync::Arc;

use anyhow::{anyhow, bail, Context as _, Result};
use async_trait::async_trait;

use cryptography::context::Context;
use cryptography::utils::serialization::{Deserializable, Serializable};

use crate::messages::wire::{schema_version, MessageType, ProtocolMessage};
use b4::api_types::{
    ConfirmMessageRequest, ContentType, CreateBoardRequest, GetBlobsResponse,
    InitiateMessageRequest, InitiateMessageResponse,
};

use crate::board::transport::{RequestHeaders, StagedRef, Transport};

/// HTTP client for a single b4 board. Not parameterized by `C`: it moves opaque
/// bytes, and the `Transport<C>` impl (de)serializes per `C`.
pub struct HttpTransport {
    client: reqwest::Client,
    base_url: String,
    board: String,
    /// Extra headers for the board service's own requests; `None` sends none.
    headers: Option<Arc<dyn RequestHeaders + Send + Sync>>,
}

impl HttpTransport {
    pub fn new(base_url: &str, board: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            board: board.to_string(),
            headers: None,
        }
    }

    /// Send `headers` with every request to the board service: the message
    /// list, `initiate` and `confirm`. They are asked for before each request
    /// and never sent to the presigned storage URLs.
    pub fn with_headers(mut self, headers: Arc<dyn RequestHeaders + Send + Sync>) -> Self {
        self.headers = Some(headers);
        self
    }

    /// A request to the board service itself, carrying the embedder's headers.
    /// Presigned storage requests are built from `self.client` directly.
    async fn board_request(
        &self,
        method: reqwest::Method,
        url: &str,
    ) -> Result<reqwest::RequestBuilder> {
        let mut request = self.client.request(method, url);
        if let Some(headers) = &self.headers {
            let headers = headers
                .headers()
                .await
                .context("failed to get the board request headers")?;
            for (name, value) in headers {
                request = request.header(name, value);
            }
        }
        Ok(request)
    }

    /// Create the board on b4. Board creation is untrusted namespacing (§8.3): a
    /// duplicate/failed create is an availability concern only.
    pub async fn create_board(base_url: &str, board: &str) -> Result<()> {
        let client = reqwest::Client::new();
        let url = format!("{}/boards", base_url.trim_end_matches('/'));
        let resp = client
            .post(&url)
            .json(&CreateBoardRequest {
                name: board.to_string(),
            })
            .send()
            .await?;
        if !resp.status().is_success() {
            bail!("failed to create board {}: HTTP {}", board, resp.status());
        }
        Ok(())
    }

    /// Fetch every message on the board as raw bytes, enforcing the version
    /// exact-match (§10.1). S3-backed bodies are downloaded via the presigned URL.
    async fn fetch_raw(&self) -> Result<Vec<Vec<u8>>> {
        let url = format!("{}/boards/{}/messages", self.base_url, self.board);
        let resp = self
            .board_request(reqwest::Method::GET, &url)
            .await?
            .send()
            .await?;
        if !resp.status().is_success() {
            bail!("failed to fetch messages: HTTP {}", resp.status());
        }
        let body: GetBlobsResponse = resp.json().await?;

        let expected_version = schema_version();
        let mut out = Vec::with_capacity(body.messages.len());
        for m in body.messages {
            if m.message.version != expected_version {
                bail!(
                    "version mismatch (§10.1): message {} has version {:?}, expected {:?}",
                    m.message.id,
                    m.message.version,
                    expected_version
                );
            }
            let bytes = match m.message.content_type {
                ContentType::Inline { data } => data,
                ContentType::S3 { key: _ } => {
                    let url = m
                        .download_url
                        .ok_or_else(|| anyhow!("S3 message missing download_url"))?;
                    let s3 = self.client.get(&url).send().await?;
                    if !s3.status().is_success() {
                        bail!("failed to download S3 body: HTTP {}", s3.status());
                    }
                    s3.bytes().await?.to_vec()
                }
            };
            out.push(bytes);
        }
        Ok(out)
    }

    /// Stage bytes (§6.4): `initiate` to reserve an id and a presigned URL, then
    /// upload the body to S3. Nothing is visible on the board yet — b4 records
    /// no row until `confirm` — so a failure here has published nothing and
    /// claimed no slot.
    ///
    /// b4 only offers the S3 path for bodies above `MAX_INLINE_MESSAGE_SIZE`,
    /// which is `0`, so every real message is staged. If b4 ever answers with the
    /// inline path we refuse loudly rather than silently degrade: an inline
    /// message's bytes travel in the `confirm` request, so it could not be
    /// re-sent later from the durable record alone (§6.2 forbids keeping the
    /// body), which is precisely the guarantee staging exists to provide.
    async fn stage_bytes(&self, bytes: Vec<u8>) -> Result<StagedRef> {
        let initiate_url = format!("{}/boards/{}/messages/initiate", self.base_url, self.board);
        let init: InitiateMessageResponse = {
            let resp = self
                .board_request(reqwest::Method::POST, &initiate_url)
                .await?
                .json(&InitiateMessageRequest { size: bytes.len() })
                .send()
                .await?;
            if !resp.status().is_success() {
                bail!("initiate failed: HTTP {}", resp.status());
            }
            resp.json().await?
        };

        if !init.should_upload {
            bail!(
                "b4 offered inline storage for a {}-byte message; the outgoing mailbox \
                 requires staged (S3) bodies so a recorded post can be re-sent without \
                 retaining the body locally (§6.4)",
                bytes.len()
            );
        }
        let upload_url = init
            .upload_url
            .ok_or_else(|| anyhow!("server asked to upload but gave no URL"))?;
        let put = self.client.put(&upload_url).body(bytes).send().await?;
        if !put.status().is_success() {
            bail!("S3 upload failed: HTTP {}", put.status());
        }
        Ok(StagedRef(init.message_id))
    }

    /// Commit a staged message (§6.4): `confirm` by id. b4 reconstructs the S3
    /// key from the board name and the id, so the body is not re-sent, and this
    /// is exactly what makes a recorded post re-publishable after a restart.
    async fn commit_staged(&self, staged: &StagedRef) -> Result<()> {
        let confirm_url = format!(
            "{}/boards/{}/messages/{}/confirm",
            self.base_url, self.board, staged.0
        );
        let confirm = self
            .board_request(reqwest::Method::POST, &confirm_url)
            .await?
            .json(&ConfirmMessageRequest {
                data: None,
                version: schema_version(),
            })
            .send()
            .await?;
        if !confirm.status().is_success() {
            bail!("confirm failed: HTTP {}", confirm.status());
        }
        Ok(())
    }
}

#[async_trait(?Send)]
impl<C: Context> Transport<C> for HttpTransport {
    async fn fetch_configuration(&self) -> Result<ProtocolMessage<C>> {
        for bytes in self.fetch_raw().await? {
            let wm = ProtocolMessage::<C>::deser(&bytes)
                .map_err(|e| anyhow!("failed to deserialize wire message: {:?}", e))?;
            if wm.message_type == MessageType::Configuration {
                return Ok(wm);
            }
        }
        bail!("board {} has no Configuration message", self.board)
    }

    async fn fetch(&self) -> Result<Vec<ProtocolMessage<C>>> {
        let mut out = Vec::new();
        for bytes in self.fetch_raw().await? {
            let wm = ProtocolMessage::<C>::deser(&bytes)
                .map_err(|e| anyhow!("failed to deserialize wire message: {:?}", e))?;
            if wm.message_type != MessageType::Configuration {
                out.push(wm);
            }
        }
        Ok(out)
    }

    async fn stage(&self, message: &ProtocolMessage<C>) -> Result<StagedRef> {
        self.stage_bytes(message.ser()).await
    }

    async fn commit(&self, staged: &StagedRef) -> Result<()> {
        self.commit_staged(staged).await
    }
}

#[cfg(test)]
mod tests {
    //! The embedder's headers reach the board service's own calls, fresh for
    //! each, and never the presigned storage URLs or `create_board`. The peer is
    //! a std `TcpListener` playing both b4 and its storage.

    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread;

    use anyhow::{anyhow, bail, Result};
    use async_trait::async_trait;
    use b4::api_types::{
        ContentType, GetBlobsResponse, InitiateMessageResponse, MessageBlob, MessageBlobWithUrl,
    };
    use cryptography::context::RistrettoCtx;

    use super::HttpTransport;
    use crate::board::transport::{RequestHeaders, StagedRef, Transport};
    use crate::messages::wire::schema_version;

    const BODY: &[u8] = b"staged body";

    /// One request the peer received.
    #[derive(Debug, PartialEq)]
    struct Seen {
        method: String,
        path: String,
        authorization: Option<String>,
    }

    fn seen(method: &str, path: &str, authorization: Option<&str>) -> Seen {
        Seen {
            method: method.to_string(),
            path: path.to_string(),
            authorization: authorization.map(str::to_string),
        }
    }

    /// b4 and its storage on one listener: canned HTTP/1.1 answers for the
    /// board `board`, one request per connection, each recorded before it is
    /// answered (so the log is complete once the client has its response).
    struct Peer {
        base_url: String,
        log: Arc<Mutex<Vec<Seen>>>,
    }

    impl Peer {
        fn start() -> Result<Peer> {
            let listener = TcpListener::bind("127.0.0.1:0")?;
            let base_url = format!("http://{}", listener.local_addr()?);
            let log = Arc::new(Mutex::new(Vec::new()));
            let (url, requests) = (base_url.clone(), Arc::clone(&log));
            thread::spawn(move || {
                for stream in listener.incoming().flatten() {
                    let _ = serve(stream, &url, &requests);
                }
            });
            Ok(Peer { base_url, log })
        }

        fn seen(&self) -> Vec<Seen> {
            std::mem::take(&mut *self.log.lock().expect("peer log poisoned"))
        }
    }

    fn serve(mut stream: TcpStream, base_url: &str, log: &Mutex<Vec<Seen>>) -> std::io::Result<()> {
        let mut buf = Vec::new();
        let mut chunk = [0u8; 4096];
        let head_end = loop {
            if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                break i;
            }
            let n = stream.read(&mut chunk)?;
            if n == 0 {
                return Ok(());
            }
            buf.extend_from_slice(&chunk[..n]);
        };
        let head = String::from_utf8_lossy(&buf[..head_end]).into_owned();
        let mut lines = head.split("\r\n");
        let mut request_line = lines.next().unwrap_or_default().split(' ');
        let method = request_line.next().unwrap_or_default().to_string();
        let path = request_line.next().unwrap_or_default().to_string();
        let header = |name: &str| {
            lines.clone().find_map(|line| {
                let (key, value) = line.split_once(':')?;
                key.trim()
                    .eq_ignore_ascii_case(name)
                    .then(|| value.trim().to_string())
            })
        };
        let authorization = header("authorization");
        let length: usize = header("content-length")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        // Drain the body: closing with unread bytes would reset the connection.
        while buf.len() < head_end + 4 + length {
            let n = stream.read(&mut chunk)?;
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..n]);
        }

        let (status, body) = match format!("{method} {path}").as_str() {
            "POST /boards" => ("200 OK", Vec::new()),
            "GET /boards/board/messages" => (
                "200 OK",
                serde_json::to_vec(&GetBlobsResponse {
                    messages: vec![MessageBlobWithUrl {
                        message: MessageBlob {
                            id: "1".to_string(),
                            content_type: ContentType::S3 {
                                key: "board/1".to_string(),
                            },
                            version: schema_version(),
                        },
                        download_url: Some(format!("{base_url}/storage/1")),
                    }],
                })?,
            ),
            "GET /storage/1" => ("200 OK", BODY.to_vec()),
            "POST /boards/board/messages/initiate" => (
                "200 OK",
                serde_json::to_vec(&InitiateMessageResponse {
                    message_id: "2".to_string(),
                    upload_url: Some(format!("{base_url}/storage/2")),
                    should_upload: true,
                })?,
            ),
            "PUT /storage/2" => ("200 OK", Vec::new()),
            "POST /boards/board/messages/2/confirm" => ("200 OK", br#"{"success":true}"#.to_vec()),
            _ => ("404 Not Found", Vec::new()),
        };
        log.lock().expect("peer log poisoned").push(Seen {
            method,
            path,
            authorization,
        });
        write!(
            stream,
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\
             Connection: close\r\n\r\n",
            body.len()
        )?;
        stream.write_all(&body)
    }

    /// `Authorization: Bearer t<n>`, `n` counting the calls.
    #[derive(Default)]
    struct CountingToken {
        calls: AtomicUsize,
    }

    #[async_trait(?Send)]
    impl RequestHeaders for CountingToken {
        async fn headers(&self) -> Result<Vec<(String, String)>> {
            let n = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
            Ok(vec![("Authorization".to_string(), format!("Bearer t{n}"))])
        }
    }

    /// A token source that cannot give a token.
    struct NoToken;

    #[async_trait(?Send)]
    impl RequestHeaders for NoToken {
        async fn headers(&self) -> Result<Vec<(String, String)>> {
            Err(anyhow!("no token"))
        }
    }

    /// What a trustee does with the board: list (and download), stage
    /// (initiate and upload), commit (confirm).
    async fn list_stage_commit(transport: &HttpTransport) -> Result<()> {
        assert_eq!(transport.fetch_raw().await?, vec![BODY.to_vec()]);
        let staged = transport.stage_bytes(BODY.to_vec()).await?;
        assert_eq!(staged, StagedRef("2".to_string()));
        Transport::<RistrettoCtx>::commit(transport, &staged).await
    }

    #[tokio::test]
    async fn headers_reach_board_calls_only_fresh_each_time() -> Result<()> {
        let peer = Peer::start()?;
        let token = Arc::new(CountingToken::default());
        let transport = HttpTransport::new(&peer.base_url, "board").with_headers(token.clone());

        HttpTransport::create_board(&peer.base_url, "board").await?;
        list_stage_commit(&transport).await?;

        assert_eq!(
            peer.seen(),
            vec![
                seen("POST", "/boards", None),
                seen("GET", "/boards/board/messages", Some("Bearer t1")),
                seen("GET", "/storage/1", None),
                seen("POST", "/boards/board/messages/initiate", Some("Bearer t2")),
                seen("PUT", "/storage/2", None),
                seen(
                    "POST",
                    "/boards/board/messages/2/confirm",
                    Some("Bearer t3")
                ),
            ]
        );
        assert_eq!(token.calls.load(Ordering::SeqCst), 3);
        Ok(())
    }

    #[tokio::test]
    async fn no_headers_by_default() -> Result<()> {
        let peer = Peer::start()?;
        let transport = HttpTransport::new(&peer.base_url, "board");

        list_stage_commit(&transport).await?;

        assert_eq!(
            peer.seen(),
            vec![
                seen("GET", "/boards/board/messages", None),
                seen("GET", "/storage/1", None),
                seen("POST", "/boards/board/messages/initiate", None),
                seen("PUT", "/storage/2", None),
                seen("POST", "/boards/board/messages/2/confirm", None),
            ]
        );
        Ok(())
    }

    #[tokio::test]
    async fn headers_error_fails_the_call_and_sends_nothing() -> Result<()> {
        let peer = Peer::start()?;
        let transport = HttpTransport::new(&peer.base_url, "board").with_headers(Arc::new(NoToken));

        let err = match Transport::<RistrettoCtx>::fetch(&transport).await {
            Ok(_) => bail!("fetch succeeded without headers"),
            Err(err) => err,
        };
        assert!(format!("{err:#}").contains("no token"), "{err:#}");
        assert!(transport.stage_bytes(BODY.to_vec()).await.is_err());
        let staged = StagedRef("2".to_string());
        assert!(Transport::<RistrettoCtx>::commit(&transport, &staged)
            .await
            .is_err());

        assert!(peer.seen().is_empty());
        Ok(())
    }
}
