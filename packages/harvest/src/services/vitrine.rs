// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! A ballot's Vitrine receipt at cast (step-integration.md, section 2.5).
//! Once Windmill has accepted a ballot, Harvest waits a bounded time on the
//! sidecar's internal listener for the `ballots` log to sequence its entry.
//! Transparency never blocks voting: whatever the sidecar does, the cast
//! succeeds, and the portal gets `unavailable` with the entry, to ask for the
//! receipt later.

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use electoral_log::adapters::ballot_box::{QueueState, VitrineEntry};
use electoral_log::vitrine::Submitted;
use std::fmt::Display;
use std::str::FromStr;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};
use tracing::{error, info, warn};
use windmill::services::insert_cast_vote::CastVitrine;
use windmill::services::vitrine::client::{Service, MAX_WAIT_MS};
use windmill::types::vitrine::{
    VitrineCast, VitrineCastKind, VitrineCastReason,
};

const BUDGET_MS_ENV: &str = "HARVEST_VITRINE_BUDGET_MS";
const WAIT_MS_ENV: &str = "HARVEST_VITRINE_WAIT_MS";
const BREAKER_FAILURES_ENV: &str = "HARVEST_VITRINE_BREAKER_FAILURES";
const BREAKER_OPEN_SECS_ENV: &str = "HARVEST_VITRINE_BREAKER_OPEN_SECS";

/// How long a cast waits for its receipt in all.
const DEFAULT_BUDGET_MS: u64 = 800;
/// The `timeout` Harvest gives the sidecar: below the budget, so that the
/// sidecar answers before Harvest gives up.
const DEFAULT_WAIT_MS: u64 = 600;
const DEFAULT_BREAKER_FAILURES: u32 = 5;
const DEFAULT_BREAKER_OPEN_SECS: u64 = 30;
/// The longest answer Harvest reads. A receipt takes a few kilobytes.
const MAX_ANSWER_SIZE: usize = 1 << 20;
/// The body of a `202` without a promise.
const QUEUED_ANSWER: &str = "queued";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReceiptSettings {
    /// How long a cast waits for its receipt in all.
    pub budget: Duration,
    /// The `timeout` of `GET wait`, in milliseconds.
    pub wait_ms: u64,
    /// The failures in a row that open the breaker.
    pub breaker_failures: u32,
    /// How long the breaker stays open.
    pub breaker_open: Duration,
}

impl Default for ReceiptSettings {
    fn default() -> Self {
        ReceiptSettings {
            budget: Duration::from_millis(DEFAULT_BUDGET_MS),
            wait_ms: DEFAULT_WAIT_MS,
            breaker_failures: DEFAULT_BREAKER_FAILURES,
            breaker_open: Duration::from_secs(DEFAULT_BREAKER_OPEN_SECS),
        }
    }
}

fn env_number<T: FromStr + Display + Copy>(name: &str, default: T) -> T {
    match std::env::var(name) {
        Ok(value) if !value.is_empty() => value.parse().unwrap_or_else(|_| {
            warn!("{name} isn't a number: using {default}");
            default
        }),
        _ => default,
    }
}

impl ReceiptSettings {
    /// From `HARVEST_VITRINE_BUDGET_MS`, `HARVEST_VITRINE_WAIT_MS`,
    /// `HARVEST_VITRINE_BREAKER_FAILURES` and
    /// `HARVEST_VITRINE_BREAKER_OPEN_SECS`.
    pub fn from_env() -> Self {
        ReceiptSettings {
            budget: Duration::from_millis(env_number(
                BUDGET_MS_ENV,
                DEFAULT_BUDGET_MS,
            )),
            wait_ms: env_number(WAIT_MS_ENV, DEFAULT_WAIT_MS).min(MAX_WAIT_MS),
            breaker_failures: env_number(
                BREAKER_FAILURES_ENV,
                DEFAULT_BREAKER_FAILURES,
            )
            .max(1),
            breaker_open: Duration::from_secs(env_number(
                BREAKER_OPEN_SECS_ENV,
                DEFAULT_BREAKER_OPEN_SECS,
            )),
        }
    }
}

#[derive(Debug, Default)]
struct BreakerState {
    failures: u32,
    open_until: Option<Instant>,
}

/// Skips the sidecar for a while after failures in a row, so that a sick
/// sidecar costs casts no time. When the period ends, one cast tries again:
/// its success closes the breaker, and its failure opens it for another
/// period.
#[derive(Debug)]
pub struct Breaker {
    failures_to_open: u32,
    open_for: Duration,
    state: Mutex<BreakerState>,
}

impl Breaker {
    pub fn new(failures_to_open: u32, open_for: Duration) -> Self {
        Breaker {
            failures_to_open: failures_to_open.max(1),
            open_for,
            state: Mutex::new(BreakerState::default()),
        }
    }

    fn state(&self) -> MutexGuard<'_, BreakerState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Whether a call may go to the sidecar at `now`.
    pub fn allow(&self, now: Instant) -> bool {
        let mut state = self.state();
        match state.open_until {
            Some(until) if now < until => false,
            Some(_) => {
                state.open_until = Some(now + self.open_for);
                true
            }
            None => true,
        }
    }

    pub fn succeeded(&self) {
        *self.state() = BreakerState::default();
    }

    pub fn failed(&self, now: Instant) {
        let mut state = self.state();
        state.failures = state.failures.saturating_add(1);
        if state.failures >= self.failures_to_open {
            if state.open_until.is_none() {
                warn!(
                    "Vitrine receipts at cast are off for {} s after {} \
                     failures",
                    self.open_for.as_secs(),
                    state.failures
                );
            }
            state.open_until = Some(now + self.open_for);
        }
    }
}

fn answered(kind: VitrineCastKind, text: String) -> VitrineCast {
    VitrineCast {
        kind,
        text: Some(text),
        reason: None,
    }
}

fn queued(entry: &[u8]) -> VitrineCast {
    VitrineCast {
        kind: VitrineCastKind::Queued,
        text: Some(STANDARD.encode(entry)),
        reason: Some(VitrineCastReason::Queued),
    }
}

fn unavailable(entry: Option<&[u8]>, reason: VitrineCastReason) -> VitrineCast {
    VitrineCast {
        kind: VitrineCastKind::Unavailable,
        text: entry.map(|entry| STANDARD.encode(entry)),
        reason: Some(reason),
    }
}

/// Whether the reason says the sidecar is unwell, which counts against the
/// breaker. The other reasons are answers.
fn is_failure(reason: VitrineCastReason) -> bool {
    matches!(
        reason,
        VitrineCastReason::Busy
            | VitrineCastReason::LogUnavailable
            | VitrineCastReason::Timeout
            | VitrineCastReason::Unreachable
            | VitrineCastReason::BadResponse
    )
}

/// What an answer of `GET wait` means for the entry's cast.
pub fn judge(status: u16, body: &str, entry: &[u8]) -> VitrineCast {
    let submitted = Submitted::parse(body)
        .ok()
        .filter(|submitted| submitted.is_for_one_of(&[entry.to_vec()]));
    let reason = match (status, submitted) {
        (200, Some(Submitted::Receipt { .. })) => {
            return answered(VitrineCastKind::Receipt, body.to_string())
        }
        (202, Some(Submitted::Promise { .. })) => {
            return answered(VitrineCastKind::Promise, body.to_string())
        }
        (202, None) if body.trim_end() == QUEUED_ANSWER => {
            return queued(entry)
        }
        (403, _) => VitrineCastReason::Frozen,
        (404, _) => VitrineCastReason::UnknownEntry,
        (422, _) => VitrineCastReason::Rejected,
        (429, _) => VitrineCastReason::Busy,
        (500..=599, _) => VitrineCastReason::LogUnavailable,
        _ => VitrineCastReason::BadResponse,
    };
    unavailable(Some(entry), reason)
}

fn transport_reason(error: &reqwest::Error) -> VitrineCastReason {
    if error.is_timeout() {
        VitrineCastReason::Timeout
    } else if error.is_connect() {
        VitrineCastReason::Unreachable
    } else {
        VitrineCastReason::BadResponse
    }
}

/// The sidecar's internal listener, over HTTP/2 cleartext.
#[derive(Debug)]
struct Sidecar {
    http: reqwest::Client,
    base: String,
}

impl Sidecar {
    fn new(base: &str) -> reqwest::Result<Self> {
        Ok(Sidecar {
            http: reqwest::Client::builder().http2_prior_knowledge().build()?,
            base: base.trim_end_matches('/').to_string(),
        })
    }

    async fn read(
        mut response: reqwest::Response,
    ) -> Result<String, VitrineCastReason> {
        let mut body = Vec::new();
        while let Some(chunk) =
            response.chunk().await.map_err(|e| transport_reason(&e))?
        {
            if body.len() + chunk.len() > MAX_ANSWER_SIZE {
                return Err(VitrineCastReason::BadResponse);
            }
            body.extend_from_slice(&chunk);
        }
        String::from_utf8(body).map_err(|_| VitrineCastReason::BadResponse)
    }

    /// `GET <log>/wait/<leaf hash>?timeout=<ms>`.
    async fn wait(
        &self,
        log: &str,
        entry: &VitrineEntry,
        wait_ms: u64,
    ) -> VitrineCast {
        let url = format!(
            "{}/{log}/wait/{}?timeout={wait_ms}",
            self.base,
            hex::encode(entry.leaf_hash)
        );
        let response = match self.http.get(url).send().await {
            Ok(response) => response,
            Err(error) => {
                return unavailable(
                    Some(&entry.entry),
                    transport_reason(&error),
                )
            }
        };
        let status = response.status().as_u16();
        match Self::read(response).await {
            Ok(body) => judge(status, &body, &entry.entry),
            Err(reason) => unavailable(Some(&entry.entry), reason),
        }
    }
}

/// The receipts that casts wait for, shared by the cast route.
#[derive(Debug)]
pub struct CastReceipts {
    sidecar: Option<Sidecar>,
    settings: ReceiptSettings,
    breaker: Breaker,
}

impl CastReceipts {
    /// A client of the sidecar at `base`, or of none: then every cast in a
    /// transparent event is `unavailable`.
    pub fn new(
        base: Option<&str>,
        settings: ReceiptSettings,
    ) -> reqwest::Result<Self> {
        Ok(CastReceipts {
            sidecar: base.map(Sidecar::new).transpose()?,
            settings,
            breaker: Breaker::new(
                settings.breaker_failures,
                settings.breaker_open,
            ),
        })
    }

    /// From `HARVEST_VITRINE_INTERNAL_URL` and the settings' variables.
    pub fn from_env() -> Self {
        let url_env = Service::Harvest.internal_url_env();
        let base = std::env::var(&url_env).ok().filter(|url| !url.is_empty());
        let settings = ReceiptSettings::from_env();
        if base.is_none() {
            info!("{url_env} isn't set: casts get no Vitrine receipt");
        }
        Self::new(base.as_deref(), settings).unwrap_or_else(|e| {
            error!("Error building the Vitrine sidecar's client: {e}");
            CastReceipts {
                sidecar: None,
                settings,
                breaker: Breaker::new(
                    settings.breaker_failures,
                    settings.breaker_open,
                ),
            }
        })
    }

    /// What the cast tells the voter's portal about its ballot's entry, or
    /// `None` when the event's transparency is off. It never fails.
    pub async fn cast(
        &self,
        vitrine: Option<CastVitrine>,
    ) -> Option<VitrineCast> {
        let vitrine = vitrine?;
        let cast = self.receipt(vitrine.log.as_deref(), vitrine.entry).await;
        match cast.reason {
            Some(VitrineCastReason::Queued) | None => {}
            Some(reason) => {
                warn!(log = ?vitrine.log, "No Vitrine receipt at cast: {reason}")
            }
        }
        Some(cast)
    }

    async fn receipt(
        &self,
        log: Option<&str>,
        entry: Option<VitrineEntry>,
    ) -> VitrineCast {
        let Some(log) = log else {
            return unavailable(None, VitrineCastReason::NoLog);
        };
        let Some(entry) = entry else {
            return unavailable(None, VitrineCastReason::NotQueued);
        };
        if entry.queue == QueueState::NotQueued {
            return unavailable(
                Some(&entry.entry),
                VitrineCastReason::NotQueued,
            );
        }
        let Some(sidecar) = &self.sidecar else {
            return unavailable(
                Some(&entry.entry),
                VitrineCastReason::Unreachable,
            );
        };
        if !self.breaker.allow(Instant::now()) {
            return unavailable(
                Some(&entry.entry),
                VitrineCastReason::BreakerOpen,
            );
        }
        let cast = tokio::time::timeout(
            self.settings.budget,
            sidecar.wait(log, &entry, self.settings.wait_ms),
        )
        .await
        .unwrap_or_else(|_| {
            unavailable(Some(&entry.entry), VitrineCastReason::Timeout)
        });
        match cast.reason {
            Some(reason) if is_failure(reason) => {
                self.breaker.failed(Instant::now())
            }
            _ => self.breaker.succeeded(),
        }
        cast
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use electoral_log::vitrine::leaf_hash;
    use std::sync::Arc;
    use tokio::io::AsyncWriteExt;
    use tokio::net::TcpListener;

    const LOG: &str = "ballots-log";

    fn entry() -> VitrineEntry {
        let entry = b"vitrine/election/v1 test entry".to_vec();
        VitrineEntry {
            leaf_hash: leaf_hash(&entry),
            entry,
            queue: QueueState::Queued,
        }
    }

    fn cast(entry: VitrineEntry) -> Option<CastVitrine> {
        Some(CastVitrine {
            log: Some(LOG.into()),
            entry: Some(entry),
        })
    }

    fn receipt_for(entry: &[u8]) -> String {
        format!(
            "c2sp.org/tlog-proof@v1\nextra {}\nindex 7\n",
            STANDARD.encode(entry)
        )
    }

    fn promise_for(entry: &[u8]) -> String {
        format!(
            "vitrine/promise/v1\nleaf {}\n",
            STANDARD.encode(leaf_hash(entry))
        )
    }

    fn settings() -> ReceiptSettings {
        ReceiptSettings {
            budget: Duration::from_millis(300),
            wait_ms: 200,
            breaker_failures: 2,
            breaker_open: Duration::from_millis(300),
        }
    }

    #[derive(Clone)]
    struct Reply {
        status: u16,
        body: String,
        delay: Duration,
    }

    /// A sidecar's internal listener that gives every request the current
    /// reply, and records the requests' paths.
    struct FakeSidecar {
        url: String,
        reply: Arc<Mutex<Reply>>,
        paths: Arc<Mutex<Vec<String>>>,
    }

    impl FakeSidecar {
        async fn start(status: u16, body: &str) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let url = format!("http://{}", listener.local_addr().unwrap());
            let reply = Arc::new(Mutex::new(Reply {
                status,
                body: body.to_string(),
                delay: Duration::ZERO,
            }));
            let paths = Arc::new(Mutex::new(Vec::new()));
            let (shared_reply, shared_paths) = (reply.clone(), paths.clone());
            tokio::spawn(async move {
                while let Ok((socket, _)) = listener.accept().await {
                    let reply = shared_reply.clone();
                    let paths = shared_paths.clone();
                    tokio::spawn(async move {
                        let Ok(mut connection) =
                            h2::server::handshake(socket).await
                        else {
                            return;
                        };
                        while let Some(Ok((request, mut respond))) =
                            connection.accept().await
                        {
                            paths.lock().unwrap().push(
                                request
                                    .uri()
                                    .path_and_query()
                                    .unwrap()
                                    .to_string(),
                            );
                            let reply = reply.lock().unwrap().clone();
                            tokio::spawn(async move {
                                tokio::time::sleep(reply.delay).await;
                                let response = http::Response::builder()
                                    .status(reply.status)
                                    .body(())
                                    .unwrap();
                                if let Ok(mut stream) =
                                    respond.send_response(response, false)
                                {
                                    let _ = stream.send_data(
                                        Bytes::from(reply.body),
                                        true,
                                    );
                                }
                            });
                        }
                    });
                }
            });
            FakeSidecar { url, reply, paths }
        }

        fn answer(&self, status: u16, body: &str) {
            let mut reply = self.reply.lock().unwrap();
            reply.status = status;
            reply.body = body.to_string();
        }

        fn delay(&self, delay: Duration) {
            self.reply.lock().unwrap().delay = delay;
        }

        fn requests(&self) -> usize {
            self.paths.lock().unwrap().len()
        }

        fn receipts(&self) -> CastReceipts {
            CastReceipts::new(Some(&self.url), settings()).unwrap()
        }
    }

    /// The URL of a sidecar that isn't running.
    async fn stopped_sidecar() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        drop(listener);
        url
    }

    fn unavailable_for(
        entry: &VitrineEntry,
        reason: VitrineCastReason,
    ) -> Option<VitrineCast> {
        Some(VitrineCast {
            kind: VitrineCastKind::Unavailable,
            text: Some(STANDARD.encode(&entry.entry)),
            reason: Some(reason),
        })
    }

    #[test]
    fn the_budget_outlasts_the_sidecars_wait() {
        let defaults = ReceiptSettings::default();
        assert_eq!(defaults.budget, Duration::from_millis(800));
        assert_eq!(defaults.wait_ms, 600);
        assert!(Duration::from_millis(defaults.wait_ms) < defaults.budget);
    }

    #[tokio::test]
    async fn a_receipt_for_the_entry_is_the_casts() {
        let entry = entry();
        let sidecar = FakeSidecar::start(200, &receipt_for(&entry.entry)).await;
        let got = sidecar.receipts().cast(cast(entry.clone())).await;
        assert_eq!(
            got,
            Some(VitrineCast {
                kind: VitrineCastKind::Receipt,
                text: Some(receipt_for(&entry.entry)),
                reason: None,
            })
        );
        assert_eq!(
            sidecar.paths.lock().unwrap().as_slice(),
            [format!(
                "/{LOG}/wait/{}?timeout=200",
                hex::encode(entry.leaf_hash)
            )]
        );
    }

    #[tokio::test]
    async fn a_promise_or_the_queue_answer_without_a_receipt() {
        let entry = entry();
        let sidecar = FakeSidecar::start(202, &promise_for(&entry.entry)).await;
        let receipts = sidecar.receipts();
        let got = receipts.cast(cast(entry.clone())).await.unwrap();
        assert_eq!(got.kind, VitrineCastKind::Promise);
        assert_eq!(got.text, Some(promise_for(&entry.entry)));

        sidecar.answer(202, "queued\n");
        assert_eq!(
            receipts.cast(cast(entry.clone())).await,
            Some(VitrineCast {
                kind: VitrineCastKind::Queued,
                text: Some(STANDARD.encode(&entry.entry)),
                reason: Some(VitrineCastReason::Queued),
            })
        );
    }

    #[test]
    fn the_sidecars_refusals_have_their_reasons() {
        let entry = entry();
        let other = b"another entry".to_vec();
        for (status, body, reason) in [
            (404, "{}", VitrineCastReason::UnknownEntry),
            (422, "{}", VitrineCastReason::Rejected),
            (403, "{}", VitrineCastReason::Frozen),
            (429, "{}", VitrineCastReason::Busy),
            (503, "{}", VitrineCastReason::LogUnavailable),
            (200, "garbage", VitrineCastReason::BadResponse),
            (200, &receipt_for(&other), VitrineCastReason::BadResponse),
            (202, &promise_for(&other), VitrineCastReason::BadResponse),
            (
                202,
                &receipt_for(&entry.entry),
                VitrineCastReason::BadResponse,
            ),
            (
                200,
                &promise_for(&entry.entry),
                VitrineCastReason::BadResponse,
            ),
            (301, "", VitrineCastReason::BadResponse),
        ] {
            assert_eq!(
                Some(judge(status, body, &entry.entry)),
                unavailable_for(&entry, reason),
                "{status} {body:?}"
            );
        }
    }

    #[tokio::test]
    async fn casting_goes_on_while_the_sidecar_is_stopped() {
        let entry = entry();
        let receipts =
            CastReceipts::new(Some(&stopped_sidecar().await), settings())
                .unwrap();
        assert_eq!(
            receipts.cast(cast(entry.clone())).await,
            unavailable_for(&entry, VitrineCastReason::Unreachable)
        );
    }

    #[tokio::test]
    async fn casting_goes_on_when_the_sidecar_is_slower_than_the_budget() {
        let entry = entry();
        let sidecar = FakeSidecar::start(200, &receipt_for(&entry.entry)).await;
        sidecar.delay(Duration::from_secs(5));
        let started = Instant::now();
        assert_eq!(
            sidecar.receipts().cast(cast(entry.clone())).await,
            unavailable_for(&entry, VitrineCastReason::Timeout)
        );
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[tokio::test]
    async fn casting_goes_on_when_the_sidecar_answers_garbage() {
        let entry = entry();
        let sidecar = FakeSidecar::start(200, "garbage").await;
        assert_eq!(
            sidecar.receipts().cast(cast(entry.clone())).await,
            unavailable_for(&entry, VitrineCastReason::BadResponse)
        );

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let _ = socket.write_all(b"HTTP/1.1 200 OK\r\n\r\nno").await;
            }
        });
        let got = CastReceipts::new(Some(&url), settings())
            .unwrap()
            .cast(cast(entry.clone()))
            .await
            .unwrap();
        assert_eq!(got.kind, VitrineCastKind::Unavailable);
        assert_eq!(got.text, Some(STANDARD.encode(&entry.entry)));
    }

    #[tokio::test]
    async fn casts_without_a_queued_entry_or_a_sidecar_are_unavailable() {
        let entry = entry();
        let receipts = CastReceipts::new(None, settings()).unwrap();
        assert_eq!(receipts.cast(None).await, None);
        assert_eq!(
            receipts.cast(cast(entry.clone())).await,
            unavailable_for(&entry, VitrineCastReason::Unreachable)
        );
        assert_eq!(
            receipts
                .cast(Some(CastVitrine {
                    log: None,
                    entry: None
                }))
                .await,
            Some(VitrineCast {
                kind: VitrineCastKind::Unavailable,
                text: None,
                reason: Some(VitrineCastReason::NoLog),
            })
        );
        let not_queued = VitrineEntry {
            queue: QueueState::NotQueued,
            ..entry.clone()
        };
        assert_eq!(
            receipts.cast(cast(not_queued)).await,
            unavailable_for(&entry, VitrineCastReason::NotQueued)
        );
    }

    #[tokio::test]
    async fn the_breaker_opens_after_failures_and_closes_again() {
        let entry = entry();
        let sidecar = FakeSidecar::start(503, "{}").await;
        let receipts = sidecar.receipts();
        for _ in 0..2 {
            assert_eq!(
                receipts.cast(cast(entry.clone())).await,
                unavailable_for(&entry, VitrineCastReason::LogUnavailable)
            );
        }
        assert_eq!(
            receipts.cast(cast(entry.clone())).await,
            unavailable_for(&entry, VitrineCastReason::BreakerOpen)
        );
        assert_eq!(sidecar.requests(), 2);

        sidecar.answer(200, &receipt_for(&entry.entry));
        tokio::time::sleep(settings().breaker_open).await;
        for _ in 0..2 {
            assert_eq!(
                receipts.cast(cast(entry.clone())).await.unwrap().kind,
                VitrineCastKind::Receipt
            );
        }
        assert_eq!(sidecar.requests(), 4);
    }

    #[test]
    fn the_breaker_lets_one_call_try_once_the_period_ends() {
        let breaker = Breaker::new(2, Duration::from_secs(30));
        let start = Instant::now();
        breaker.failed(start);
        assert!(breaker.allow(start));
        breaker.failed(start);
        assert!(!breaker.allow(start + Duration::from_secs(29)));
        let later = start + Duration::from_secs(30);
        assert!(breaker.allow(later));
        assert!(!breaker.allow(later), "one call tries at a time");
        breaker.failed(later);
        assert!(!breaker.allow(later + Duration::from_secs(29)));
        let end = later + Duration::from_secs(30);
        assert!(breaker.allow(end));
        breaker.succeeded();
        assert!(breaker.allow(end));
        breaker.failed(end);
        assert!(breaker.allow(end), "one failure doesn't open it");
    }

    #[test]
    fn only_a_sick_sidecar_counts_against_the_breaker() {
        for reason in [
            VitrineCastReason::Busy,
            VitrineCastReason::LogUnavailable,
            VitrineCastReason::Timeout,
            VitrineCastReason::Unreachable,
            VitrineCastReason::BadResponse,
        ] {
            assert!(is_failure(reason), "{reason}");
        }
        for reason in [
            VitrineCastReason::Queued,
            VitrineCastReason::UnknownEntry,
            VitrineCastReason::Rejected,
            VitrineCastReason::Frozen,
            VitrineCastReason::BreakerOpen,
            VitrineCastReason::NotQueued,
            VitrineCastReason::NoLog,
        ] {
            assert!(!is_failure(reason), "{reason}");
        }
    }
}
