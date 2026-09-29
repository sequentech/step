// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Monitoring dashboards: what an administrator may configure, and what the
//! platform refuses.
//!
//! A dashboard is widgets on a grid. Every widget reads one governed data
//! source through that source's query templates, and draws with dbt Charts
//! YAML. The split is deliberate: *what is counted* belongs to the data source
//! and is code ([`sources`]), while *what is shown and how* is configuration
//! ([`config`]) that an administrator edits in the Admin Portal.
//!
//! Configuration is checked here ([`policy`]) rather than in each consumer, so
//! the Admin Portal's editor, Harvest's save route and the preset tests all
//! reach the same verdict. That is also why everything in this module must stay
//! **pure** — no database, no IO, no clock: it runs in the browser too.
//!
//! See <https://github.com/sequentech/meta/issues/13624>.

/// Evaluating a query against the counts a snapshot holds.
pub mod compute;

/// One document checked alone or against the event's other documents, as
/// the editor asks on every keystroke.
pub mod check;

/// Configuration as authored: widgets, dashboards, themes and settings.
pub mod config;

/// What the snapshot job stores for one source at one scope.
pub mod payload;

/// What the platform refuses in configuration, and every reason why.
pub mod policy;

/// The settings, themes, widgets and dashboards shipped for a kind of
/// deployment, as YAML documents compiled in.
pub mod presets;

/// What validation reports.
pub mod problem;

/// The dbt Charts board sent to the renderer for one widget.
pub mod render_request;

/// What an event's stored documents amount to, and what a save or a reset
/// may write.
pub mod revision;

/// Made-up payloads in the shape each producer writes, for previews and
/// tests before any snapshot exists.
pub mod sample;

/// Which slice of the event a widget shows, and the key it is stored under.
pub mod scope;

/// Turning selector values into the concrete queries a widget runs.
pub mod resolve;

/// What the snapshot job keeps of each voter: facts derived as the settings
/// say.
pub mod voter;

/// The governed data sources: their counting units, measures, dimensions and
/// query templates. Adding one is a code change, by design.
pub mod sources;

/// The browser's view of [`check`].
#[cfg(feature = "wasm")]
pub mod wasm;

pub use config::{
    ConfigKind, ConfigSet, Dashboard, LayoutItem, Query, Selector, Settings,
    Theme, Widget,
};
pub use policy::{
    parse_dashboard, parse_settings, parse_theme, parse_widget, validate_set,
};
pub use problem::{Code, Problem, Report, Severity};
pub use resolve::{
    resolve_widget, DynamicOptionValues, ResolvedQuery, ResolvedWidget,
    SelectorState,
};
pub use sources::{
    CountingUnit, DataSourceId, Measure, QueryTemplate, SourceSpec,
};
