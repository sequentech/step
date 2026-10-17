// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Projects planning domain and application services, independent of elections.
pub mod application;
pub mod csv_io;
pub mod domain;
pub use application::Projects;
