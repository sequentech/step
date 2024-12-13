// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::consolidation::ecies_encrypt::ECIES_TOOL_PATH;
use crate::services::shell::run_shell_command;
use anyhow::{Context, Result};
use tracing::{info, instrument};

#[instrument(skip_all, err)]
pub fn ecdsa_sign_data(
    pk12_file_path_string: &str,
    password: &str,
    data_path: &str,
) -> Result<String> {
    let command = format!(
        "java -jar {} sign-ec {} {} {}",
        ECIES_TOOL_PATH, pk12_file_path_string, data_path, password
    );

    let encrypted_base64 = run_shell_command(&command)?.replace("\n", "");

    info!("ecdsa_sign_data: '{}'", encrypted_base64);

    Ok(encrypted_base64)
}
