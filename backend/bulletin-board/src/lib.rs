// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#![feature(io_error_more)]
#![feature(option_result_contains)]
#![feature(stmt_expr_attributes)]

#[macro_use]
extern crate lazy_static;

pub use board_config::*;
pub use error::*;
pub use proto::*;
pub use util::BoardUuid;

pub mod board_config;
pub mod client;
pub mod entry;
pub mod error;
pub mod permissions;
pub mod proto;
pub mod signature;
pub mod util;

#[cfg(feature = "build-server")]
pub mod backend_trillian;

#[cfg(feature = "build-server")]
pub mod service;
