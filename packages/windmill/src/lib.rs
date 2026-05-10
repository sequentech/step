// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//! Windmill executes background work for the Sequent platform: Celery tasks,
//! Hasura-backed Postgres access, Keycloak and vault integration, reports,
//! imports/exports, and WASM-backed plugins.
#![allow(clippy::too_many_arguments)]
#![recursion_limit = "256"]
#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate quick_error;

pub mod postgres;
pub mod services;
pub mod tasks;
pub mod types;

#[cfg(test)]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;
