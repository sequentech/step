// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Secret backends (AWS Secrets Manager, `HashiCorp` Vault, environment)
//! for tasks that need credentials or key material.

mod aws_secret_manager;
mod env_var_master_secret;
mod hashicorp_vault;
#[allow(clippy::module_inception)]
/// allow module to have the same name as its containing module
pub mod vault;

pub use vault::*;
