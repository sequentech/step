// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The election event import bundle: what it contains, and whether it is valid.
//!
//! This is the single definition of the format shared by everything that
//! produces or consumes an import — windmill's importer on the server, and the
//! configuration tools in `beyond/packages` in the browser.
//!
//! It lives here rather than in `beyond` because the dependency between the
//! repositories runs one way: `beyond` is a git submodule of step and
//! path-depends on this crate, so step cannot depend on `beyond`. Anything
//! windmill must use has to be here. `sequent-core` also already compiles to
//! WASM and is already vendored into the front ends as a package, so both
//! consumers reach this module through paths that already carry production
//! code.
//!
//! Everything in this module must stay **pure** — no database, no IO, no
//! clock — or it cannot run in a browser, and the browser is where a delivery
//! engineer wants the answer.
//!
//! See `beyond/docs/docusaurus/docs/engineering/election-config-architecture.md`
//! and <https://github.com/sequentech/meta/issues/12769>.

pub mod emit;
pub mod paths;
pub mod problem;
pub mod report;
pub mod schema;
pub mod validate;

#[cfg(test)]
mod validate_tests;

pub use emit::{json_csv, plain_csv, JsonField};
pub use paths::{coerce_cell, deep_merge, expand, Cell};
pub use problem::{Code, Problem, Report as ValidationReport, Severity};
pub use report::{EReportEncryption, Report, ReportCronConfig, ReportType};
pub use schema::ImportElectionEventSchema;
pub use validate::validate;
