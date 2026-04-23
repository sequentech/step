// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, NaiveDateTime, Utc};
use sequent_core::util::temp_path::generate_temp_file;
use std::fs;
use std::process::Command;

// The standard PEM (Privacy-Enhanced Mail) file format conventionally requires five dashes
const CERT_BEGIN: &str = "-----BEGIN CERTIFICATE-----";
const CERT_END: &str = "-----END CERTIFICATE-----";

pub struct ParsedCertificate {
    pub common_name: String,
    pub subject: String,
    pub issuer_common_name: String,
    pub issuer: String,
    pub not_before: DateTime<Utc>,
    pub not_after: DateTime<Utc>,
    pub fingerprint_sha256: String,
    pub serial_number: String,
    pub pem: String,
}

/// Splits a PEM bundle (potentially containing multiple certificates) into
/// individual PEM strings, one per certificate.
pub fn split_pem_bundle(pem_content: &str) -> Vec<String> {
    let mut certs = Vec::new();
    let mut current = String::new();
    let mut in_cert = false;

    for line in pem_content.lines() {
        let trimmed = line.trim();
        if trimmed == CERT_BEGIN {
            in_cert = true;
            current.clear();
            current.push_str(trimmed);
            current.push('\n');
        } else if trimmed == CERT_END {
            current.push_str(trimmed);
            current.push('\n');
            if in_cert {
                certs.push(current.clone());
                current.clear();
            }
            in_cert = false;
        } else if in_cert {
            current.push_str(trimmed);
            current.push('\n');
        }
    }

    certs
}

/// Parses a single PEM-encoded X.509 certificate and extracts its metadata
/// using OpenSSL command-line tools.
pub fn parse_certificate_pem(pem: &str) -> Result<ParsedCertificate> {
    let cert_temp_file =
        generate_temp_file("cert", ".pem").with_context(|| "Error creating temp PEM file")?;
    let cert_path = cert_temp_file.path();
    fs::write(cert_path, pem)
        .with_context(|| format!("Error writing PEM to temp file {}", cert_path.display()))?;

    let raw_output = Command::new("openssl")
        .args([
            "x509",
            "-in",
            &cert_path.to_string_lossy(),
            "-noout",
            "-subject",
            "-issuer",
            "-startdate",
            "-enddate",
            "-fingerprint",
            "-sha256",
            "-serial",
        ])
        .output()
        .with_context(|| "Error running openssl x509 to parse certificate")?;
    if !raw_output.status.success() {
        return Err(anyhow!(
            "openssl x509 failed: {}",
            String::from_utf8_lossy(&raw_output.stderr)
        ));
    }
    let output = String::from_utf8_lossy(&raw_output.stdout).into_owned();

    parse_openssl_x509_output(&output, pem)
}

fn parse_openssl_x509_output(output: &str, pem: &str) -> Result<ParsedCertificate> {
    let mut subject = String::new();
    let mut issuer = String::new();
    let mut not_before_str = String::new();
    let mut not_after_str = String::new();
    let mut fingerprint_sha256 = String::new();
    let mut serial_number = String::new();

    for line in output.lines() {
        if let Some(v) = line.strip_prefix("subject=") {
            subject = v.trim().to_string();
        } else if let Some(v) = line.strip_prefix("issuer=") {
            issuer = v.trim().to_string();
        } else if let Some(v) = line.strip_prefix("notBefore=") {
            not_before_str = v.trim().to_string();
        } else if let Some(v) = line.strip_prefix("notAfter=") {
            not_after_str = v.trim().to_string();
        } else if let Some(v) = line.strip_prefix("sha256 Fingerprint=") {
            fingerprint_sha256 = v.trim().to_string();
        } else if let Some(v) = line.strip_prefix("serial=") {
            serial_number = v.trim().to_string();
        }
    }

    if subject.is_empty() {
        return Err(anyhow!(
            "Failed to parse certificate: subject not found in openssl output"
        ));
    }

    let common_name = extract_cn(&subject).unwrap_or_else(|| subject.clone());
    let issuer_common_name = extract_cn(&issuer).unwrap_or_else(|| issuer.clone());

    let not_before = parse_openssl_date(&not_before_str)
        .with_context(|| format!("Failed to parse notBefore: '{not_before_str}'"))?;
    let not_after = parse_openssl_date(&not_after_str)
        .with_context(|| format!("Failed to parse notAfter: '{not_after_str}'"))?;

    Ok(ParsedCertificate {
        common_name,
        subject,
        issuer_common_name,
        issuer,
        not_before,
        not_after,
        fingerprint_sha256,
        serial_number,
        pem: pem.to_string(),
    })
}

/// Extracts the CN value from a distinguished name string such as
/// "CN=My Root CA, O=Org, C=US" or "CN = My CA,O=Org".
pub fn extract_cn(rdns: &str) -> Option<String> {
    for part in rdns.split(',') {
        let part = part.trim();
        if let Some(v) = part.strip_prefix("CN=") {
            return Some(v.trim().to_string());
        }
        if let Some(v) = part.strip_prefix("CN =") {
            return Some(v.trim().to_string());
        }
    }
    None
}

/// Parses the OpenSSL date format: "Jan  1 00:00:00 2020 GMT"
pub fn parse_openssl_date(date_str: &str) -> Result<DateTime<Utc>> {
    let dt = NaiveDateTime::parse_from_str(date_str.trim(), "%b %e %H:%M:%S %Y %Z")
        .with_context(|| format!("Unrecognised date format: '{date_str}'"))?;
    Ok(dt.and_utc())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_pem_bundle_empty() {
        assert!(split_pem_bundle("").is_empty());
    }

    #[test]
    fn test_split_pem_bundle_no_certs() {
        assert!(split_pem_bundle("just some random text").is_empty());
    }

    #[test]
    fn test_split_pem_bundle_single_cert() {
        let pem = "-----BEGIN CERTIFICATE-----\nABCD\n-----END CERTIFICATE-----\n";
        let certs = split_pem_bundle(pem);
        assert_eq!(certs.len(), 1);
        assert!(certs[0].contains("-----BEGIN CERTIFICATE-----"));
        assert!(certs[0].contains("-----END CERTIFICATE-----"));
    }

    #[test]
    fn test_split_pem_bundle_two_certs() {
        let pem = "-----BEGIN CERTIFICATE-----\nAAAA\n-----END CERTIFICATE-----\n\
                   -----BEGIN CERTIFICATE-----\nBBBB\n-----END CERTIFICATE-----\n";
        let certs = split_pem_bundle(pem);
        assert_eq!(certs.len(), 2);
    }

    #[test]
    fn test_extract_cn_simple() {
        assert_eq!(
            extract_cn("CN=My CA, O=Org, C=US"),
            Some("My CA".to_string())
        );
    }

    #[test]
    fn test_extract_cn_with_spaces() {
        assert_eq!(extract_cn("CN = Root CA"), Some("Root CA".to_string()));
    }

    #[test]
    fn test_extract_cn_not_found() {
        assert_eq!(extract_cn("O=Org, C=US"), None);
    }

    #[test]
    fn test_parse_openssl_date() {
        let result = parse_openssl_date("Jan  1 00:00:00 2020 GMT");
        assert!(result.is_ok());
        let dt = result.unwrap();
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2020-01-01");
    }

    #[test]
    fn test_parse_openssl_date_invalid() {
        assert!(parse_openssl_date("not a date").is_err());
    }

    #[test]
    fn test_not_after_valid_cert_is_in_future() {
        let not_after = parse_openssl_date("Jan  1 00:00:00 2099 GMT").unwrap();
        assert!(not_after > Utc::now());
    }

    #[test]
    fn test_not_after_expired_cert_is_in_past() {
        let not_after = parse_openssl_date("Jan  1 00:00:00 2000 GMT").unwrap();
        assert!(not_after < Utc::now());
    }
}
