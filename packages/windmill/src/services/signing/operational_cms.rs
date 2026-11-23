// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Server-built PAdES signed attributes for an enrolled P-256 device key.
//! The physical key signs the returned DER SET bytes, not the document
//! digest or the context-specific wrapper. No private key is loaded here.

use cms::cert::{CertificateChoices, IssuerAndSerialNumber};
use cms::content_info::{CmsVersion, ContentInfo};
use cms::signed_data::{
    CertificateSet, EncapsulatedContentInfo, SignedAttributes, SignedData, SignerIdentifier,
    SignerInfo, SignerInfos,
};
use der::asn1::{ObjectIdentifier, OctetString, SetOfVec};
use der::{Any, Decode, Encode, Sequence};
use ring::signature::{UnparsedPublicKey, ECDSA_P256_SHA256_ASN1};
use sha2::{Digest, Sha256};
use spki::AlgorithmIdentifierOwned;
use x509_cert::attr::Attribute;
use x509_cert::Certificate;

const MAX_CMS_BYTES: usize = 16 * 1024;
const MAX_CHAIN_LENGTH: usize = 8;
const ID_DATA: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.7.1");
const ID_SIGNED_DATA: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.7.2");
const ID_CONTENT_TYPE: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.9.3");
const ID_MESSAGE_DIGEST: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.9.4");
const ID_SIGNING_CERT_V2: ObjectIdentifier =
    ObjectIdentifier::new_unwrap("1.2.840.113549.1.9.16.2.47");
const ID_SHA256: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.16.840.1.101.3.4.2.1");
const ID_EC_KEY: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.10045.2.1");
const ID_P256: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.10045.3.1.7");
const ID_ECDSA_SHA256: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.10045.4.3.2");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationalCmsError {
    InvalidCertificate,
    UnenrolledKey,
    UnverifiedSignature,
    InvalidDer,
    TooLarge,
}

impl std::fmt::Display for OperationalCmsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidCertificate => "invalid P-256 signing certificate",
            Self::UnenrolledKey => "certificate does not match the enrolled key",
            Self::UnverifiedSignature => "device signature did not verify",
            Self::InvalidDer => "could not encode canonical signed attributes",
            Self::TooLarge => "the CMS would not fit the PDF signature field",
        })
    }
}
impl std::error::Error for OperationalCmsError {}

impl From<der::Error> for OperationalCmsError {
    fn from(_: der::Error) -> Self {
        Self::InvalidDer
    }
}

// SHA-256 DEFAULT is omitted; issuerSerial is OPTIONAL. SignerInfo has the
// exact issuer-and-serial and the ESS hash independently pins this cert DER.
#[derive(Sequence)]
struct EssCertIdV2 {
    certificate_hash: OctetString,
}

#[derive(Sequence)]
struct SigningCertificateV2 {
    certs: Vec<EssCertIdV2>,
}

fn attribute(oid: ObjectIdentifier, value: Any) -> der::Result<Attribute> {
    Ok(Attribute {
        oid,
        values: SetOfVec::try_from(vec![value])?,
    })
}

/// Derived only from current server-owned cert enrollment and frozen
/// prepared-PDF ByteRange digest. The caller verifies chain/revocation and
/// matches the prepared revision under its request and parent lease.
pub struct OperationalCms {
    attributes: SignedAttributes,
    exact_message: Vec<u8>,
    sid: IssuerAndSerialNumber,
    certificates: CertificateSet,
    public_key: Vec<u8>,
}

impl OperationalCms {
    pub fn prepare(
        chain_der: &[Vec<u8>],
        enrolled_fingerprint_sha256: [u8; 32],
        enrolled_spki_sha256: [u8; 32],
        pdf_byte_range_sha256: [u8; 32],
    ) -> Result<Self, OperationalCmsError> {
        let mut total = 0usize;
        let mut certificates = Vec::new();
        if chain_der.is_empty() || chain_der.len() > MAX_CHAIN_LENGTH {
            return Err(OperationalCmsError::InvalidCertificate);
        }
        for cert_der in chain_der {
            total = total.saturating_add(cert_der.len());
            if total >= MAX_CMS_BYTES {
                return Err(OperationalCmsError::TooLarge);
            }
            let cert = Certificate::from_der(cert_der)
                .map_err(|_| OperationalCmsError::InvalidCertificate)?;
            if cert.to_der()?.as_slice() != cert_der {
                return Err(OperationalCmsError::InvalidCertificate);
            }
            certificates.push(CertificateChoices::Certificate(cert));
        }
        let Some(CertificateChoices::Certificate(leaf)) = certificates.first() else {
            return Err(OperationalCmsError::InvalidCertificate);
        };
        let certificate_sha256: [u8; 32] = Sha256::digest(&chain_der[0]).into();
        let spki = &leaf.tbs_certificate.subject_public_key_info;
        let spki_sha256: [u8; 32] = Sha256::digest(spki.to_der()?).into();
        if certificate_sha256 != enrolled_fingerprint_sha256 || spki_sha256 != enrolled_spki_sha256
        {
            return Err(OperationalCmsError::UnenrolledKey);
        }
        if spki.algorithm.oid != ID_EC_KEY
            || spki
                .algorithm
                .parameters
                .as_ref()
                .and_then(|p| p.decode_as::<ObjectIdentifier>().ok())
                != Some(ID_P256)
        {
            return Err(OperationalCmsError::InvalidCertificate);
        }
        let public_key = spki
            .subject_public_key
            .as_bytes()
            .filter(|key| key.len() == 65 && key[0] == 4)
            .ok_or(OperationalCmsError::InvalidCertificate)?
            .to_vec();
        let sid = IssuerAndSerialNumber {
            issuer: leaf.tbs_certificate.issuer.clone(),
            serial_number: leaf.tbs_certificate.serial_number.clone(),
        };
        let ess = SigningCertificateV2 {
            certs: vec![EssCertIdV2 {
                certificate_hash: OctetString::new(certificate_sha256.to_vec())?,
            }],
        };
        let attributes = SetOfVec::try_from(vec![
            attribute(ID_CONTENT_TYPE, Any::encode_from(&ID_DATA)?)?,
            attribute(
                ID_MESSAGE_DIGEST,
                Any::encode_from(&OctetString::new(pdf_byte_range_sha256.to_vec())?)?,
            )?,
            attribute(ID_SIGNING_CERT_V2, Any::encode_from(&ess)?)?,
        ])?;
        let exact_message = attributes.to_der()?;
        let certificates = CertificateSet(SetOfVec::try_from(certificates)?);
        Ok(Self {
            attributes,
            exact_message,
            sid,
            certificates,
            public_key,
        })
    }

    pub fn exact_message(&self) -> &[u8] {
        &self.exact_message
    }

    /// Verify the device's DER P-256 signature before embedding it. The
    /// final signed PDF still goes through the existing independent CMS,
    /// trust-chain and parent revision verification in `pades::finalize`.
    pub fn finish(self, device_signature_der: &[u8]) -> Result<Vec<u8>, OperationalCmsError> {
        UnparsedPublicKey::new(&ECDSA_P256_SHA256_ASN1, &self.public_key)
            .verify(&self.exact_message, device_signature_der)
            .map_err(|_| OperationalCmsError::UnverifiedSignature)?;
        let digest = AlgorithmIdentifierOwned {
            oid: ID_SHA256,
            parameters: None,
        };
        let signed_data = SignedData {
            version: CmsVersion::V1,
            digest_algorithms: SetOfVec::try_from(vec![digest.clone()])?,
            encap_content_info: EncapsulatedContentInfo {
                econtent_type: ID_DATA,
                econtent: None,
            },
            certificates: Some(self.certificates),
            crls: None,
            signer_infos: SignerInfos(SetOfVec::try_from(vec![SignerInfo {
                version: CmsVersion::V1,
                sid: SignerIdentifier::IssuerAndSerialNumber(self.sid),
                digest_alg: digest,
                signed_attrs: Some(self.attributes),
                signature_algorithm: AlgorithmIdentifierOwned {
                    oid: ID_ECDSA_SHA256,
                    parameters: None,
                },
                signature: OctetString::new(device_signature_der.to_vec())?,
                unsigned_attrs: None,
            }])?),
        };
        let result = ContentInfo {
            content_type: ID_SIGNED_DATA,
            content: Any::encode_from(&signed_data)?,
        }
        .to_der()?;
        if result.len() > MAX_CMS_BYTES {
            return Err(OperationalCmsError::TooLarge);
        }
        Ok(result)
    }
}
