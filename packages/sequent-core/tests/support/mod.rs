// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Fallible fixtures report their cause through the Rust test harness.

/// A failed setup step or unexpected result must fail the test, never be skipped.
pub type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
