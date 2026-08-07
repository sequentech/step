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

/// Turning a source document's rows into a bundle. Needs the templates, so it
/// shares their feature.
#[cfg(feature = "election_config_templates")]
pub mod build;

/// What a bundle becomes as files. Needs the builder, so it shares its feature;
/// the zip writer itself is behind `election_config_archive`.
#[cfg(feature = "election_config_templates")]
pub mod archive;

/// The Election Architect's plan, and how it becomes rows the builder reads.
/// Needs the builder's feature, since compiling a plan means building a bundle.
#[cfg(feature = "election_config_templates")]
pub mod architect;

pub mod branding;
pub mod emit;
pub mod ids;
pub mod paths;
pub mod presets;

/// Bundles with known verdicts, shared by every caller of [`validate`] so the two
/// reach the same answer rather than each agreeing with itself.
pub mod fixtures;
pub mod problem;

/// Rendering the base entity templates, behind its own feature so a front end
/// that only validates an existing bundle carries no template engine.
#[cfg(feature = "election_config_templates")]
pub mod render;

pub mod report;
pub mod schema;
pub mod sheet;
pub mod validate;

/// Reading `.xlsx`, behind its own feature so front ends with no workbook to read
/// do not carry a spreadsheet library.
#[cfg(feature = "election_config_xlsx")]
pub mod xlsx;

#[cfg(test)]
mod validate_tests;

#[cfg(feature = "election_config_templates")]
#[cfg(feature = "election_config_templates")]
pub use architect::{validate_plan, Blueprint};
#[cfg(feature = "election_config_templates")]
pub use archive::{layout, Artifact, Layout};
pub use build::{
    build, BuildOptions, Bundle, CommunicationTemplate, JsonTable, PlainTable,
};
pub use emit::{json_csv, plain_csv, JsonField};
pub use ids::IdFactory;
pub use paths::{coerce_cell, deep_merge, expand, Cell};
#[cfg(feature = "election_config_templates")]
pub use presets::{AuthPreset, RealmPatch};
pub use problem::{Code, Problem, Report as ValidationReport, Severity};
#[cfg(feature = "election_config_templates")]
pub use render::TemplateSet;
pub use report::{EReportEncryption, Report, ReportCronConfig, ReportType};
pub use schema::ImportElectionEventSchema;
pub use sheet::{Origin, Row, Sheet, Workbook};
pub use validate::validate;
