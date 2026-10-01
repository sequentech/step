// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! PAdES signatures by several signers, one incremental revision each.
//!
//! Flow:
//! 1. [`append_signature_page`] builds the base revision (a full rewrite with
//!    empty signature fields).
//! 2. Prepare, for each signer: [`prepare_revision`] appends an incremental
//!    update that fills one field with placeholders and a visible appearance.
//!    The caller persists the parent's SHA-256, the field index, the
//!    [`SignatureAppearance`] and [`PreparedRevision::digest_sha256`], and the
//!    signer's browser signs that digest as a detached CMS.
//! 3. Approve: [`rebuild_revision`] rebuilds the same bytes from the latest
//!    revision and the persisted data (preparing is deterministic), so no
//!    stored blob is trusted; [`finalize`] verifies the CMS and embeds it.
//!
//! Every text printed on the page comes from the caller, already translated.
//! Text is set in the standard Helvetica font with WinAnsi encoding.

use chrono::{DateTime, Utc};
use cms::content_info::ContentInfo;
use cms::signed_data::SignedData;
use der::asn1::{ObjectIdentifier, OctetString};
use der::{Any, Decode, Tag, Tagged};
use lopdf::content::{Content, Operation};
use lopdf::xref::XrefType;
use lopdf::{
    dictionary, Dictionary, Document, IncrementalDocument, Object, ObjectId, Stream, StringFormat,
};
use openssl::cms::{CMSOptions, CmsContentInfo};
use openssl::stack::Stack;
use openssl::x509::store::X509StoreBuilder;
use openssl::x509::verify::X509VerifyFlags;
use openssl::x509::{X509PurposeId, X509};
use sha2::{Digest, Sha256};

/// Bytes reserved for the DER CMS. The file holds them as hex, so twice as
/// many. Fits a chain of 4–5 certificates plus a future timestamp token.
pub const CMS_PLACEHOLDER_BYTES: usize = 16 * 1024;
/// Written with the widest integers a ByteRange needs, then patched in place.
const BYTE_RANGE_PLACEHOLDER: i64 = 9_999_999_999;
/// `CMS_CADES`: makes `CMS_verify` check the ESS signingCertificate(V2)
/// attribute against the signer. The openssl crate has no constant for it.
const CMS_CADES: u32 = 0x10_0000;
/// OpenSSL 3.0, the first release that honours `CMS_CADES` in `CMS_verify`.
const OPENSSL_3: i64 = 0x3000_0000;
const ID_SIGNED_DATA: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.7.2");
const ID_DATA: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.7.1");
const ID_SHA256: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.16.840.1.101.3.4.2.1");
const ID_MESSAGE_DIGEST: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.9.4");
const ID_SIGNING_CERTIFICATE_V2: ObjectIdentifier =
    ObjectIdentifier::new_unwrap("1.2.840.113549.1.9.16.2.47");
/// rsaEncryption, sha256WithRSAEncryption and ecdsa-with-SHA256.
const SIGNATURE_ALGORITHMS: [ObjectIdentifier; 3] = [
    ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.1"),
    ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.11"),
    ObjectIdentifier::new_unwrap("1.2.840.10045.4.3.2"),
];
/// Keys a cross-reference stream puts in the trailer.
const XREF_STREAM_TRAILER_KEYS: [&[u8]; 7] = [
    b"Type",
    b"W",
    b"Index",
    b"Length",
    b"Filter",
    b"DecodeParms",
    b"XRefStm",
];
const MIN_PDF_VERSION: &str = "1.7";
const FIELD_NAME_PREFIX: &str = "Signature";
const FONT_NAME: &str = "F1";
/// A4 in points, for a page tree without a MediaBox.
const DEFAULT_MEDIA_BOX: [f32; 4] = [0.0, 0.0, 595.28, 841.89];
const MAX_PAGE_TREE_DEPTH: usize = 64;
/// Caller texts are clipped (with an ellipsis) to these many characters,
/// which keeps layout and string escaping cheap.
const MAX_TEXT_CHARS: usize = 256;
const MAX_CERTIFICATION_CHARS: usize = 4096;
const ELLIPSIS: char = '…';

const MARGIN: f32 = 56.0;
const TITLE_SIZE: f32 = 14.0;
const BODY_SIZE: f32 = 11.0;
const LEADING: f32 = 1.3;
const LABEL_SIZE: f32 = 10.0;
const MIN_TEXT_SIZE: f32 = 6.0;
const BOX_HEIGHT: f32 = 92.0;
const BOX_GAP: f32 = 14.0;
/// Height of the label strip under each signature widget.
const LABEL_STRIP: f32 = 22.0;
const WIDGET_INSET: f32 = 4.0;
const APPEARANCE_PADDING: f32 = 4.0;
const APPEARANCE_TEXT_SIZE: f32 = 10.0;
/// `/F 4`: print the annotation.
const ANNOTATION_PRINT: i64 = 4;
/// `/SigFlags 3`: SignaturesExist | AppendOnly.
const SIG_FLAGS: i64 = 3;

#[derive(Debug, thiserror::Error)]
pub enum PadesError {
    #[error("invalid PDF: {0}")]
    Pdf(#[from] lopdf::Error),
    #[error("writing the PDF failed: {0}")]
    Write(#[from] std::io::Error),
    #[error("the PDF is encrypted; signed documents must not be encrypted")]
    Encrypted,
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("signature field {0} not found")]
    FieldNotFound(String),
    #[error("signature field {0} is already signed")]
    FieldAlreadySigned(String),
    /// The prepared revision does not extend the latest signed revision; the
    /// client must prepare again (HTTP 409).
    #[error("the prepared revision no longer extends the latest signed revision")]
    StaleRevision,
    #[error("the prepared revision is malformed: {0}")]
    MalformedRevision(String),
    /// The rebuilt revision is not the one whose digest the signer was given.
    #[error("the rebuilt revision differs from the prepared one")]
    RevisionMismatch,
    #[error("the CMS is {size} bytes; the placeholder holds {max}")]
    CmsTooLarge { size: usize, max: usize },
    #[error("CMS verification failed: {0}")]
    CmsVerification(String),
    #[error("cryptography error: {0}")]
    Crypto(#[from] openssl::error::ErrorStack),
}

pub type Result<T, E = PadesError> = std::result::Result<T, E>;

/// One signature box on the appended page.
#[derive(Debug, Clone)]
pub struct SignatureFieldSpec {
    /// Printed under the box, e.g. the member's name and role.
    pub label: String,
    /// The field's tooltip (`/TU`).
    pub tooltip: String,
}

/// Texts of the appended page, already translated.
#[derive(Debug, Clone)]
pub struct SignaturePageTexts {
    pub title: String,
    /// The certification sentence above the boxes, wrapped to the page.
    pub certification: String,
}

/// What a signer's revision shows and records.
#[derive(Debug, Clone)]
pub struct SignatureAppearance {
    /// `/Name` of the signature dictionary.
    pub signer_name: String,
    /// `/M` of the signature dictionary, the claimed signing time.
    pub signing_time: DateTime<Utc>,
    /// `/Reason` of the signature dictionary.
    pub reason: Option<String>,
    /// The visible lines, already translated and formatted (for example the
    /// signer, the time in the event's timezone and the signing code).
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureFieldState {
    pub name: String,
    pub signed: bool,
}

/// How [`verify_cms`] and [`finalize`] check the signer's certificate.
pub enum CmsTrust<'a> {
    /// Trust the approval certificate as it is: the CMS must be signed by it,
    /// and signingCertificateV2 must name it. The certificate chain,
    /// revocation and key usage are checked by the certificate verifier.
    SignerOnly,
    /// Also build a chain from the signer to one of `anchors` (self-signed).
    Chain {
        anchors: &'a [X509],
        intermediates: &'a [X509],
    },
}

/// An incremental update that fills one signature field with placeholders.
#[derive(Debug, Clone)]
pub struct PreparedRevision {
    bytes: Vec<u8>,
    byte_range: [usize; 4],
    digest_sha256: [u8; 32],
    parent_sha256: [u8; 32],
    parent_len: usize,
}

impl PreparedRevision {
    fn new(
        bytes: Vec<u8>,
        byte_range: [usize; 4],
        parent_sha256: [u8; 32],
        parent_len: usize,
    ) -> Result<Self> {
        check_placeholder_layout(&bytes, byte_range, parent_len)?;
        let mut prepared = PreparedRevision {
            bytes,
            byte_range,
            digest_sha256: [0; 32],
            parent_sha256,
            parent_len,
        };
        prepared.digest_sha256 = Sha256::digest(prepared.signed_content()).into();
        Ok(prepared)
    }

    /// The whole revision, with `/Contents` still zero.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// `[0, a, b, c]`: the signature covers `bytes[0..a]` and `bytes[b..b + c]`;
    /// `bytes[a..b]` is the `<…>` placeholder.
    pub fn byte_range(&self) -> [usize; 4] {
        self.byte_range
    }

    /// SHA-256 of [`Self::signed_content`]: the CMS messageDigest.
    pub fn digest_sha256(&self) -> [u8; 32] {
        self.digest_sha256
    }

    /// SHA-256 of the revision this one was built on.
    pub fn parent_sha256(&self) -> [u8; 32] {
        self.parent_sha256
    }

    pub fn parent_len(&self) -> usize {
        self.parent_len
    }

    /// The bytes the signature covers.
    pub fn signed_content(&self) -> Vec<u8> {
        let [_, a, b, c] = self.byte_range;
        [&self.bytes[..a], &self.bytes[b..b + c]].concat()
    }
}

/// The name of the `index`-th (0-based) signature field.
pub fn signature_field_name(index: usize) -> String {
    format!("{FIELD_NAME_PREFIX}{}", index + 1)
}

/// Builds the base revision: `pdf` plus one or more pages with the caller's
/// texts and one empty signature field per entry of `fields`. A full rewrite
/// with a classic cross-reference table, which every later incremental update
/// inherits.
pub fn append_signature_page(
    pdf: &[u8],
    fields: &[SignatureFieldSpec],
    texts: &SignaturePageTexts,
) -> Result<Vec<u8>> {
    if fields.is_empty() {
        return Err(PadesError::InvalidInput(
            "at least one signature field is required".to_string(),
        ));
    }
    let mut doc = Document::load_mem(pdf)?;
    refuse_encrypted(&doc)?;
    // A full rewrite would invalidate existing signatures.
    refuse_signed(&doc)?;

    // One classic xref table: object streams are expanded on load and not
    // written back; the keys of an xref-stream trailer would corrupt it.
    doc.reference_table.cross_reference_type = XrefType::CrossReferenceTable;
    scrub_xref_stream_keys(&mut doc.trailer);
    doc.trailer.remove(b"Prev");
    if !doc.trailer.has(b"ID") {
        let id = Sha256::digest(pdf)[..16].to_vec();
        doc.trailer.set(
            "ID",
            vec![
                Object::String(id.clone(), StringFormat::Hexadecimal),
                Object::String(id, StringFormat::Hexadecimal),
            ],
        );
    }
    // ETSI.CAdES.detached is defined for PDF 1.7 (ISO 32000 extensions).
    if doc.version.as_str() < MIN_PDF_VERSION {
        doc.version = MIN_PDF_VERSION.to_string();
    }

    let existing = field_ids(&doc)?
        .into_iter()
        .filter_map(|id| field_name(&doc, id))
        .collect::<Vec<_>>();
    if let Some(name) = (0..fields.len())
        .map(signature_field_name)
        .find(|name| existing.contains(name))
    {
        return Err(PadesError::InvalidInput(format!(
            "the PDF already has a field named {name}"
        )));
    }

    let last_page = *doc
        .get_pages()
        .values()
        .last()
        .ok_or_else(|| PadesError::InvalidInput("the PDF has no pages".to_string()))?;
    let media_box = inherited_media_box(&doc, last_page)?;
    let pages_id = doc.catalog()?.get(b"Pages")?.as_reference()?;
    let font_id = doc.add_object(helvetica());

    let mut layout = PageLayout::new(&mut doc, media_box);
    for line in wrap(
        &clip(&texts.title, MAX_TEXT_CHARS),
        TITLE_SIZE,
        layout.width(),
    ) {
        layout.text(line, TITLE_SIZE);
    }
    layout.gap(TITLE_SIZE * 0.5);
    let certification = clip(&texts.certification, MAX_CERTIFICATION_CHARS);
    for line in wrap(&certification, BODY_SIZE, layout.width()) {
        layout.text(line, BODY_SIZE);
    }
    layout.gap(BODY_SIZE);
    let mut widgets = Vec::with_capacity(fields.len());
    for (index, spec) in fields.iter().enumerate() {
        let (page_id, rect) = layout.signature_box(&spec.label);
        let widget = Object::Reference(add_widget(layout.doc, page_id, rect, index, spec));
        layout.current().widgets.push(widget.clone());
        widgets.push(widget);
    }
    let pages = layout.finish();

    let page_count = pages.len();
    for page in pages {
        let content_id = doc.add_object(Stream::new(
            dictionary! {},
            Content {
                operations: page.operations,
            }
            .encode()?,
        ));
        doc.objects.insert(
            page.id,
            Object::Dictionary(dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
                "MediaBox" => media_box.iter().map(|v| Object::Real(*v)).collect::<Vec<_>>(),
                "CropBox" => media_box.iter().map(|v| Object::Real(*v)).collect::<Vec<_>>(),
                "Rotate" => 0,
                "Resources" => dictionary! { "Font" => dictionary! { FONT_NAME => font_id } },
                "Contents" => content_id,
                "Annots" => page.widgets,
            }),
        );
        doc.get_object_mut(pages_id)?
            .as_dict_mut()?
            .get_mut(b"Kids")?
            .as_array_mut()?
            .push(Object::Reference(page.id));
    }
    let root_pages = doc.get_object_mut(pages_id)?.as_dict_mut()?;
    let count = root_pages.get(b"Count")?.as_i64()?;
    root_pages.set("Count", count + page_count as i64);

    // The AcroForm can be missing, a reference or a direct dictionary.
    let acro_form_id = match doc.catalog()?.get(b"AcroForm").ok().cloned() {
        Some(Object::Reference(id)) => id,
        Some(Object::Dictionary(dict)) => doc.add_object(dict),
        _ => doc.add_object(dictionary! {}),
    };
    let fields_id = match doc.get_dictionary(acro_form_id)?.get(b"Fields") {
        Ok(Object::Reference(id)) => Some(*id),
        _ => None,
    };
    match fields_id {
        Some(id) => doc.get_object_mut(id)?.as_array_mut()?.extend(widgets),
        None => {
            let acro_form = doc.get_object_mut(acro_form_id)?.as_dict_mut()?;
            if !acro_form.has(b"Fields") {
                acro_form.set("Fields", Vec::<Object>::new());
            }
            acro_form
                .get_mut(b"Fields")?
                .as_array_mut()?
                .extend(widgets);
        }
    }
    let acro_form = doc.get_object_mut(acro_form_id)?.as_dict_mut()?;
    acro_form.set("SigFlags", SIG_FLAGS);
    // Viewers would regenerate appearances, which breaks signatures.
    acro_form.remove(b"NeedAppearances");
    doc.catalog_mut()?.set("AcroForm", acro_form_id);

    let mut out = Vec::with_capacity(pdf.len() + 8 * 1024);
    doc.save_to(&mut out)?;
    Ok(out)
}

/// The signature fields of `pdf`, in AcroForm order.
pub fn signature_fields(pdf: &[u8]) -> Result<Vec<SignatureFieldState>> {
    let doc = Document::load_mem(pdf)?;
    Ok(field_ids(&doc)?
        .into_iter()
        .filter_map(|id| {
            let field = doc.get_dictionary(id).ok()?;
            is_signature_field(field).then(|| SignatureFieldState {
                name: field_name(&doc, id).unwrap_or_default(),
                signed: field.has(b"V"),
            })
        })
        .collect())
}

/// Appends an incremental update to `latest` (the latest signed revision, or
/// the base) that fills field `field_index` with a signature dictionary
/// carrying placeholders and with a visible appearance.
pub fn prepare_revision(
    latest: &[u8],
    field_index: usize,
    appearance: &SignatureAppearance,
) -> Result<PreparedRevision> {
    let doc = Document::load_mem(latest)?;
    refuse_encrypted(&doc)?;
    let name = signature_field_name(field_index);
    let widget_id = find_signature_field(&doc, &name)?;
    let mut widget = doc.get_dictionary(widget_id)?.clone();
    if widget.has(b"V") {
        return Err(PadesError::FieldAlreadySigned(name));
    }
    let rect = rect(&widget)?;
    let (width, height) = (rect[2] - rect[0], rect[3] - rect[1]);
    let version = doc.version.clone();

    let mut inc = IncrementalDocument::create_from(latest.to_vec(), doc);
    let new = &mut inc.new_document;
    new.version = version;
    // The new trailer is a clone of the previous one, plus /Prev.
    scrub_xref_stream_keys(&mut new.trailer);
    // lopdf's max_id ignores the object number of a previous xref stream, so
    // never allocate below the previous trailer's /Size.
    let previous_size = new.trailer.get(b"Size").and_then(Object::as_i64)?;
    let previous_max = u32::try_from(previous_size.saturating_sub(1))
        .map_err(|_| PadesError::InvalidInput("invalid trailer /Size".to_string()))?;
    new.max_id = new.max_id.max(previous_max);

    // The signature dictionary: its first keys are fixed, so the placeholders
    // come before any caller text in the written object.
    let mut signature = dictionary! {
        "Type" => "Sig",
        "Filter" => "Adobe.PPKLite",
        "SubFilter" => "ETSI.CAdES.detached",
        "ByteRange" => vec![
            Object::Integer(0),
            Object::Integer(BYTE_RANGE_PLACEHOLDER),
            Object::Integer(BYTE_RANGE_PLACEHOLDER),
            Object::Integer(BYTE_RANGE_PLACEHOLDER),
        ],
        "Contents" => Object::String(vec![0; CMS_PLACEHOLDER_BYTES], StringFormat::Hexadecimal),
        "M" => Object::string_literal(appearance.signing_time.format("D:%Y%m%d%H%M%S+00'00'").to_string()),
        "Name" => text_string(&clip(&appearance.signer_name, MAX_TEXT_CHARS)),
    };
    if let Some(reason) = &appearance.reason {
        signature.set("Reason", text_string(&clip(reason, MAX_TEXT_CHARS)));
    }
    let signature_id = new.add_object(signature);
    let appearance_id = new.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject",
            "Subtype" => "Form",
            "BBox" => vec![0.into(), 0.into(), Object::Real(width), Object::Real(height)],
            "Resources" => dictionary! { "Font" => dictionary! { FONT_NAME => helvetica() } },
        },
        Content {
            operations: appearance_operations(&appearance.lines, width, height),
        }
        .encode()?,
    ));
    widget.set("V", signature_id);
    widget.set("AP", dictionary! { "N" => appearance_id });
    new.set_object(widget_id, widget);

    let mut bytes = Vec::with_capacity(latest.len() + 2 * CMS_PLACEHOLDER_BYTES + 4096);
    inc.save_to(&mut bytes)?;

    let byte_range = patch_byte_range(&mut bytes, latest.len(), signature_id)?;
    PreparedRevision::new(
        bytes,
        byte_range,
        Sha256::digest(latest).into(),
        latest.len(),
    )
}

/// Rebuilds, at approval time, the revision prepared earlier for a signer, from
/// the current latest revision and the data persisted at prepare time.
///
/// `StaleRevision` when `latest` is not the parent the revision was prepared
/// on (another signer went first: the client prepares again);
/// `RevisionMismatch` when the rebuilt revision's digest is not the one the
/// signer was given (the persisted data changed).
pub fn rebuild_revision(
    latest: &[u8],
    parent_sha256: [u8; 32],
    field_index: usize,
    appearance: &SignatureAppearance,
    digest_sha256: [u8; 32],
) -> Result<PreparedRevision> {
    if <[u8; 32]>::from(Sha256::digest(latest)) != parent_sha256 {
        return Err(PadesError::StaleRevision);
    }
    let rebuilt = prepare_revision(latest, field_index, appearance)?;
    if rebuilt.digest_sha256 != digest_sha256 {
        return Err(PadesError::RevisionMismatch);
    }
    Ok(rebuilt)
}

/// Completes a signer's revision: checks that `prepared` still extends
/// `latest`, verifies the CMS over the covered bytes (see [`verify_cms`]) and
/// writes it into the placeholder. Returns the signed revision.
///
/// The caller must serialize `finalize` and persisting its result on the
/// document's latest revision (a row lock or a compare-and-swap on the latest
/// revision), so that two approvals cannot both extend the same parent.
pub fn finalize(
    prepared: &PreparedRevision,
    latest: &[u8],
    cms_der: &[u8],
    signer: &X509,
    trust: &CmsTrust<'_>,
) -> Result<Vec<u8>> {
    ensure_extends(prepared, latest)?;
    check_cms_size(cms_der)?;
    verify_cms(cms_der, &prepared.signed_content(), signer, trust)?;
    embed_cms(prepared, cms_der)
}

/// Refuses a prepared revision that does not extend `latest`, the current
/// latest signed revision (or the base): another signer went first.
pub fn ensure_extends(prepared: &PreparedRevision, latest: &[u8]) -> Result<()> {
    let extends = prepared.parent_len == latest.len()
        && prepared.parent_sha256 == <[u8; 32]>::from(Sha256::digest(latest))
        && prepared.bytes.starts_with(latest);
    if extends {
        Ok(())
    } else {
        Err(PadesError::StaleRevision)
    }
}

/// Verifies a detached CMS over `signed_content` (the ByteRange bytes).
///
/// The CMS must have the PAdES profile: one SignerInfo, SHA-256 digests,
/// detached id-data content, signed attributes with one messageDigest equal
/// to SHA-256(`signed_content`) and a signingCertificateV2, and an RSA or
/// ECDSA-with-SHA-256 signature. signingTime may be present or not.
///
/// The signer must be `signer` (the approval certificate; certificates
/// embedded in the CMS are not used to find it) and signingCertificateV2 must
/// name it. The certificate purpose is not checked (signing certificates
/// often carry only clientAuth); key usage is the certificate verifier's job.
pub fn verify_cms(
    cms_der: &[u8],
    signed_content: &[u8],
    signer: &X509,
    trust: &CmsTrust<'_>,
) -> Result<()> {
    if openssl::version::number() < OPENSSL_3 {
        return Err(PadesError::CmsVerification(
            "OpenSSL 3 is required to check signingCertificateV2".to_string(),
        ));
    }
    check_cms_profile(cms_der, &Sha256::digest(signed_content))?;
    let mut store = X509StoreBuilder::new()?;
    store.set_purpose(X509PurposeId::ANY)?;
    match trust {
        CmsTrust::SignerOnly => {
            store.add_cert(signer.clone())?;
            store.set_flags(X509VerifyFlags::PARTIAL_CHAIN)?;
        }
        CmsTrust::Chain {
            anchors,
            intermediates,
        } => {
            // Without PARTIAL_CHAIN a chain must still end at a self-signed
            // anchor, so intermediates in the store only help to build it.
            for cert in anchors.iter().chain(intermediates.iter()) {
                store.add_cert(cert.clone())?;
            }
        }
    }
    let store = store.build();
    let mut signers = Stack::new()?;
    signers.push(signer.clone())?;
    let flags = CMSOptions::BINARY | CMSOptions::NOINTERN | CMSOptions::from_bits_retain(CMS_CADES);
    let mut cms = CmsContentInfo::from_der(cms_der)
        .map_err(|e| PadesError::CmsVerification(e.to_string()))?;
    cms.verify(
        Some(&signers),
        Some(&store),
        Some(signed_content),
        None,
        flags,
    )
    .map_err(|e| PadesError::CmsVerification(e.to_string()))
}

/// Writes the CMS into the `/Contents` placeholder. The covered bytes do not
/// change, so the digest stays valid.
pub(crate) fn embed_cms(prepared: &PreparedRevision, cms_der: &[u8]) -> Result<Vec<u8>> {
    check_cms_size(cms_der)?;
    let start = prepared.byte_range[1] + 1;
    let hex = hex::encode(cms_der);
    let mut out = prepared.bytes.clone();
    out[start..start + hex.len()].copy_from_slice(hex.as_bytes());
    Ok(out)
}

fn check_cms_size(cms_der: &[u8]) -> Result<()> {
    if cms_der.len() > CMS_PLACEHOLDER_BYTES {
        return Err(PadesError::CmsTooLarge {
            size: cms_der.len(),
            max: CMS_PLACEHOLDER_BYTES,
        });
    }
    Ok(())
}

/// Parameters of a digest or signature algorithm: absent or NULL.
fn no_parameters(parameters: &Option<Any>) -> bool {
    parameters.as_ref().is_none_or(|p| p.tag() == Tag::Null)
}

/// Refuses a CMS outside the PAdES profile; OpenSSL alone accepts, for
/// example, SHA-1 digests, attached content or extra signers.
fn check_cms_profile(cms_der: &[u8], digest: &[u8]) -> Result<()> {
    let refuse = |reason: &str| Err(PadesError::CmsVerification(reason.to_string()));
    let invalid = |e: der::Error| PadesError::CmsVerification(format!("invalid CMS: {e}"));
    let info = ContentInfo::from_der(cms_der).map_err(invalid)?;
    if info.content_type != ID_SIGNED_DATA {
        return refuse("the CMS is not SignedData");
    }
    let signed_data: SignedData = info.content.decode_as().map_err(invalid)?;
    let sha256_only = signed_data.digest_algorithms.len() == 1
        && signed_data
            .digest_algorithms
            .iter()
            .all(|a| a.oid == ID_SHA256 && no_parameters(&a.parameters));
    if !sha256_only {
        return refuse("the digest algorithms must be SHA-256");
    }
    let content = &signed_data.encap_content_info;
    if content.econtent.is_some() || content.econtent_type != ID_DATA {
        return refuse("the CMS must be detached id-data");
    }
    let [signer_info] = signed_data.signer_infos.0.as_slice() else {
        return refuse("the CMS must have exactly one signer");
    };
    if signer_info.digest_alg.oid != ID_SHA256 || !no_parameters(&signer_info.digest_alg.parameters)
    {
        return refuse("the signer's digest algorithm must be SHA-256");
    }
    let algorithm = &signer_info.signature_algorithm;
    if !SIGNATURE_ALGORITHMS.contains(&algorithm.oid) || !no_parameters(&algorithm.parameters) {
        return refuse("the signature algorithm must be RSA or ECDSA with SHA-256");
    }
    let Some(attributes) = &signer_info.signed_attrs else {
        return refuse("the CMS has no signed attributes");
    };
    let digests: Vec<&Any> = attributes
        .iter()
        .filter(|a| a.oid == ID_MESSAGE_DIGEST)
        .flat_map(|a| a.values.iter())
        .collect();
    let [message_digest] = digests.as_slice() else {
        return refuse("the CMS must have exactly one messageDigest");
    };
    let message_digest: OctetString = message_digest.decode_as().map_err(invalid)?;
    if message_digest.as_bytes() != digest {
        return refuse("messageDigest is not the prepared revision's digest");
    }
    if !attributes
        .iter()
        .any(|a| a.oid == ID_SIGNING_CERTIFICATE_V2)
    {
        return refuse("the CMS has no signingCertificateV2");
    }
    Ok(())
}

fn refuse_encrypted(doc: &Document) -> Result<()> {
    if doc.encryption_state.is_some() || doc.trailer.has(b"Encrypt") {
        return Err(PadesError::Encrypted);
    }
    Ok(())
}

fn scrub_xref_stream_keys(trailer: &mut Dictionary) {
    for key in XREF_STREAM_TRAILER_KEYS {
        trailer.remove(key);
    }
}

fn helvetica() -> Dictionary {
    dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
        "Encoding" => "WinAnsiEncoding",
    }
}

/// The top-level AcroForm fields (the signature fields are created flat).
fn field_ids(doc: &Document) -> Result<Vec<ObjectId>> {
    let Ok(acro_form) = doc.catalog()?.get(b"AcroForm") else {
        return Ok(Vec::new());
    };
    let (_, acro_form) = doc.dereference(acro_form)?;
    let Ok(fields) = acro_form.as_dict()?.get(b"Fields") else {
        return Ok(Vec::new());
    };
    let (_, fields) = doc.dereference(fields)?;
    Ok(fields
        .as_array()?
        .iter()
        .filter_map(|f| f.as_reference().ok())
        .collect())
}

fn field_name(doc: &Document, id: ObjectId) -> Option<String> {
    let name = doc.get_dictionary(id).ok()?.get(b"T").ok()?.as_str().ok()?;
    Some(decode_text_string(name))
}

fn find_signature_field(doc: &Document, name: &str) -> Result<ObjectId> {
    field_ids(doc)?
        .into_iter()
        .find(|id| {
            doc.get_dictionary(*id).is_ok_and(is_signature_field)
                && field_name(doc, *id).as_deref() == Some(name)
        })
        .ok_or_else(|| PadesError::FieldNotFound(name.to_string()))
}

fn numbers(object: &Object) -> Result<[f32; 4]> {
    let values = object
        .as_array()?
        .iter()
        .map(Object::as_float)
        .collect::<Result<Vec<_>, _>>()?;
    <[f32; 4]>::try_from(values)
        .map_err(|_| PadesError::InvalidInput("a rectangle needs 4 numbers".to_string()))
}

/// A rectangle as `[llx, lly, urx, ury]`.
fn normalise([x1, y1, x2, y2]: [f32; 4]) -> [f32; 4] {
    [x1.min(x2), y1.min(y2), x1.max(x2), y1.max(y2)]
}

fn rect(widget: &Dictionary) -> Result<[f32; 4]> {
    Ok(normalise(numbers(widget.get(b"Rect")?)?))
}

fn is_signature_field(field: &Dictionary) -> bool {
    field.get(b"FT").and_then(Object::as_name).ok() == Some(b"Sig")
}

fn refuse_signed(doc: &Document) -> Result<()> {
    let catalog = doc.catalog()?;
    let signed_field = field_ids(doc)?.into_iter().any(|id| {
        doc.get_dictionary(id)
            .is_ok_and(|f| is_signature_field(f) && f.has(b"V"))
    });
    if signed_field || catalog.has(b"Perms") || catalog.has(b"DSS") {
        return Err(PadesError::InvalidInput(
            "the PDF is already signed".to_string(),
        ));
    }
    Ok(())
}

/// `text`, cut to `max_chars` characters with an ellipsis.
fn clip(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let mut clipped: String = text.chars().take(max_chars.saturating_sub(1)).collect();
    clipped.push(ELLIPSIS);
    clipped
}

/// The MediaBox of a page, inherited through the page tree.
fn inherited_media_box(doc: &Document, page_id: ObjectId) -> Result<[f32; 4]> {
    let mut node = doc.get_dictionary(page_id)?;
    for _ in 0..MAX_PAGE_TREE_DEPTH {
        if let Ok(media_box) = node.get(b"MediaBox") {
            let (_, media_box) = doc.dereference(media_box)?;
            return Ok(normalise(numbers(media_box)?));
        }
        match node.get(b"Parent").and_then(Object::as_reference) {
            Ok(parent) => node = doc.get_dictionary(parent)?,
            Err(_) => break,
        }
    }
    Ok(DEFAULT_MEDIA_BOX)
}

fn add_widget(
    doc: &mut Document,
    page_id: ObjectId,
    rect: [f32; 4],
    index: usize,
    spec: &SignatureFieldSpec,
) -> ObjectId {
    // A merged field and widget; no /V until signed.
    doc.add_object(dictionary! {
        "Type" => "Annot",
        "Subtype" => "Widget",
        "FT" => "Sig",
        "T" => Object::string_literal(signature_field_name(index)),
        "TU" => text_string(&clip(&spec.tooltip, MAX_TEXT_CHARS)),
        "Rect" => rect.iter().map(|v| Object::Real(*v)).collect::<Vec<_>>(),
        "F" => ANNOTATION_PRINT,
        "P" => page_id,
    })
}

struct PageContent {
    id: ObjectId,
    operations: Vec<Operation>,
    widgets: Vec<Object>,
}

/// Lays out the appended pages top-down, starting a new page when the next
/// element does not fit.
struct PageLayout<'a> {
    doc: &'a mut Document,
    media_box: [f32; 4],
    pages: Vec<PageContent>,
    y: f32,
}

impl<'a> PageLayout<'a> {
    fn new(doc: &'a mut Document, media_box: [f32; 4]) -> Self {
        let mut layout = PageLayout {
            doc,
            media_box,
            pages: Vec::new(),
            y: 0.0,
        };
        layout.new_page();
        layout
    }

    fn new_page(&mut self) {
        self.pages.push(PageContent {
            id: self.doc.new_object_id(),
            operations: Vec::new(),
            widgets: Vec::new(),
        });
        self.y = self.media_box[3] - MARGIN;
    }

    fn left(&self) -> f32 {
        self.media_box[0] + MARGIN
    }

    fn width(&self) -> f32 {
        (self.media_box[2] - self.media_box[0] - 2.0 * MARGIN).max(MARGIN)
    }

    /// Makes room for `height` points, on a new page if needed.
    fn reserve(&mut self, height: f32) {
        let page_is_empty = self.y >= self.media_box[3] - MARGIN;
        if self.y - height < self.media_box[1] + MARGIN && !page_is_empty {
            self.new_page();
        }
    }

    fn current(&mut self) -> &mut PageContent {
        // `new` always pushes a first page.
        let last = self.pages.len() - 1;
        &mut self.pages[last]
    }

    fn gap(&mut self, height: f32) {
        self.y -= height;
    }

    fn text(&mut self, text: Vec<u8>, size: f32) {
        self.reserve(size * LEADING);
        self.y -= size * LEADING;
        let (x, y) = (self.left(), self.y);
        show_text(&mut self.current().operations, size, x, y, text);
    }

    /// Draws a box with its label and returns the page and the widget
    /// rectangle for its signature field.
    fn signature_box(&mut self, label: &str) -> (ObjectId, [f32; 4]) {
        self.reserve(BOX_HEIGHT);
        let (left, width, top) = (self.left(), self.width(), self.y);
        let bottom = top - BOX_HEIGHT;
        let label = win_ansi(&clip(label, MAX_TEXT_CHARS));
        let label_size = fit_size(&label, LABEL_SIZE, width - 2.0 * WIDGET_INSET);
        let page = self.current();
        page.operations.extend([
            Operation::new(
                "re",
                vec![
                    Object::Real(left),
                    Object::Real(bottom),
                    Object::Real(width),
                    Object::Real(BOX_HEIGHT),
                ],
            ),
            Operation::new("S", vec![]),
        ]);
        show_text(
            &mut page.operations,
            label_size,
            left + WIDGET_INSET + 2.0,
            bottom + (LABEL_STRIP - label_size) / 2.0,
            label,
        );
        let rect = [
            left + WIDGET_INSET,
            bottom + LABEL_STRIP,
            left + width - WIDGET_INSET,
            top - WIDGET_INSET,
        ];
        let page_id = page.id;
        self.y = bottom - BOX_GAP;
        (page_id, rect)
    }

    fn finish(self) -> Vec<PageContent> {
        self.pages
    }
}

fn show_text(operations: &mut Vec<Operation>, size: f32, x: f32, y: f32, text: Vec<u8>) {
    operations.extend([
        Operation::new("BT", vec![]),
        Operation::new(
            "Tf",
            vec![Object::Name(FONT_NAME.into()), Object::Real(size)],
        ),
        Operation::new("Td", vec![Object::Real(x), Object::Real(y)]),
        Operation::new("Tj", vec![Object::String(text, StringFormat::Literal)]),
        Operation::new("ET", vec![]),
    ]);
}

fn appearance_operations(lines: &[String], width: f32, height: f32) -> Vec<Operation> {
    let mut operations = vec![
        Operation::new("q", vec![]),
        Operation::new(
            "rg",
            vec![Object::Real(0.94), Object::Real(0.96), Object::Real(1.0)],
        ),
        Operation::new(
            "re",
            vec![
                0.into(),
                0.into(),
                Object::Real(width),
                Object::Real(height),
            ],
        ),
        Operation::new("f", vec![]),
        Operation::new("Q", vec![]),
    ];
    if lines.is_empty() {
        return operations;
    }
    let available = (height - 2.0 * APPEARANCE_PADDING).max(MIN_TEXT_SIZE);
    // Lines that do not fit even at the minimum size end in an ellipsis line.
    let max_lines = ((available / (MIN_TEXT_SIZE * LEADING)).floor() as usize).max(1);
    let mut lines: Vec<String> = lines.iter().map(|l| clip(l, MAX_TEXT_CHARS)).collect();
    if lines.len() > max_lines {
        lines.truncate(max_lines - 1);
        lines.push(ELLIPSIS.to_string());
    }
    let size =
        (available / (lines.len() as f32 * LEADING)).clamp(MIN_TEXT_SIZE, APPEARANCE_TEXT_SIZE);
    let mut y = height - APPEARANCE_PADDING;
    for line in lines {
        let text = win_ansi(&line);
        let line_size = fit_size(&text, size, width - 2.0 * APPEARANCE_PADDING);
        y -= size * LEADING;
        show_text(
            &mut operations,
            line_size,
            APPEARANCE_PADDING,
            y + (size * LEADING - size) / 2.0,
            text,
        );
    }
    operations
}

/// A PDF text string: a literal for printable ASCII, else UTF-16BE with a BOM.
fn text_string(text: &str) -> Object {
    if text.chars().all(|c| (' '..='~').contains(&c)) {
        Object::String(text.as_bytes().to_vec(), StringFormat::Literal)
    } else {
        let mut bytes = vec![0xfe, 0xff];
        bytes.extend(text.encode_utf16().flat_map(u16::to_be_bytes));
        Object::String(bytes, StringFormat::Hexadecimal)
    }
}

fn decode_text_string(bytes: &[u8]) -> String {
    match bytes {
        [0xfe, 0xff, rest @ ..] => {
            let units: Vec<u16> = rest
                .chunks_exact(2)
                .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
                .collect();
            String::from_utf16_lossy(&units)
        }
        _ => String::from_utf8_lossy(bytes).into_owned(),
    }
}

/// Encodes `text` as WinAnsi for Helvetica. Characters outside it print `?`.
fn win_ansi(text: &str) -> Vec<u8> {
    text.chars()
        .map(|c| match c {
            ' '..='~' | '\u{a0}'..='\u{ff}' => c as u8,
            '€' => 0x80,
            '‚' => 0x82,
            'ƒ' => 0x83,
            '„' => 0x84,
            '…' => 0x85,
            '†' => 0x86,
            '‡' => 0x87,
            'ˆ' => 0x88,
            '‰' => 0x89,
            'Š' => 0x8a,
            '‹' => 0x8b,
            'Œ' => 0x8c,
            'Ž' => 0x8e,
            '‘' => 0x91,
            '’' => 0x92,
            '“' => 0x93,
            '”' => 0x94,
            '•' => 0x95,
            '–' => 0x96,
            '—' => 0x97,
            '˜' => 0x98,
            '™' => 0x99,
            'š' => 0x9a,
            '›' => 0x9b,
            'œ' => 0x9c,
            'ž' => 0x9e,
            'Ÿ' => 0x9f,
            c if c.is_whitespace() => b' ',
            _ => b'?',
        })
        .collect()
}

/// Helvetica advance widths (1/1000 em) for WinAnsi codes 32–126.
const HELVETICA_WIDTHS: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278,
    278, // space–/
    556, 556, 556, 556, 556, 556, 556, 556, 556, 556, 278, 278, 584, 584, 584, 556, // 0–?
    1015, 667, 667, 722, 722, 667, 611, 778, 722, 278, 500, 667, 556, 833, 722, 778, // @–O
    667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611, 278, 278, 278, 469, 556, // P–_
    333, 556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500, 222, 833, 556, 556, // `–o
    556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, 334, 260, 334, 584, // p–~
];
/// A conservative width for the other WinAnsi codes (accented capitals).
const HELVETICA_OTHER_WIDTH: u16 = 722;

fn text_width(text: &[u8], size: f32) -> f32 {
    let units: u32 = text
        .iter()
        .map(|&b| {
            let width = match b {
                32..=126 => HELVETICA_WIDTHS[usize::from(b - 32)],
                _ => HELVETICA_OTHER_WIDTH,
            };
            u32::from(width)
        })
        .sum();
    units as f32 * size / 1000.0
}

/// The largest size up to `preferred` at which `text` fits `width`.
fn fit_size(text: &[u8], preferred: f32, width: f32) -> f32 {
    let natural = text_width(text, preferred);
    if natural <= width || natural == 0.0 {
        preferred
    } else {
        (preferred * width / natural).max(MIN_TEXT_SIZE)
    }
}

/// Wraps `text` into WinAnsi lines no wider than `width`.
fn wrap(text: &str, size: f32, width: f32) -> Vec<Vec<u8>> {
    let mut lines: Vec<Vec<u8>> = Vec::new();
    let mut current: Vec<u8> = Vec::new();
    for word in text.split_whitespace().map(win_ansi) {
        let candidate = if current.is_empty() {
            word.clone()
        } else {
            [current.as_slice(), b" ", &word].concat()
        };
        if text_width(&candidate, size) <= width || current.is_empty() {
            current = candidate;
        } else {
            lines.push(std::mem::replace(&mut current, word));
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn find(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    haystack
        .get(from..)?
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|p| p + from)
}

/// Finds the placeholders of the signature object in the appended part and
/// writes the real ByteRange, space-padded to the placeholder's width.
fn patch_byte_range(
    bytes: &mut [u8],
    parent_len: usize,
    signature_id: ObjectId,
) -> Result<[usize; 4]> {
    let missing = |what: &str| PadesError::MalformedRevision(format!("{what} not found"));
    let header = format!("\n{} {} obj", signature_id.0, signature_id.1);
    let start = find(bytes, header.as_bytes(), parent_len).ok_or_else(|| missing("signature"))?;
    let end = find(bytes, b"endobj", start).ok_or_else(|| missing("endobj"))?;
    let object = &bytes[start..end];
    let key = find(object, b"/ByteRange", 0).ok_or_else(|| missing("/ByteRange"))?;
    let open = find(object, b"[", key).ok_or_else(|| missing("ByteRange ["))?;
    let close = find(object, b"]", open).ok_or_else(|| missing("ByteRange ]"))?;
    let contents = find(object, b"/Contents", 0).ok_or_else(|| missing("/Contents"))?;
    let lt = find(object, b"<", contents).ok_or_else(|| missing("Contents <"))?;
    let gt = find(object, b">", lt).ok_or_else(|| missing("Contents >"))?;
    let placeholder =
        format!("0 {BYTE_RANGE_PLACEHOLDER} {BYTE_RANGE_PLACEHOLDER} {BYTE_RANGE_PLACEHOLDER}");
    if &object[open + 1..close] != placeholder.as_bytes() {
        return Err(PadesError::MalformedRevision(
            "unexpected ByteRange placeholder".to_string(),
        ));
    }

    let (a, b) = (start + lt, start + gt + 1);
    let c = bytes.len() - b;
    let range = format!("0 {a} {b} {c}");
    let slot = placeholder.len();
    if range.len() > slot {
        return Err(PadesError::InvalidInput("the PDF is too large".to_string()));
    }
    bytes[start + open + 1..start + close].copy_from_slice(format!("{range:<slot$}").as_bytes());
    Ok([0, a, b, c])
}

fn check_placeholder_layout(bytes: &[u8], byte_range: [usize; 4], parent_len: usize) -> Result<()> {
    let malformed = |what: &str| Err(PadesError::MalformedRevision(what.to_string()));
    let [start, a, b, c] = byte_range;
    if start != 0 {
        return malformed("the ByteRange must start at 0");
    }
    if b.checked_add(c) != Some(bytes.len()) {
        return malformed("the ByteRange must end at the end of the revision");
    }
    if b.checked_sub(a) != Some(2 * CMS_PLACEHOLDER_BYTES + 2) {
        return malformed("the ByteRange gap must be the /Contents placeholder");
    }
    if a <= parent_len {
        return malformed("the placeholder must be in the appended part");
    }
    if bytes[a] != b'<' || bytes[b - 1] != b'>' {
        return malformed("the ByteRange gap must be a hex string");
    }
    if bytes[a + 1..b - 1].iter().any(|&h| h != b'0') {
        return malformed("the /Contents placeholder is not empty");
    }
    Ok(())
}
