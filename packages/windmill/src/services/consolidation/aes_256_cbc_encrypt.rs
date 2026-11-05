// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{Context, Result};
use sequent_core::signatures::shell::{build_command, run_command};
use std::process::Command;
use tracing::instrument;

const AES_PASSWORD_ENV: &str = "AES_PASSWORD";

#[derive(Debug, Clone, Copy)]
enum AesDirection {
    Encrypt,
    Decrypt,
}

impl AesDirection {
    fn flag(self) -> &'static str {
        match self {
            AesDirection::Encrypt => "-e",
            AesDirection::Decrypt => "-d",
        }
    }
}

// Equivalent to:
// openssl enc -aes-256-cbc -e -in $input_file_path -out $output_file_path -pass env:AES_PASSWORD -md md5
fn aes_256_cbc_command(
    direction: AesDirection,
    input_file_path: &str,
    output_file_path: &str,
    password: &str,
) -> Command {
    let passin = format!("env:{AES_PASSWORD_ENV}");
    let mut command = build_command(
        "openssl",
        &[
            "enc",
            "-aes-256-cbc",
            direction.flag(),
            "-in",
            input_file_path,
            "-out",
            output_file_path,
            "-pass",
            &passin,
            "-md",
            "md5",
        ],
    );
    command.env(AES_PASSWORD_ENV, password);
    command
}

#[instrument(skip(password), err)]
pub fn encrypt_file_aes_256_cbc(
    input_file_path: &str,
    output_file_path: &str,
    password: &str,
) -> Result<()> {
    run_command(aes_256_cbc_command(
        AesDirection::Encrypt,
        input_file_path,
        output_file_path,
        password,
    ))
    .with_context(|| format!("Error encrypting file {input_file_path} to {output_file_path}"))?;
    Ok(())
}

#[instrument(skip(password), err)]
pub fn decrypt_file_aes_256_cbc(
    input_file_path: &str,
    output_file_path: &str,
    password: &str,
) -> Result<()> {
    run_command(aes_256_cbc_command(
        AesDirection::Decrypt,
        input_file_path,
        output_file_path,
        password,
    ))
    .with_context(|| format!("Error decrypting file {input_file_path} to {output_file_path}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    const HOSTILE_PASSWORD: &str = "pass word; rm -rf / $(touch pwned) `id` | cat";

    fn assert_password_only_in_environment(command: &Command) {
        assert_eq!(command.get_program(), "openssl");
        assert!(!command
            .get_args()
            .any(|arg| arg.to_string_lossy().contains(HOSTILE_PASSWORD)));
        assert!(command
            .get_envs()
            .any(|(key, value)| key == AES_PASSWORD_ENV
                && value == Some(OsStr::new(HOSTILE_PASSWORD))));
    }

    #[test]
    fn test_encrypt_command_passes_password_through_environment() {
        let command = aes_256_cbc_command(
            AesDirection::Encrypt,
            "/tmp/in",
            "/tmp/out",
            HOSTILE_PASSWORD,
        );
        let args: Vec<&OsStr> = command.get_args().collect();
        assert_eq!(
            args,
            vec![
                "enc",
                "-aes-256-cbc",
                "-e",
                "-in",
                "/tmp/in",
                "-out",
                "/tmp/out",
                "-pass",
                "env:AES_PASSWORD",
                "-md",
                "md5"
            ]
        );
        assert_password_only_in_environment(&command);
    }

    #[test]
    fn test_decrypt_command_passes_password_through_environment() {
        let command = aes_256_cbc_command(
            AesDirection::Decrypt,
            "/tmp/in",
            "/tmp/out",
            HOSTILE_PASSWORD,
        );
        let args: Vec<&OsStr> = command.get_args().collect();
        assert_eq!(args[2], "-d");
        assert_eq!(args[8], "env:AES_PASSWORD");
        assert_password_only_in_environment(&command);
    }

    #[test]
    fn test_encrypt_then_decrypt_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let plain = dir.path().join("plain");
        let encrypted = dir.path().join("encrypted");
        let decrypted = dir.path().join("decrypted");
        std::fs::write(&plain, b"ballot data").unwrap();
        let to_str = |path: &std::path::Path| path.to_str().unwrap().to_string();

        encrypt_file_aes_256_cbc(&to_str(&plain), &to_str(&encrypted), HOSTILE_PASSWORD).unwrap();
        decrypt_file_aes_256_cbc(&to_str(&encrypted), &to_str(&decrypted), HOSTILE_PASSWORD)
            .unwrap();
        assert_eq!(std::fs::read(&decrypted).unwrap(), b"ballot data");
    }
}
