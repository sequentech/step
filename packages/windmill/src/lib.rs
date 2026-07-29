// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#![recursion_limit = "256"]
#[macro_use]
extern crate quick_error; 

pub mod postgres;
pub mod services;
pub mod tasks;
pub mod types;

#[cfg(test)]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;
