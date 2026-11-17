// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The central key service: the only component that talks to the HSM (a
//! YubiHSM 2 or, explicitly, SoftHSM) or KMS. Everything else is a caller,
//! authenticated by mTLS and authorized per key, operation and purpose.

pub mod api;
pub mod audit;
pub mod config;
pub mod formats;
pub mod officials;
pub mod pkcs11;
pub mod policy;
pub mod provider;
pub mod registry;
pub mod server;
pub mod service;
pub mod tls;
