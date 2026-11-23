// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The server-built CMS of an operational PDF signature: the signed
//! attributes the device signs are derived from the enrolled certificate and
//! the prepared revision's digest only; a device signature is embedded only
//! if it verifies over exactly those bytes with the enrolled P-256 key; and
//! every other certificate, key, chain or signature is refused.
//!
//! No hardware here: the "device" is a software P-256 key of the test PKI.

#[path = "support/signing_pki.rs"]
mod signing_pki;

use cms::content_info::ContentInfo;
use cms::signed_data::SignedData;
use der::{Decode, Encode};
use openssl::hash::MessageDigest;
use openssl::x509::X509NameBuilder;
use sha2::{Digest, Sha256};
use signing_pki::{ec_key, issue_named, Issued, Pki, Spec};
use windmill::services::signing::certificates::CertificateIdentity;
use windmill::services::signing::operational_cms::{OperationalCms, OperationalCmsError};

/// The largest CMS a PDF signature field takes.
const MAX_CMS_BYTES: usize = 16 * 1024;

fn digest_of(hex_text: &str) -> [u8; 32] {
    hex::decode(hex_text).unwrap().try_into().unwrap()
}

/// What an enrollment records of `signer`: the SHA-256 of its certificate
/// and of its public key, as the server computes them.
fn enrolled(signer: &Issued) -> ([u8; 32], [u8; 32]) {
    let identity = CertificateIdentity::of(&signer.cert, &[]).unwrap();
    (
        digest_of(&identity.fingerprint_sha256),
        digest_of(&identity.spki_sha256),
    )
}

fn pdf_digest() -> [u8; 32] {
    Sha256::digest(b"the byte range of a prepared PDF revision").into()
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

/// Why `prepare` refuses.
fn refusal(
    chain: &[Vec<u8>],
    fingerprint: [u8; 32],
    spki: [u8; 32],
) -> Option<OperationalCmsError> {
    OperationalCms::prepare(chain, fingerprint, spki, pdf_digest()).err()
}

#[test]
fn a_device_signature_over_the_exact_attributes_becomes_the_cms() {
    let pki = Pki::get();
    let official = &pki.jose;
    let chain = vec![official.der(), pki.individual_ca.der()];
    let (fingerprint, spki) = enrolled(official);
    let digest = pdf_digest();

    let prepared = OperationalCms::prepare(&chain, fingerprint, spki, digest).unwrap();
    let message = prepared.exact_message().to_vec();
    // A DER SET OF attributes, naming the PDF digest and this certificate.
    assert_eq!(message[0], 0x31);
    assert!(contains(&message, &digest));
    assert!(contains(&message, &fingerprint));
    // Nothing of it depends on when or how often it is prepared.
    assert_eq!(
        OperationalCms::prepare(&chain, fingerprint, spki, digest)
            .unwrap()
            .exact_message(),
        message.as_slice()
    );
    // Another digest is another message.
    let other: [u8; 32] = Sha256::digest(b"another revision").into();
    assert_ne!(
        OperationalCms::prepare(&chain, fingerprint, spki, other)
            .unwrap()
            .exact_message(),
        message.as_slice()
    );

    let signature = official.sign(&message);
    let cms = prepared.finish(&signature).unwrap();
    assert!(cms.len() <= MAX_CMS_BYTES);

    let info = ContentInfo::from_der(&cms).unwrap();
    assert_eq!(info.content_type.to_string(), "1.2.840.113549.1.7.2");
    let signed: SignedData = info.content.decode_as().unwrap();
    // Detached: the PDF is not inside.
    assert_eq!(
        signed.encap_content_info.econtent_type.to_string(),
        "1.2.840.113549.1.7.1"
    );
    assert!(signed.encap_content_info.econtent.is_none());
    assert_eq!(signed.certificates.as_ref().unwrap().0.len(), 2);
    assert!(contains(&cms, &official.der()));
    assert!(contains(&cms, &pki.individual_ca.der()));
    assert_eq!(signed.signer_infos.0.len(), 1);
    let signer = signed.signer_infos.0.get(0).unwrap();
    assert_eq!(
        signer.signed_attrs.as_ref().unwrap().to_der().unwrap(),
        message
    );
    assert_eq!(signer.signature.as_bytes(), signature.as_slice());
    assert_eq!(
        signer.signature_algorithm.oid.to_string(),
        "1.2.840.10045.4.3.2"
    );
    assert_eq!(signer.digest_alg.oid.to_string(), "2.16.840.1.101.3.4.2.1");
    assert!(signer.unsigned_attrs.is_none());
}

#[test]
fn a_chain_that_is_not_the_enrolled_p256_certificate_is_refused() {
    let pki = Pki::get();
    let official = &pki.jose;
    let (fingerprint, spki) = enrolled(official);
    let chain = vec![official.der(), pki.individual_ca.der()];

    // No certificate, or more than a chain has.
    assert_eq!(
        refusal(&[], fingerprint, spki),
        Some(OperationalCmsError::InvalidCertificate)
    );
    assert_eq!(
        refusal(&vec![official.der(); 9], fingerprint, spki),
        Some(OperationalCmsError::InvalidCertificate)
    );
    // Bytes that are no certificate, or a certificate and something more.
    assert_eq!(
        refusal(&[b"not a certificate".to_vec()], fingerprint, spki),
        Some(OperationalCmsError::InvalidCertificate)
    );
    let mut trailing = official.der();
    trailing.push(0);
    assert_eq!(
        refusal(&[trailing], fingerprint, spki),
        Some(OperationalCmsError::InvalidCertificate)
    );
    // More than the signature field holds, before anything is parsed.
    assert_eq!(
        refusal(&[vec![0_u8; MAX_CMS_BYTES]], fingerprint, spki),
        Some(OperationalCmsError::TooLarge)
    );
    assert_eq!(
        refusal(
            &[official.der(), vec![0_u8; MAX_CMS_BYTES]],
            fingerprint,
            spki
        ),
        Some(OperationalCmsError::TooLarge)
    );

    // Another certificate than the enrolled one, or another key.
    assert_eq!(
        refusal(&chain, [0_u8; 32], spki),
        Some(OperationalCmsError::UnenrolledKey)
    );
    assert_eq!(
        refusal(&chain, fingerprint, [0_u8; 32]),
        Some(OperationalCmsError::UnenrolledKey)
    );
    let (other_fingerprint, other_spki) = enrolled(&pki.juan_b);
    assert_eq!(
        refusal(&chain, other_fingerprint, other_spki),
        Some(OperationalCmsError::UnenrolledKey)
    );
    // The issuer first: the leaf is what must be enrolled.
    assert_eq!(
        refusal(
            &[pki.individual_ca.der(), official.der()],
            fingerprint,
            spki
        ),
        Some(OperationalCmsError::UnenrolledKey)
    );

    // An enrolled key that is not P-256: RSA, and another curve.
    for other in [&pki.maria, &pki.p384] {
        let (fingerprint, spki) = enrolled(other);
        assert_eq!(
            refusal(&[other.der(), pki.individual_ca.der()], fingerprint, spki),
            Some(OperationalCmsError::InvalidCertificate)
        );
    }

    // The same certificate twice is no set of certificates.
    assert_eq!(
        refusal(&[official.der(), official.der()], fingerprint, spki),
        Some(OperationalCmsError::InvalidDer)
    );
}

#[test]
fn a_signature_that_does_not_verify_is_not_embedded() {
    let pki = Pki::get();
    let official = &pki.jose;
    let chain = vec![official.der()];
    let (fingerprint, spki) = enrolled(official);
    let prepare = || OperationalCms::prepare(&chain, fingerprint, spki, pdf_digest()).unwrap();
    let message = prepare().exact_message().to_vec();

    // Over the PDF digest instead of the attributes.
    assert_eq!(
        prepare().finish(&official.sign(&pdf_digest())),
        Err(OperationalCmsError::UnverifiedSignature)
    );
    // By another P-256 key, or an RSA key.
    assert_eq!(
        prepare().finish(&pki.juan_b.sign(&message)),
        Err(OperationalCmsError::UnverifiedSignature)
    );
    assert_eq!(
        prepare().finish(&pki.maria.sign(&message)),
        Err(OperationalCmsError::UnverifiedSignature)
    );
    // Not a signature at all.
    assert_eq!(
        prepare().finish(b""),
        Err(OperationalCmsError::UnverifiedSignature)
    );
    let mut tampered = official.sign(&message);
    let last = tampered.len() - 1;
    tampered[last] ^= 1;
    assert_eq!(
        prepare().finish(&tampered),
        Err(OperationalCmsError::UnverifiedSignature)
    );
    // The right one still goes in.
    assert!(prepare().finish(&official.sign(&message)).is_ok());
}

/// A P-256 certificate of the test CA whose subject has `units` more
/// organizational units of 64 characters (75 bytes of DER each).
fn padded_official(issuer: &Issued, units: usize) -> Issued {
    let key = ec_key();
    let mut name = X509NameBuilder::new().unwrap();
    name.append_entry_by_text("CN", "Padded Official").unwrap();
    for unit in 0..units {
        name.append_entry_by_text("OU", &format!("{unit:064}"))
            .unwrap();
    }
    let cert = issue_named(
        &Spec::signer(&[], 2001),
        name.build(),
        &key,
        Some(issuer),
        MessageDigest::sha256(),
    );
    Issued { cert, key }
}

#[test]
fn a_cms_that_would_not_fit_the_signature_field_is_refused() {
    let pki = Pki::get();
    // A certificate just under the limit: it is taken, and the CMS around
    // it (the attributes, the signer and the signature) is over the limit.
    let window = 16_150..16_380;
    let mut units = 208_usize;
    let mut tries = 0;
    let official = loop {
        let candidate = padded_official(&pki.individual_ca, units);
        let length = candidate.der().len();
        if window.contains(&length) {
            break candidate;
        }
        tries += 1;
        assert!(tries < 8, "no certificate of the wanted size: {length}");
        units = if length < window.start {
            units + (16_260 - length) / 75 + 1
        } else {
            units - ((length - 16_260) / 75 + 1)
        };
    };
    let (fingerprint, spki) = enrolled(&official);
    let prepared =
        OperationalCms::prepare(&[official.der()], fingerprint, spki, pdf_digest()).unwrap();
    let signature = official.sign(prepared.exact_message());
    assert_eq!(
        prepared.finish(&signature),
        Err(OperationalCmsError::TooLarge)
    );
}

#[test]
fn each_refusal_says_what_it_is() {
    for (error, text) in [
        (
            OperationalCmsError::InvalidCertificate,
            "invalid P-256 signing certificate",
        ),
        (
            OperationalCmsError::UnenrolledKey,
            "certificate does not match the enrolled key",
        ),
        (
            OperationalCmsError::UnverifiedSignature,
            "device signature did not verify",
        ),
        (
            OperationalCmsError::InvalidDer,
            "could not encode canonical signed attributes",
        ),
        (
            OperationalCmsError::TooLarge,
            "the CMS would not fit the PDF signature field",
        ),
    ] {
        assert_eq!(error.to_string(), text);
        let boxed: Box<dyn std::error::Error> = Box::new(error);
        assert_eq!(boxed.to_string(), text);
    }
}
