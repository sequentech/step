// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#![recursion_limit = "256"]
#[macro_use]
extern crate quick_error;

// Shared database fixtures also compile inside this crate's unit tests.
#[cfg(test)]
extern crate self as windmill;

pub mod adapters;
pub mod domain;
pub mod ports;
pub mod postgres;
pub mod services;
pub mod tasks;
pub mod types;

#[cfg(test)]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;
