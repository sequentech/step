// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! What the application would log: every event and the fields of every new
//! span, at every level.

use std::future::Future;
use std::io::Write;
use std::sync::{Arc, Mutex};
use tracing::instrument::WithSubscriber;
use tracing_subscriber::fmt::format::FmtSpan;

#[derive(Clone, Default)]
struct LogBuffer(Arc<Mutex<Vec<u8>>>);

impl Write for LogBuffer {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().expect("log buffer").extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Runs `run` with a subscriber that writes what it logs, and returns its
/// output and the logs.
pub async fn with_captured_logs<T>(
    run: impl Future<Output = T>,
) -> (T, String) {
    let logs = LogBuffer::default();
    let writer = logs.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_max_level(tracing::Level::TRACE)
        .with_span_events(FmtSpan::NEW)
        .with_writer(move || writer.clone())
        .finish();
    let output = run.with_subscriber(subscriber).await;
    let contents = logs.0.lock().expect("log buffer").clone();
    (output, String::from_utf8_lossy(&contents).into_owned())
}
