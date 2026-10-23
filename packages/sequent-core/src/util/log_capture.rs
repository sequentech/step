// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use std::fmt::{self, Write as _};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::subscriber::DefaultGuard;
use tracing::{Event, Metadata, Subscriber};

/// Records the fields of every span and event emitted on the current thread,
/// at every level, so tests can check what would reach the logs.
#[derive(Clone, Default)]
pub(crate) struct LogCapture {
    output: Arc<Mutex<String>>,
    next_span_id: Arc<AtomicU64>,
}

impl LogCapture {
    pub(crate) fn install() -> (Self, DefaultGuard) {
        let capture = Self::default();
        let guard = tracing::subscriber::set_default(capture.clone());
        (capture, guard)
    }

    pub(crate) fn contents(&self) -> String {
        self.output
            .lock()
            .map(|output| output.clone())
            .unwrap_or_default()
    }

    fn append(&self, line: String) {
        if let Ok(mut output) = self.output.lock() {
            output.push_str(&line);
            output.push('\n');
        }
    }
}

struct FieldWriter(String);

impl Visit for FieldWriter {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        let _ = write!(self.0, " {}={:?}", field.name(), value);
    }
}

impl Subscriber for LogCapture {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, attributes: &Attributes<'_>) -> Id {
        let mut writer = FieldWriter(attributes.metadata().name().to_string());
        attributes.record(&mut writer);
        self.append(writer.0);
        Id::from_u64(self.next_span_id.fetch_add(1, Ordering::Relaxed) + 1)
    }

    fn record(&self, _span: &Id, values: &Record<'_>) {
        let mut writer = FieldWriter(String::new());
        values.record(&mut writer);
        self.append(writer.0);
    }

    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

    fn event(&self, event: &Event<'_>) {
        let mut writer = FieldWriter(event.metadata().level().to_string());
        event.record(&mut writer);
        self.append(writer.0);
    }

    fn enter(&self, _span: &Id) {}

    fn exit(&self, _span: &Id) {}
}
