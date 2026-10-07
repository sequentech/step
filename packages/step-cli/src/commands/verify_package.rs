// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Check a signed configuration package the way the platform's import does.
//!
//! Everything is read from files, so it runs on an air-gapped machine with
//! the organization's root certificates and nothing else. The checks are
//! `sequent_core::election_config::package_verify`'s, the same ones windmill
//! runs before importing and the Election Architect runs when it opens a
//! package.

use anyhow::{anyhow, Context, Result};
use clap::Args;
use colored::Colorize;
use sequent_core::election_config::package_verify::{
    freshness, verify_package_at, Freshness, LastImport, PackageTrust, VerifiedPackage,
};
use sequent_core::election_config::{Severity, ValidationReport};
use std::fs;
use std::path::{Path, PathBuf};

/// Check a signed configuration package before importing it
#[derive(Args)]
#[command(about)]
pub struct VerifyPackage {
    /// The package zip
    #[arg(short = 'p', long, value_name = "PACKAGE")]
    package: PathBuf,

    /// PEM file with the root certificates a package's signing key must
    /// chain to
    #[arg(long, value_name = "PEM")]
    roots: PathBuf,

    /// PEM file with the root certificates the approvers' certificates must
    /// chain to
    #[arg(long, value_name = "PEM")]
    staff_roots: PathBuf,

    /// PEM file with revocation lists to apply, besides the package's own
    #[arg(long = "crl", value_name = "PEM")]
    crls: Vec<PathBuf>,

    /// How many approvals from different people a package needs
    #[arg(long, value_name = "COUNT")]
    required_approvals: u16,

    /// The revision of the last package imported for this configuration
    #[arg(long, value_name = "REVISION", requires = "last_manifest_sha256")]
    last_revision: Option<u64>,

    /// The SHA-256 of the last imported package's manifest.json
    #[arg(long, value_name = "SHA256", requires = "last_revision")]
    last_manifest_sha256: Option<String>,
}

impl VerifyPackage {
    pub fn run(&self) {
        if let Err(error) = self.verify() {
            eprintln!("{} {error:#}", "error:".red().bold());
            std::process::exit(1);
        }
    }

    fn verify(&self) -> Result<()> {
        let bytes = fs::read(&self.package)
            .with_context(|| format!("could not read {}", self.package.display()))?;
        let trust = self.trust()?;
        let verified = verify_package_at(&bytes, &trust, chrono::Utc::now()).map_err(|report| {
            report_problems(&report);
            anyhow!(
                "{} did not verify; nothing in it may be imported",
                self.package.display()
            )
        })?;
        let last = self
            .last_revision
            .zip(self.last_manifest_sha256.clone())
            .map(|(revision, manifest_sha256)| LastImport {
                revision,
                manifest_sha256,
            });
        let fresh = freshness(&verified, last.as_ref())
            .map_err(|problem| anyhow!("{} — {}", problem.path, problem.message))?;
        print_summary(&verified, fresh);
        Ok(())
    }

    fn trust(&self) -> Result<PackageTrust> {
        let lists = self
            .crls
            .iter()
            .map(|path| read_text(path))
            .collect::<Result<Vec<_>>>()?;
        PackageTrust::from_pem(
            &read_text(&self.roots)?,
            &read_text(&self.staff_roots)?,
            &lists,
            self.required_approvals,
        )
        .map_err(|problem| anyhow!("{} — {}", problem.path, problem.message))
    }
}

fn read_text(path: &Path) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("could not read {}", path.display()))
}

fn print_summary(verified: &VerifiedPackage, fresh: Freshness) {
    let configuration = &verified.manifest.configuration;
    println!("{} the package verifies", "ok:".green().bold());
    println!(
        "  configuration  {} ({})",
        configuration.name, configuration.external_id
    );
    println!("  revision       {}", configuration.revision);
    println!("  manifest       sha256:{}", verified.manifest_sha256);
    println!(
        "  signed by      {} (serial {}), {}",
        verified.signer.subject, verified.signer.serial, verified.manifest.produced.at
    );
    for (approval, approver) in verified.manifest.approvals.iter().zip(&verified.approvers) {
        println!(
            "  approved by    {} ({}), {}",
            approval.name, approver.subject, approval.signed_at
        );
    }
    println!(
        "  files          {}, ballot designs {}",
        verified.manifest.content.files.len(),
        verified.manifest.content.ballot_designs.len()
    );
    if fresh == Freshness::AlreadyImported {
        println!(
            "{} this package is the one imported last",
            "note:".yellow().bold()
        );
    }
}

fn report_problems(report: &ValidationReport) {
    for problem in &report.problems {
        let label = match problem.severity {
            Severity::Error => "error:".red().bold(),
            Severity::Warning => "warning:".yellow().bold(),
        };
        println!("{label} {} — {}", problem.path.bold(), problem.message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    use clap::Parser;
    use rcgen::{
        date_time_ymd, BasicConstraints, CertificateParams, CertificateRevocationListParams,
        CertifiedIssuer, DnType, IsCa, KeyIdMethod, KeyPair, KeyUsagePurpose, SerialNumber,
        PKCS_ECDSA_P256_SHA256,
    };
    use ring::rand::SystemRandom;
    use ring::signature::{EcdsaKeyPair, ECDSA_P256_SHA256_ASN1_SIGNING};
    use sequent_core::election_config::archive::{zip, Artifact};
    use sequent_core::election_config::manifest::{
        approval_payload, file_entries, package, sha256_hex, Approval, ConfigurationRevision,
        Content, Manifest, ManifestFormat, Produced,
    };
    use sequent_core::signing::SignatureAlgorithm;
    use std::collections::BTreeMap;
    use tempfile::TempDir;

    #[derive(Parser)]
    struct Cli {
        #[command(flatten)]
        verify: VerifyPackage,
    }

    #[test]
    fn the_last_import_needs_both_its_revision_and_its_digest() {
        let parsed = Cli::try_parse_from([
            "verify-package",
            "--package",
            "p.zip",
            "--roots",
            "roots.pem",
            "--staff-roots",
            "staff.pem",
            "--required-approvals",
            "2",
            "--last-revision",
            "7",
        ]);
        assert!(parsed.is_err());
    }

    #[test]
    fn an_unreadable_package_is_an_error_not_a_pass() {
        let cli = Cli::try_parse_from([
            "verify-package",
            "--package",
            "/nonexistent/p.zip",
            "--roots",
            "roots.pem",
            "--staff-roots",
            "staff.pem",
            "--required-approvals",
            "2",
        ])
        .unwrap();
        assert!(cli.verify.verify().is_err());
    }

    type Ca = CertifiedIssuer<'static, KeyPair>;

    fn ca(name: &str) -> Ca {
        let mut params = CertificateParams::new(Vec::<String>::new()).unwrap();
        params.distinguished_name.push(DnType::CommonName, name);
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.key_usages = vec![
            KeyUsagePurpose::KeyCertSign,
            KeyUsagePurpose::CrlSign,
            KeyUsagePurpose::DigitalSignature,
        ];
        params.not_before = date_time_ymd(2020, 1, 1);
        params.not_after = date_time_ymd(2040, 1, 1);
        params.key_identifier_method = KeyIdMethod::Sha256;
        CertifiedIssuer::self_signed(
            params,
            KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256).unwrap(),
        )
        .unwrap()
    }

    /// A person's or a key's signing certificate.
    struct Signer {
        chain_pem: String,
        key: KeyPair,
    }

    fn leaf(name: &str, issuer: &Ca, serial: u64) -> Signer {
        let mut params = CertificateParams::new(Vec::<String>::new()).unwrap();
        params.distinguished_name.push(DnType::CommonName, name);
        params.serial_number = Some(SerialNumber::from(serial));
        params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
        params.not_before = date_time_ymd(2025, 1, 1);
        params.not_after = date_time_ymd(2035, 1, 1);
        let key = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256).unwrap();
        let certificate = params.signed_by(&key, issuer).unwrap();
        Signer {
            chain_pem: format!("{}{}", certificate.pem(), issuer.pem()),
            key,
        }
    }

    fn sign(key: &KeyPair, message: &[u8]) -> Vec<u8> {
        let rng = SystemRandom::new();
        let pair =
            EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &key.serialize_der(), &rng)
                .unwrap();
        pair.sign(&rng, message).unwrap().as_ref().to_vec()
    }

    fn empty_crl(issuer: &Ca) -> String {
        CertificateRevocationListParams {
            this_update: date_time_ymd(2026, 6, 1),
            next_update: date_time_ymd(2026, 7, 1),
            crl_number: SerialNumber::from(1u64),
            issuing_distribution_point: None,
            revoked_certs: Vec::new(),
            key_identifier_method: KeyIdMethod::Sha256,
        }
        .signed_by(issuer)
        .unwrap()
        .pem()
        .unwrap()
    }

    const REVISION: u64 = 8;

    /// A package signed by the organization's key and approved by two of its
    /// staff, on disk next to the roots that vouch for them.
    struct Delivery {
        directory: TempDir,
        manifest_sha256: String,
    }

    impl Delivery {
        fn path(&self, name: &str) -> PathBuf {
            self.directory.path().join(name)
        }

        fn command(&self) -> VerifyPackage {
            VerifyPackage {
                package: self.path("package.zip"),
                roots: self.path("roots.pem"),
                staff_roots: self.path("staff.pem"),
                crls: vec![self.path("crl.pem")],
                required_approvals: 2,
                last_revision: None,
                last_manifest_sha256: None,
            }
        }
    }

    fn delivery() -> Delivery {
        let root = ca("Organization Root");
        let staff_root = ca("Staff Root");
        let key = leaf("Configuration Signing Key", &root, 1);
        let members = vec![
            Artifact {
                name: "official_election_setup.zip".to_string(),
                bytes: zip(&[Artifact {
                    name: "export_election_event-1.json".to_string(),
                    bytes: b"{}\n".to_vec(),
                }])
                .unwrap(),
            },
            Artifact {
                name: "blueprint.json".to_string(),
                bytes: b"{\"version\":4}".to_vec(),
            },
        ];
        let content = Content {
            files: file_entries(&members).unwrap(),
            ballot_designs: Vec::new(),
            reports: Vec::new(),
        };
        let mut manifest = Manifest {
            format: ManifestFormat::V1,
            configuration: ConfigurationRevision {
                external_id: "ov-2028".to_string(),
                name: "Overseas Voting 2028".to_string(),
                revision: REVISION,
                previous: None,
            },
            content_sha256: content.sha256().unwrap(),
            content,
            approvals: Vec::new(),
            produced: Produced {
                at: "2026-06-15T08:00:00Z".to_string(),
                custodian: "Key Custodian".to_string(),
                key_label: "configuration-signing".to_string(),
                producer: BTreeMap::new(),
            },
            revocation_lists: Vec::new(),
        };
        let payload = approval_payload(
            &manifest.configuration.external_id,
            manifest.configuration.revision,
            &manifest.content_sha256,
        )
        .unwrap();
        for (serial, name) in [(10, "Configuration Manager"), (11, "Security Officer")] {
            let approver = leaf(name, &staff_root, serial);
            manifest.approvals.push(Approval {
                name: name.to_string(),
                role: "approver".to_string(),
                signed_at: "2026-06-14T10:00:00Z".to_string(),
                algorithm: SignatureAlgorithm::EcdsaP256Sha256,
                certificate_chain: approver.chain_pem.clone(),
                signature: STANDARD.encode(sign(&approver.key, payload.as_bytes())),
            });
        }
        let bytes = manifest.to_bytes().unwrap();
        let signed = package(
            "package.zip",
            &members,
            &bytes,
            &sign(&key.key, &bytes),
            &key.chain_pem,
        )
        .unwrap();

        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join("package.zip"), signed.bytes).unwrap();
        fs::write(directory.path().join("roots.pem"), root.pem()).unwrap();
        fs::write(directory.path().join("staff.pem"), staff_root.pem()).unwrap();
        fs::write(directory.path().join("crl.pem"), empty_crl(&root)).unwrap();
        Delivery {
            directory,
            manifest_sha256: sha256_hex(&bytes),
        }
    }

    fn error_of(command: &VerifyPackage) -> String {
        format!("{:#}", command.verify().unwrap_err())
    }

    #[test]
    fn a_signed_and_approved_package_verifies() {
        let delivery = delivery();
        delivery.command().verify().unwrap();
        // Returns instead of exiting with a failure.
        delivery.command().run();
    }

    #[test]
    fn the_package_imported_last_still_verifies() {
        let delivery = delivery();
        let command = VerifyPackage {
            last_revision: Some(REVISION),
            last_manifest_sha256: Some(delivery.manifest_sha256.clone()),
            ..delivery.command()
        };
        command.verify().unwrap();
    }

    #[test]
    fn a_revision_older_than_the_last_import_is_refused() {
        let delivery = delivery();
        let command = VerifyPackage {
            last_revision: Some(REVISION + 1),
            last_manifest_sha256: Some("ab".repeat(32)),
            ..delivery.command()
        };
        assert!(command.verify().is_err());
    }

    #[test]
    fn a_package_with_too_few_approvals_does_not_verify() {
        let delivery = delivery();
        let command = VerifyPackage {
            required_approvals: 3,
            ..delivery.command()
        };
        assert!(error_of(&command).contains("did not verify"));
    }

    #[test]
    fn a_package_changed_after_signing_does_not_verify() {
        let delivery = delivery();
        let mut bytes = fs::read(delivery.path("package.zip")).unwrap();
        let last = bytes.len() / 2;
        bytes[last] ^= 0xff;
        fs::write(delivery.path("package.zip"), bytes).unwrap();
        assert!(delivery.command().verify().is_err());
    }

    #[test]
    fn roots_that_are_not_certificates_are_an_error() {
        let delivery = delivery();
        fs::write(delivery.path("roots.pem"), "not a certificate").unwrap();
        assert!(!error_of(&delivery.command()).contains("did not verify"));
    }

    #[test]
    fn a_missing_roots_file_is_named() {
        let delivery = delivery();
        let command = VerifyPackage {
            staff_roots: delivery.path("absent.pem"),
            ..delivery.command()
        };
        assert!(error_of(&command).contains("absent.pem"));
    }

    #[test]
    fn a_missing_revocation_list_is_named() {
        let delivery = delivery();
        let command = VerifyPackage {
            crls: vec![delivery.path("absent-crl.pem")],
            ..delivery.command()
        };
        assert!(error_of(&command).contains("absent-crl.pem"));
    }
}
