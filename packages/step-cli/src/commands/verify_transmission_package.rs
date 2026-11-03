// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Check an election returns transmission package received by hand: that
//! it opens with the destination's key, and that what it holds is what the
//! OVCS sent and the board members signed.
//!
//! Everything is read from files, so it runs on an air-gapped machine with
//! no Java. The checks are windmill's
//! `consolidation::verify_transmission_package`.

use anyhow::{anyhow, Context, Result};
use clap::Args;
use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};
use windmill::services::consolidation::verify_transmission_package::{
    grouped_sha256, verify_transmission_package, CertificateStatus, SignatureStatus,
    TransmissionContents, VerifyOptions,
};

/// Check an election returns transmission package (all_servers.zip)
#[derive(Args)]
#[command(about)]
pub struct VerifyTransmissionPackage {
    /// The package: all_servers.zip, or one destination's er_<station>.zip
    #[arg(value_name = "PACKAGE")]
    package: PathBuf,

    /// PEM file with the destination's private key
    #[arg(long, value_name = "PEM")]
    key: PathBuf,

    /// PEM file with the OVCS public key the package is signed with
    #[arg(long, value_name = "PEM")]
    ovcs_key: PathBuf,

    /// The destination's folder in the package; by default, the one that
    /// opens with the key
    #[arg(long, value_name = "TAG")]
    destination: Option<String>,

    /// PEM file with the event's root and intermediate CA certificates the
    /// board members' certificates must chain to
    #[arg(long, value_name = "PEM")]
    ca: Option<PathBuf>,

    /// PEM file with the board members' certificates
    #[arg(long, value_name = "PEM")]
    certificates: Option<PathBuf>,

    /// How many board members must have signed
    #[arg(long, value_name = "COUNT", default_value_t = 0)]
    required_signatures: usize,

    /// The election returns' SHA-256 shown where the package was downloaded
    #[arg(long, value_name = "SHA256")]
    expect_sha256: Option<String>,
}

impl VerifyTransmissionPackage {
    pub fn run(&self) {
        match self.verify() {
            Ok(contents) => print_summary(&contents),
            Err(error) => {
                eprintln!("{} {error:#}", "error:".red().bold());
                std::process::exit(1);
            }
        }
    }

    fn options(&self) -> Result<VerifyOptions> {
        let mut options = VerifyOptions::new(&read(&self.key)?, &read(&self.ovcs_key)?)?
            .with_required_signatures(self.required_signatures);
        if let Some(destination) = &self.destination {
            options = options.with_destination(destination);
        }
        let ca = self.ca.as_deref().map(read).transpose()?;
        let certificates = self.certificates.as_deref().map(read).transpose()?;
        options = options.with_member_certificates(ca.as_deref(), certificates.as_deref())?;
        if let Some(expected) = &self.expect_sha256 {
            options = options.with_expected_sha256(expected)?;
        }
        Ok(options)
    }

    fn verify(&self) -> Result<TransmissionContents> {
        let options = self.options()?;
        let package = read(&self.package)?;
        verify_transmission_package(&package, &options).map_err(|refusal| {
            let reasons: Vec<String> = refusal
                .refusals
                .iter()
                .map(|reason| format!("\n  - {reason}"))
                .collect();
            anyhow!(
                "{} is refused; its election returns must not be accepted:{}",
                self.package.display(),
                reasons.concat()
            )
        })
    }
}

fn read(path: &Path) -> Result<Vec<u8>> {
    fs::read(path).with_context(|| format!("could not read {}", path.display()))
}

fn signer_line(signature: SignatureStatus, certificate: &CertificateStatus) -> String {
    let signature = match signature {
        SignatureStatus::Valid => "signature valid",
        SignatureStatus::Invalid => "signature NOT valid",
        SignatureStatus::Missing => "did not sign",
    };
    match certificate {
        CertificateStatus::NotChecked => signature.to_string(),
        CertificateStatus::Matched { subject } => format!("{signature}, certificate {subject}"),
        CertificateStatus::Chained { subject } => {
            format!("{signature}, certificate {subject} chains to the CA")
        }
        CertificateStatus::NotFound => format!("{signature}, no certificate"),
        CertificateStatus::NotChained { subject, reason } => {
            format!("{signature}, certificate {subject} does not chain: {reason}")
        }
    }
}

fn print_summary(contents: &TransmissionContents) {
    println!("{} the transmission package verifies", "ok:".green().bold());
    if let Some(destination) = &contents.destination {
        println!("  destination    {destination}");
    }
    println!(
        "  sealed for     its key, SHA-256 {}",
        grouped_sha256(&contents.destination_key_sha256)
    );
    println!(
        "  station        {} {}",
        contents.station_id, contents.station_name
    );
    println!(
        "  event          {} {}",
        contents.event_id, contents.event_name
    );
    println!(
        "  transaction    {}",
        contents.transaction_id.as_deref().unwrap_or("-")
    );
    println!("  SHA-256        {}", grouped_sha256(&contents.eml_sha256));
    println!("  OVCS           signature valid");
    for signer in &contents.signers {
        println!(
            "  signed by      {} ({}): {}",
            signer.name,
            signer.id,
            signer_line(signer.signature, &signer.certificate)
        );
    }
    println!("  signatures     {} valid", contents.valid_signatures);
    for contest in &contents.contests {
        println!(
            "  contest        {} {} ({})",
            contest.id, contest.name, contest.election_name
        );
        for metric in &contest.metrics {
            println!("    {:<4} {:<44} {}", metric.id, metric.kind, metric.value);
        }
        for selection in &contest.selections {
            let candidates: Vec<String> = selection
                .candidates
                .iter()
                .map(|candidate| format!("{} {}", candidate.id, candidate.name))
                .collect();
            println!(
                "    {:<49} {}",
                candidates.join(", "),
                selection.valid_votes
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct Cli {
        #[command(flatten)]
        verify: VerifyTransmissionPackage,
    }

    fn fixture(name: &str) -> String {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../windmill/tests/fixtures/transmission")
            .join(name)
            .to_string_lossy()
            .into_owned()
    }

    fn command(extra: &[&str]) -> VerifyTransmissionPackage {
        let mut args = vec![
            "verify-transmission-package".to_string(),
            fixture("all_servers.zip"),
            "--key".to_string(),
            fixture("destination-1.key.pem"),
            "--ovcs-key".to_string(),
            fixture("ovcs.pub.pem"),
        ];
        args.extend(extra.iter().map(|arg| arg.to_string()));
        Cli::try_parse_from(args).unwrap().verify
    }

    fn error_of(command: &VerifyTransmissionPackage) -> String {
        format!("{:#}", command.verify().unwrap_err())
    }

    #[test]
    fn the_key_and_the_ovcs_key_are_required() {
        assert!(Cli::try_parse_from(["verify-transmission-package", "p.zip"]).is_err());
        assert!(
            Cli::try_parse_from(["verify-transmission-package", "p.zip", "--key", "k.pem"])
                .is_err()
        );
    }

    #[test]
    fn the_ovcs_package_verifies_with_every_check() {
        let ca = fixture("ca.pem");
        let certificates = fixture("certificates.pem");
        let command = command(&[
            "--ca",
            &ca,
            "--certificates",
            &certificates,
            "--required-signatures",
            "2",
            "--destination",
            "ccs-1",
            "--expect-sha256",
            "deb2f350 60a6082b 2cee9a20 04b21133 835e3a39 eaa7bf2e 9d3fa500 7f64feed",
        ]);
        let contents = command.verify().unwrap();
        assert_eq!(contents.transaction_id.as_deref(), Some("3141"));
        // Returns instead of exiting with a failure.
        command.run();
    }

    #[test]
    fn a_refusal_names_the_package_and_each_reason() {
        let other_ca = fixture("other-ca.pem");
        let certificates = fixture("certificates.pem");
        let message = error_of(&command(&[
            "--ca",
            &other_ca,
            "--certificates",
            &certificates,
            "--required-signatures",
            "3",
        ]));
        assert!(message.contains("all_servers.zip is refused"));
        assert!(message.contains("does not chain to the CA"));
        assert!(message.contains("3 required"));
    }

    #[test]
    fn another_destinations_key_is_refused() {
        let message = error_of(&command(&["--destination", "ccs-2"]));
        assert!(message.contains("does not open with this key"));
    }

    #[test]
    fn a_wrong_expected_sha256_is_refused() {
        let message = error_of(&command(&["--expect-sha256", &"0".repeat(64)]));
        assert!(message.contains("not the 0000"));
    }

    #[test]
    fn a_malformed_expected_sha256_is_an_error() {
        let message = error_of(&command(&["--expect-sha256", "abc"]));
        assert!(message.contains("not a SHA-256"));
    }

    #[test]
    fn a_missing_file_is_named() {
        let message = error_of(&command(&["--ca", "/nonexistent/ca.pem"]));
        assert!(message.contains("/nonexistent/ca.pem"));
    }

    #[test]
    fn a_changed_package_is_refused() {
        let directory = tempfile::tempdir().unwrap();
        let mut bytes = fs::read(fixture("all_servers.zip")).unwrap();
        let returns = {
            let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&bytes)).unwrap();
            let entry = archive.by_name("ccs-1/er_1234567.zip").unwrap();
            (entry.data_start() + entry.compressed_size() / 2) as usize
        };
        bytes[returns] ^= 0xff;
        let changed = directory.path().join("all_servers.zip");
        fs::write(&changed, bytes).unwrap();
        let command = VerifyTransmissionPackage {
            package: changed,
            ..command(&[])
        };
        assert!(error_of(&command).contains("is refused"));
    }

    #[test]
    fn signer_lines_say_what_was_checked() {
        assert_eq!(
            signer_line(SignatureStatus::Missing, &CertificateStatus::NotChecked),
            "did not sign"
        );
        assert_eq!(
            signer_line(SignatureStatus::Invalid, &CertificateStatus::NotFound),
            "signature NOT valid, no certificate"
        );
        assert_eq!(
            signer_line(
                SignatureStatus::Valid,
                &CertificateStatus::Matched {
                    subject: "CN=A".to_string()
                }
            ),
            "signature valid, certificate CN=A"
        );
        assert!(signer_line(
            SignatureStatus::Valid,
            &CertificateStatus::NotChained {
                subject: "CN=A".to_string(),
                reason: "unknown issuer".to_string()
            }
        )
        .ends_with("does not chain: unknown issuer"));
    }
}
