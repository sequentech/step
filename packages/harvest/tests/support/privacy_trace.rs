// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use std::future::Future;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use tracing::instrument::WithSubscriber;

#[derive(Clone)]
struct Sink(Arc<Mutex<Vec<u8>>>);
impl Write for Sink {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// TRACE enables every level; FULL captures span creation, updates, enter/exit
/// and close, so skipping event messages alone cannot satisfy this check.
pub async fn capture(requests: impl Future<Output = ()>) -> String {
    let bytes = Arc::new(Mutex::new(Vec::new()));
    let sink = Sink(bytes.clone());
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_ansi(false)
        .without_time()
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::FULL)
        .with_writer(move || sink.clone())
        .finish();
    async {
        tracing::error!("privacy-trace-positive-control");
        requests.await;
    }
    .with_subscriber(subscriber)
    .await;
    let result = String::from_utf8(bytes.lock().unwrap().clone()).unwrap();
    assert!(result.contains("privacy-trace-positive-control"));
    result
}

pub fn assert_private(logs: &str, ballot: &str) {
    for forbidden in [
        "privacy-capture-voter-0123",
        "privacy-capture-name-0123",
        "198.51.100.73",
        "AQ",
        "privacy-capture-agent-0123",
        ballot,
    ] {
        assert!(
            !logs.contains(forbidden),
            "private ballot attribution appeared in tracing output"
        );
    }
}
