// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::domain::readiness::ReadinessReport;
use anyhow::Result;
use std::future::Future;

/// The signed entry must be durably appended before this returns success.
pub trait ReadinessRecorder: Sync {
    fn record(&self, report: &ReadinessReport) -> impl Future<Output = Result<()>> + Send;
}
