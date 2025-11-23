// SPDX-FileCopyrightText: 2025 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{
    bindings::plugins_manager::extra_services_manager::cli_service::run_shell_command_derive_public_key_from_p12,
    services::signatures::ECIES_TOOL_PATH,
};

pub fn derive_public_key_from_p12(
    pk12_file_path_string: &str,
    password: &str,
) -> Result<String, String> {
    let public_pem = run_shell_command_derive_public_key_from_p12(
        ECIES_TOOL_PATH,
        pk12_file_path_string,
        password,
    )?;
    println!("public pem: '{}'", public_pem);

    Ok(public_pem)
}
