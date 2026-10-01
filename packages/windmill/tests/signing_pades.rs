// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Multi-signer PAdES revisions (`services::signing::pades`).
//!
//! The PKI, the input PDFs and the detached CMS are generated here. The CMS is
//! built by hand in DER, like a browser (PKIjs) builds it: signed attributes
//! contentType, messageDigest and signingCertificateV2, signingTime optional,
//! RSA signature algorithm parameters absent or NULL.

use chrono::{DateTime, TimeZone, Utc};
use lopdf::content::{Content, Operation};
use lopdf::encryption::{EncryptionState, EncryptionVersion, Permissions};
use lopdf::{dictionary, Document, Object, ObjectId, Stream};
use openssl::asn1::{Asn1Integer, Asn1Time};
use openssl::bn::BigNum;
use openssl::ec::{EcGroup, EcKey};
use openssl::hash::MessageDigest;
use openssl::nid::Nid;
use openssl::pkey::{PKey, Private};
use openssl::rsa::Rsa;
use openssl::sign::Signer;
use openssl::x509::extension::{
    AuthorityKeyIdentifier, BasicConstraints, ExtendedKeyUsage, KeyUsage, SubjectKeyIdentifier,
};
use openssl::x509::{X509Builder, X509NameBuilder, X509};
use sha2::{Digest, Sha256};
use windmill::services::signing::pades::{
    append_signature_page, ensure_extends, finalize, prepare_revision, rebuild_revision,
    signature_field_name, signature_fields, verify_cms, CmsTrust, PadesError, PreparedRevision,
    SignatureAppearance, SignatureFieldSpec, SignaturePageTexts, CMS_PLACEHOLDER_BYTES,
};

// ---------------------------------------------------------------- DER helpers

fn der_length(len: usize) -> Vec<u8> {
    if len < 0x80 {
        return vec![len as u8];
    }
    let bytes: Vec<u8> = len
        .to_be_bytes()
        .into_iter()
        .skip_while(|b| *b == 0)
        .collect();
    let mut out = vec![0x80 | bytes.len() as u8];
    out.extend(bytes);
    out
}

fn tlv(tag: u8, parts: &[&[u8]]) -> Vec<u8> {
    let content: Vec<u8> = parts.concat();
    let mut out = vec![tag];
    out.extend(der_length(content.len()));
    out.extend(content);
    out
}

fn seq(parts: &[&[u8]]) -> Vec<u8> {
    tlv(0x30, parts)
}

fn set(parts: &[&[u8]]) -> Vec<u8> {
    tlv(0x31, parts)
}

fn octets(bytes: &[u8]) -> Vec<u8> {
    tlv(0x04, &[bytes])
}

fn oid(dotted: &str) -> Vec<u8> {
    let arcs: Vec<u64> = dotted.split('.').map(|a| a.parse().unwrap()).collect();
    let mut body = vec![(arcs[0] * 40 + arcs[1]) as u8];
    for &arc in &arcs[2..] {
        let mut chunk = vec![(arc & 0x7f) as u8];
        let mut rest = arc >> 7;
        while rest > 0 {
            chunk.push(0x80 | (rest & 0x7f) as u8);
            rest >>= 7;
        }
        chunk.reverse();
        body.extend(chunk);
    }
    tlv(0x06, &[&body])
}

fn unsigned_integer(be: &[u8]) -> Vec<u8> {
    let mut body: Vec<u8> = be.iter().copied().skip_while(|b| *b == 0).collect();
    if body.is_empty() || body[0] & 0x80 != 0 {
        body.insert(0, 0);
    }
    tlv(0x02, &[&body])
}

/// Length of the DER object at the start of `bytes` (the /Contents hex keeps
/// its zero padding after the CMS).
fn der_object_len(bytes: &[u8]) -> usize {
    let first = bytes[1] as usize;
    if first < 0x80 {
        return 2 + first;
    }
    let n = first & 0x7f;
    let len = bytes[2..2 + n]
        .iter()
        .fold(0usize, |acc, b| (acc << 8) | *b as usize);
    2 + n + len
}

const OID_SIGNED_DATA: &str = "1.2.840.113549.1.7.2";
const OID_DATA: &str = "1.2.840.113549.1.7.1";
const OID_SHA256: &str = "2.16.840.1.101.3.4.2.1";
const OID_CONTENT_TYPE: &str = "1.2.840.113549.1.9.3";
const OID_MESSAGE_DIGEST: &str = "1.2.840.113549.1.9.4";
const OID_SIGNING_TIME: &str = "1.2.840.113549.1.9.5";
const OID_SIGNING_CERTIFICATE_V2: &str = "1.2.840.113549.1.9.16.2.47";
const OID_SHA256_WITH_RSA: &str = "1.2.840.113549.1.1.11";
const OID_ECDSA_WITH_SHA256: &str = "1.2.840.10045.4.3.2";

const OID_SHA1: &str = "1.3.14.3.2.26";
const OID_SHA1_WITH_RSA: &str = "1.2.840.113549.1.1.5";
const OID_SIGNING_CERTIFICATE_V1: &str = "1.2.840.113549.1.9.16.2.12";

#[derive(Clone, Copy)]
enum RsaParams {
    /// PKIjs leaves the AlgorithmIdentifier parameters out.
    Absent,
    Null,
}

#[derive(Clone, Copy, PartialEq)]
enum DigestKind {
    Sha256,
    Sha1,
}

#[derive(Clone, Copy, PartialEq)]
enum EssKind {
    V2,
    /// The legacy SigningCertificate (SHA-1 cert hash) only.
    V1Only,
}

struct CmsOptions<'a> {
    signing_time: bool,
    rsa_params: RsaParams,
    digest: DigestKind,
    /// Certificate named by the ESS attribute; `None` = the signer.
    ess_certificate: Option<&'a X509>,
    ess: EssKind,
    /// Certificates embedded in SignedData.certificates.
    embedded: Vec<&'a X509>,
    /// Carry the content inside the CMS (not detached).
    attached: bool,
    signer_infos: usize,
    /// messageDigest over this content instead of the signed content.
    message_digest_of: Option<&'a [u8]>,
}

impl<'a> CmsOptions<'a> {
    fn browser(signer: &'a X509) -> Self {
        CmsOptions {
            signing_time: false,
            rsa_params: RsaParams::Absent,
            digest: DigestKind::Sha256,
            ess_certificate: None,
            ess: EssKind::V2,
            embedded: vec![signer],
            attached: false,
            signer_infos: 1,
            message_digest_of: None,
        }
    }
}

/// CAdES SignedData over `content`, detached unless `opts.attached`.
fn build_cms(content: &[u8], signer: &Pki, opts: &CmsOptions) -> Vec<u8> {
    let (digest_oid, md) = match opts.digest {
        DigestKind::Sha256 => (OID_SHA256, MessageDigest::sha256()),
        DigestKind::Sha1 => (OID_SHA1, MessageDigest::sha1()),
    };
    let digest = openssl::hash::hash(md, opts.message_digest_of.unwrap_or(content)).unwrap();
    let ess_cert = opts.ess_certificate.unwrap_or(&signer.cert);
    let ess_der = ess_cert.to_der().unwrap();

    let ess_attribute = match opts.ess {
        // SigningCertificateV2 { certs SEQUENCE OF ESSCertIDv2 { certHash } }
        // with the default hash algorithm (SHA-256) left out.
        EssKind::V2 => seq(&[
            &oid(OID_SIGNING_CERTIFICATE_V2),
            &set(&[&seq(&[&seq(&[&seq(&[&octets(&Sha256::digest(
                &ess_der,
            ))])])])]),
        ]),
        // SigningCertificate { certs SEQUENCE OF ESSCertID { certHash (SHA-1) } }
        EssKind::V1Only => {
            let sha1 = openssl::hash::hash(MessageDigest::sha1(), &ess_der).unwrap();
            seq(&[
                &oid(OID_SIGNING_CERTIFICATE_V1),
                &set(&[&seq(&[&seq(&[&seq(&[&octets(&sha1)])])])]),
            ])
        }
    };
    let mut attributes = vec![
        seq(&[&oid(OID_CONTENT_TYPE), &set(&[&oid(OID_DATA)])]),
        seq(&[&oid(OID_MESSAGE_DIGEST), &set(&[&octets(&digest)])]),
        ess_attribute,
    ];
    if opts.signing_time {
        attributes.push(seq(&[
            &oid(OID_SIGNING_TIME),
            &set(&[&tlv(0x17, &[b"260930120000Z"])]),
        ]));
    }
    attributes.sort();
    let attributes: Vec<u8> = attributes.concat();
    let to_sign = tlv(0x31, &[&attributes]);

    let mut s = Signer::new(md, &signer.key).unwrap();
    let signature = s.sign_oneshot_to_vec(&to_sign).unwrap();
    let rsa_oid = match opts.digest {
        DigestKind::Sha256 => OID_SHA256_WITH_RSA,
        DigestKind::Sha1 => OID_SHA1_WITH_RSA,
    };
    let signature_algorithm = if signer.key.ec_key().is_ok() {
        seq(&[&oid(OID_ECDSA_WITH_SHA256)])
    } else {
        match opts.rsa_params {
            RsaParams::Absent => seq(&[&oid(rsa_oid)]),
            RsaParams::Null => seq(&[&oid(rsa_oid), &[0x05, 0x00]]),
        }
    };
    let digest_algorithm = seq(&[&oid(digest_oid)]);
    let serial = signer.cert.serial_number().to_bn().unwrap().to_vec();
    let issuer_and_serial = seq(&[
        &signer.cert.issuer_name().to_der().unwrap(),
        &unsigned_integer(&serial),
    ]);
    let signer_info = seq(&[
        &unsigned_integer(&[1]),
        &issuer_and_serial,
        &digest_algorithm,
        &tlv(0xa0, &[&attributes]),
        &signature_algorithm,
        &octets(&signature),
    ]);
    let certificates: Vec<u8> = opts
        .embedded
        .iter()
        .map(|c| c.to_der().unwrap())
        .collect::<Vec<_>>()
        .concat();
    let encap_content = if opts.attached {
        seq(&[&oid(OID_DATA), &tlv(0xa0, &[&octets(content)])])
    } else {
        seq(&[&oid(OID_DATA)])
    };
    let signer_infos = vec![signer_info; opts.signer_infos].concat();
    let signed_data = seq(&[
        &unsigned_integer(&[1]),
        &set(&[&digest_algorithm]),
        &encap_content,
        &tlv(0xa0, &[&certificates]),
        &set(&[&signer_infos]),
    ]);
    seq(&[&oid(OID_SIGNED_DATA), &tlv(0xa0, &[&signed_data])])
}

// ------------------------------------------------------------------------ PKI

struct Pki {
    cert: X509,
    key: PKey<Private>,
}

enum KeyKind {
    Rsa,
    Ec,
}

enum Role {
    Ca,
    Signer,
    ClientAuthOnlySigner,
}

fn new_key(kind: KeyKind) -> PKey<Private> {
    match kind {
        KeyKind::Rsa => PKey::from_rsa(Rsa::generate(2048).unwrap()).unwrap(),
        KeyKind::Ec => {
            let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap();
            PKey::from_ec_key(EcKey::generate(&group).unwrap()).unwrap()
        }
    }
}

fn issue(cn: &str, kind: KeyKind, role: Role, issuer: Option<&Pki>, serial: u32) -> Pki {
    let key = new_key(kind);
    let mut name = X509NameBuilder::new().unwrap();
    name.append_entry_by_text("CN", cn).unwrap();
    let name = name.build();
    let mut b = X509Builder::new().unwrap();
    b.set_version(2).unwrap();
    let serial = Asn1Integer::from_bn(&BigNum::from_u32(serial).unwrap()).unwrap();
    b.set_serial_number(&serial).unwrap();
    b.set_subject_name(&name).unwrap();
    b.set_issuer_name(issuer.map_or(&*name, |i| i.cert.subject_name()))
        .unwrap();
    b.set_pubkey(&key).unwrap();
    let now = Utc::now().timestamp();
    b.set_not_before(&Asn1Time::from_unix(now - 3600).unwrap())
        .unwrap();
    b.set_not_after(&Asn1Time::days_from_now(365).unwrap())
        .unwrap();
    match role {
        Role::Ca => {
            b.append_extension(BasicConstraints::new().critical().ca().build().unwrap())
                .unwrap();
            b.append_extension(
                KeyUsage::new()
                    .critical()
                    .key_cert_sign()
                    .crl_sign()
                    .build()
                    .unwrap(),
            )
            .unwrap();
        }
        Role::Signer | Role::ClientAuthOnlySigner => {
            b.append_extension(
                KeyUsage::new()
                    .critical()
                    .digital_signature()
                    .non_repudiation()
                    .build()
                    .unwrap(),
            )
            .unwrap();
            if let Role::ClientAuthOnlySigner = role {
                // National-ID style: OpenSSL's default S/MIME purpose refuses it.
                b.append_extension(ExtendedKeyUsage::new().client_auth().build().unwrap())
                    .unwrap();
            }
        }
    }
    let ski = SubjectKeyIdentifier::new()
        .build(&b.x509v3_context(issuer.map(|i| &*i.cert), None))
        .unwrap();
    b.append_extension(ski).unwrap();
    if let Some(i) = issuer {
        let aki = AuthorityKeyIdentifier::new()
            .keyid(true)
            .build(&b.x509v3_context(Some(&i.cert), None))
            .unwrap();
        b.append_extension(aki).unwrap();
    }
    let signing_key = issuer.map_or(&key, |i| &i.key);
    b.sign(signing_key, MessageDigest::sha256()).unwrap();
    Pki {
        cert: b.build(),
        key,
    }
}

struct TestPki {
    root: Pki,
    intermediate: Pki,
    signers: Vec<Pki>,
}

fn test_pki() -> TestPki {
    let root = issue("Test Root", KeyKind::Ec, Role::Ca, None, 1);
    let intermediate = issue("Test Staff CA", KeyKind::Ec, Role::Ca, Some(&root), 2);
    let signers = vec![
        issue(
            "Signer RSA",
            KeyKind::Rsa,
            Role::Signer,
            Some(&intermediate),
            10,
        ),
        issue(
            "Signer EC",
            KeyKind::Ec,
            Role::Signer,
            Some(&intermediate),
            11,
        ),
        issue(
            "Signer clientAuth",
            KeyKind::Rsa,
            Role::ClientAuthOnlySigner,
            Some(&intermediate),
            12,
        ),
    ];
    TestPki {
        root,
        intermediate,
        signers,
    }
}

impl TestPki {
    fn chain(&self) -> CmsTrust<'_> {
        CmsTrust::Chain {
            anchors: std::slice::from_ref(&self.root.cert),
            intermediates: std::slice::from_ref(&self.intermediate.cert),
        }
    }
}

// ----------------------------------------------------------------------- PDFs

enum XrefLayout {
    Table,
    ObjectStreams,
}

/// A one-page report, MediaBox inherited from the page tree.
fn report_pdf(layout: XrefLayout) -> Vec<u8> {
    save(report_doc(), layout)
}

fn save(mut doc: Document, layout: XrefLayout) -> Vec<u8> {
    let mut out = Vec::new();
    match layout {
        XrefLayout::Table => doc.save_to(&mut out).unwrap(),
        XrefLayout::ObjectStreams => doc.save_modern(&mut out).unwrap(),
    }
    out
}

fn report_doc() -> Document {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let font_id = doc.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Courier",
    });
    let content = Content {
        operations: vec![
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec!["F1".into(), 24.into()]),
            Operation::new("Td", vec![72.into(), 700.into()]),
            Operation::new("Tj", vec![Object::string_literal("Election return")]),
            Operation::new("ET", vec![]),
        ],
    };
    let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
    let page_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Contents" => content_id,
        "Resources" => dictionary! { "Font" => dictionary! { "F1" => font_id } },
    });
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        }),
    );
    let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog_id);
    doc
}

// ------------------------------------------------------------ configurations

/// Translation-driven texts; two configurations make hardcoded strings fail.
struct Locale {
    title: &'static str,
    certification: &'static str,
    signed_by: &'static str,
    date: &'static str,
    code: &'static str,
    reason: &'static str,
}

const ENGLISH: Locale = Locale {
    title: "Certification",
    certification: "We, the members of the board of {organization}, certify that these election returns are true and correct. Each of us signed them with our digital certificate.",
    signed_by: "Digitally signed by {name}",
    date: "Date: {time}",
    code: "Signing code: {code}",
    reason: "Signing code {code}",
};

const SPANISH: Locale = Locale {
    title: "Certificación",
    certification: "Los miembros de la junta de {organization} certificamos que estas actas son verídicas y correctas.",
    signed_by: "Firmado digitalmente por {name}",
    date: "Fecha: {time}",
    code: "Código de firma: {code}",
    reason: "Código de firma {code}",
};

struct Member {
    name: &'static str,
    role: &'static str,
}

const BOARD: [Member; 3] = [
    Member {
        name: "MARIA SANTOS",
        role: "Chairperson",
    },
    Member {
        name: "JOSÉ PEÑA",
        role: "Poll Clerk",
    },
    Member {
        name: "ANA CRUZ",
        role: "Third Member",
    },
];

fn page_texts(locale: &Locale, organization: &str) -> SignaturePageTexts {
    SignaturePageTexts {
        title: locale.title.to_string(),
        certification: locale.certification.replace("{organization}", organization),
    }
}

fn field_specs(members: &[Member]) -> Vec<SignatureFieldSpec> {
    members
        .iter()
        .map(|m| SignatureFieldSpec {
            label: format!("{} - {}", m.name, m.role),
            tooltip: m.name.to_string(),
        })
        .collect()
}

fn signing_time(i: usize) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 30, 12, i as u32, 5).unwrap()
}

const CODE: &str = "7F3A-91C2";

fn appearance_lines(locale: &Locale, name: &str, time: DateTime<Utc>) -> Vec<String> {
    vec![
        locale.signed_by.replace("{name}", name),
        locale
            .date
            .replace("{time}", &time.format("%Y-%m-%d %H:%M:%S UTC").to_string()),
        locale.code.replace("{code}", CODE),
    ]
}

fn appearance(locale: &Locale, name: &str, time: DateTime<Utc>) -> SignatureAppearance {
    SignatureAppearance {
        signer_name: name.to_string(),
        signing_time: time,
        reason: Some(locale.reason.replace("{code}", CODE)),
        lines: appearance_lines(locale, name, time),
    }
}

/// Latin-1 text as the single-byte WinAnsi codes Helvetica prints.
fn latin1(s: &str) -> Vec<u8> {
    s.chars()
        .map(|c| u8::try_from(u32::from(c)).unwrap())
        .collect()
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

// ------------------------------------------------------- reading signatures

/// A signature as a validator sees it: read with lopdf from the file bytes.
struct EmbeddedSignature {
    field_name: String,
    byte_range: [usize; 4],
    cms: Vec<u8>,
    sig_dict: lopdf::Dictionary,
}

fn fields(doc: &Document) -> Vec<ObjectId> {
    let catalog = doc.catalog().unwrap();
    let acro = catalog.get(b"AcroForm").unwrap();
    let (_, acro) = doc.dereference(acro).unwrap();
    acro.as_dict()
        .unwrap()
        .get(b"Fields")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f.as_reference().unwrap())
        .collect()
}

fn embedded_signatures(pdf: &[u8]) -> Vec<EmbeddedSignature> {
    let doc = Document::load_mem(pdf).unwrap();
    fields(&doc)
        .into_iter()
        .filter_map(|id| {
            let field = doc.get_dictionary(id).unwrap();
            let sig_id = field.get(b"V").ok()?.as_reference().unwrap();
            let sig = doc.get_dictionary(sig_id).unwrap().clone();
            let range: Vec<usize> = sig
                .get(b"ByteRange")
                .unwrap()
                .as_array()
                .unwrap()
                .iter()
                .map(|o| o.as_i64().unwrap() as usize)
                .collect();
            let contents = sig.get(b"Contents").unwrap().as_str().unwrap();
            let cms = contents[..der_object_len(contents)].to_vec();
            Some(EmbeddedSignature {
                field_name: String::from_utf8(field.get(b"T").unwrap().as_str().unwrap().to_vec())
                    .unwrap(),
                byte_range: [range[0], range[1], range[2], range[3]],
                cms,
                sig_dict: sig,
            })
        })
        .collect()
}

fn last_page_id(doc: &Document) -> ObjectId {
    *doc.get_pages().values().last().unwrap()
}

fn covered(pdf: &[u8], r: [usize; 4]) -> Vec<u8> {
    [&pdf[r[0]..r[0] + r[1]], &pdf[r[2]..r[2] + r[3]]].concat()
}

fn appearance_stream(pdf: &[u8], field_index: usize) -> Vec<u8> {
    let doc = Document::load_mem(pdf).unwrap();
    let widget = doc
        .get_dictionary(fields(&doc)[field_index])
        .unwrap()
        .clone();
    let ap = widget.get(b"AP").unwrap().as_dict().unwrap();
    let n = ap.get(b"N").unwrap().as_reference().unwrap();
    doc.get_object(n)
        .unwrap()
        .as_stream()
        .unwrap()
        .content
        .clone()
}

/// Signs field `index` of `latest` and returns (prepared, signed bytes).
fn sign(
    latest: &[u8],
    index: usize,
    pki: &TestPki,
    locale: &Locale,
    opts: &CmsOptions,
) -> (PreparedRevision, Vec<u8>) {
    let signer = &pki.signers[index];
    let prepared = prepare_revision(
        latest,
        index,
        &appearance(locale, BOARD[index].name, signing_time(index)),
    )
    .unwrap();
    let cms = build_cms(&prepared.signed_content(), signer, opts);
    let signed = finalize(&prepared, latest, &cms, &signer.cert, &pki.chain()).unwrap();
    (prepared, signed)
}

fn base(layout: XrefLayout, locale: &Locale) -> Vec<u8> {
    append_signature_page(
        &report_pdf(layout),
        &field_specs(&BOARD),
        &page_texts(locale, "Example Board"),
    )
    .unwrap()
}

// ---------------------------------------------------------------------- tests

#[test]
fn base_revision_appends_a_page_with_empty_signature_fields() {
    let input = report_pdf(XrefLayout::Table);
    let base = base(XrefLayout::Table, &ENGLISH);

    let doc = Document::load_mem(&base).unwrap();
    assert_eq!(doc.get_pages().len(), 2);
    let states = signature_fields(&base).unwrap();
    let names: Vec<&str> = states.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["Signature1", "Signature2", "Signature3"]);
    assert!(states.iter().all(|s| !s.signed));
    assert_eq!(signature_field_name(2), "Signature3");

    let acro = doc.catalog().unwrap().get(b"AcroForm").unwrap();
    let (_, acro) = doc.dereference(acro).unwrap();
    let acro = acro.as_dict().unwrap();
    assert_eq!(acro.get(b"SigFlags").unwrap().as_i64().unwrap(), 3);
    assert!(!acro.has(b"NeedAppearances"));
    // Each widget sits on its page: /P names a page whose /Annots lists it.
    for id in fields(&doc) {
        let page = doc
            .get_dictionary(id)
            .unwrap()
            .get(b"P")
            .unwrap()
            .as_reference()
            .unwrap();
        let annots = doc
            .get_dictionary(page)
            .unwrap()
            .get(b"Annots")
            .unwrap()
            .as_array()
            .unwrap();
        assert!(annots.contains(&Object::Reference(id)));
        assert_eq!(page, last_page_id(&doc));
    }
    // A full rewrite with one classic xref table and an /ID.
    assert!(doc.trailer.has(b"ID"));
    assert!(!doc.trailer.has(b"Prev"));
    assert_eq!(base.windows(6).filter(|w| w == b"\nxref\n").count(), 1);
    assert!(!contains(&base, b"/XRef") && !contains(&base, b"/ObjStm"));
    assert_ne!(base, input);

    // The page prints the caller's texts (Latin-1 as WinAnsi).
    let last_page = *doc.get_pages().values().last().unwrap();
    let content = doc.get_page_content(last_page).unwrap();
    assert!(contains(&content, b"(Certification)"));
    assert!(contains(&content, b"Example Board"));
    assert!(contains(&content, &latin1("JOSÉ PEÑA - Poll Clerk")));
}

#[test]
fn page_texts_come_from_the_configuration() {
    let base = base(XrefLayout::Table, &SPANISH);
    let doc = Document::load_mem(&base).unwrap();
    let last_page = *doc.get_pages().values().last().unwrap();
    let content = doc.get_page_content(last_page).unwrap();
    assert!(contains(&content, &latin1("(Certificación)")));
    assert!(!contains(&content, b"Certification"));
}

#[test]
fn three_signers_make_three_revisions_that_all_verify() {
    let pki = test_pki();
    let mut revisions = vec![base(XrefLayout::Table, &ENGLISH)];
    for i in 0..3 {
        // Signer 1: browser encoding (no signingTime, RSA params absent),
        // signer 2: EC with the chain embedded, signer 3: openssl CLI style
        // (signingTime, RSA params NULL) with a clientAuth-only certificate.
        let mut opts = CmsOptions::browser(&pki.signers[i].cert);
        if i == 1 {
            opts.embedded.push(&pki.intermediate.cert);
        }
        if i == 2 {
            opts.signing_time = true;
            opts.rsa_params = RsaParams::Null;
        }
        let (prepared, signed) = sign(revisions.last().unwrap(), i, &pki, &ENGLISH, &opts);
        // Append-only: every revision extends the previous one byte for byte.
        assert!(prepared.bytes().starts_with(revisions.last().unwrap()));
        assert!(signed.starts_with(revisions.last().unwrap()));
        assert_eq!(signed.len(), prepared.bytes().len());
        revisions.push(signed);
    }

    let last = revisions.last().unwrap();
    let signatures = embedded_signatures(last);
    assert_eq!(signatures.len(), 3);
    for (i, sig) in signatures.iter().enumerate() {
        assert_eq!(sig.field_name, signature_field_name(i));
        // Each signature covers exactly its own revision, all but /Contents.
        let [start, a, b, c] = sig.byte_range;
        assert_eq!(start, 0);
        assert_eq!(b + c, revisions[i + 1].len());
        assert_eq!(b - a, 2 * CMS_PLACEHOLDER_BYTES + 2);
        assert_eq!(last[a], b'<');
        assert_eq!(last[b - 1], b'>');
        // Earlier signatures still verify in the final file.
        let content = covered(last, sig.byte_range);
        verify_cms(&sig.cms, &content, &pki.signers[i].cert, &pki.chain()).unwrap();
        verify_cms(
            &sig.cms,
            &content,
            &pki.signers[i].cert,
            &CmsTrust::SignerOnly,
        )
        .unwrap();

        assert_eq!(
            sig.sig_dict.get(b"SubFilter").unwrap().as_name().unwrap(),
            b"ETSI.CAdES.detached"
        );
        assert_eq!(
            sig.sig_dict.get(b"Filter").unwrap().as_name().unwrap(),
            b"Adobe.PPKLite"
        );
        let m = signing_time(i).format("D:%Y%m%d%H%M%S").to_string();
        assert!(sig
            .sig_dict
            .get(b"M")
            .unwrap()
            .as_str()
            .unwrap()
            .starts_with(m.as_bytes()));
    }
    assert!(signature_fields(last).unwrap().iter().all(|s| s.signed));
}

#[test]
fn appearance_prints_the_name_time_and_code_from_the_caller() {
    for locale in [&ENGLISH, &SPANISH] {
        let latest = base(XrefLayout::Table, locale);
        let prepared = prepare_revision(
            &latest,
            1,
            &appearance(locale, BOARD[1].name, signing_time(1)),
        )
        .unwrap();
        let stream = appearance_stream(prepared.bytes(), 1);
        for line in appearance_lines(locale, BOARD[1].name, signing_time(1)) {
            let literal = [b"(".as_slice(), &latin1(&line), b")"].concat();
            assert!(contains(&stream, &literal), "missing {line}");
        }
        assert!(contains(&stream, &latin1(BOARD[1].name)));
        assert!(contains(&stream, b"2026-09-30 12:01:05 UTC"));
        assert!(contains(&stream, CODE.as_bytes()));
    }
}

#[test]
fn tampered_content_fails_verification() {
    let pki = test_pki();
    let latest = base(XrefLayout::Table, &ENGLISH);
    let prepared = prepare_revision(
        &latest,
        0,
        &appearance(&ENGLISH, "MARIA SANTOS", signing_time(0)),
    )
    .unwrap();
    let signer = &pki.signers[0];
    let cms = build_cms(
        &prepared.signed_content(),
        signer,
        &CmsOptions::browser(&signer.cert),
    );
    // Control: the untouched content verifies.
    verify_cms(&cms, &prepared.signed_content(), &signer.cert, &pki.chain()).unwrap();

    let mut content = prepared.signed_content();
    content[10] ^= 1;
    let err = verify_cms(&cms, &content, &signer.cert, &pki.chain()).unwrap_err();
    assert!(matches!(err, PadesError::CmsVerification(_)), "{err}");
}

#[test]
fn a_cms_from_another_certificate_fails() {
    let pki = test_pki();
    let latest = base(XrefLayout::Table, &ENGLISH);
    let prepared = prepare_revision(
        &latest,
        0,
        &appearance(&ENGLISH, "MARIA SANTOS", signing_time(0)),
    )
    .unwrap();
    let content = prepared.signed_content();
    let cms = build_cms(
        &content,
        &pki.signers[1],
        &CmsOptions::browser(&pki.signers[1].cert),
    );
    verify_cms(&cms, &content, &pki.signers[1].cert, &pki.chain()).unwrap();

    // The approval certificate is signer 0; the CMS embeds signer 1's.
    for trust in [pki.chain(), CmsTrust::SignerOnly] {
        let err = verify_cms(&cms, &content, &pki.signers[0].cert, &trust).unwrap_err();
        assert!(matches!(err, PadesError::CmsVerification(_)), "{err}");
    }
}

#[test]
fn signing_certificate_v2_must_name_the_signer() {
    let pki = test_pki();
    let content = b"byte range content".to_vec();
    let signer = &pki.signers[0];
    let mut opts = CmsOptions::browser(&signer.cert);
    opts.ess_certificate = Some(&pki.signers[1].cert);
    let cms = build_cms(&content, signer, &opts);
    for trust in [pki.chain(), CmsTrust::SignerOnly] {
        let err = verify_cms(&cms, &content, &signer.cert, &trust).unwrap_err();
        assert!(matches!(err, PadesError::CmsVerification(_)), "{err}");
    }
    // Control: the same CMS naming the signer verifies.
    let cms = build_cms(&content, signer, &CmsOptions::browser(&signer.cert));
    verify_cms(&cms, &content, &signer.cert, &pki.chain()).unwrap();
}

#[test]
fn a_chain_to_an_untrusted_root_fails() {
    let pki = test_pki();
    let other_root = issue("Other Root", KeyKind::Ec, Role::Ca, None, 99);
    let content = b"byte range content".to_vec();
    let signer = &pki.signers[1];
    let cms = build_cms(&content, signer, &CmsOptions::browser(&signer.cert));
    let trust = CmsTrust::Chain {
        anchors: std::slice::from_ref(&other_root.cert),
        intermediates: std::slice::from_ref(&pki.intermediate.cert),
    };
    let err = verify_cms(&cms, &content, &signer.cert, &trust).unwrap_err();
    assert!(matches!(err, PadesError::CmsVerification(_)), "{err}");
}

#[test]
fn a_prepare_on_an_older_revision_is_stale() {
    let pki = test_pki();
    let opts = |i: usize| CmsOptions::browser(&pki.signers[i].cert);
    let rev0 = base(XrefLayout::Table, &ENGLISH);
    let (_, rev1) = sign(&rev0, 0, &pki, &ENGLISH, &opts(0));
    // Signer 3 prepares on revision 1 ...
    let stale = prepare_revision(
        &rev1,
        2,
        &appearance(&ENGLISH, BOARD[2].name, signing_time(2)),
    )
    .unwrap();
    ensure_extends(&stale, &rev1).unwrap();
    // ... and approves after revision 2 exists.
    let (_, rev2) = sign(&rev1, 1, &pki, &ENGLISH, &opts(1));
    let err = ensure_extends(&stale, &rev2).unwrap_err();
    assert!(matches!(err, PadesError::StaleRevision), "{err}");
    // A fresh prepare on revision 2 extends it.
    let fresh = prepare_revision(
        &rev2,
        2,
        &appearance(&ENGLISH, BOARD[2].name, signing_time(2)),
    )
    .unwrap();
    ensure_extends(&fresh, &rev2).unwrap();
    assert_eq!(
        fresh.parent_sha256(),
        <[u8; 32]>::from(Sha256::digest(&rev2))
    );
    assert_eq!(fresh.parent_len(), rev2.len());
}

#[test]
fn an_object_stream_input_is_normalised_and_signs() {
    let input = report_pdf(XrefLayout::ObjectStreams);
    assert!(contains(&input, b"/ObjStm") && contains(&input, b"/XRef"));
    let pki = test_pki();
    let base = base(XrefLayout::ObjectStreams, &ENGLISH);
    assert!(!contains(&base, b"/ObjStm") && !contains(&base, b"/XRef"));
    let mut latest = base;
    for i in 0..3 {
        let opts = CmsOptions::browser(&pki.signers[i].cert);
        latest = sign(&latest, i, &pki, &ENGLISH, &opts).1;
    }
    let signatures = embedded_signatures(&latest);
    assert_eq!(signatures.len(), 3);
    for (i, sig) in signatures.iter().enumerate() {
        let content = covered(&latest, sig.byte_range);
        verify_cms(&sig.cms, &content, &pki.signers[i].cert, &pki.chain()).unwrap();
    }
}

#[test]
fn a_signed_field_cannot_be_prepared_again() {
    let pki = test_pki();
    let rev0 = base(XrefLayout::Table, &ENGLISH);
    let (_, rev1) = sign(
        &rev0,
        0,
        &pki,
        &ENGLISH,
        &CmsOptions::browser(&pki.signers[0].cert),
    );
    let err = prepare_revision(&rev1, 0, &appearance(&ENGLISH, "X", signing_time(0))).unwrap_err();
    assert!(matches!(err, PadesError::FieldAlreadySigned(_)), "{err}");
    let err = prepare_revision(&rev1, 3, &appearance(&ENGLISH, "X", signing_time(0))).unwrap_err();
    assert!(matches!(err, PadesError::FieldNotFound(_)), "{err}");
}

#[test]
fn many_signers_continue_on_further_pages() {
    let members: Vec<Member> = (0..12)
        .map(|_| Member {
            name: "MEMBER",
            role: "Role",
        })
        .collect();
    let base = append_signature_page(
        &report_pdf(XrefLayout::Table),
        &field_specs(&members),
        &page_texts(&ENGLISH, "Board"),
    )
    .unwrap();
    let doc = Document::load_mem(&base).unwrap();
    assert!(doc.get_pages().len() >= 3);
    assert_eq!(signature_fields(&base).unwrap().len(), 12);
    // Every widget is listed on its page and lies inside its MediaBox.
    for id in fields(&doc) {
        let page = doc
            .get_dictionary(id)
            .unwrap()
            .get(b"P")
            .unwrap()
            .as_reference()
            .unwrap();
        let annots = doc
            .get_dictionary(page)
            .unwrap()
            .get(b"Annots")
            .unwrap()
            .as_array()
            .unwrap();
        assert!(annots.contains(&Object::Reference(id)));
        let rect: Vec<f32> = doc
            .get_dictionary(id)
            .unwrap()
            .get(b"Rect")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(|o| o.as_float().unwrap())
            .collect();
        assert!(rect[1] >= 0.0 && rect[3] <= 842.0, "{rect:?}");
    }
}

#[test]
fn an_empty_field_list_is_refused() {
    let err = append_signature_page(
        &report_pdf(XrefLayout::Table),
        &[],
        &page_texts(&ENGLISH, "Board"),
    )
    .unwrap_err();
    assert!(matches!(err, PadesError::InvalidInput(_)), "{err}");
}

#[test]
fn new_objects_are_numbered_from_the_previous_trailer_size() {
    // A previous xref stream's own number is not in lopdf's max_id; model it
    // by raising the trailer's /Size (same width, so offsets stay valid).
    let mut latest = base(XrefLayout::Table, &ENGLISH);
    let trailer = latest.windows(7).rposition(|w| w == b"trailer").unwrap();
    let start = find(&latest, b"/Size ", trailer).unwrap() + b"/Size ".len();
    let digits = latest[start..]
        .iter()
        .take_while(|b| b.is_ascii_digit())
        .count();
    let size: i64 = String::from_utf8_lossy(&latest[start..start + digits])
        .parse()
        .unwrap();
    let raised = "9".repeat(digits);
    assert!(raised.parse::<i64>().unwrap() > size);
    latest[start..start + digits].copy_from_slice(raised.as_bytes());
    let raised: i64 = raised.parse().unwrap();

    let prepared = prepare_revision(
        &latest,
        0,
        &appearance(&ENGLISH, "MARIA SANTOS", signing_time(0)),
    )
    .unwrap();
    let appended = String::from_utf8_lossy(&prepared.bytes()[latest.len()..]).into_owned();
    let ids: Vec<i64> = appended
        .lines()
        .filter_map(|l| l.strip_suffix(" 0 obj"))
        .filter_map(|id| id.trim().parse().ok())
        .collect();
    // The widget keeps its id; the /Sig and the appearance are new.
    assert_eq!(ids.len(), 3, "{ids:?}");
    assert_eq!(ids.iter().filter(|id| **id < size).count(), 1, "{ids:?}");
    assert!(
        ids.iter().filter(|id| **id >= size).all(|id| *id >= raised),
        "{ids:?}"
    );
    let trailer = Document::load_mem(prepared.bytes()).unwrap().trailer;
    assert!(trailer.get(b"Size").unwrap().as_i64().unwrap() >= raised + 2);
}

fn find(hay: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    hay[from..]
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|p| p + from)
}

#[test]
fn an_oversized_cms_is_refused() {
    let pki = test_pki();
    let latest = base(XrefLayout::Table, &ENGLISH);
    let prepared = prepare_revision(
        &latest,
        0,
        &appearance(&ENGLISH, "MARIA SANTOS", signing_time(0)),
    )
    .unwrap();
    let signer = &pki.signers[0].cert;
    let err = finalize(
        &prepared,
        &latest,
        &vec![0x30; CMS_PLACEHOLDER_BYTES + 1],
        signer,
        &pki.chain(),
    )
    .unwrap_err();
    assert!(matches!(err, PadesError::CmsTooLarge { .. }), "{err}");
    // Control: a placeholder-sized blob passes the size check and then fails
    // as a CMS.
    let err = finalize(
        &prepared,
        &latest,
        &vec![0x30; CMS_PLACEHOLDER_BYTES],
        signer,
        &pki.chain(),
    )
    .unwrap_err();
    assert!(matches!(err, PadesError::CmsVerification(_)), "{err}");
}

#[test]
fn text_outside_winansi_does_not_break_the_page() {
    let pki = test_pki();
    let members = [Member {
        name: "Ελένη 李",
        role: "Chair",
    }];
    let base = append_signature_page(
        &report_pdf(XrefLayout::Table),
        &field_specs(&members),
        &page_texts(&ENGLISH, "Board"),
    )
    .unwrap();
    let prepared =
        prepare_revision(&base, 0, &appearance(&ENGLISH, "Ελένη 李", signing_time(0))).unwrap();
    let signer = &pki.signers[0];
    let cms = build_cms(
        &prepared.signed_content(),
        signer,
        &CmsOptions::browser(&signer.cert),
    );
    let signed = finalize(&prepared, &base, &cms, &signer.cert, &CmsTrust::SignerOnly).unwrap();
    let sig = &embedded_signatures(&signed)[0];
    // /Name is a PDF text string: UTF-16BE with a byte order mark.
    let name = sig.sig_dict.get(b"Name").unwrap().as_str().unwrap();
    let utf16: Vec<u8> = "Ελένη 李"
        .encode_utf16()
        .flat_map(|u| u.to_be_bytes())
        .collect();
    assert_eq!(name, [&[0xfe, 0xff][..], &utf16].concat());
    // The Helvetica appearance shows replacement characters instead.
    assert!(contains(
        &appearance_stream(&signed, 0),
        b"(Digitally signed by ????? ?)"
    ));
}

#[test]
fn a_rebuilt_revision_must_match_the_prepared_digest() {
    let pki = test_pki();
    let latest = base(XrefLayout::Table, &ENGLISH);
    let shown = appearance(&ENGLISH, BOARD[0].name, signing_time(0));
    // Prepare: the caller persists the parent hash, the field, the
    // appearance and the digest handed to the browser.
    let prepared = prepare_revision(&latest, 0, &shown).unwrap();
    let (parent, digest) = (prepared.parent_sha256(), prepared.digest_sha256());
    let signer = &pki.signers[0];
    let cms = build_cms(
        &prepared.signed_content(),
        signer,
        &CmsOptions::browser(&signer.cert),
    );

    // Approve: preparing is deterministic, so the rebuild is byte-identical.
    let rebuilt = rebuild_revision(&latest, parent, 0, &shown, digest).unwrap();
    assert_eq!(rebuilt.bytes(), prepared.bytes());
    finalize(&rebuilt, &latest, &cms, &signer.cert, &pki.chain()).unwrap();

    // A tampered appearance (another name) does not rebuild the same bytes.
    let mut tampered = shown.clone();
    tampered.signer_name = "SOMEONE ELSE".to_string();
    tampered.lines[0] = ENGLISH.signed_by.replace("{name}", "SOMEONE ELSE");
    let err = rebuild_revision(&latest, parent, 0, &tampered, digest).unwrap_err();
    assert!(matches!(err, PadesError::RevisionMismatch), "{err}");

    // A tampered in-file /ByteRange changes what is covered: its digest is
    // not the prepared one, and a CMS over it does not finalize.
    let mut bytes = prepared.bytes().to_vec();
    let [_, a, b, c] = prepared.byte_range();
    let range = format!("0 {a} {b} {c}");
    let at = find(&bytes, range.as_bytes(), latest.len()).unwrap();
    let shifted = format!("0 {} {} {}", a - 1, b, c);
    bytes[at..at + shifted.len()].copy_from_slice(shifted.as_bytes());
    let covered_tampered = covered(&bytes, [0, a - 1, b, c]);
    let tampered_digest: [u8; 32] = Sha256::digest(&covered_tampered).into();
    let err = rebuild_revision(&latest, parent, 0, &shown, tampered_digest).unwrap_err();
    assert!(matches!(err, PadesError::RevisionMismatch), "{err}");
    let cms_over_tampered = build_cms(
        &covered_tampered,
        signer,
        &CmsOptions::browser(&signer.cert),
    );
    let err = finalize(
        &rebuilt,
        &latest,
        &cms_over_tampered,
        &signer.cert,
        &pki.chain(),
    )
    .unwrap_err();
    assert!(matches!(err, PadesError::CmsVerification(_)), "{err}");
}

#[test]
fn a_rebuild_on_a_newer_revision_is_stale() {
    let pki = test_pki();
    let rev0 = base(XrefLayout::Table, &ENGLISH);
    let (_, rev1) = sign(
        &rev0,
        0,
        &pki,
        &ENGLISH,
        &CmsOptions::browser(&pki.signers[0].cert),
    );
    let shown = appearance(&ENGLISH, BOARD[2].name, signing_time(2));
    let prepared = prepare_revision(&rev1, 2, &shown).unwrap();
    let (_, rev2) = sign(
        &rev1,
        1,
        &pki,
        &ENGLISH,
        &CmsOptions::browser(&pki.signers[1].cert),
    );
    let err = rebuild_revision(
        &rev2,
        prepared.parent_sha256(),
        2,
        &shown,
        prepared.digest_sha256(),
    )
    .unwrap_err();
    assert!(matches!(err, PadesError::StaleRevision), "{err}");
    // finalize on the stale revision is refused too.
    let cms = build_cms(
        &prepared.signed_content(),
        &pki.signers[2],
        &CmsOptions::browser(&pki.signers[2].cert),
    );
    let err = finalize(&prepared, &rev2, &cms, &pki.signers[2].cert, &pki.chain()).unwrap_err();
    assert!(matches!(err, PadesError::StaleRevision), "{err}");
}

/// Each CMS outside the PAdES profile, all of them otherwise valid
/// signatures by the approval certificate.
#[test]
fn a_cms_outside_the_pades_profile_is_refused() {
    let pki = test_pki();
    let latest = base(XrefLayout::Table, &ENGLISH);
    let prepared = prepare_revision(
        &latest,
        0,
        &appearance(&ENGLISH, BOARD[0].name, signing_time(0)),
    )
    .unwrap();
    let content = prepared.signed_content();
    let signer = &pki.signers[0];
    let other_content = b"other content".to_vec();
    let cases: Vec<(&str, CmsOptions)> = vec![
        ("SHA-1 digest", {
            let mut o = CmsOptions::browser(&signer.cert);
            o.digest = DigestKind::Sha1;
            o
        }),
        ("attached content", {
            let mut o = CmsOptions::browser(&signer.cert);
            o.attached = true;
            o
        }),
        ("two SignerInfos", {
            let mut o = CmsOptions::browser(&signer.cert);
            o.signer_infos = 2;
            o
        }),
        ("signingCertificate v1 only", {
            let mut o = CmsOptions::browser(&signer.cert);
            o.ess = EssKind::V1Only;
            o
        }),
        ("wrong messageDigest", {
            let mut o = CmsOptions::browser(&signer.cert);
            o.message_digest_of = Some(&other_content);
            o
        }),
    ];
    // Control: the browser profile finalizes.
    let good = build_cms(&content, signer, &CmsOptions::browser(&signer.cert));
    finalize(&prepared, &latest, &good, &signer.cert, &pki.chain()).unwrap();
    for (case, opts) in cases {
        let cms = build_cms(&content, signer, &opts);
        let err = finalize(&prepared, &latest, &cms, &signer.cert, &pki.chain()).unwrap_err();
        assert!(
            matches!(err, PadesError::CmsVerification(_)),
            "{case}: {err}"
        );
    }
}

#[test]
fn an_encrypted_input_is_refused() {
    for user_password in ["", "secret"] {
        let mut doc = report_doc();
        doc.trailer.set(
            "ID",
            vec![
                Object::String(vec![1; 16], lopdf::StringFormat::Hexadecimal),
                Object::String(vec![1; 16], lopdf::StringFormat::Hexadecimal),
            ],
        );
        let state = EncryptionState::try_from(EncryptionVersion::V2 {
            document: &doc,
            owner_password: "owner",
            user_password,
            key_length: 128,
            permissions: Permissions::default(),
        })
        .unwrap();
        doc.encrypt(&state).unwrap();
        let err = append_signature_page(
            &save(doc, XrefLayout::Table),
            &field_specs(&BOARD),
            &page_texts(&ENGLISH, "Board"),
        )
        .unwrap_err();
        assert!(
            matches!(err, PadesError::Encrypted),
            "{user_password}: {err}"
        );
    }
}

#[test]
fn an_already_signed_input_is_refused() {
    let pki = test_pki();
    let rev0 = base(XrefLayout::Table, &ENGLISH);
    let (_, rev1) = sign(
        &rev0,
        0,
        &pki,
        &ENGLISH,
        &CmsOptions::browser(&pki.signers[0].cert),
    );
    let refuse = |pdf: &[u8]| {
        append_signature_page(pdf, &field_specs(&BOARD), &page_texts(&ENGLISH, "Board"))
            .unwrap_err()
    };
    assert!(matches!(refuse(&rev1), PadesError::InvalidInput(_)));
    for key in ["Perms", "DSS"] {
        let mut doc = report_doc();
        doc.catalog_mut().unwrap().set(key, dictionary! {});
        let err = refuse(&save(doc, XrefLayout::Table));
        assert!(matches!(err, PadesError::InvalidInput(_)), "{key}: {err}");
    }
    // Control: an unsigned base (empty signature fields) is not "signed".
    let err = refuse(&rev0);
    assert!(
        matches!(&err, PadesError::InvalidInput(m) if m.contains("already has a field")),
        "{err}"
    );
}

#[test]
fn a_field_name_collision_is_refused() {
    let mut doc = report_doc();
    let field = doc.add_object(dictionary! {
        "FT" => "Tx",
        "T" => Object::string_literal(signature_field_name(1)),
    });
    doc.catalog_mut().unwrap().set(
        "AcroForm",
        dictionary! { "Fields" => vec![Object::Reference(field)] },
    );
    let err = append_signature_page(
        &save(doc, XrefLayout::Table),
        &field_specs(&BOARD),
        &page_texts(&ENGLISH, "Board"),
    )
    .unwrap_err();
    assert!(matches!(err, PadesError::InvalidInput(_)), "{err}");
}

#[test]
fn a_direct_acro_form_with_referenced_fields_is_extended() {
    let pki = test_pki();
    let mut doc = report_doc();
    let comments = doc.add_object(dictionary! {
        "FT" => "Tx",
        "T" => Object::string_literal("Comments"),
    });
    let fields = doc.add_object(Object::Array(vec![Object::Reference(comments)]));
    doc.catalog_mut().unwrap().set(
        "AcroForm",
        dictionary! { "Fields" => fields, "NeedAppearances" => true },
    );
    let base = append_signature_page(
        &save(doc, XrefLayout::Table),
        &field_specs(&BOARD),
        &page_texts(&ENGLISH, "Board"),
    )
    .unwrap();

    let doc = Document::load_mem(&base).unwrap();
    let all = fields_of(&doc);
    assert_eq!(all.len(), 4);
    assert_eq!(all[0], comments);
    let acro = doc.catalog().unwrap().get(b"AcroForm").unwrap();
    let (_, acro) = doc.dereference(acro).unwrap();
    assert!(!acro.as_dict().unwrap().has(b"NeedAppearances"));
    let names: Vec<String> = signature_fields(&base)
        .unwrap()
        .into_iter()
        .map(|f| f.name)
        .collect();
    assert_eq!(names, ["Signature1", "Signature2", "Signature3"]);
    let (_, rev1) = sign(
        &base,
        0,
        &pki,
        &ENGLISH,
        &CmsOptions::browser(&pki.signers[0].cert),
    );
    assert!(signature_fields(&rev1).unwrap()[0].signed);
}

fn fields_of(doc: &Document) -> Vec<ObjectId> {
    let acro = doc.catalog().unwrap().get(b"AcroForm").unwrap();
    let (_, acro) = doc.dereference(acro).unwrap();
    let fields = acro.as_dict().unwrap().get(b"Fields").unwrap();
    let (_, fields) = doc.dereference(fields).unwrap();
    fields
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f.as_reference().unwrap())
        .collect()
}

/// The string operands of the `Tj` operators of a content stream.
fn shown_strings(content: &[u8]) -> Vec<Vec<u8>> {
    Content::decode(content)
        .unwrap()
        .operations
        .into_iter()
        .filter(|op| op.operator == "Tj")
        .map(|op| op.operands[0].as_str().unwrap().to_vec())
        .collect()
}

#[test]
fn delimiters_in_caller_texts_are_escaped() {
    let tricky = "A (B)) \\ C (";
    let members = [Member {
        name: "A (B)) \\ C (",
        role: "R",
    }];
    let base = append_signature_page(
        &report_pdf(XrefLayout::Table),
        &field_specs(&members),
        &SignaturePageTexts {
            title: tricky.to_string(),
            certification: tricky.to_string(),
        },
    )
    .unwrap();
    let doc = Document::load_mem(&base).unwrap();
    let page = doc.get_page_content(last_page_id(&doc)).unwrap();
    let strings = shown_strings(&page);
    assert!(strings.contains(&tricky.as_bytes().to_vec()), "{strings:?}");
    assert!(strings.contains(&format!("{tricky} - R").into_bytes()));

    let mut shown = appearance(&ENGLISH, tricky, signing_time(0));
    shown.lines = vec![tricky.to_string()];
    let prepared = prepare_revision(&base, 0, &shown).unwrap();
    let strings = shown_strings(&appearance_stream(prepared.bytes(), 0));
    assert_eq!(strings, [tricky.as_bytes().to_vec()]);
    let sig = Document::load_mem(prepared.bytes()).unwrap();
    let name = embedded_names(&sig);
    assert_eq!(name, [tricky.as_bytes().to_vec()]);
}

fn embedded_names(doc: &Document) -> Vec<Vec<u8>> {
    fields(doc)
        .into_iter()
        .filter_map(|id| {
            let v = doc.get_dictionary(id).unwrap().get(b"V").ok()?;
            let sig = doc.get_dictionary(v.as_reference().unwrap()).unwrap();
            Some(sig.get(b"Name").unwrap().as_str().unwrap().to_vec())
        })
        .collect()
}

#[test]
fn long_caller_texts_are_clipped_and_fitted() {
    let long = "X".repeat(5000);
    let members = [Member {
        name: "MEMBER",
        role: "Role",
    }];
    let base = append_signature_page(
        &report_pdf(XrefLayout::Table),
        &field_specs(&members),
        &SignaturePageTexts {
            title: long.clone(),
            certification: long.clone(),
        },
    )
    .unwrap();
    let doc = Document::load_mem(&base).unwrap();
    let mut shown_total = 0;
    for page in doc.get_pages().values().skip(1) {
        for s in shown_strings(&doc.get_page_content(*page).unwrap()) {
            shown_total += s.len();
            // The title and certification are clipped with an ellipsis.
            assert!(s.len() < 5000);
        }
    }
    assert!(shown_total < 5000, "{shown_total}");

    // Many appearance lines: the ones that don't fit end in an ellipsis
    // line, and every line stays inside the box.
    let mut shown = appearance(&ENGLISH, &long, signing_time(0));
    shown.lines = (0..40).map(|i| format!("line {i}")).collect();
    let prepared = prepare_revision(&base, 0, &shown).unwrap();
    let stream = appearance_stream(prepared.bytes(), 0);
    let strings = shown_strings(&stream);
    assert!(strings.len() < 40);
    assert_eq!(strings.last().unwrap(), &vec![0x85]); // WinAnsi ellipsis
    for op in Content::decode(&stream).unwrap().operations {
        if op.operator == "Td" {
            assert!(op.operands[1].as_float().unwrap() >= 0.0);
        }
    }
    // /Name is clipped too.
    let sig = Document::load_mem(prepared.bytes()).unwrap();
    let name = &embedded_names(&sig)[0];
    assert!(name.len() < 1000);
}

#[test]
fn preparing_is_deterministic() {
    let latest = base(XrefLayout::Table, &ENGLISH);
    let shown = appearance(&ENGLISH, BOARD[1].name, signing_time(1));
    let one = prepare_revision(&latest, 1, &shown).unwrap();
    let two = prepare_revision(&latest, 1, &shown).unwrap();
    assert_eq!(one.bytes(), two.bytes());
    assert_eq!(one.digest_sha256(), two.digest_sha256());
}
