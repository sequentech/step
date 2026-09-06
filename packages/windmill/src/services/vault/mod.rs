// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

mod aws_secret_manager;
mod env_var_master_secret;
mod hashicorp_vault;
#[expect(
    clippy::module_inception,
    reason = "Preserve the existing services::vault::vault module path used by vault callers."
)]
pub mod vault;

pub use vault::*;
