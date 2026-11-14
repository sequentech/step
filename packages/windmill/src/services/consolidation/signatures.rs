// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use openssl::pkcs12::Pkcs12;
use openssl::pkey::PKey;
use sequent_core::signatures::ecies_encrypt::ecies_tool_command_with_secret;
use sequent_core::signatures::shell::{build_command, run_command};
use sequent_core::util::temp_path::*;
use std::ffi::OsStr;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::process::Command;
use tempfile::{tempdir, NamedTempFile, TempPath};
use tracing::{info, instrument};

const P12_PASSWORD_ENV: &str = "P12_PASSWORD";

#[instrument(skip_all, err)]
pub fn get_pk12_id(p12_path: &str, password: &str) -> Result<openssl::pkey::Id> {
    // Read the .p12 file
    let mut file = fs::File::open(p12_path)?;
    let mut p12_data = Vec::new();
    file.read_to_end(&mut p12_data)?;

    // Parse the .p12 file
    let pkcs12 = Pkcs12::from_der(&p12_data)?;
    let parsed = pkcs12.parse2(password)?;
    let pkey = parsed.pkey.ok_or(anyhow!("Can't find pkey"))?;

    // Return the key type
    Ok(pkey.id())
}

fn ecdsa_sign_command(pk12_file_path_string: &str, password: &str, data_path: &str) -> Command {
    ecies_tool_command_with_secret(&["sign-ec", pk12_file_path_string, data_path], password)
}

#[instrument(skip_all, err)]
pub fn ecdsa_sign_data(
    pk12_file_path_string: &str,
    password: &str,
    data_path: &str,
) -> Result<String> {
    let encrypted_base64 = run_command(ecdsa_sign_command(
        pk12_file_path_string,
        password,
        data_path,
    ))?
    .replace("\n", "");

    info!("ecdsa_sign_data: '{}'", encrypted_base64);

    Ok(encrypted_base64)
}

fn p12_cert_command(p12_file_path: &Path, password: &str, cert_path: &Path) -> Command {
    let passin = format!("env:{P12_PASSWORD_ENV}");
    let mut command = build_command(
        "openssl",
        &[
            OsStr::new("pkcs12"),
            OsStr::new("-in"),
            p12_file_path.as_os_str(),
            OsStr::new("-passin"),
            OsStr::new(&passin),
            OsStr::new("-nokeys"),
            OsStr::new("-out"),
            cert_path.as_os_str(),
        ],
    );
    command.env(P12_PASSWORD_ENV, password);
    command
}

#[instrument(skip_all, err)]
pub fn get_p12_cert(p12_file: &NamedTempFile, password: &str) -> Result<TempPath> {
    let cert_temp_file =
        generate_temp_file("p12", "cert").with_context(|| "Error creating temp file")?;
    let cert_temp_path = cert_temp_file.into_temp_path();

    run_command(p12_cert_command(p12_file.path(), password, &cert_temp_path))?;

    Ok(cert_temp_path)
}

#[instrument(err, ret)]
pub fn get_p12_fingerprint(p12_cert_path: &TempPath) -> Result<String> {
    let fingerprint = run_command(build_command(
        "openssl",
        &[
            OsStr::new("x509"),
            OsStr::new("-in"),
            p12_cert_path.as_os_str(),
            OsStr::new("-noout"),
            OsStr::new("-fingerprint"),
            OsStr::new("-sha256"),
        ],
    ))?
    .replace("\n", "");

    Ok(fingerprint)
}

#[instrument(skip_all, err)]
pub fn check_certificate_cas(
    p12_cert_path: &TempPath,
    root_ca: &str,
    intermediate_cas: &str,
) -> Result<()> {
    // Create a temporary directory
    let temp_dir = tempdir()?;

    // Get the path to the temporary directory
    let temp_dir_path = temp_dir.path().to_path_buf();

    // write root ca
    let root_ca_file_path = temp_dir_path.join("root-ca.cer");
    fs::write(root_ca_file_path.clone(), root_ca)?;

    // write root ca
    let intermediate_ca_file_path = temp_dir_path.join("intermediate-ca.cer");
    fs::write(intermediate_ca_file_path.clone(), intermediate_cas)?;

    let verify_result = run_command(build_command(
        "openssl",
        &[
            OsStr::new("verify"),
            OsStr::new("-CAfile"),
            root_ca_file_path.as_os_str(),
            OsStr::new("-untrusted"),
            intermediate_ca_file_path.as_os_str(),
            p12_cert_path.as_os_str(),
        ],
    ))?
    .replace("\n", "");

    if !verify_result.ends_with(": OK") {
        return Err(anyhow!(verify_result));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequent_core::signatures::ecies_encrypt::{ECIES_SECRET_ENV, ECIES_TOOL_PATH};

    const HOSTILE_PASSWORD: &str = "pass word; rm -rf / $(touch pwned) `id` | cat";

    #[test]
    fn test_ecdsa_sign_command_passes_password_through_environment() {
        let command = ecdsa_sign_command("/tmp/key.p12", HOSTILE_PASSWORD, "/tmp/data");
        assert_eq!(command.get_program(), "java");
        let args: Vec<&OsStr> = command.get_args().collect();
        assert_eq!(
            args,
            vec![
                "-jar",
                ECIES_TOOL_PATH,
                "sign-ec",
                "/tmp/key.p12",
                "/tmp/data",
                "env:ECIES_SECRET"
            ]
        );
        assert!(!command
            .get_args()
            .any(|arg| arg.to_string_lossy().contains(HOSTILE_PASSWORD)));
        assert!(command
            .get_envs()
            .any(|(key, value)| key == ECIES_SECRET_ENV
                && value == Some(OsStr::new(HOSTILE_PASSWORD))));
    }

    #[test]
    fn test_p12_cert_command_passes_password_through_environment() {
        let command = p12_cert_command(
            Path::new("/tmp/key.p12"),
            HOSTILE_PASSWORD,
            Path::new("/tmp/cert"),
        );
        assert_eq!(command.get_program(), "openssl");
        let args: Vec<&OsStr> = command.get_args().collect();
        assert_eq!(
            args,
            vec![
                "pkcs12",
                "-in",
                "/tmp/key.p12",
                "-passin",
                "env:P12_PASSWORD",
                "-nokeys",
                "-out",
                "/tmp/cert"
            ]
        );
        assert!(command.get_envs().any(
            |(key, value)| key == "P12_PASSWORD" && value == Some(OsStr::new(HOSTILE_PASSWORD))
        ));
    }
}
