// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#![recursion_limit = "256"]
#![feature(result_flattening)]
#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate quick_error;

pub mod postgres;
pub mod services;
pub mod sqlite;
pub mod tasks;
pub mod types;
