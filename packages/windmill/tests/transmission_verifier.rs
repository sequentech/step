// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The receiver's check of an ER transmission package (SW-F-0226). The
//! fixture package was made with the ecies-tool jar, the OpenSSL command and
//! p12 board member signatures, as the OVCS makes them; the other packages
//! are built here, natively, the same way.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use openssl::asn1::Asn1Time;
use openssl::bn::{BigNum, MsbOption};
use openssl::ec::{EcGroup, EcKey};
use openssl::hash::MessageDigest;
use openssl::nid::Nid;
use openssl::pkey::{PKey, Private};
use openssl::rsa::Rsa;
use openssl::sign::Signer;
use openssl::x509::extension::{
    AuthorityKeyIdentifier, BasicConstraints, KeyUsage, SubjectKeyIdentifier,
};
use openssl::x509::{X509Name, X509};
use sha2::{Digest, Sha256};
use std::io::{Cursor, Write};
use std::path::PathBuf;
use windmill::services::consolidation::eml_types::{ACMJson, ACMTrustee};
use windmill::services::consolidation::sealing::{ecies_encrypt, openssl_enc_encrypt};
use windmill::services::consolidation::verify_transmission_package::{
    grouped_sha256, public_key_sha256, verify_transmission_package, CertificateStatus, Refusal,
    SignatureStatus, TransmissionRefusal, VerifyOptions,
};
use windmill::services::consolidation::xz_compress::xz_compress;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

const FIXTURE_SHA256: &str = "DEB2F35060A6082B2CEE9A2004B21133835E3A39EAA7BF2E9D3FA5007F64FEED";
const STATION: &str = "1234567";

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/transmission")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn fixture_options(key: &str) -> VerifyOptions {
    VerifyOptions::new(&fixture(key), &fixture("ovcs.pub.pem"))
        .unwrap()
        .with_member_certificates(
            Some(fixture("ca.pem").as_slice()),
            Some(fixture("certificates.pem").as_slice()),
        )
        .unwrap()
        .with_required_signatures(2)
}

fn refusals(result: Result<impl std::fmt::Debug, TransmissionRefusal>) -> Vec<Refusal> {
    result.expect_err("the package must be refused").refusals
}

// The package the OVCS made

#[test]
fn the_ovcs_package_verifies_for_each_destination() {
    let package = fixture("all_servers.zip");
    for (key, public, destination) in [
        ("destination-1.key.pem", "destination-1.pub.pem", "ccs-1"),
        ("destination-2.key.pem", "destination-2.pub.pem", "ccs-2"),
    ] {
        let verified = verify_transmission_package(&package, &fixture_options(key)).unwrap();
        assert_eq!(verified.destination.as_deref(), Some(destination));
        let public = PKey::public_key_from_pem(&fixture(public)).unwrap();
        assert_eq!(
            verified.destination_key_sha256,
            public_key_sha256(&public).unwrap()
        );
        assert_eq!(verified.eml, fixture("er.eml"));
        assert_eq!(verified.eml_sha256, FIXTURE_SHA256);
        assert_eq!(verified.declared_sha256, FIXTURE_SHA256);
        assert_eq!(verified.station_id, STATION);
        assert_eq!(verified.station_name, "Station Alpha");
        assert_eq!(verified.event_id, "EV-2026");
        assert_eq!(verified.transaction_id.as_deref(), Some("3141"));
        assert_eq!(verified.valid_signatures, 2);
        let names: Vec<&str> = verified.signers.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["Ana Board Member", "Ben Board Member"]);
        for signer in &verified.signers {
            assert_eq!(signer.signature, SignatureStatus::Valid);
            assert!(matches!(
                &signer.certificate,
                CertificateStatus::Chained { subject } if subject.contains(&signer.name)
            ));
        }
        let contests: Vec<(&str, usize)> = verified
            .contests
            .iter()
            .map(|contest| (contest.id.as_str(), contest.selections.len()))
            .collect();
        assert_eq!(contests, [("PRES", 2), ("SEN", 1)]);
        let president = &verified.contests[0];
        assert_eq!(president.election_name, "National Election");
        assert_eq!(
            president.selections[1].candidates[0].name,
            "Bob Sample & Co"
        );
        assert_eq!(president.selections[0].valid_votes, 60);
        let metrics: Vec<(&str, i64)> = president
            .metrics
            .iter()
            .map(|metric| (metric.id.as_str(), metric.value))
            .collect();
        assert_eq!(metrics, [("RV", 120), ("BB", 100), ("VB", 97), ("IB", 3)]);
    }
}

#[test]
fn the_sha256_from_the_page_is_accepted_in_groups_and_any_case() {
    let options = fixture_options("destination-1.key.pem")
        .with_expected_sha256(&grouped_sha256(FIXTURE_SHA256).to_lowercase())
        .unwrap();
    verify_transmission_package(&fixture("all_servers.zip"), &options).unwrap();
}

#[test]
fn another_sha256_than_the_page_shows_is_refused() {
    let expected = "00".repeat(32);
    let options = fixture_options("destination-1.key.pem")
        .with_expected_sha256(&expected)
        .unwrap();
    assert_eq!(
        refusals(verify_transmission_package(
            &fixture("all_servers.zip"),
            &options
        )),
        [Refusal::ExpectedHashMismatch {
            expected,
            computed: FIXTURE_SHA256.to_string(),
        }]
    );
}

#[test]
fn a_named_destination_does_not_open_with_another_destinations_key() {
    let options = fixture_options("destination-1.key.pem").with_destination("ccs-2");
    assert_eq!(
        refusals(verify_transmission_package(
            &fixture("all_servers.zip"),
            &options
        )),
        [Refusal::SealedForAnotherKey {
            destinations: vec!["ccs-2".to_string()],
        }]
    );
    let options = fixture_options("destination-1.key.pem").with_destination("ccs-1");
    verify_transmission_package(&fixture("all_servers.zip"), &options).unwrap();
}

#[test]
fn an_unknown_destination_is_refused() {
    let options = fixture_options("destination-1.key.pem").with_destination("ccs-9");
    assert_eq!(
        refusals(verify_transmission_package(
            &fixture("all_servers.zip"),
            &options
        )),
        [Refusal::UnknownDestination {
            requested: "ccs-9".to_string(),
            available: vec!["ccs-1".to_string(), "ccs-2".to_string()],
        }]
    );
}

#[test]
fn a_key_of_no_destination_opens_nothing() {
    let stranger = ec_key().private_key_to_pem_pkcs8().unwrap();
    let options = VerifyOptions::new(&stranger, &fixture("ovcs.pub.pem")).unwrap();
    assert_eq!(
        refusals(verify_transmission_package(
            &fixture("all_servers.zip"),
            &options
        )),
        [Refusal::SealedForAnotherKey {
            destinations: vec!["ccs-1".to_string(), "ccs-2".to_string()],
        }]
    );
}

#[test]
fn another_ovcs_key_is_refused() {
    let other = PKey::public_key_from_der(&ec_key().public_key_to_der().unwrap()).unwrap();
    let options = VerifyOptions::new(
        &fixture("destination-1.key.pem"),
        &other.public_key_to_pem().unwrap(),
    )
    .unwrap();
    assert_eq!(
        refusals(verify_transmission_package(
            &fixture("all_servers.zip"),
            &options
        )),
        [Refusal::OvcsKeyMismatch, Refusal::OvcsSignatureInvalid]
    );
}

#[test]
fn member_certificates_that_do_not_chain_to_the_ca_are_refused() {
    let options = VerifyOptions::new(&fixture("destination-1.key.pem"), &fixture("ovcs.pub.pem"))
        .unwrap()
        .with_member_certificates(
            Some(fixture("other-ca.pem").as_slice()),
            Some(fixture("certificates.pem").as_slice()),
        )
        .unwrap();
    let refused = verify_transmission_package(&fixture("all_servers.zip"), &options).unwrap_err();
    assert_eq!(refused.refusals.len(), 2);
    assert!(refused
        .refusals
        .iter()
        .all(|refusal| matches!(refusal, Refusal::MemberCertificateNotChained { .. })));
    let contents = refused.contents.expect("the package opened");
    assert_eq!(contents.valid_signatures, 0);
}

#[test]
fn a_ca_without_member_certificates_refuses_every_member() {
    let options = VerifyOptions::new(&fixture("destination-1.key.pem"), &fixture("ovcs.pub.pem"))
        .unwrap()
        .with_member_certificates(Some(fixture("ca.pem").as_slice()), None)
        .unwrap();
    let refused = refusals(verify_transmission_package(
        &fixture("all_servers.zip"),
        &options,
    ));
    assert_eq!(
        refused,
        [
            Refusal::MemberCertificateNotFound {
                member: "Ana Board Member (SBEI-1)".to_string()
            },
            Refusal::MemberCertificateNotFound {
                member: "Ben Board Member (SBEI-2)".to_string()
            },
        ]
    );
}

#[test]
fn member_certificates_without_a_ca_are_matched_by_key() {
    let options = VerifyOptions::new(&fixture("destination-1.key.pem"), &fixture("ovcs.pub.pem"))
        .unwrap()
        .with_member_certificates(None, Some(fixture("certificates.pem").as_slice()))
        .unwrap();
    let verified = verify_transmission_package(&fixture("all_servers.zip"), &options).unwrap();
    assert!(verified
        .signers
        .iter()
        .all(|signer| matches!(signer.certificate, CertificateStatus::Matched { .. })));
}

#[test]
fn more_signatures_than_the_package_has_are_refused() {
    let options = fixture_options("destination-1.key.pem").with_required_signatures(3);
    assert_eq!(
        refusals(verify_transmission_package(
            &fixture("all_servers.zip"),
            &options
        )),
        [Refusal::TooFewSignatures {
            required: 3,
            valid: 2
        }]
    );
}

#[test]
fn a_changed_byte_anywhere_is_refused_or_changes_nothing() {
    let package = fixture("all_servers.zip");
    let options = fixture_options("destination-1.key.pem");
    let original = fixture("er.eml");
    let returns = {
        let mut archive = zip::ZipArchive::new(Cursor::new(&package)).unwrap();
        let entry = archive.by_name(&format!("ccs-1/er_{STATION}.zip")).unwrap();
        let start = entry.data_start() as usize;
        start..start + entry.compressed_size() as usize
    };
    for index in 0..package.len() {
        let mut changed = package.clone();
        changed[index] ^= 0x5a;
        match verify_transmission_package(&changed, &options) {
            Err(_) => {}
            // Zip metadata (times, attributes) and the other destination's
            // package are not what this destination is sent.
            Ok(verified) => {
                assert!(!returns.contains(&index), "byte {index}");
                assert_eq!(verified.eml, original, "byte {index}");
            }
        }
    }
}

#[test]
fn bad_options_are_errors_before_any_check() {
    let ovcs = fixture("ovcs.pub.pem");
    assert!(VerifyOptions::new(b"not a key", &ovcs).is_err());
    let rsa = PKey::from_rsa(Rsa::generate(2048).unwrap()).unwrap();
    assert!(VerifyOptions::new(&rsa.private_key_to_pem_pkcs8().unwrap(), &ovcs).is_err());
    let key = fixture("destination-1.key.pem");
    assert!(VerifyOptions::new(&key, b"not a key").is_err());
    let options = VerifyOptions::new(&key, &ovcs).unwrap();
    assert!(options.clone().with_expected_sha256("ABCD").is_err());
    assert!(options
        .clone()
        .with_member_certificates(Some(b"no certificate".as_slice()), None)
        .is_err());
    assert!(options
        .with_member_certificates(None, Some(b"-----BEGIN CERTIFICATE-----\n".as_slice()))
        .is_err());
}

#[test]
fn what_is_not_a_package_is_refused() {
    let options = fixture_options("destination-1.key.pem");
    assert!(matches!(
        refusals(verify_transmission_package(b"not a zip", &options))[..],
        [Refusal::UnreadablePackage(_)]
    ));
    let empty = zip_of(&[("readme.txt", b"nothing".to_vec())]);
    assert_eq!(
        refusals(verify_transmission_package(&empty, &options)),
        [Refusal::NoElectionReturns]
    );
    let incomplete = zip_of(&[("ccs-1/er_1.zip", zip_of(&[("er_1.json", b"{}".to_vec())]))]);
    assert!(matches!(
        refusals(verify_transmission_package(&incomplete, &options))[..],
        [Refusal::UnreadablePackage(_)]
    ));
    let malformed = zip_of(&[(
        "ccs-1/er_1.zip",
        zip_of(&[
            ("er_1.json", b"{}".to_vec()),
            ("er_1.exz", b"sealed".to_vec()),
        ]),
    )]);
    assert!(matches!(
        refusals(verify_transmission_package(&malformed, &options))[..],
        [Refusal::UnreadablePackage(_)]
    ));
}

// Packages built here

fn ec_key() -> PKey<Private> {
    let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap();
    PKey::from_ec_key(EcKey::generate(&group).unwrap()).unwrap()
}

fn sign(key: &PKey<Private>, data: &[u8]) -> String {
    let mut signer = Signer::new(MessageDigest::sha256(), key).unwrap();
    signer.update(data).unwrap();
    BASE64.encode(signer.sign_to_vec().unwrap())
}

fn public_pem(key: &PKey<Private>) -> String {
    String::from_utf8(key.public_key_to_pem().unwrap()).unwrap()
}

fn sha256_upper(data: &[u8]) -> String {
    hex::encode_upper(Sha256::digest(data))
}

fn certificate(
    name: &str,
    key: &PKey<Private>,
    issuer: Option<(&X509, &PKey<Private>)>,
    ca: bool,
) -> X509 {
    let mut subject = X509Name::builder().unwrap();
    subject.append_entry_by_nid(Nid::COMMONNAME, name).unwrap();
    let subject = subject.build();
    let mut builder = X509::builder().unwrap();
    builder.set_version(2).unwrap();
    let mut serial = BigNum::new().unwrap();
    serial.rand(64, MsbOption::MAYBE_ZERO, false).unwrap();
    builder
        .set_serial_number(&serial.to_asn1_integer().unwrap())
        .unwrap();
    builder.set_subject_name(&subject).unwrap();
    builder
        .set_issuer_name(issuer.map_or(&*subject, |(issuer, _)| issuer.subject_name()))
        .unwrap();
    builder.set_pubkey(key).unwrap();
    builder
        .set_not_before(&Asn1Time::days_from_now(0).unwrap())
        .unwrap();
    builder
        .set_not_after(&Asn1Time::days_from_now(3650).unwrap())
        .unwrap();
    let mut constraints = BasicConstraints::new();
    constraints.critical();
    if ca {
        constraints.ca();
    }
    builder
        .append_extension(constraints.build().unwrap())
        .unwrap();
    let mut usage = KeyUsage::new();
    usage.critical();
    if ca {
        usage.key_cert_sign().crl_sign();
    } else {
        usage.digital_signature().non_repudiation();
    }
    builder.append_extension(usage.build().unwrap()).unwrap();
    let identifier = SubjectKeyIdentifier::new()
        .build(&builder.x509v3_context(issuer.map(|(issuer, _)| &**issuer), None))
        .unwrap();
    builder.append_extension(identifier).unwrap();
    if issuer.is_some() {
        let authority = AuthorityKeyIdentifier::new()
            .keyid(true)
            .build(&builder.x509v3_context(issuer.map(|(issuer, _)| &**issuer), None))
            .unwrap();
        builder.append_extension(authority).unwrap();
    }
    let signing_key = issuer.map_or(key, |(_, issuer_key)| issuer_key);
    builder.sign(signing_key, MessageDigest::sha256()).unwrap();
    builder.build()
}

struct Member {
    id: String,
    name: String,
    key: PKey<Private>,
    certificate: X509,
}

/// The OVCS, two destinations and a board of two, under one CA.
struct World {
    ovcs: PKey<Private>,
    destinations: Vec<(String, PKey<Private>)>,
    ca: X509,
    members: Vec<Member>,
}

/// What goes into a package: the EML sealed, the hash declared, and the
/// bytes the OVCS signed.
struct Contents<'a> {
    eml: &'a [u8],
    declared_sha256: String,
    ovcs_signed: &'a [u8],
    members: Vec<ACMTrustee>,
}

impl World {
    fn new() -> World {
        let ca_key = ec_key();
        let ca = certificate("Board CA", &ca_key, None, true);
        let rsa = PKey::from_rsa(Rsa::generate(2048).unwrap()).unwrap();
        let members = [("SBEI-1", "Ana", ec_key()), ("SBEI-2", "Ben", rsa)]
            .into_iter()
            .map(|(id, name, key)| Member {
                id: id.to_string(),
                name: name.to_string(),
                certificate: certificate(name, &key, Some((&ca, &ca_key)), false),
                key,
            })
            .collect();
        World {
            ovcs: ec_key(),
            destinations: vec![
                ("ccs-1".to_string(), ec_key()),
                ("ccs-2".to_string(), ec_key()),
            ],
            ca,
            members,
        }
    }

    fn trustee(member: &Member, signed: &[u8]) -> ACMTrustee {
        ACMTrustee {
            id: member.id.clone(),
            signature: Some(sign(&member.key, signed)),
            publickey: Some(public_pem(&member.key)),
            name: member.name.clone(),
        }
    }

    fn trustees(&self, signed: &[u8]) -> Vec<ACMTrustee> {
        self.members
            .iter()
            .map(|member| World::trustee(member, signed))
            .collect()
    }

    fn certificates_pem(&self) -> Vec<u8> {
        self.members
            .iter()
            .flat_map(|member| member.certificate.to_pem().unwrap())
            .collect()
    }

    fn options(&self, destination: usize) -> VerifyOptions {
        VerifyOptions::new(
            &self.destinations[destination]
                .1
                .private_key_to_pem_pkcs8()
                .unwrap(),
            &self.ovcs.public_key_to_pem().unwrap(),
        )
        .unwrap()
        .with_member_certificates(
            Some(self.ca.to_pem().unwrap().as_slice()),
            Some(self.certificates_pem().as_slice()),
        )
        .unwrap()
        .with_required_signatures(2)
    }

    fn honest(&self, eml: &'_ [u8]) -> Vec<u8> {
        self.package(&Contents {
            eml,
            declared_sha256: sha256_upper(eml),
            ovcs_signed: eml,
            members: self.trustees(eml),
        })
    }

    fn er_zip(&self, contents: &Contents, destination: &PKey<Private>) -> Vec<u8> {
        let password = random_password();
        let exz =
            openssl_enc_encrypt(password.as_bytes(), &xz_compress(contents.eml).unwrap()).unwrap();
        let encrypted_key = ecies_encrypt(destination, password.as_bytes()).unwrap();
        let json = ACMJson {
            device_id: "SQUNT420535311".to_string(),
            serial_number: "SEQ-NT-52706782".to_string(),
            station_id: STATION.to_string(),
            station_name: "Station Alpha".to_string(),
            event_id: "EV-2026".to_string(),
            event_name: "Overseas Voting 2026".to_string(),
            sha256_hash: contents.declared_sha256.clone(),
            encrypted_key: BASE64.encode(encrypted_key),
            members: contents.members.clone(),
            ip_address: "192.168.1.67".to_string(),
            mac_address: "3C:7E:5A:89:4D:2F".to_string(),
            er_datetime: "10/15/2026 10:00:00 AM".to_string(),
            signature: sign(&self.ovcs, contents.ovcs_signed),
            publickey: public_pem(&self.ovcs),
            transfer_start: "10/15/2026 10:00:00 AM".to_string(),
        };
        zip_of(&[
            (format!("er_{STATION}.exz").as_str(), exz),
            (
                format!("er_{STATION}.json").as_str(),
                serde_json::to_vec_pretty(&json).unwrap(),
            ),
        ])
    }

    fn package(&self, contents: &Contents) -> Vec<u8> {
        let entries: Vec<(String, Vec<u8>)> = self
            .destinations
            .iter()
            .map(|(tag, key)| {
                (
                    format!("{tag}/er_{STATION}.zip"),
                    self.er_zip(contents, key),
                )
            })
            .collect();
        let entries: Vec<(&str, Vec<u8>)> = entries
            .iter()
            .map(|(name, bytes)| (name.as_str(), bytes.clone()))
            .collect();
        zip_of(&entries)
    }
}

/// 64 hex digits, as the OVCS's AES passwords.
fn random_password() -> String {
    let mut bytes = [0u8; 32];
    openssl::rand::rand_bytes(&mut bytes).unwrap();
    hex::encode(bytes)
}

/// A zip of `entries`, with a folder entry before each folder's files as
/// the OVCS writes them.
fn zip_of(entries: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut folders = std::collections::BTreeSet::new();
    for (name, bytes) in entries {
        if let Some((folder, _)) = name.rsplit_once('/') {
            if folders.insert(folder.to_string()) {
                writer.add_directory(folder, options).unwrap();
            }
        }
        writer.start_file(*name, options).unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

#[test]
fn a_package_built_natively_verifies_for_each_destination() {
    let world = World::new();
    let eml = fixture("er.eml");
    let package = world.honest(&eml);
    for (index, (tag, _)) in world.destinations.iter().enumerate() {
        let verified = verify_transmission_package(&package, &world.options(index)).unwrap();
        assert_eq!(verified.destination.as_deref(), Some(tag.as_str()));
        assert_eq!(verified.eml, eml);
        assert_eq!(verified.valid_signatures, 2);
    }
}

#[test]
fn returns_changed_and_sealed_again_are_refused() {
    let world = World::new();
    let eml = fixture("er.eml");
    let forged = String::from_utf8(eml.clone())
        .unwrap()
        .replace("<ValidVotes>60</ValidVotes>", "<ValidVotes>90</ValidVotes>")
        .into_bytes();
    let package = world.package(&Contents {
        eml: &forged,
        declared_sha256: sha256_upper(&eml),
        ovcs_signed: &eml,
        members: world.trustees(&eml),
    });
    let refused = verify_transmission_package(&package, &world.options(0)).unwrap_err();
    assert_eq!(
        refused.refusals,
        [
            Refusal::HashMismatch {
                declared: sha256_upper(&eml),
                computed: sha256_upper(&forged),
            },
            Refusal::OvcsSignatureInvalid,
            Refusal::MemberSignatureInvalid {
                member: "Ana (SBEI-1)".to_string()
            },
            Refusal::MemberSignatureInvalid {
                member: "Ben (SBEI-2)".to_string()
            },
            Refusal::TooFewSignatures {
                required: 2,
                valid: 0
            },
        ]
    );
    let contents = refused.contents.expect("the package opened");
    assert_eq!(contents.contests[0].selections[0].valid_votes, 90);
}

#[test]
fn returns_changed_with_their_sha256_replaced_are_refused() {
    let world = World::new();
    let eml = fixture("er.eml");
    let forged = String::from_utf8(eml.clone())
        .unwrap()
        .replace("<ValidVotes>37</ValidVotes>", "<ValidVotes>73</ValidVotes>")
        .into_bytes();
    let package = world.package(&Contents {
        eml: &forged,
        declared_sha256: sha256_upper(&forged),
        ovcs_signed: &eml,
        members: world.trustees(&eml),
    });
    let refused = refusals(verify_transmission_package(&package, &world.options(1)));
    assert!(!refused
        .iter()
        .any(|refusal| matches!(refusal, Refusal::HashMismatch { .. })));
    assert!(refused.contains(&Refusal::OvcsSignatureInvalid));
    assert_eq!(
        refused
            .iter()
            .filter(|refusal| matches!(refusal, Refusal::MemberSignatureInvalid { .. }))
            .count(),
        2
    );
}

#[test]
fn a_missing_member_signature_is_below_the_required_count() {
    let world = World::new();
    let eml = fixture("er.eml");
    let mut members = world.trustees(&eml);
    members[1].signature = None;
    let package = world.package(&Contents {
        eml: &eml,
        declared_sha256: sha256_upper(&eml),
        ovcs_signed: &eml,
        members,
    });
    let refused = verify_transmission_package(&package, &world.options(0)).unwrap_err();
    assert_eq!(
        refused.refusals,
        [Refusal::TooFewSignatures {
            required: 2,
            valid: 1
        }]
    );
    let contents = refused.contents.expect("the package opened");
    assert_eq!(contents.signers[1].signature, SignatureStatus::Missing);
    let options = world.options(0).with_required_signatures(1);
    verify_transmission_package(&package, &options).unwrap();
}

#[test]
fn a_member_signing_twice_counts_once() {
    let world = World::new();
    let eml = fixture("er.eml");
    let ana = World::trustee(&world.members[0], &eml);
    let package = world.package(&Contents {
        eml: &eml,
        declared_sha256: sha256_upper(&eml),
        ovcs_signed: &eml,
        members: vec![
            ana.clone(),
            ACMTrustee {
                id: "SBEI-3".to_string(),
                ..ana
            },
        ],
    });
    assert_eq!(
        refusals(verify_transmission_package(&package, &world.options(0))),
        [Refusal::TooFewSignatures {
            required: 2,
            valid: 1
        }]
    );
}

#[test]
fn a_member_outside_the_board_ca_is_refused() {
    let world = World::new();
    let eml = fixture("er.eml");
    let outsider_key = ec_key();
    let outsider_ca_key = ec_key();
    let outsider_ca = certificate("Other CA", &outsider_ca_key, None, true);
    let outsider = Member {
        id: "SBEI-9".to_string(),
        name: "Eve".to_string(),
        certificate: certificate(
            "Eve",
            &outsider_key,
            Some((&outsider_ca, &outsider_ca_key)),
            false,
        ),
        key: outsider_key,
    };
    let mut members = world.trustees(&eml);
    members.push(World::trustee(&outsider, &eml));
    let package = world.package(&Contents {
        eml: &eml,
        declared_sha256: sha256_upper(&eml),
        ovcs_signed: &eml,
        members,
    });
    // Without Eve's certificate she has none; with it, it does not chain.
    assert_eq!(
        refusals(verify_transmission_package(&package, &world.options(0))),
        [Refusal::MemberCertificateNotFound {
            member: "Eve (SBEI-9)".to_string()
        }]
    );
    let mut certificates = world.certificates_pem();
    certificates.extend(outsider.certificate.to_pem().unwrap());
    let options = world
        .options(0)
        .with_member_certificates(
            Some(world.ca.to_pem().unwrap().as_slice()),
            Some(certificates.as_slice()),
        )
        .unwrap()
        .with_required_signatures(3);
    let refused = refusals(verify_transmission_package(&package, &options));
    assert!(matches!(
        &refused[..],
        [
            Refusal::MemberCertificateNotChained { member, .. },
            Refusal::TooFewSignatures { required: 3, valid: 2 },
        ] if member == "Eve (SBEI-9)"
    ));
}

#[test]
fn a_changed_er_file_sealed_in_a_new_zip_is_refused() {
    let world = World::new();
    let eml = fixture("er.eml");
    let contents = Contents {
        eml: &eml,
        declared_sha256: sha256_upper(&eml),
        ovcs_signed: &eml,
        members: world.trustees(&eml),
    };
    let er_zip = world.er_zip(&contents, &world.destinations[0].1);
    let mut archive = zip::ZipArchive::new(Cursor::new(er_zip)).unwrap();
    let mut read = |name: &str| {
        let mut bytes = vec![];
        std::io::Read::read_to_end(&mut archive.by_name(name).unwrap(), &mut bytes).unwrap();
        bytes
    };
    let exz_name = format!("er_{STATION}.exz");
    let json_name = format!("er_{STATION}.json");
    let json = read(&json_name);
    let exz = read(&exz_name);
    for index in [16, exz.len() / 2, exz.len() - 1] {
        let mut changed = exz.clone();
        changed[index] ^= 0x01;
        let package = zip_of(&[(
            "ccs-1/er_1234567.zip",
            zip_of(&[
                (exz_name.as_str(), changed),
                (json_name.as_str(), json.clone()),
            ]),
        )]);
        assert!(
            matches!(
                refusals(verify_transmission_package(&package, &world.options(0)))[..],
                [Refusal::UnreadableErFile(_)]
            ),
            "byte {index}"
        );
    }
}

#[test]
fn a_lone_destination_package_verifies() {
    let world = World::new();
    let eml = fixture("er.eml");
    let contents = Contents {
        eml: &eml,
        declared_sha256: sha256_upper(&eml),
        ovcs_signed: &eml,
        members: world.trustees(&eml),
    };
    let er_zip = world.er_zip(&contents, &world.destinations[1].1);
    let verified = verify_transmission_package(&er_zip, &world.options(1)).unwrap();
    assert_eq!(verified.destination, None);
    assert_eq!(
        refusals(verify_transmission_package(&er_zip, &world.options(0))),
        [Refusal::SealedForAnotherKey {
            destinations: vec![format!("er_{STATION}")],
        }]
    );
}

#[test]
fn returns_that_are_not_eml_are_refused_after_the_other_checks() {
    let world = World::new();
    let eml = b"not xml".to_vec();
    let refused = verify_transmission_package(&world.honest(&eml), &world.options(0)).unwrap_err();
    assert!(matches!(refused.refusals[..], [Refusal::UnreadableEml(_)]));
}

/// The jar opens what the native ECIES seals. Needs Java and the jar:
/// `ECIES_TOOL_JAR=external-bin/ecies-tool.jar cargo test -p windmill
/// --test transmission_verifier -- --ignored`.
#[test]
#[ignore = "needs Java and ECIES_TOOL_JAR"]
fn the_jar_opens_what_is_sealed_natively() {
    let jar = std::env::var("ECIES_TOOL_JAR").expect("ECIES_TOOL_JAR");
    let directory = tempfile::tempdir().unwrap();
    let key = directory.path().join("key.pem");
    std::fs::write(&key, fixture("destination-1.key.pem")).unwrap();
    let public = PKey::public_key_from_pem(&fixture("destination-1.pub.pem")).unwrap();
    for plaintext in ["a", &"0123456789abcdef".repeat(4), &"x".repeat(200)] {
        let sealed = BASE64.encode(ecies_encrypt(&public, plaintext.as_bytes()).unwrap());
        let output = std::process::Command::new("java")
            .args(["-jar", &jar, "decrypt"])
            .arg(&key)
            .arg(&sealed)
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            String::from_utf8(output.stdout).unwrap().trim_end(),
            plaintext
        );
    }
}
