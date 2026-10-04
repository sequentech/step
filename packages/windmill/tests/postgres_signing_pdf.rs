// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! PDF signatures wired into requests (design §6): the signature page at
//! report generation, `prepare_pdf`, and the approve step's PAdES revision.
//!
//! Each signer prepares a revision, signs its digest as a browser does (a
//! detached CMS built here in DER), and approves; every signed revision is
//! checked with the PAdES verifier. Rule numbers, labels and the
//! organization come from the configuration each test uses, and the tests
//! run under two of them.

#[path = "support/schema.rs"]
mod schema;
#[path = "support/signing.rs"]
mod signing;

use async_trait::async_trait;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use chrono::{DateTime, TimeZone, Utc};
use deadpool_postgres::{Client, Transaction};
use lopdf::content::{Content, Operation};
use lopdf::{dictionary, Document, Object, Stream};
use openssl::asn1::Asn1Time;
use openssl::bn::BigNum;
use openssl::hash::MessageDigest;
use openssl::pkey::{PKey, Private};
use openssl::rsa::Rsa;
use openssl::sign::Signer;
use openssl::x509::{X509NameBuilder, X509};
use sequent_core::election_config::ReportType;
use sequent_core::signing::{
    CertificateCheckId, DocumentRevisionState, RequesterSigning, SignatureAlgorithm, SigningAction,
    SigningRequestStatus, SigningRequirement, SigningRule,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use signing::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use windmill::postgres::signing::*;
use windmill::postgres::signing_document_revision::{list_document_revisions, DocumentRevisionRow};
use windmill::services::signing::approve::{
    approve, ApproveInput, ApproveOutcome, SigningServices,
};
use windmill::services::signing::guard::{GuardOutcome, GuardRequest, SigningDocument};
use windmill::services::signing::pades::{signature_fields, verify_cms, CmsTrust};
use windmill::services::signing::page_texts::{
    page_languages, shipped_wordings, PageWording, FALLBACK_LANGUAGE,
};
use windmill::services::signing::pdf::StoredAppearance;
use windmill::services::signing::pdf::{
    appearance_lines, latest_signed_document, prepare_pdf, report_delivery, report_signature_page,
    report_signing_action, signer_title, tally_signing_action, DocumentLink, DocumentRevisionView,
    PdfDocumentSigner, PdfPrepared, PdfSources, ReportDelivery, RevisionStore, SignatureFacts,
    SignerTitles,
};
use windmill::services::signing::requests::{get_panel, SigningPanel};
use windmill::services::signing::{InvalidReason, SigningCaller, SigningError, SigningResult};

const ER: SigningAction = SigningAction::GenerateElectionReturns;

/// Two configurations: a Post label, how many sign, and the organization's
/// display name.
const PRESETS: [(&str, u16, &str); 2] = [
    ("madrid-pe", 3, "Commission on Elections"),
    ("faculty-of-science", 2, "Student Council"),
];

// ------------------------------------------------------------ the documents

/// Documents in memory, as the object storage would keep them.
#[derive(Default)]
struct MemoryStore {
    documents: Mutex<HashMap<Uuid, Vec<u8>>>,
}

impl MemoryStore {
    fn put(&self, bytes: &[u8]) -> Uuid {
        let id = Uuid::new_v4();
        self.documents.lock().unwrap().insert(id, bytes.to_vec());
        id
    }

    fn get(&self, id: Uuid) -> Vec<u8> {
        self.documents.lock().unwrap()[&id].clone()
    }
}

#[async_trait]
impl RevisionStore for MemoryStore {
    async fn load(
        &self,
        _tx: &Transaction<'_>,
        _tenant_id: Uuid,
        _election_event_id: Uuid,
        document_id: Uuid,
    ) -> anyhow::Result<Vec<u8>> {
        self.documents
            .lock()
            .unwrap()
            .get(&document_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("no document {document_id}"))
    }

    async fn store(
        &self,
        _tx: &Transaction<'_>,
        _tenant_id: Uuid,
        _election_event_id: Uuid,
        _file_name: &str,
        content: &[u8],
    ) -> anyhow::Result<Uuid> {
        Ok(self.put(content))
    }

    async fn link(
        &self,
        _tx: &Transaction<'_>,
        _tenant_id: Uuid,
        _election_event_id: Uuid,
        document_id: Uuid,
    ) -> anyhow::Result<Option<DocumentLink>> {
        let known = self.documents.lock().unwrap().contains_key(&document_id);
        Ok(known.then(|| DocumentLink {
            url: format!("memory://{document_id}"),
            name: Some(format!("document-{document_id}.pdf")),
        }))
    }
}

impl MemoryStore {
    /// What a link points at.
    fn fetch(&self, url: &str) -> Vec<u8> {
        self.get(Uuid::parse_str(url.strip_prefix("memory://").unwrap()).unwrap())
    }
}

/// A one-page report, as Chromium would render it.
fn report_pdf() -> Vec<u8> {
    let mut doc = Document::with_version("1.4");
    let pages_id = doc.new_object_id();
    let font_id = doc.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica",
    });
    let content = Content {
        operations: vec![
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec!["F1".into(), 18.into()]),
            Operation::new("Td", vec![72.into(), 760.into()]),
            Operation::new("Tj", vec![Object::string_literal("Election returns")]),
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
    let mut out = vec![];
    doc.save_to(&mut out).unwrap();
    out
}

fn contains(haystack: &[u8], needle: &str) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle.as_bytes())
}

fn hex_sha(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

// ----------------------------------------------- signers and the browser CMS

/// A signer with a real key: its certificate is what the fake verifier
/// knows, and the CMS is made with its key.
#[derive(Clone)]
struct PdfSigner {
    caller: SigningCaller,
    certificate: TestCert,
    x509: X509,
    key: PKey<Private>,
}

fn self_signed(common_name: &str) -> (X509, PKey<Private>) {
    let key = PKey::from_rsa(Rsa::generate(2048).unwrap()).unwrap();
    let mut name = X509NameBuilder::new().unwrap();
    name.append_entry_by_text("CN", common_name).unwrap();
    let name = name.build();
    let mut builder = X509::builder().unwrap();
    builder.set_version(2).unwrap();
    let serial = BigNum::from_u32(rand_serial())
        .unwrap()
        .to_asn1_integer()
        .unwrap();
    builder.set_serial_number(&serial).unwrap();
    builder.set_subject_name(&name).unwrap();
    builder.set_issuer_name(&name).unwrap();
    builder.set_pubkey(&key).unwrap();
    builder
        .set_not_before(&Asn1Time::days_from_now(0).unwrap())
        .unwrap();
    builder
        .set_not_after(&Asn1Time::days_from_now(30).unwrap())
        .unwrap();
    builder.sign(&key, MessageDigest::sha256()).unwrap();
    (builder.build(), key)
}

fn rand_serial() -> u32 {
    u32::from_be_bytes(Uuid::new_v4().as_bytes()[..4].try_into().unwrap()) | 1
}

/// `n` signers of `action` with registered certificates.
async fn pdf_signers(w: &World, action: SigningAction, n: usize) -> Vec<PdfSigner> {
    let mut signers = vec![];
    for i in 0..n {
        let user = format!("sbei-{i}");
        let (x509, key) = self_signed(&user);
        let pem = String::from_utf8(x509.to_pem().unwrap()).unwrap();
        let certificate = cert_with_pem(&user, &user, &user, &pem);
        w.register(&user, &certificate, None).await;
        signers.push(PdfSigner {
            caller: w.signer(&user, action),
            certificate,
            x509,
            key,
        });
    }
    signers
}

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
    let content = parts.concat();
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

/// A detached CAdES SignedData over a digest, as the widget (PKIjs) makes
/// it: contentType, messageDigest and signingCertificateV2, no signingTime.
fn browser_cms(digest: &[u8], certificate: &X509, key: &PKey<Private>) -> Vec<u8> {
    let certificate_der = certificate.to_der().unwrap();
    let mut attributes = vec![
        seq(&[
            &oid("1.2.840.113549.1.9.3"),
            &set(&[&oid("1.2.840.113549.1.7.1")]),
        ]),
        seq(&[&oid("1.2.840.113549.1.9.4"), &set(&[&octets(digest)])]),
        seq(&[
            &oid("1.2.840.113549.1.9.16.2.47"),
            &set(&[&seq(&[&seq(&[&seq(&[&octets(&Sha256::digest(
                &certificate_der,
            ))])])])]),
        ]),
    ];
    attributes.sort();
    let attributes = attributes.concat();
    let mut signer = Signer::new(MessageDigest::sha256(), key).unwrap();
    let signature = signer
        .sign_oneshot_to_vec(&tlv(0x31, &[&attributes]))
        .unwrap();
    let digest_algorithm = seq(&[&oid("2.16.840.1.101.3.4.2.1")]);
    let serial = certificate.serial_number().to_bn().unwrap().to_vec();
    let signer_info = seq(&[
        &unsigned_integer(&[1]),
        &seq(&[
            &certificate.issuer_name().to_der().unwrap(),
            &unsigned_integer(&serial),
        ]),
        &digest_algorithm,
        &tlv(0xa0, &[&attributes]),
        &seq(&[&oid("1.2.840.113549.1.1.11")]),
        &octets(&signature),
    ]);
    let signed_data = seq(&[
        &unsigned_integer(&[1]),
        &set(&[&digest_algorithm]),
        &seq(&[&oid("1.2.840.113549.1.7.1")]),
        &tlv(0xa0, &[&certificate_der]),
        &set(&[&signer_info]),
    ]);
    seq(&[&oid("1.2.840.113549.1.7.2"), &tlv(0xa0, &[&signed_data])])
}

/// The CMS a revision carries in its `/Contents`, without the zero padding,
/// and the bytes its ByteRange covers.
fn embedded(bytes: &[u8], byte_range: [usize; 4]) -> (Vec<u8>, Vec<u8>) {
    let [_, a, b, c] = byte_range;
    let padded = hex::decode(&bytes[a + 1..b - 1]).unwrap();
    let length = match padded[1] {
        short if short < 0x80 => 2 + short as usize,
        long => {
            let n = (long & 0x7f) as usize;
            2 + n
                + padded[2..2 + n]
                    .iter()
                    .fold(0, |acc, b| (acc << 8) | *b as usize)
        }
    };
    (
        padded[..length].to_vec(),
        [&bytes[..a], &bytes[b..b + c]].concat(),
    )
}

fn byte_range(row: &DocumentRevisionRow) -> [usize; 4] {
    serde_json::from_value(row.byte_range.clone().unwrap()).unwrap()
}

// ------------------------------------------------------------ the steps

/// The approve step's services, with PDF signatures kept in `store`.
fn pdf_services(
    store: &Arc<MemoryStore>,
    signers: &[&PdfSigner],
    executor: Arc<FakeExecutor>,
) -> SigningServices {
    let certificates: Vec<&TestCert> = signers.iter().map(|s| &s.certificate).collect();
    let mut services = services(FakeVerifier::knowing(&certificates), vec![executor]);
    services.documents = Arc::new(PdfDocumentSigner::new(store.clone()));
    services
}

async fn set_display_name(w: &World, display_name: Option<&str>) {
    let client = w.pool.get().await.unwrap();
    client
        .execute(
            "UPDATE sequent_backend.tenant SET settings = $2 WHERE id = $1",
            &[
                &w.tenant,
                &display_name.map(|name| json!({ "display_name": name })),
            ],
        )
        .await
        .unwrap();
}

async fn signature_page(w: &World, action: SigningAction, pdf: &[u8]) -> Option<Vec<u8>> {
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    report_signature_page(&tx, w.tenant, w.event, action, pdf)
        .await
        .unwrap()
}

/// A request to sign the election returns, whose document is the base
/// with the signature page.
async fn er_request(w: &World, store: &MemoryStore) -> (Uuid, String) {
    let base = signature_page(w, ER, &report_pdf())
        .await
        .expect("the rule requires signatures");
    let document_id = store.put(&base);
    start_on(w, &base, document_id).await.unwrap()
}

/// Starts the election returns' request on `base`, stored as `document_id`.
async fn start_on(w: &World, base: &[u8], document_id: Uuid) -> SigningResult<(Uuid, String)> {
    let sha256 = hex_sha(base);
    let requester = caller("operator", &[], &[&w.label]);
    let outcome = w
        .guard_request(
            &requester,
            &GuardRequest {
                action: ER,
                scope: w.scope(ER),
                subject: json!({
                    "report_type": "ELECTORAL_RESULTS",
                    "document_sha256": sha256,
                    "template_id": null,
                }),
                document: Some(SigningDocument {
                    document_id: Some(document_id),
                    sha256: sha256.clone(),
                }),
                config_revision: None,
            },
            at(0),
        )
        .await?;
    match outcome {
        GuardOutcome::SigningRequired(summary) => Ok((summary.id, summary.code)),
        GuardOutcome::Proceed => panic!("the election returns need signatures"),
    }
}

/// The board's titles, as the signer directory gives them: the first
/// signer is the chairperson; the others have none.
struct BoardTitles;

const CHAIRPERSON: &str = "Chairperson";

#[async_trait]
impl SignerTitles for BoardTitles {
    async fn title(
        &self,
        _tenant_id: Uuid,
        action: SigningAction,
        user_id: &str,
    ) -> anyhow::Result<Option<String>> {
        Ok((action == ER && user_id == "sbei-0").then(|| CHAIRPERSON.to_owned()))
    }
}

fn english() -> &'static PageWording {
    &shipped_wordings().unwrap()[FALLBACK_LANGUAGE]
}

async fn prepare(
    w: &World,
    store: &MemoryStore,
    signer: &PdfSigner,
    request_id: Uuid,
    now: DateTime<Utc>,
) -> SigningResult<PdfPrepared> {
    let mut client = w.pool.get().await.unwrap();
    prepare_pdf(
        &mut client,
        PdfSources {
            store,
            titles: &BoardTitles,
        },
        &signer.caller,
        w.tenant,
        request_id,
        &[signer.certificate.identity.pem.clone()],
        now,
    )
    .await
}

/// The CMS the signer's browser makes for a prepared revision.
fn cms_for(signer: &PdfSigner, prepared: &PdfPrepared) -> Vec<u8> {
    let digest = BASE64.decode(&prepared.digest_b64).unwrap();
    browser_cms(&digest, &signer.x509, &signer.key)
}

async fn sign_pdf(
    w: &World,
    services: &SigningServices,
    signer: &PdfSigner,
    request_id: Uuid,
    pdf_cms: Option<Vec<u8>>,
    revision: Option<i32>,
) -> SigningResult<ApproveOutcome> {
    let request = w.request(request_id).await;
    let mut client = w.pool.get().await.unwrap();
    approve(
        &mut client,
        services,
        &signer.caller,
        w.tenant,
        &ApproveInput {
            request_id,
            chain_pem: vec![signer.certificate.identity.pem.clone()],
            algorithm: SignatureAlgorithm::RsaPkcs1Sha256,
            payload_signature: signature_over(&request.canonical_payload),
            document_signature: None,
            pdf_cms,
            revision,
        },
        at(2),
    )
    .await
}

async fn revisions(w: &World, request_id: Uuid) -> Vec<DocumentRevisionRow> {
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    list_document_revisions(&tx, w.tenant, w.event, request_id)
        .await
        .unwrap()
}

async fn latest(w: &World, request_id: Uuid) -> DocumentRevisionRow {
    let request = w.request(request_id).await;
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    latest_signed_document(&tx, &request)
        .await
        .unwrap()
        .unwrap()
}

fn refused(result: SigningResult<impl std::fmt::Debug>) -> CertificateCheckId {
    match result {
        Err(SigningError::Refused { check, .. }) => check,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

// ------------------------------------------------------------------ tests

#[tokio::test]
async fn each_signer_adds_a_revision_and_every_revision_verifies() {
    for (label, required, organization) in PRESETS {
        let w = world(label).await;
        set_display_name(&w, Some(organization)).await;
        w.rule(ER, required, RequesterSigning::NotAllowed, None)
            .await;
        let store = Arc::new(MemoryStore::default());
        let signers = pdf_signers(&w, ER, usize::from(required)).await;
        let executor = Arc::new(FakeExecutor::new(ER, FakeBehaviour::Succeed(json!({}))));
        let services = pdf_services(
            &store,
            &signers.iter().collect::<Vec<_>>(),
            executor.clone(),
        );
        let (request_id, code) = er_request(&w, &store).await;

        for (i, signer) in signers.iter().enumerate() {
            let prepared = prepare(&w, &store, signer, request_id, at(1))
                .await
                .unwrap();
            assert_eq!(prepared.signing_time, at(1), "{label}");
            let outcome = sign_pdf(
                &w,
                &services,
                signer,
                request_id,
                Some(cms_for(signer, &prepared)),
                Some(prepared.revision),
            )
            .await
            .unwrap();
            assert_eq!(outcome.count, i as i64 + 1, "{label}");

            // The signer's revision is the document now, and fills field i.
            let now = latest(&w, request_id).await;
            assert_eq!(now.state, DocumentRevisionState::Signed);
            assert_eq!(now.field_index, Some(i as i32));
            assert_eq!(
                now.prepared_for_user.as_deref(),
                Some(signer.caller.user_id.as_str())
            );
            let bytes = store.get(now.document_id.unwrap());
            assert_eq!(hex_sha(&bytes), now.sha256);
            let signed: Vec<bool> = signature_fields(&bytes)
                .unwrap()
                .into_iter()
                .map(|field| field.signed)
                .collect();
            let mut expected = vec![false; usize::from(required)];
            expected[..=i].fill(true);
            assert_eq!(signed, expected, "{label}: after signer {i}");
            // The appearance prints the signer, their title, the time, the
            // certificate's issuer, the code and the document before
            // signatures (self-signed: the signer issued it).
            assert!(contains(
                &bytes,
                &format!("Digitally signed by {}", signer.caller.user_id)
            ));
            assert!(contains(&bytes, &signer.caller.display_name));
            // Only the first signer has a title: printed once, in their
            // revision and in every later one.
            assert_eq!(
                bytes
                    .windows(CHAIRPERSON.len() + 2)
                    .filter(|w| *w == format!("({CHAIRPERSON})").as_bytes())
                    .count(),
                1,
                "{label}: signer {i}"
            );
            assert!(contains(
                &bytes,
                &format!("Date: {}", at(1).format("%Y-%m-%d %H:%M:%S UTC"))
            ));
            assert!(
                contains(&bytes, &format!("(Issuer: {})", signer.caller.user_id)),
                "{label}"
            );
            assert!(
                contains(&bytes, &format!("Signing code: {code}")),
                "{label}"
            );
            let base_sha256 = w.request(request_id).await.document_sha256.unwrap();
            assert!(
                contains(
                    &bytes,
                    &format!("(Document SHA-256 before signatures: {base_sha256})")
                ),
                "{label}"
            );
        }
        assert_eq!(
            w.request(request_id).await.status,
            SigningRequestStatus::Executed,
            "{label}"
        );
        assert_eq!(executor.runs().len(), 1);

        // base, then a prepared and a signed revision per signer.
        let rows = revisions(&w, request_id).await;
        let mut states = vec![DocumentRevisionState::Base];
        for _ in 0..required {
            states.extend([
                DocumentRevisionState::Prepared,
                DocumentRevisionState::Signed,
            ]);
        }
        assert_eq!(
            rows.iter().map(|row| row.state).collect::<Vec<_>>(),
            states,
            "{label}"
        );
        assert_eq!(
            rows.iter().map(|row| row.revision).collect::<Vec<_>>(),
            (0..rows.len() as i32).collect::<Vec<_>>()
        );

        // Each signed revision verifies on its own and inside the final
        // document, which extends every earlier revision.
        let signed: Vec<&DocumentRevisionRow> = rows
            .iter()
            .filter(|row| row.state == DocumentRevisionState::Signed)
            .collect();
        let last = store.get(signed.last().unwrap().document_id.unwrap());
        let mut parent = rows[0].sha256.clone();
        for (row, signer) in signed.iter().zip(&signers) {
            let bytes = store.get(row.document_id.unwrap());
            assert_eq!(row.parent_sha256.as_deref(), Some(parent.as_str()));
            assert!(last.starts_with(&bytes));
            let (cms, covered) = embedded(&bytes, byte_range(row));
            assert_eq!(hex_sha(&covered), row.digest_sha256.clone().unwrap());
            verify_cms(&cms, &covered, &signer.x509, &CmsTrust::SignerOnly).unwrap();
            let (cms, covered) = embedded(&last, byte_range(row));
            verify_cms(&cms, &covered, &signer.x509, &CmsTrust::SignerOnly).unwrap();
            // Signed by someone else, the same bytes don't verify.
            let other = signers
                .iter()
                .find(|s| s.caller.user_id != signer.caller.user_id)
                .unwrap();
            assert!(verify_cms(&cms, &covered, &other.x509, &CmsTrust::SignerOnly).is_err());
            parent = row.sha256.clone();
        }
        // The approvals keep the CMS they were made with.
        let approvals = w.approvals(request_id).await;
        assert!(approvals.iter().all(|approval| approval.pdf_cms.is_some()));
    }
}

#[tokio::test]
async fn a_stale_prepare_is_refused_until_the_signer_prepares_again() {
    for (label, required, _) in PRESETS {
        let w = world(label).await;
        w.rule(ER, required, RequesterSigning::NotAllowed, None)
            .await;
        let store = Arc::new(MemoryStore::default());
        let signers = pdf_signers(&w, ER, 2).await;
        let executor = Arc::new(FakeExecutor::new(ER, FakeBehaviour::Succeed(json!({}))));
        let services = pdf_services(&store, &signers.iter().collect::<Vec<_>>(), executor);
        let (request_id, _) = er_request(&w, &store).await;
        let (alice, bob) = (&signers[0], &signers[1]);

        // Both prepare on the base: both get the first field.
        let alices = prepare(&w, &store, alice, request_id, at(1)).await.unwrap();
        let bobs = prepare(&w, &store, bob, request_id, at(1)).await.unwrap();
        let fields: Vec<Option<i32>> = revisions(&w, request_id)
            .await
            .iter()
            .filter(|row| row.state == DocumentRevisionState::Prepared)
            .map(|row| row.field_index)
            .collect();
        assert_eq!(fields, [Some(0), Some(0)]);

        sign_pdf(
            &w,
            &services,
            alice,
            request_id,
            Some(cms_for(alice, &alices)),
            Some(alices.revision),
        )
        .await
        .unwrap();
        // Bob's revision no longer extends the document.
        let stale = sign_pdf(
            &w,
            &services,
            bob,
            request_id,
            Some(cms_for(bob, &bobs)),
            Some(bobs.revision),
        )
        .await;
        assert!(
            matches!(stale, Err(SigningError::StaleRevision(_))),
            "{label}: {stale:?}"
        );
        assert_eq!(w.approvals(request_id).await.len(), 1);
        assert_eq!(
            w.steps().await,
            ["SigningRequestCreated", "SigningRequestSigned"]
        );

        // Prepared again, Bob fills the second field.
        let again = prepare(&w, &store, bob, request_id, at(1)).await.unwrap();
        assert_ne!(again.digest_b64, bobs.digest_b64);
        let outcome = sign_pdf(
            &w,
            &services,
            bob,
            request_id,
            Some(cms_for(bob, &again)),
            Some(again.revision),
        )
        .await
        .unwrap();
        assert_eq!(outcome.count, 2);
        assert_eq!(latest(&w, request_id).await.field_index, Some(1));

        // A signer who signed prepares no other field.
        if required > 2 {
            assert_eq!(
                refused(prepare(&w, &store, alice, request_id, at(1)).await),
                CertificateCheckId::AlreadySigned
            );
        }
    }
}

#[tokio::test]
async fn a_tampered_signature_or_revision_is_refused_and_logged() {
    let (label, required, _) = PRESETS[0];
    let w = world(label).await;
    w.rule(ER, required, RequesterSigning::NotAllowed, None)
        .await;
    let store = Arc::new(MemoryStore::default());
    let signers = pdf_signers(&w, ER, 2).await;
    let executor = Arc::new(FakeExecutor::new(ER, FakeBehaviour::Succeed(json!({}))));
    let services = pdf_services(&store, &signers.iter().collect::<Vec<_>>(), executor);
    let (request_id, _) = er_request(&w, &store).await;
    let (alice, bob) = (&signers[0], &signers[1]);
    let prepared = prepare(&w, &store, alice, request_id, at(1)).await.unwrap();

    // A CMS over another digest.
    let mut digest = BASE64.decode(&prepared.digest_b64).unwrap();
    digest[0] ^= 1;
    let wrong_digest = browser_cms(&digest, &alice.x509, &alice.key);
    // A CMS made with another key.
    let other_key = cms_for(bob, &prepared);
    // Bytes that are no CMS.
    let garbage = b"not a cms".to_vec();
    for cms in [wrong_digest, other_key, garbage] {
        assert_eq!(
            refused(
                sign_pdf(
                    &w,
                    &services,
                    alice,
                    request_id,
                    Some(cms),
                    Some(prepared.revision)
                )
                .await
            ),
            CertificateCheckId::Signature
        );
    }
    // Without the PDF signature or its revision: invalid, not logged.
    for (cms, revision) in [
        (None, Some(prepared.revision)),
        (Some(cms_for(alice, &prepared)), None),
    ] {
        let result = sign_pdf(&w, &services, alice, request_id, cms, revision).await;
        assert!(
            matches!(
                result,
                Err(SigningError::Invalid {
                    reason: InvalidReason::Document,
                    ..
                })
            ),
            "{result:?}"
        );
    }
    // A revision that isn't hers, or Alice's revision for Bob: refused.
    assert_eq!(
        refused(
            sign_pdf(
                &w,
                &services,
                alice,
                request_id,
                Some(cms_for(alice, &prepared)),
                Some(prepared.revision + 100),
            )
            .await
        ),
        CertificateCheckId::Signature
    );
    assert_eq!(
        refused(
            sign_pdf(
                &w,
                &services,
                bob,
                request_id,
                Some(cms_for(bob, &prepared)),
                Some(prepared.revision),
            )
            .await
        ),
        CertificateCheckId::Signature
    );

    // Another certificate on the same document, right away: throttled.
    let mut borrowed = alice.clone();
    borrowed.certificate = bob.certificate.clone();
    assert!(matches!(
        prepare(&w, &store, &borrowed, request_id, at(1)).await,
        Err(SigningError::Conflict(_))
    ));

    // What the revision is rebuilt from changed since it was prepared: its
    // printed signer, or the certificate it was prepared for.
    let client = w.pool.get().await.unwrap();
    let original: serde_json::Value = client
        .query_one(
            "SELECT appearance FROM sequent_backend.signing_document_revision
             WHERE request_id = $1 AND revision = $2",
            &[&request_id, &prepared.revision],
        )
        .await
        .unwrap()
        .get(0);
    let mut mallory = original.clone();
    mallory["signer_name"] = json!("Mallory");
    let mut bobs_certificate = original.clone();
    bobs_certificate["certificate_sha256"] = json!(hex_sha(&bob.x509.to_der().unwrap()));
    for tampered in [mallory, bobs_certificate] {
        client
            .execute(
                "UPDATE sequent_backend.signing_document_revision SET appearance = $3
                 WHERE request_id = $1 AND revision = $2",
                &[&request_id, &prepared.revision, &tampered],
            )
            .await
            .unwrap();
        assert_eq!(
            refused(
                sign_pdf(
                    &w,
                    &services,
                    alice,
                    request_id,
                    Some(cms_for(alice, &prepared)),
                    Some(prepared.revision)
                )
                .await
            ),
            CertificateCheckId::Signature
        );
    }

    // Nothing was signed or stored; each person's refusals were logged
    // once (the log is throttled per person and request).
    assert!(w.approvals(request_id).await.is_empty());
    assert_eq!(
        latest(&w, request_id).await.state,
        DocumentRevisionState::Base
    );
    assert_eq!(store.documents.lock().unwrap().len(), 1);
    let refusals: Vec<Option<String>> = w
        .outbox()
        .await
        .into_iter()
        .filter(|row| row.1 == "SigningSignatureRefused" && row.2 == "USER")
        .map(|row| row.4)
        .collect();
    assert_eq!(
        refusals,
        [Some("sbei-0".to_string()), Some("sbei-1".to_string())]
    );

    // Control: the revision restored, preparing again answers the same one,
    // and signing it is taken.
    client
        .execute(
            "UPDATE sequent_backend.signing_document_revision SET appearance = $3
             WHERE request_id = $1 AND revision = $2",
            &[&request_id, &prepared.revision, &original],
        )
        .await
        .unwrap();
    let again = prepare(&w, &store, alice, request_id, at(5)).await.unwrap();
    assert_eq!(again, prepared);
    sign_pdf(
        &w,
        &services,
        alice,
        request_id,
        Some(cms_for(alice, &again)),
        Some(again.revision),
    )
    .await
    .unwrap();
    assert_eq!(
        latest(&w, request_id).await.state,
        DocumentRevisionState::Signed
    );
}

#[tokio::test]
async fn only_a_signer_of_a_waiting_pdf_request_prepares() {
    let (label, required, _) = PRESETS[1];
    let w = world(label).await;
    w.rule(ER, required, RequesterSigning::NotAllowed, None)
        .await;
    let store = Arc::new(MemoryStore::default());
    let signers = pdf_signers(&w, ER, 1).await;
    let (request_id, _) = er_request(&w, &store).await;

    // Without the sign permission, or outside the Post.
    let mut outsider = signers[0].clone();
    outsider.caller = caller(
        "sbei-0",
        &[SigningAction::GenerateReports.sign_permission()],
        &[&w.label],
    );
    assert!(matches!(
        prepare(&w, &store, &outsider, request_id, at(1)).await,
        Err(SigningError::Forbidden(_))
    ));
    outsider.caller = caller("sbei-0", &[ER.sign_permission()], &["another-post"]);
    assert!(matches!(
        prepare(&w, &store, &outsider, request_id, at(1)).await,
        Err(SigningError::Forbidden(_))
    ));

    // A request without a PDF.
    w.rule(
        SigningAction::CloseVoting,
        1,
        RequesterSigning::NotAllowed,
        None,
    )
    .await;
    let close = w
        .start(
            &caller("operator", &[], &[]),
            SigningAction::CloseVoting,
            subject(1),
            at(0),
        )
        .await;
    let mut closer = signers[0].clone();
    closer.caller = w.signer("sbei-0", SigningAction::CloseVoting);
    assert!(matches!(
        prepare(&w, &store, &closer, close.id, at(1)).await,
        Err(SigningError::Invalid { .. })
    ));

    // A cancelled one.
    let client = w.pool.get().await.unwrap();
    client
        .execute(
            "UPDATE sequent_backend.signing_request SET status = 'cancelled', cancel_reason = 'by-operator'
             WHERE id = $1",
            &[&request_id],
        )
        .await
        .unwrap();
    assert!(matches!(
        prepare(&w, &store, &signers[0], request_id, at(1)).await,
        Err(SigningError::Closed {
            status: SigningRequestStatus::Cancelled,
            ..
        })
    ));
}

#[tokio::test]
async fn the_signature_page_is_added_only_when_the_rule_requires_signatures() {
    // (label, signatures, display name; none = the tenant's slug)
    let configurations: [(&str, u16, Option<&str>); 2] = [
        ("madrid-pe", 3, Some("Commission on Elections")),
        ("faculty-of-science", 2, None),
    ];
    for (label, required, display_name) in configurations {
        let w = world(label).await;
        set_display_name(&w, display_name).await;
        let organization = display_name
            .map(str::to_owned)
            .unwrap_or_else(|| format!("tenant-{}", w.tenant));
        let report = report_pdf();

        // No rule, or a rule that needs no signatures: the report as it is.
        assert_eq!(signature_page(&w, ER, &report).await, None, "{label}");
        {
            let mut client = w.pool.get().await.unwrap();
            let tx = client.transaction().await.unwrap();
            upsert_signing_rule(
                &tx,
                w.tenant,
                w.event,
                &SigningRule {
                    action: ER,
                    requirement: SigningRequirement::NotRequired,
                    signatures: required,
                    requester_signing: RequesterSigning::NotAllowed,
                    expires_minutes: None,
                    revision: 0,
                },
                0,
                "configuration-manager",
                None,
            )
            .await
            .unwrap()
            .unwrap();
            tx.commit().await.unwrap();
        }
        assert_eq!(signature_page(&w, ER, &report).await, None, "{label}");

        // Required: one empty field per signature, and the action's texts.
        w.rule(ER, required, RequesterSigning::NotAllowed, None)
            .await;
        for (action, certify) in [
            (
                ER,
                "We certify that these election returns are true and correct.",
            ),
            (
                SigningAction::GenerateReports,
                "We certify that this report is true and correct.",
            ),
        ] {
            if action == SigningAction::GenerateReports {
                assert_eq!(signature_page(&w, action, &report).await, None);
                w.rule(action, required, RequesterSigning::NotAllowed, None)
                    .await;
            }
            let base = signature_page(&w, action, &report).await.unwrap();
            assert!(base.starts_with(b"%PDF-1.7"));
            let fields = signature_fields(&base).unwrap();
            assert_eq!(fields.len(), usize::from(required), "{label}");
            assert!(fields.iter().all(|field| !field.signed));
            assert_eq!(fields[0].name, "Signature1");
            assert!(contains(&base, certify), "{label}: {action}");
            assert!(contains(&base, &organization), "{label}: {organization}");
            assert!(!contains(&base, "/Encrypt"));
        }
    }
}

/// The panel of a request as `viewer` sees it, with an empty Keycloak
/// directory (the signers come from the approvals).
async fn panel(
    w: &World,
    documents: &PdfDocumentSigner,
    viewer: &SigningCaller,
    request_id: Uuid,
) -> SigningPanel {
    let mut hasura = w.pool.get().await.unwrap();
    let htx = hasura.transaction().await.unwrap();
    let mut keycloak = w.pool.get().await.unwrap();
    let ktx = keycloak.transaction().await.unwrap();
    keycloak_tables(&ktx).await;
    ktx.execute(
        "INSERT INTO realm VALUES ('r', $1)",
        &[&format!("tenant-{}", w.tenant)],
    )
    .await
    .unwrap();
    get_panel(&htx, &ktx, documents, viewer, w.tenant, request_id)
        .await
        .unwrap()
}

#[tokio::test]
async fn the_panel_links_the_signed_document_to_check_and_the_latest_revision_to_view() {
    for (label, required, _) in PRESETS {
        let w = world(label).await;
        w.rule(ER, required, RequesterSigning::NotAllowed, None)
            .await;
        let store = Arc::new(MemoryStore::default());
        let documents = PdfDocumentSigner::new(store.clone());
        let signers = pdf_signers(&w, ER, 1).await;
        let executor = Arc::new(FakeExecutor::new(ER, FakeBehaviour::Succeed(json!({}))));
        let services = pdf_services(&store, &signers.iter().collect::<Vec<_>>(), executor);
        let (request_id, _) = er_request(&w, &store).await;
        let request = w.request(request_id).await;
        let base_sha256 = request.document_sha256.clone().unwrap();
        let base_url = format!("memory://{}", request.document_id.unwrap());
        let viewer = &signers[0].caller;

        // The document the payload names, to check: its hash is the payload's.
        let before = panel(&w, &documents, viewer, request_id).await;
        assert_eq!(before.document_url.as_deref(), Some(base_url.as_str()));
        assert_eq!(
            before.document_name,
            Some(format!("document-{}.pdf", request.document_id.unwrap()))
        );
        assert_eq!(hex_sha(&store.fetch(&base_url)), base_sha256);
        let unsigned = DocumentRevisionView {
            revision: 0,
            sha256: base_sha256.clone(),
            signed_count: 0,
            url: Some(base_url.clone()),
        };
        assert_eq!(before.document_revision, Some(unsigned.clone()), "{label}");
        // Prepared (the base recorded), still nothing signed.
        let prepared = prepare(&w, &store, &signers[0], request_id, at(1))
            .await
            .unwrap();
        assert_eq!(
            panel(&w, &documents, viewer, request_id)
                .await
                .document_revision,
            Some(unsigned)
        );

        sign_pdf(
            &w,
            &services,
            &signers[0],
            request_id,
            Some(cms_for(&signers[0], &prepared)),
            Some(prepared.revision),
        )
        .await
        .unwrap();
        let after = panel(&w, &documents, viewer, request_id).await;
        // The signed document stays the one to check; the latest revision
        // is the one to view.
        assert_eq!(after.document_url.as_deref(), Some(base_url.as_str()));
        let latest = latest(&w, request_id).await;
        let latest_url = format!("memory://{}", latest.document_id.unwrap());
        assert_eq!(
            after.document_revision,
            Some(DocumentRevisionView {
                revision: latest.revision,
                sha256: latest.sha256.clone(),
                signed_count: 1,
                url: Some(latest_url.clone()),
            }),
            "{label}"
        );
        let viewed = store.fetch(&latest_url);
        assert_eq!(hex_sha(&viewed), latest.sha256);
        assert!(viewed.starts_with(&store.fetch(&base_url)));
    }

    // A request without a document links none.
    let w = world("no-document").await;
    w.rule(
        SigningAction::CloseVoting,
        1,
        RequesterSigning::NotAllowed,
        None,
    )
    .await;
    let close = w
        .start(
            &caller("operator", &[], &[]),
            SigningAction::CloseVoting,
            subject(1),
            at(0),
        )
        .await;
    let documents = PdfDocumentSigner::new(Arc::new(MemoryStore::default()));
    let viewer = w.signer("sbei-0", SigningAction::CloseVoting);
    let none = panel(&w, &documents, &viewer, close.id).await;
    assert_eq!(
        (
            none.document_url,
            none.document_name,
            none.document_revision
        ),
        (None, None, None)
    );
}

/// Configures the event's timezones: `primary` and the others it names.
async fn configure_zones(w: &World, configured: &[&str], primary: &str) {
    let client = w.pool.get().await.unwrap();
    client
        .execute(
            "UPDATE sequent_backend.election_event
             SET presentation = COALESCE(presentation, '{}'::jsonb)
                 || jsonb_build_object('timezones', $3::jsonb)
             WHERE tenant_id = $1 AND id = $2",
            &[
                &w.tenant,
                &w.event,
                &json!({"configured": configured, "primary": primary, "logs": "election"}),
            ],
        )
        .await
        .unwrap();
}

/// The two configurations zones are proven under, as (configured, primary).
const ZONE_CONFIGURATIONS: [(&[&str], &str); 2] = [
    (&["Asia/Manila", "Asia/Dubai"], "Asia/Manila"),
    (&["Europe/Madrid", "Atlantic/Canary"], "Europe/Madrid"),
];

#[tokio::test]
async fn the_signature_prints_the_time_in_the_events_zone() {
    for (configured, primary) in ZONE_CONFIGURATIONS {
        let w = world(&format!("post-{primary}")).await;
        configure_zones(&w, configured, primary).await;
        w.rule(ER, 2, RequesterSigning::NotAllowed, None).await;
        let store = Arc::new(MemoryStore::default());
        let signers = pdf_signers(&w, ER, 1).await;
        let (request_id, code) = er_request(&w, &store).await;
        let prepared = prepare(&w, &store, &signers[0], request_id, at(1))
            .await
            .unwrap();

        let row = revisions(&w, request_id)
            .await
            .into_iter()
            .find(|row| row.revision == prepared.revision)
            .unwrap();
        let appearance: StoredAppearance = serde_json::from_value(row.appearance.unwrap()).unwrap();
        let tz: chrono_tz::Tz = primary.parse().unwrap();
        let base_sha256 = w.request(request_id).await.document_sha256.unwrap();
        assert_eq!(
            appearance.lines,
            appearance_lines(
                english(),
                &SignatureFacts {
                    certificate_name: "sbei-0",
                    display_name: "sbei-0 display",
                    title: Some(CHAIRPERSON),
                    issuer: Some("sbei-0"),
                    signing_time: at(1),
                    zone: Some(tz),
                    code: &code,
                    document_sha256: Some(&base_sha256),
                }
            )
        );
        let local = at(1).with_timezone(&tz).format("%Y-%m-%d %H:%M:%S");
        assert_eq!(appearance.lines[3], format!("Date: {local} {primary}"));
        assert_eq!(appearance.signing_time, at(1));
    }
}

#[tokio::test]
async fn an_event_without_timezones_signs_in_utc() {
    let w = world("post-utc").await;
    w.rule(ER, 2, RequesterSigning::NotAllowed, None).await;
    let store = MemoryStore::default();
    let signers = pdf_signers(&w, ER, 1).await;
    let (request_id, _) = er_request(&w, &store).await;
    let prepared = prepare(&w, &store, &signers[0], request_id, at(1))
        .await
        .unwrap();
    let row = revisions(&w, request_id)
        .await
        .into_iter()
        .find(|row| row.revision == prepared.revision)
        .unwrap();
    let appearance: StoredAppearance = serde_json::from_value(row.appearance.unwrap()).unwrap();
    let utc = at(1).format("%Y-%m-%d %H:%M:%S");
    assert_eq!(appearance.lines[3], format!("Date: {utc} UTC"));
}

#[tokio::test]
async fn a_pdf_request_without_its_document_takes_no_signature() {
    let w = world("madrid-pe").await;
    w.rule(ER, 2, RequesterSigning::NotAllowed, None).await;
    let store = Arc::new(MemoryStore::default());
    let signers = pdf_signers(&w, ER, 1).await;
    let executor = Arc::new(FakeExecutor::new(ER, FakeBehaviour::Succeed(json!({}))));
    let services = pdf_services(&store, &signers.iter().collect::<Vec<_>>(), executor);
    let sha256 = hex_sha(b"returns");
    let outcome = w
        .guard_request(
            &caller("operator", &[], &[&w.label]),
            &GuardRequest {
                action: ER,
                scope: w.scope(ER),
                subject: json!({
                    "report_type": "ELECTORAL_RESULTS",
                    "document_sha256": sha256,
                    "template_id": null,
                }),
                document: Some(SigningDocument {
                    document_id: None,
                    sha256,
                }),
                config_revision: None,
            },
            at(0),
        )
        .await
        .unwrap();
    let GuardOutcome::SigningRequired(request) = outcome else {
        panic!("the election returns need signatures");
    };
    let signer = &signers[0];
    assert!(matches!(
        prepare(&w, &store, signer, request.id, at(1)).await,
        Err(SigningError::Invalid { .. })
    ));
    for pdf_cms in [None, Some(b"cms".to_vec())] {
        let result = sign_pdf(&w, &services, signer, request.id, pdf_cms, Some(1)).await;
        assert!(
            matches!(result, Err(SigningError::Invalid { .. })),
            "{result:?}"
        );
    }
    assert!(w.approvals(request.id).await.is_empty());
}

#[test]
fn reports_map_to_their_signing_action() {
    // The Reports tab holds the participation report of a Post.
    assert_eq!(
        report_signing_action(&ReportType::PARTICIPATION_REPORT),
        Some(SigningAction::GenerateReports)
    );
    // The tally holds the election returns and the initialization report.
    assert_eq!(
        tally_signing_action(&ReportType::ELECTORAL_RESULTS),
        Some(ER)
    );
    assert_eq!(
        tally_signing_action(&ReportType::INITIALIZATION_REPORT),
        Some(SigningAction::GenerateReports)
    );
    // Not signed: not a Post's report, or not one PDF (known gaps).
    for report in [
        ReportType::ELECTORAL_RESULTS,
        ReportType::INITIALIZATION_REPORT,
        ReportType::MANUAL_VERIFICATION,
        ReportType::ACTIVITY_LOGS,
        ReportType::BALLOT_IMAGES,
        ReportType::BALLOT_RECEIPT,
        ReportType::CREDENTIALS,
    ] {
        assert_eq!(report_signing_action(&report), None, "{report}");
    }
    for report in [
        ReportType::PARTICIPATION_REPORT,
        ReportType::MANUAL_VERIFICATION,
        ReportType::ACTIVITY_LOGS,
    ] {
        assert_eq!(tally_signing_action(&report), None, "{report}");
    }
}

#[test]
fn the_appearance_prints_the_time_in_the_events_zone() {
    let time = Utc.with_ymd_and_hms(2026, 10, 1, 20, 30, 5).unwrap();
    let sha256 = hex_sha(b"returns");
    // The certificate's holder, the account's name when it differs, the
    // title, the time, the issuer, the code and the unsigned document.
    assert_eq!(
        appearance_lines(
            english(),
            &SignatureFacts {
                certificate_name: "MARIA SANTOS DELA CRUZ",
                display_name: "Maria Santos",
                title: Some("Chairperson"),
                issuer: Some("PNPKI Individual CA"),
                signing_time: time,
                zone: None,
                code: "7F3A-91C2",
                document_sha256: Some(&sha256),
            }
        ),
        [
            "Digitally signed by MARIA SANTOS DELA CRUZ".to_owned(),
            "Maria Santos".to_owned(),
            "Chairperson".to_owned(),
            "Date: 2026-10-01 20:30:05 UTC".to_owned(),
            "Issuer: PNPKI Individual CA".to_owned(),
            "Signing code: 7F3A-91C2".to_owned(),
            format!("Document SHA-256 before signatures: {sha256}"),
        ]
    );
    // Without a title, an issuer name or a document, those lines are left
    // out; a blank one counts as none.
    assert_eq!(
        appearance_lines(
            english(),
            &SignatureFacts {
                certificate_name: "Ana Reyes",
                display_name: "Ana Reyes",
                title: Some("  "),
                issuer: None,
                signing_time: time,
                zone: Some(chrono_tz::Asia::Manila),
                code: "ABCD-EFGH",
                document_sha256: None,
            }
        ),
        [
            "Digitally signed by Ana Reyes",
            "Date: 2026-10-02 04:30:05 Asia/Manila",
            "Signing code: ABCD-EFGH",
        ]
    );
}

#[test]
fn a_report_that_needs_signatures_is_kept_to_sign_not_released_encrypted_or_mailed() {
    // A scheduled, password-protected report whose action needs signatures.
    assert_eq!(
        report_delivery(true, true, true),
        ReportDelivery::AwaitSignatures
    );
    assert_eq!(
        report_delivery(true, false, false),
        ReportDelivery::AwaitSignatures
    );
    // Without signatures it is released as it always was.
    for (encrypt, send_email) in [(true, true), (false, true), (true, false), (false, false)] {
        assert_eq!(
            report_delivery(false, encrypt, send_email),
            ReportDelivery::Release {
                encrypt,
                send_email
            }
        );
    }
}

/// How many signing revisions of each state a request has.
async fn states(w: &World, request_id: Uuid) -> Vec<DocumentRevisionState> {
    revisions(w, request_id)
        .await
        .into_iter()
        .map(|row| row.state)
        .collect()
}

#[tokio::test]
async fn an_approval_that_conflicts_stores_no_signed_revision() {
    let (label, required, _) = PRESETS[0];
    let w = world(label).await;
    w.rule(ER, required, RequesterSigning::NotAllowed, None)
        .await;
    let store = Arc::new(MemoryStore::default());
    let signers = pdf_signers(&w, ER, 1).await;
    let executor = Arc::new(FakeExecutor::new(ER, FakeBehaviour::Succeed(json!({}))));
    let services = pdf_services(&store, &signers.iter().collect::<Vec<_>>(), executor);
    let (request_id, _) = er_request(&w, &store).await;
    let alice = &signers[0];
    let prepared = prepare(&w, &store, alice, request_id, at(1)).await.unwrap();

    // The approval insert meets a conflict the checks before it didn't see
    // (as a race would): the same approval appears just before it.
    let name = format!("conflict_{}", request_id.simple());
    let client = w.pool.get().await.unwrap();
    client
        .batch_execute(&format!(
            "CREATE FUNCTION sequent_backend.{name}() RETURNS trigger AS $$
             BEGIN
                 IF NEW.request_id = '{request_id}' AND pg_trigger_depth() = 1 THEN
                     INSERT INTO sequent_backend.signing_approval SELECT (NEW).*;
                 END IF;
                 RETURN NEW;
             END $$ LANGUAGE plpgsql;
             CREATE TRIGGER {name} BEFORE INSERT ON sequent_backend.signing_approval
                 FOR EACH ROW EXECUTE FUNCTION sequent_backend.{name}();"
        ))
        .await
        .unwrap();
    let result = sign_pdf(
        &w,
        &services,
        alice,
        request_id,
        Some(cms_for(alice, &prepared)),
        Some(prepared.revision),
    )
    .await;
    client
        .batch_execute(&format!(
            "DROP TRIGGER {name} ON sequent_backend.signing_approval;
             DROP FUNCTION sequent_backend.{name}();"
        ))
        .await
        .unwrap();
    assert_eq!(refused(result), CertificateCheckId::AlreadySigned);
    // Neither the approval nor a signed revision, nor a stored document.
    assert!(w.approvals(request_id).await.is_empty());
    assert_eq!(
        states(&w, request_id).await,
        [DocumentRevisionState::Base, DocumentRevisionState::Prepared]
    );
    assert_eq!(store.documents.lock().unwrap().len(), 1);
    assert!(w
        .steps()
        .await
        .contains(&"SigningSignatureRefused".to_string()));
}

#[tokio::test]
async fn concurrent_prepares_and_approvals_take_one_field_at_a_time() {
    for (label, required, _) in PRESETS {
        let w = world(label).await;
        w.rule(ER, required, RequesterSigning::NotAllowed, None)
            .await;
        let store = Arc::new(MemoryStore::default());
        let signers = pdf_signers(&w, ER, 2).await;
        let executor = Arc::new(FakeExecutor::new(ER, FakeBehaviour::Succeed(json!({}))));
        let services = pdf_services(&store, &signers.iter().collect::<Vec<_>>(), executor);
        let (request_id, _) = er_request(&w, &store).await;
        let (alice, bob) = (&signers[0], &signers[1]);

        // Two connections at once: one base, both on the first field.
        let (a, b) = tokio::join!(
            prepare(&w, &store, alice, request_id, at(1)),
            prepare(&w, &store, bob, request_id, at(1)),
        );
        let (a, b) = (a.unwrap(), b.unwrap());
        assert_ne!(a.revision, b.revision);
        let rows = revisions(&w, request_id).await;
        assert_eq!(
            rows.iter()
                .filter(|row| row.state == DocumentRevisionState::Base)
                .count(),
            1,
            "{label}"
        );
        assert!(rows
            .iter()
            .filter(|row| row.state == DocumentRevisionState::Prepared)
            .all(|row| row.field_index == Some(0)));

        // Both approve at once: one fills the field, the other is stale.
        let (first, second) = tokio::join!(
            sign_pdf(
                &w,
                &services,
                alice,
                request_id,
                Some(cms_for(alice, &a)),
                Some(a.revision)
            ),
            sign_pdf(
                &w,
                &services,
                bob,
                request_id,
                Some(cms_for(bob, &b)),
                Some(b.revision)
            ),
        );
        let outcomes = [first, second];
        assert_eq!(
            outcomes.iter().filter(|outcome| outcome.is_ok()).count(),
            1,
            "{label}: {outcomes:?}"
        );
        assert!(outcomes
            .iter()
            .any(|outcome| matches!(outcome, Err(SigningError::StaleRevision(_)))));
        assert_eq!(w.approvals(request_id).await.len(), 1);
        assert_eq!(store.documents.lock().unwrap().len(), 2);
        assert_eq!(latest(&w, request_id).await.field_index, Some(0));
    }
}

#[tokio::test]
async fn refused_prepares_are_logged() {
    // The requester, when the rule doesn't let them sign.
    let (label, required, _) = PRESETS[0];
    let w = world(label).await;
    w.rule(ER, required, RequesterSigning::NotAllowed, None)
        .await;
    let store = Arc::new(MemoryStore::default());
    let signers = pdf_signers(&w, ER, 1).await;
    let (request_id, _) = er_request(&w, &store).await;
    let mut requester = signers[0].clone();
    requester.caller = caller("operator", &[ER.sign_permission()], &[&w.label]);
    assert!(matches!(
        prepare(&w, &store, &requester, request_id, at(1)).await,
        Err(SigningError::Forbidden(_))
    ));
    let refusals: Vec<(String, Option<String>)> = w
        .outbox()
        .await
        .into_iter()
        .filter(|row| row.2 == "USER")
        .map(|row| (row.1, row.4))
        .collect();
    assert_eq!(
        refusals.last(),
        Some(&(
            "SigningSignatureRefused".to_string(),
            Some("operator".to_string())
        ))
    );

    // A request past its time expires.
    let w = world(label).await;
    w.rule(ER, required, RequesterSigning::NotAllowed, Some(30))
        .await;
    let signers = pdf_signers(&w, ER, 1).await;
    let (request_id, _) = er_request(&w, &store).await;
    w.make_overdue(request_id).await;
    let expired = prepare(&w, &store, &signers[0], request_id, at(1)).await;
    assert!(
        matches!(
            expired,
            Err(SigningError::Closed {
                status: SigningRequestStatus::Expired,
                ..
            })
        ),
        "{expired:?}"
    );
    assert_eq!(
        w.request(request_id).await.status,
        SigningRequestStatus::Expired
    );
    assert!(w
        .steps()
        .await
        .contains(&"SigningRequestExpired".to_string()));

    // A signer who signed, in the two-signature configuration.
    let (label, required, _) = PRESETS[1];
    let w = world(label).await;
    w.rule(ER, required, RequesterSigning::NotAllowed, None)
        .await;
    let signers = pdf_signers(&w, ER, 1).await;
    let executor = Arc::new(FakeExecutor::new(ER, FakeBehaviour::Succeed(json!({}))));
    let services = pdf_services(&store, &signers.iter().collect::<Vec<_>>(), executor);
    let (request_id, _) = er_request(&w, &store).await;
    let alice = &signers[0];
    let prepared = prepare(&w, &store, alice, request_id, at(1)).await.unwrap();
    sign_pdf(
        &w,
        &services,
        alice,
        request_id,
        Some(cms_for(alice, &prepared)),
        Some(prepared.revision),
    )
    .await
    .unwrap();
    assert_eq!(
        refused(prepare(&w, &store, alice, request_id, at(2)).await),
        CertificateCheckId::AlreadySigned
    );
    assert_eq!(
        w.steps().await,
        [
            "SigningRequestCreated",
            "SigningRequestSigned",
            "SigningSignatureRefused"
        ]
    );
}

#[tokio::test]
async fn a_document_whose_fields_dont_fit_the_request_is_refused() {
    let w = world("madrid-pe").await;
    // Generated for three signatures; the rule then asks for two.
    w.rule(ER, 3, RequesterSigning::NotAllowed, None).await;
    let base = signature_page(&w, ER, &report_pdf()).await.unwrap();
    w.rule(ER, 2, RequesterSigning::NotAllowed, None).await;
    let store = Arc::new(MemoryStore::default());
    let document_id = store.put(&base);
    let (request_id, _) = start_on(&w, &base, document_id).await.unwrap();
    let signers = pdf_signers(&w, ER, 1).await;
    assert!(matches!(
        prepare(&w, &store, &signers[0], request_id, at(1)).await,
        Err(SigningError::Invalid {
            reason: InvalidReason::DocumentFields,
            ..
        })
    ));
    assert!(revisions(&w, request_id).await.is_empty());
}

/// Stores a document row for `document_id` with `annotations`.
async fn document_row(w: &World, document_id: Uuid, annotations: serde_json::Value) {
    let client = w.pool.get().await.unwrap();
    client
        .execute(
            "INSERT INTO sequent_backend.document
                 (id, tenant_id, election_event_id, name, media_type, size, annotations)
             VALUES ($1, $2, $3, 'report.pdf', 'application/pdf', 1, $4)
             ON CONFLICT (id) DO UPDATE SET annotations = EXCLUDED.annotations",
            &[&document_id, &w.tenant, &w.event, &annotations],
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn documents_that_need_more_to_read_are_never_signing_documents() {
    for annotations in [
        json!({"access": {"voter_secret_attributes": true}}),
        json!({"access": {"password_secret_id": "secret-1"}}),
    ] {
        let w = world("madrid-pe").await;
        w.rule(ER, 2, RequesterSigning::NotAllowed, None).await;
        let store = Arc::new(MemoryStore::default());
        let base = signature_page(&w, ER, &report_pdf()).await.unwrap();

        // Refused when a request would start on it.
        let restricted = store.put(&base);
        document_row(&w, restricted, annotations.clone()).await;
        assert!(matches!(
            start_on(&w, &base, restricted).await,
            Err(SigningError::Invalid {
                reason: InvalidReason::Document,
                ..
            })
        ));

        // Restricted after the request started: never prepared or linked.
        let document_id = store.put(&base);
        document_row(&w, document_id, json!({})).await;
        let (request_id, _) = start_on(&w, &base, document_id).await.unwrap();
        document_row(&w, document_id, annotations).await;
        let signers = pdf_signers(&w, ER, 1).await;
        assert!(matches!(
            prepare(&w, &store, &signers[0], request_id, at(1)).await,
            Err(SigningError::Invalid {
                reason: InvalidReason::Document,
                ..
            })
        ));
        let documents = PdfDocumentSigner::new(store.clone());
        let shown = panel(&w, &documents, &signers[0].caller, request_id).await;
        assert_eq!((shown.document_url, shown.document_revision), (None, None));
    }
}

#[tokio::test]
async fn the_revisions_keep_one_base_and_one_signature_per_field() {
    let w = world("madrid-pe").await;
    w.rule(ER, 2, RequesterSigning::NotAllowed, None).await;
    let store = Arc::new(MemoryStore::default());
    let (request_id, _) = er_request(&w, &store).await;
    let client = w.pool.get().await.unwrap();
    let insert =
        |revision: i32, state: &'static str, document: Option<Uuid>, field: Option<i32>| {
            let client = &client;
            let w = &w;
            async move {
                client
                    .execute(
                        "INSERT INTO sequent_backend.signing_document_revision
                         (tenant_id, election_event_id, request_id, revision, document_id,
                          sha256, state, field_index)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
                        &[
                            &w.tenant,
                            &w.event,
                            &request_id,
                            &revision,
                            &document,
                            &hex_sha(b"revision"),
                            &state,
                            &field,
                        ],
                    )
                    .await
                    .map_err(|error| {
                        error
                            .as_db_error()
                            .and_then(|db| db.constraint().map(str::to_owned))
                    })
            }
        };
    let document = Some(Uuid::new_v4());
    assert!(insert(0, "base", document, None).await.is_ok());
    assert_eq!(
        insert(1, "base", document, None).await,
        Err(Some("signing_document_revision_one_base".into()))
    );
    // Only a prepared revision goes without a document.
    assert!(insert(2, "prepared", None, Some(0)).await.is_ok());
    assert_eq!(
        insert(3, "signed", None, Some(0)).await,
        Err(Some("signing_document_revision_stored".into()))
    );
    assert!(insert(4, "signed", document, Some(0)).await.is_ok());
    assert_eq!(
        insert(5, "signed", document, Some(0)).await,
        Err(Some("signing_document_revision_field_signed_once".into()))
    );
    assert!(insert(6, "signed", document, Some(1)).await.is_ok());
}

async fn set_languages(w: &World, event: Option<&str>, tenant: Option<&str>) {
    let language_conf = |code: Option<&str>| {
        code.map(|code| json!({ "language_conf": { "default_language_code": code } }))
    };
    w.execute(
        "UPDATE sequent_backend.election_event SET presentation = $2 WHERE id = $1",
        &[&w.event, &language_conf(event)],
    )
    .await;
    w.execute(
        "UPDATE sequent_backend.tenant SET settings = $2 WHERE id = $1",
        &[&w.tenant, &language_conf(tenant)],
    )
    .await;
}

async fn languages_of(w: &World) -> Vec<String> {
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    page_languages(&tx, w.tenant, w.event).await.unwrap()
}

/// The page is in the event's default language, else the tenant's.
#[tokio::test]
async fn the_page_languages_are_the_events_then_the_tenants() {
    let w = world("madrid-pe").await;
    set_languages(&w, Some("es"), Some("fr")).await;
    assert_eq!(languages_of(&w).await, ["es", "fr"]);
    set_languages(&w, None, Some("fr")).await;
    assert_eq!(languages_of(&w).await, ["fr"]);
    set_languages(&w, Some(" "), None).await;
    assert!(languages_of(&w).await.is_empty());
    // Another tenant's event of the same id is not this event.
    set_languages(&w, Some("es"), None).await;
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let other_tenant = Uuid::new_v4();
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
        &[&other_tenant, &format!("tenant-{other_tenant}")],
    )
    .await
    .unwrap();
    assert!(page_languages(&tx, other_tenant, w.event)
        .await
        .unwrap()
        .is_empty());
}

/// A language the texts don't have prints the page in English.
#[tokio::test]
async fn an_unknown_language_prints_the_page_in_english() {
    let w = world("madrid-pe").await;
    set_languages(&w, Some("xx"), Some("yy")).await;
    w.rule(ER, 2, RequesterSigning::NotAllowed, None).await;
    let base = signature_page(&w, ER, &report_pdf()).await.unwrap();
    assert!(contains(
        &base,
        "We certify that these election returns are true and correct."
    ));
    assert!(contains(&base, "(Signature 1)"));

    let store = MemoryStore::default();
    let signers = pdf_signers(&w, ER, 1).await;
    let (request_id, _) = start_on(&w, &base, store.put(&base)).await.unwrap();
    let prepared = prepare(&w, &store, &signers[0], request_id, at(1))
        .await
        .unwrap();
    let row = revisions(&w, request_id)
        .await
        .into_iter()
        .find(|row| row.revision == prepared.revision)
        .unwrap();
    let appearance: StoredAppearance = serde_json::from_value(row.appearance.unwrap()).unwrap();
    assert_eq!(appearance.lines[0], "Digitally signed by sbei-0");
    assert_eq!(
        appearance.reason.as_deref(),
        english().certify.get(&ER).map(String::as_str)
    );
}

/// A Spanish event prints its page and signatures in Spanish; an event
/// without a language, in its tenant's (French here).
#[tokio::test]
async fn a_spanish_event_prints_the_page_in_spanish() {
    let w = world("madrid-pe").await;
    w.rule(ER, 2, RequesterSigning::NotAllowed, None).await;
    set_languages(&w, Some("es"), Some("fr")).await;
    let base = signature_page(&w, ER, &report_pdf()).await.unwrap();
    assert!(contains(
        &base,
        "Certificamos que estas actas electorales son"
    ));
    assert!(contains(&base, ": firmas digitales)"));
    assert!(contains(&base, "(Firma 1)"));
    assert!(!contains(&base, "We certify"));

    let store = MemoryStore::default();
    let signers = pdf_signers(&w, ER, 1).await;
    let (request_id, code) = start_on(&w, &base, store.put(&base)).await.unwrap();
    let prepared = prepare(&w, &store, &signers[0], request_id, at(1))
        .await
        .unwrap();
    let row = revisions(&w, request_id)
        .await
        .into_iter()
        .find(|row| row.revision == prepared.revision)
        .unwrap();
    let appearance: StoredAppearance = serde_json::from_value(row.appearance.unwrap()).unwrap();
    assert_eq!(appearance.lines[0], "Firmado digitalmente por sbei-0");
    assert!(appearance
        .lines
        .contains(&format!("Código de firma: {code}")));
    assert!(appearance
        .reason
        .as_deref()
        .unwrap()
        .starts_with("Certificamos que estas actas electorales"));

    set_languages(&w, None, Some("fr")).await;
    let base = signature_page(&w, ER, &report_pdf()).await.unwrap();
    assert!(contains(&base, "Nous certifions que ces proc"));
}

/// A signer's title is the one the signing panel shows: their `title`
/// attribute, else the group that grants them the permission.
#[tokio::test]
async fn a_signers_title_comes_from_the_signer_directory() {
    let w = world("madrid-pe").await;
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    keycloak_tables(&tx).await;
    let sign = ER.sign_permission().to_string();
    tx.batch_execute(&format!(
        "INSERT INTO realm VALUES ('r', 'tenant-{tenant}');
         INSERT INTO keycloak_role VALUES ('role', '{sign}', 'r');
         INSERT INTO keycloak_group (id, name, realm_id) VALUES ('g', 'Board of Inspectors', 'r');
         INSERT INTO group_role_mapping VALUES ('role', 'g');
         INSERT INTO user_entity VALUES ('maria', 'maria', 'Maria', 'Santos', true, 'r', NULL),
             ('jose', 'jose', 'Jose', 'Rizal', true, 'r', NULL),
             ('outsider', 'outsider', 'Out', 'Sider', true, 'r', NULL);
         INSERT INTO user_group_membership VALUES ('g', 'maria'), ('g', 'jose');
         INSERT INTO user_attribute VALUES ('title', 'Chairperson', 'maria'),
             ('title', 'Treasurer', 'outsider');",
        tenant = w.tenant
    ))
    .await
    .unwrap();
    let realm = format!("tenant-{}", w.tenant);
    let title = |user: &'static str| signer_title(&tx, &realm, ER, user);
    assert_eq!(
        title("maria").await.unwrap().as_deref(),
        Some("Chairperson")
    );
    assert_eq!(
        title("jose").await.unwrap().as_deref(),
        Some("Board of Inspectors")
    );
    // Not a signer of the action: no title.
    assert_eq!(title("outsider").await.unwrap(), None);
    assert_eq!(
        signer_title(&tx, &realm, SigningAction::GenerateReports, "maria")
            .await
            .unwrap(),
        None
    );
}

async fn refusal_checks(w: &World) -> Vec<String> {
    w.pool
        .get()
        .await
        .unwrap()
        .query(
            "SELECT body->'details'->>'check' FROM sequent_backend.signing_log_outbox
             WHERE election_event_id = $1 AND statement_kind = 'SigningSignatureRefused'
                 AND entry = 1 ORDER BY id",
            &[&w.event],
        )
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.get(0))
        .collect()
}

/// Preparing a PDF for a request whose action signs none is refused and
/// logged, as approve refuses a document signature its action doesn't
/// take; within the throttle, once.
#[tokio::test]
async fn preparing_a_pdf_for_an_action_without_one_is_refused_and_logged() {
    for (label, required, _) in PRESETS {
        let w = world(label).await;
        let close = SigningAction::CloseVoting;
        w.rule(close, required, RequesterSigning::NotAllowed, None)
            .await;
        let request = w
            .start(
                &caller("operator", &[], &[&w.label]),
                close,
                subject(1),
                at(0),
            )
            .await;
        let store = MemoryStore::default();
        let signer = &pdf_signers(&w, close, 1).await[0];
        for _ in 0..2 {
            assert!(
                matches!(
                    prepare(&w, &store, signer, request.id, at(1)).await,
                    Err(SigningError::Invalid {
                        reason: InvalidReason::Document,
                        ..
                    })
                ),
                "{label}"
            );
        }
        assert_eq!(refusal_checks(&w).await, ["document"], "{label}");
        assert!(revisions(&w, request.id).await.is_empty());
        assert_eq!(
            w.request(request.id).await.status,
            SigningRequestStatus::Waiting
        );
        w.assert_two_entries_per_step().await;
    }
}
