// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! A test PKI for staff signatures, generated with the openssl crate: a root
//! and an individual CA (RSA), a foreign EC root, and signers for every case
//! the server refuses. Revocation lists are encoded here by hand (the crate
//! can't build them) and signed with the issuer's key.
//! Include it with `#[path = "support/signing_pki.rs"] mod signing_pki;`.

#![allow(dead_code)]

use openssl::asn1::{Asn1Integer, Asn1Time};
use openssl::bn::BigNum;
use openssl::ec::{Asn1Flag, EcGroup, EcKey};
use openssl::hash::MessageDigest;
use openssl::nid::Nid;
use openssl::pkey::{PKey, Private};
use openssl::rsa::Rsa;
use openssl::sign::Signer;
use openssl::x509::extension::{
    AuthorityKeyIdentifier, BasicConstraints, ExtendedKeyUsage, KeyUsage, SubjectKeyIdentifier,
};
use openssl::x509::{X509Builder, X509Extension, X509NameBuilder, X509};
use std::sync::OnceLock;

/// The time the tests sign at: this binary's start, to the second, so
/// certificates are valid for routes that check against the clock.
pub fn pki_now() -> i64 {
    static START: OnceLock<i64> = OnceLock::new();
    *START.get_or_init(|| chrono::Utc::now().timestamp())
}
const DAY: i64 = 86_400;
const YEAR: i64 = 365 * DAY;

pub const CRL_URL: &str = "http://crl.staff-ca.invalid/individual.crl";
pub const ROOT_CRL_URL: &str = "http://crl.staff-ca.invalid/root.crl";

#[derive(Clone)]
pub struct Issued {
    pub cert: X509,
    pub key: PKey<Private>,
}

impl Issued {
    pub fn pem(&self) -> String {
        String::from_utf8(self.cert.to_pem().unwrap()).unwrap()
    }

    pub fn der(&self) -> Vec<u8> {
        self.cert.to_der().unwrap()
    }

    /// Signs `data` with SHA-256: RSA PKCS#1 v1.5 or ECDSA (DER).
    pub fn sign(&self, data: &[u8]) -> Vec<u8> {
        let mut signer = Signer::new(MessageDigest::sha256(), &self.key).unwrap();
        signer.update(data).unwrap();
        signer.sign_to_vec().unwrap()
    }
}

pub fn rsa_key() -> PKey<Private> {
    PKey::from_rsa(Rsa::generate(2048).unwrap()).unwrap()
}

pub fn ec_key() -> PKey<Private> {
    let mut group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap();
    group.set_asn1_flag(Asn1Flag::NAMED_CURVE);
    PKey::from_ec_key(EcKey::generate(&group).unwrap()).unwrap()
}

pub fn p384_key() -> PKey<Private> {
    let mut group = EcGroup::from_curve_name(Nid::SECP384R1).unwrap();
    group.set_asn1_flag(Asn1Flag::NAMED_CURVE);
    PKey::from_ec_key(EcKey::generate(&group).unwrap()).unwrap()
}

/// What a certificate says about itself.
#[derive(Clone)]
pub enum Usage {
    /// A CA: keyCertSign and cRLSign.
    Ca,
    /// A signer: digitalSignature + nonRepudiation, EKU clientAuth + email.
    Signer,
    /// keyEncipherment only.
    EncipherOnly,
    /// No key usage extension.
    None,
    /// A signer whose extended key usage is serverAuth only.
    ServerAuthOnly,
    /// A signer whose extended key usage is documentSigning (RFC 9336) only.
    DocumentSigningOnly,
}

pub struct Spec<'a> {
    /// (short name, value) pairs, e.g. [("C", "PH"), ("CN", "Maria Santos")].
    pub subject: &'a [(&'a str, &'a str)],
    pub serial: u32,
    pub not_before: i64,
    pub not_after: i64,
    pub usage: Usage,
    pub crl_url: Option<&'a str>,
}

impl<'a> Spec<'a> {
    pub fn signer(subject: &'a [(&'a str, &'a str)], serial: u32) -> Self {
        Spec {
            subject,
            serial,
            not_before: pki_now() - YEAR,
            not_after: pki_now() + YEAR,
            usage: Usage::Signer,
            crl_url: Some(CRL_URL),
        }
    }
}

/// Issues a certificate for `key`, by `issuer` (self-signed without one).
pub fn issue(spec: &Spec<'_>, key: &PKey<Private>, issuer: Option<&Issued>) -> X509 {
    let mut name = X509NameBuilder::new().unwrap();
    for (field, value) in spec.subject {
        name.append_entry_by_text(field, value).unwrap();
    }
    issue_named(spec, name.build(), key, issuer, MessageDigest::sha256())
}

/// [`issue`] with a ready subject name and the issuer's digest.
pub fn issue_named(
    spec: &Spec<'_>,
    name: openssl::x509::X509Name,
    key: &PKey<Private>,
    issuer: Option<&Issued>,
    digest: MessageDigest,
) -> X509 {
    let mut builder = X509Builder::new().unwrap();
    builder.set_version(2).unwrap();
    let serial = BigNum::from_u32(spec.serial).unwrap();
    builder
        .set_serial_number(&Asn1Integer::from_bn(&serial).unwrap())
        .unwrap();
    builder.set_subject_name(&name).unwrap();
    builder
        .set_issuer_name(issuer.map_or(&name, |issuer| issuer.cert.subject_name()))
        .unwrap();
    builder.set_pubkey(key).unwrap();
    builder
        .set_not_before(&Asn1Time::from_unix(spec.not_before).unwrap())
        .unwrap();
    builder
        .set_not_after(&Asn1Time::from_unix(spec.not_after).unwrap())
        .unwrap();
    match spec.usage {
        Usage::Ca => {
            builder
                .append_extension(BasicConstraints::new().critical().ca().build().unwrap())
                .unwrap();
            builder
                .append_extension(
                    KeyUsage::new()
                        .critical()
                        .key_cert_sign()
                        .crl_sign()
                        .build()
                        .unwrap(),
                )
                .unwrap();
        }
        Usage::Signer => {
            builder
                .append_extension(
                    KeyUsage::new()
                        .critical()
                        .digital_signature()
                        .non_repudiation()
                        .build()
                        .unwrap(),
                )
                .unwrap();
            builder
                .append_extension(
                    ExtendedKeyUsage::new()
                        .client_auth()
                        .email_protection()
                        .build()
                        .unwrap(),
                )
                .unwrap();
        }
        Usage::EncipherOnly => {
            builder
                .append_extension(
                    KeyUsage::new()
                        .critical()
                        .key_encipherment()
                        .build()
                        .unwrap(),
                )
                .unwrap();
        }
        Usage::None => {}
        Usage::ServerAuthOnly | Usage::DocumentSigningOnly => {
            builder
                .append_extension(
                    KeyUsage::new()
                        .critical()
                        .digital_signature()
                        .build()
                        .unwrap(),
                )
                .unwrap();
            let mut usage = ExtendedKeyUsage::new();
            if matches!(spec.usage, Usage::ServerAuthOnly) {
                usage.server_auth();
            } else {
                usage.other("1.3.6.1.5.5.7.3.36");
            }
            builder.append_extension(usage.build().unwrap()).unwrap();
        }
    }
    let subject_key_id = SubjectKeyIdentifier::new()
        .build(&builder.x509v3_context(issuer.map(|issuer| issuer.cert.as_ref()), None))
        .unwrap();
    builder.append_extension(subject_key_id).unwrap();
    if let Some(issuer) = issuer {
        let authority_key_id = AuthorityKeyIdentifier::new()
            .keyid(true)
            .build(&builder.x509v3_context(Some(&issuer.cert), None))
            .unwrap();
        builder.append_extension(authority_key_id).unwrap();
    }
    if let Some(url) = spec.crl_url {
        #[allow(deprecated)]
        let points = X509Extension::new_nid(
            None,
            Some(&builder.x509v3_context(issuer.map(|issuer| issuer.cert.as_ref()), None)),
            Nid::CRL_DISTRIBUTION_POINTS,
            &format!("URI:{url}"),
        )
        .unwrap();
        builder.append_extension(points).unwrap();
    }
    let signing_key = issuer.map_or(key, |issuer| &issuer.key);
    builder.sign(signing_key, digest).unwrap();
    builder.build()
}

pub fn issued(spec: &Spec<'_>, key: PKey<Private>, issuer: Option<&Issued>) -> Issued {
    Issued {
        cert: issue(spec, &key, issuer),
        key,
    }
}

// Revocation lists, DER by hand (RFC 5280 §5.1).

fn der_length(length: usize) -> Vec<u8> {
    if length < 0x80 {
        vec![length as u8]
    } else {
        let bytes: Vec<u8> = length
            .to_be_bytes()
            .into_iter()
            .skip_while(|byte| *byte == 0)
            .collect();
        let mut out = vec![0x80 | bytes.len() as u8];
        out.extend(bytes);
        out
    }
}

fn tlv(tag: u8, content: &[u8]) -> Vec<u8> {
    let mut out = vec![tag];
    out.extend(der_length(content.len()));
    out.extend(content);
    out
}

fn der_integer(value: u32) -> Vec<u8> {
    let mut bytes: Vec<u8> = value
        .to_be_bytes()
        .into_iter()
        .skip_while(|byte| *byte == 0)
        .collect();
    if bytes.is_empty() || bytes[0] & 0x80 != 0 {
        bytes.insert(0, 0);
    }
    tlv(0x02, &bytes)
}

fn der_time(unix: i64) -> Vec<u8> {
    let time = chrono::DateTime::from_timestamp(unix, 0).unwrap();
    tlv(0x17, time.format("%y%m%d%H%M%SZ").to_string().as_bytes())
}

fn signature_algorithm(key: &PKey<Private>) -> Vec<u8> {
    match key.id() {
        // sha256WithRSAEncryption, NULL parameters.
        openssl::pkey::Id::RSA => tlv(
            0x30,
            &[
                tlv(
                    0x06,
                    &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b],
                ),
                vec![0x05, 0x00],
            ]
            .concat(),
        ),
        // ecdsa-with-SHA256.
        _ => tlv(
            0x30,
            &tlv(0x06, &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x02]),
        ),
    }
}

/// A CRL extension: its OID's DER content, criticality and value.
pub fn crl_extension(oid: &[u8], critical: bool, value: &[u8]) -> Vec<u8> {
    let mut content = tlv(0x06, oid);
    if critical {
        content.extend([0x01, 0x01, 0xff]);
    }
    content.extend(tlv(0x04, value));
    tlv(0x30, &content)
}

/// cRLNumber (2.5.29.20).
pub fn crl_number(number: u32) -> Vec<u8> {
    crl_extension(&[0x55, 0x1d, 0x14], false, &der_integer(number))
}

/// deltaCRLIndicator (2.5.29.27), critical.
pub fn delta_indicator(base: u32) -> Vec<u8> {
    crl_extension(&[0x55, 0x1d, 0x1b], true, &der_integer(base))
}

/// What an issuingDistributionPoint (2.5.29.28) says.
#[derive(Default)]
pub struct Idp<'a> {
    pub uri: Option<&'a str>,
    pub only_user_certs: bool,
    pub only_ca_certs: bool,
    pub only_some_reasons: bool,
    pub indirect: bool,
}

/// An issuingDistributionPoint extension, critical.
pub fn issuing_distribution_point(idp: &Idp<'_>) -> Vec<u8> {
    let mut value = vec![];
    if let Some(uri) = idp.uri {
        // distributionPoint [0] { fullName [0] { uniformResourceIdentifier [6] } }
        value.extend(tlv(0xa0, &tlv(0xa0, &tlv(0x86, uri.as_bytes()))));
    }
    if idp.only_user_certs {
        value.extend([0x81, 0x01, 0xff]);
    }
    if idp.only_ca_certs {
        value.extend([0x82, 0x01, 0xff]);
    }
    if idp.only_some_reasons {
        // keyCompromise only.
        value.extend([0x83, 0x02, 0x06, 0x40]);
    }
    if idp.indirect {
        value.extend([0x84, 0x01, 0xff]);
    }
    crl_extension(&[0x55, 0x1d, 0x1c], true, &tlv(0x30, &value))
}

/// A critical extension nobody knows (1.2.3.4).
pub fn unknown_critical_extension() -> Vec<u8> {
    crl_extension(&[0x2a, 0x03, 0x04], true, &[0x05, 0x00])
}

/// A v2 CRL by `issuer` listing `revoked` serials, signed with `signing_key`
/// (the issuer's own key for a valid list).
pub fn crl_signed_by(
    issuer: &Issued,
    signing_key: &PKey<Private>,
    revoked: &[u32],
    this_update: i64,
    next_update: Option<i64>,
) -> Vec<u8> {
    crl_with(issuer, signing_key, revoked, this_update, next_update, &[])
}

/// [`crl_signed_by`] with crlExtensions.
pub fn crl_with(
    issuer: &Issued,
    signing_key: &PKey<Private>,
    revoked: &[u32],
    this_update: i64,
    next_update: Option<i64>,
    extensions: &[Vec<u8>],
) -> Vec<u8> {
    let algorithm = signature_algorithm(signing_key);
    let mut tbs = vec![];
    tbs.extend(der_integer(1));
    tbs.extend(&algorithm);
    tbs.extend(issuer.cert.subject_name().to_der().unwrap());
    tbs.extend(der_time(this_update));
    if let Some(next_update) = next_update {
        tbs.extend(der_time(next_update));
    }
    if !revoked.is_empty() {
        let entries: Vec<u8> = revoked
            .iter()
            .flat_map(|serial| {
                tlv(
                    0x30,
                    &[der_integer(*serial), der_time(this_update)].concat(),
                )
            })
            .collect();
        tbs.extend(tlv(0x30, &entries));
    }
    if !extensions.is_empty() {
        tbs.extend(tlv(0xa0, &tlv(0x30, &extensions.concat())));
    }
    let tbs = tlv(0x30, &tbs);
    let mut signer = Signer::new(MessageDigest::sha256(), signing_key).unwrap();
    signer.update(&tbs).unwrap();
    let signature = signer.sign_to_vec().unwrap();
    let mut bits = vec![0];
    bits.extend(signature);
    tlv(0x30, &[tbs, algorithm, tlv(0x03, &bits)].concat())
}

/// A current CRL by `issuer` listing `revoked`.
pub fn crl(issuer: &Issued, revoked: &[u32]) -> Vec<u8> {
    crl_signed_by(
        issuer,
        &issuer.key,
        revoked,
        pki_now() - DAY,
        Some(pki_now() + 6 * DAY),
    )
}

const INDIVIDUAL: &[(&str, &str)] = &[
    ("C", "PH"),
    ("O", "Test Staff PKI"),
    ("CN", "Test Staff Individual CA"),
];

pub fn person(name: &'static str, id: &'static str) -> Vec<(&'static str, &'static str)> {
    vec![
        ("C", "PH"),
        ("O", "Test Staff PKI"),
        ("OU", "Individual"),
        ("CN", name),
        ("serialNumber", id),
    ]
}

/// The whole PKI, generated once per test binary.
pub struct Pki {
    pub root: Issued,
    pub individual_ca: Issued,
    pub foreign_root: Issued,
    pub foreign_ca: Issued,
    /// RSA, valid, digitalSignature + nonRepudiation.
    pub maria: Issued,
    /// Maria's key again, a new serial: same key, other certificate.
    pub maria_reissue: Issued,
    /// EC P-256.
    pub jose: Issued,
    pub ana: Issued,
    pub expired: Issued,
    pub not_yet_valid: Issued,
    pub no_signing_usage: Issued,
    pub no_key_usage: Issued,
    /// Revoked in [`Pki::individual_crl`].
    pub revoked: Issued,
    /// Two certificates of one holder (same subject), with different keys.
    pub juan_a: Issued,
    pub juan_b: Issued,
    /// Issued by the foreign CA.
    pub foreign_signer: Issued,
    /// An EC P-384 key, which the server doesn't verify.
    pub p384: Issued,
}

pub const REVOKED_SERIAL: u32 = 1007;

impl Pki {
    pub fn get() -> &'static Pki {
        static PKI: OnceLock<Pki> = OnceLock::new();
        PKI.get_or_init(Pki::generate)
    }

    fn generate() -> Pki {
        let root = issued(
            &Spec {
                subject: &[
                    ("C", "PH"),
                    ("O", "Test Staff PKI"),
                    ("CN", "Test Staff Root CA"),
                ],
                serial: 1,
                not_before: pki_now() - 5 * YEAR,
                not_after: pki_now() + 10 * YEAR,
                usage: Usage::Ca,
                crl_url: None,
            },
            rsa_key(),
            None,
        );
        let individual_ca = issued(
            &Spec {
                subject: INDIVIDUAL,
                serial: 2,
                not_before: pki_now() - 4 * YEAR,
                not_after: pki_now() + 8 * YEAR,
                usage: Usage::Ca,
                crl_url: Some(ROOT_CRL_URL),
            },
            rsa_key(),
            Some(&root),
        );
        let foreign_root = issued(
            &Spec {
                subject: &[
                    ("C", "US"),
                    ("O", "Example"),
                    ("CN", "Example Commercial Root CA"),
                ],
                serial: 1,
                not_before: pki_now() - 5 * YEAR,
                not_after: pki_now() + 10 * YEAR,
                usage: Usage::Ca,
                crl_url: None,
            },
            ec_key(),
            None,
        );
        let foreign_ca = issued(
            &Spec {
                subject: &[
                    ("C", "US"),
                    ("O", "Example"),
                    ("CN", "Example Commercial CA"),
                ],
                serial: 2,
                not_before: pki_now() - 4 * YEAR,
                not_after: pki_now() + 8 * YEAR,
                usage: Usage::Ca,
                crl_url: None,
            },
            ec_key(),
            Some(&foreign_root),
        );
        let ca = Some(&individual_ca);
        let maria_key = rsa_key();
        let maria_subject = person("Maria Santos", "PH-0001");
        let maria = issued(&Spec::signer(&maria_subject, 1001), maria_key.clone(), ca);
        let maria_reissue = issued(&Spec::signer(&maria_subject, 1008), maria_key, ca);
        let jose = issued(
            &Spec::signer(&person("Jose Reyes", "PH-0002"), 1002),
            ec_key(),
            ca,
        );
        let ana = issued(
            &Spec::signer(&person("Ana Cruz", "PH-0003"), 1003),
            rsa_key(),
            ca,
        );
        let expired = issued(
            &Spec {
                not_before: pki_now() - 3 * YEAR,
                not_after: pki_now() - YEAR,
                ..Spec::signer(&person("Ramon Garcia", "PH-0004"), 1004)
            },
            rsa_key(),
            ca,
        );
        let not_yet_valid = issued(
            &Spec {
                not_before: pki_now() + YEAR,
                not_after: pki_now() + 3 * YEAR,
                ..Spec::signer(&person("Liza Aquino", "PH-0005"), 1005)
            },
            ec_key(),
            ca,
        );
        let no_signing_usage = issued(
            &Spec {
                usage: Usage::EncipherOnly,
                ..Spec::signer(&person("Pedro Bautista", "PH-0006"), 1006)
            },
            rsa_key(),
            ca,
        );
        let no_key_usage = issued(
            &Spec {
                usage: Usage::None,
                ..Spec::signer(&person("Nora Lim", "PH-0010"), 1010)
            },
            rsa_key(),
            ca,
        );
        let revoked = issued(
            &Spec::signer(&person("Carmen Villanueva", "PH-0007"), REVOKED_SERIAL),
            rsa_key(),
            ca,
        );
        let juan_subject = person("Juan Dela Cruz", "PH-0009");
        let juan_a = issued(&Spec::signer(&juan_subject, 1009), rsa_key(), ca);
        let juan_b = issued(&Spec::signer(&juan_subject, 1011), ec_key(), ca);
        let foreign_signer = issued(
            &Spec {
                crl_url: None,
                ..Spec::signer(&person("Rosa Mendoza", "PH-0012"), 3001)
            },
            ec_key(),
            Some(&foreign_ca),
        );
        let p384 = issued(
            &Spec::signer(&person("Tomas Ramos", "PH-0013"), 1013),
            p384_key(),
            ca,
        );
        Pki {
            root,
            individual_ca,
            foreign_root,
            foreign_ca,
            maria,
            maria_reissue,
            jose,
            ana,
            expired,
            not_yet_valid,
            no_signing_usage,
            no_key_usage,
            revoked,
            juan_a,
            juan_b,
            foreign_signer,
            p384,
        }
    }

    /// The individual CA's current list: [`REVOKED_SERIAL`] is revoked.
    pub fn individual_crl(&self) -> Vec<u8> {
        crl(&self.individual_ca, &[REVOKED_SERIAL])
    }

    /// The root's current list, empty.
    pub fn root_crl(&self) -> Vec<u8> {
        crl(&self.root, &[])
    }
}
