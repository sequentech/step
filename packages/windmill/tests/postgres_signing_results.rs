// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Transmitting results with signatures: a transmission package waits for
//! its request; the Post's transmission configuration decides who signs and
//! as whom; the last signature builds the signed package once, for the
//! package's own destinations, with the approvals' EML signatures in the 2025
//! members format, and sends nothing; only the package the request built is
//! sent; the 2025 upload is refused while signing applies. Without the rule
//! nothing changes.
//!
//! Each test commits its own tenant, since the approve step owns its
//! transactions, and runs under two configurations (with and without a
//! transmission configuration of the Post's people); numbers, labels and
//! identities come from the configuration used.

#[path = "support/schema.rs"]
mod schema;
#[path = "support/signing.rs"]
mod signing;

use async_trait::async_trait;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use deadpool_postgres::Transaction;
use openssl::hash::MessageDigest;
use openssl::pkey::{PKey, Private};
use openssl::sign::{Signer, Verifier};
use sequent_core::signing::{
    CancelReason, CertificateCheckId, RequesterSigning, SignatureAlgorithm, SigningAction,
    SigningRequestStatus,
};
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Value};
use signing::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use windmill::postgres::signing::*;
use windmill::services::consolidation::eml_types::ACMTrustee;
use windmill::services::consolidation::send_transmission_package_service::TransmissionSignaturesShort;
use windmill::services::consolidation::xz_compress::xz_compress;
use windmill::services::signing::actions::eml::{DocumentSigners, EmlDocumentSigner};
use windmill::services::signing::actions::transmission::{
    guard_transmission_package, normalise_fingerprint, sha256_hex, sign_package,
    transmission_send_check, transmission_subject_key, upload_signature_refusal, LoadedPackage,
    PackageToSign, PostSbeis, SbeiDirectory, SbeiIdentity, TransmissionPackages,
    TransmissionRefusal, TransmissionSendCheck,
};
use windmill::services::signing::actions::{
    executors, run_dispatched, EffectProgress, NoSeal, RunOutcome, SignedActionDispatcher,
    SignedActionEffects, SignedActionTask,
};
use windmill::services::signing::approve::{
    approve, ApproveInput, ApproveOutcome, DocumentSigner, NoDocumentSigner, SigningServices,
};
use windmill::services::signing::executors::{PostCommit, PostCommitTask};
use windmill::services::signing::pdf::{DocumentLink, RevisionStore};
use windmill::services::signing::{SigningCaller, SigningError, SigningResult};
use windmill::types::miru_plugin::{
    MiruCcsServer, MiruDocument, MiruDocumentIds, MiruSignature, MiruSigningRequest,
    MiruTransmissionPackageData,
};

const ACTION: SigningAction = SigningAction::TransmitResults;

/// A configuration: the Post label, how many sign, the destinations, and
/// whether the Post's people have a transmission configuration.
struct Preset {
    label: &'static str,
    required: u16,
    destinations: &'static [&'static str],
    configured: bool,
}

const PRESETS: [Preset; 2] = [
    Preset {
        label: "madrid-pe",
        required: 3,
        destinations: &["transparency", "central", "ccs-manila"],
        configured: true,
    },
    Preset {
        label: "faculty-of-science",
        required: 2,
        destinations: &["returning-office"],
        configured: false,
    },
];

const EML: &[u8] = b"<EML><Count>42</Count></EML>";

fn servers(names: &[&str]) -> Vec<MiruCcsServer> {
    names
        .iter()
        .map(|name| MiruCcsServer {
            name: name.to_string(),
            tag: format!("tag-{name}"),
            address: format!("https://{name}.invalid"),
            public_key_pem: String::new(),
            send_logs: None,
        })
        .collect()
}

/// A package's stored parts, owned for the test.
struct Parts {
    eml: Vec<u8>,
    compressed: Vec<u8>,
    servers: Vec<MiruCcsServer>,
    eml_document_id: Uuid,
}

impl Parts {
    fn new(destinations: &[&str], eml: &[u8]) -> Self {
        Parts {
            eml: eml.to_vec(),
            compressed: xz_compress(eml).unwrap(),
            servers: servers(destinations),
            eml_document_id: Uuid::new_v4(),
        }
    }

    fn to_sign<'a>(&'a self, w: &World) -> PackageToSign<'a> {
        PackageToSign {
            tenant_id: w.tenant,
            election_event_id: w.event,
            election_id: w.post,
            area_id: w.area,
            tally_session_id: "tally-1",
            eml_document_id: self.eml_document_id,
            eml: &self.eml,
            package: &self.compressed,
            servers: &self.servers,
        }
    }
}

/// A signer's certificate with its key, so its EML signature is real.
struct Signatory {
    caller: SigningCaller,
    certificate: TestCert,
    key: PKey<Private>,
}

fn self_signed(common_name: &str) -> (String, PKey<Private>) {
    use openssl::asn1::Asn1Time;
    use openssl::rsa::Rsa;
    use openssl::x509::{X509NameBuilder, X509};
    let key = PKey::from_rsa(Rsa::generate(2048).unwrap()).unwrap();
    let mut name = X509NameBuilder::new().unwrap();
    name.append_entry_by_text("CN", common_name).unwrap();
    let name = name.build();
    let mut builder = X509::builder().unwrap();
    builder.set_version(2).unwrap();
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
    (
        String::from_utf8(builder.build().to_pem().unwrap()).unwrap(),
        key,
    )
}

/// `n` signers of the Post with registered certificates.
async fn signatories(w: &World, n: usize) -> Vec<Signatory> {
    let mut all = vec![];
    for i in 0..n {
        let user = format!("sbei-{i}");
        let (pem, key) = self_signed(&format!("Member {i}"));
        let certificate = cert_with_pem(&user, &user, &user, &pem);
        w.register(&user, &certificate, None).await;
        all.push(Signatory {
            caller: w.signer(&user, ACTION),
            certificate,
            key,
        });
    }
    all
}

/// The Post's transmission configuration: each signer by its username,
/// under the id and name the central servers know.
fn configuration(signatories: &[Signatory]) -> PostSbeis {
    signatories
        .iter()
        .enumerate()
        .map(|(i, signatory)| {
            (
                signatory.caller.username.clone(),
                SbeiIdentity {
                    miru_id: format!("SBEI-{}", 70 + i),
                    miru_name: format!("Member {i} of the board"),
                    certificate_fingerprint: None,
                },
            )
        })
        .collect()
}

fn rsa_sign(key: &PKey<Private>, bytes: &[u8]) -> Vec<u8> {
    let mut signer = Signer::new(MessageDigest::sha256(), key).unwrap();
    signer.update(bytes).unwrap();
    signer.sign_to_vec().unwrap()
}

/// The person who creates the package.
fn operator(w: &World) -> SigningCaller {
    caller("operator", &[Permissions::MIRU_CREATE], &[&w.label])
}

async fn guard_package(
    w: &World,
    requester: Option<&SigningCaller>,
    package: &PackageToSign<'_>,
) -> SigningResult<Option<MiruSigningRequest>> {
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let outcome = guard_transmission_package(&tx, requester, package).await;
    if outcome.is_ok() {
        tx.commit().await.unwrap();
    }
    outcome
}

async fn transmit_requests(w: &World) -> Vec<SigningRequestRow> {
    let client = w.pool.get().await.unwrap();
    client
        .query(
            "SELECT * FROM sequent_backend.signing_request
             WHERE tenant_id = $1 AND election_event_id = $2 AND action = $3
             ORDER BY created_at, id",
            &[&w.tenant, &w.event, &ACTION.to_string()],
        )
        .await
        .unwrap()
        .into_iter()
        .map(|row| SigningRequestRow::try_from(row).unwrap())
        .collect()
}

#[tokio::test]
async fn without_the_rule_a_package_starts_no_request() {
    for preset in &PRESETS {
        let w = world(preset.label).await;
        let parts = Parts::new(preset.destinations, EML);
        let outcome = guard_package(&w, Some(&operator(&w)), &parts.to_sign(&w))
            .await
            .unwrap();
        assert_eq!(outcome, None, "{}", preset.label);
        // Not even without a requester: today's flow knows none.
        let outcome = guard_package(&w, None, &parts.to_sign(&w)).await.unwrap();
        assert_eq!(outcome, None, "{}", preset.label);
        assert!(transmit_requests(&w).await.is_empty(), "{}", preset.label);
        assert!(w.steps().await.is_empty(), "{}", preset.label);
    }
}

#[tokio::test]
async fn with_the_rule_a_package_waits_for_a_request_signing_its_eml() {
    for preset in &PRESETS {
        let label = preset.label;
        let w = world(label).await;
        w.rule(ACTION, preset.required, RequesterSigning::NotAllowed, None)
            .await;
        let parts = Parts::new(preset.destinations, EML);
        let waiting = guard_package(&w, Some(&operator(&w)), &parts.to_sign(&w))
            .await
            .unwrap()
            .expect("a request");
        assert_eq!(waiting.required, i32::from(preset.required), "{label}");

        let row = w.request(waiting.id.parse().unwrap()).await;
        assert_eq!(row.code, waiting.code);
        assert_eq!(row.status, SigningRequestStatus::Waiting);
        assert_eq!((row.election_id, row.area_id), (Some(w.post), Some(w.area)));
        assert_eq!(row.document_id, Some(parts.eml_document_id));
        assert_eq!(row.document_sha256, Some(sha256_hex(EML)));
        let mut sorted: Vec<String> = preset.destinations.iter().map(|d| d.to_string()).collect();
        sorted.sort();
        assert_eq!(
            row.subject,
            json!({
                "tally_session_id": "tally-1",
                "package_sha256": sha256_hex(&parts.compressed),
                "eml_sha256": sha256_hex(EML),
                "destinations": sorted,
            }),
            "{label}"
        );
        assert!(row
            .scope_key
            .ends_with(&transmission_subject_key("tally-1", w.area)));

        // The same package again: the same request.
        let again = guard_package(&w, Some(&operator(&w)), &parts.to_sign(&w))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(again, waiting, "{label}");

        // A package created again with other results: a new request, and
        // the earlier one's signatures no longer count.
        let recount = Parts::new(preset.destinations, b"<EML><Count>43</Count></EML>");
        let changed = guard_package(&w, Some(&operator(&w)), &recount.to_sign(&w))
            .await
            .unwrap()
            .unwrap();
        assert_ne!(changed.id, waiting.id);
        let earlier = w.request(waiting.id.parse().unwrap()).await;
        assert_eq!(earlier.status, SigningRequestStatus::Cancelled);
        assert_eq!(earlier.cancel_reason, Some(CancelReason::PayloadChanged));
    }
}

#[tokio::test]
async fn a_package_needing_signatures_needs_a_requester_of_its_post() {
    let w = world("madrid-pe").await;
    w.rule(ACTION, 2, RequesterSigning::NotAllowed, None).await;
    let parts = Parts::new(&["central"], EML);
    assert!(matches!(
        guard_package(&w, None, &parts.to_sign(&w)).await,
        Err(SigningError::Forbidden(_))
    ));
    let elsewhere = caller("operator", &[Permissions::MIRU_CREATE], &["other-post"]);
    assert!(matches!(
        guard_package(&w, Some(&elsewhere), &parts.to_sign(&w)).await,
        Err(SigningError::Forbidden(_))
    ));
    assert!(transmit_requests(&w).await.is_empty());
}

/// Records the tasks the approve step sends.
#[derive(Default)]
struct RecordingDispatcher {
    sent: Arc<Mutex<Vec<SignedActionTask>>>,
}

struct RecordedTask(SignedActionTask, Arc<Mutex<Vec<SignedActionTask>>>);

#[async_trait]
impl PostCommitTask for RecordedTask {
    async fn send(&self) -> anyhow::Result<()> {
        self.1.lock().unwrap().push(self.0);
        Ok(())
    }
}

impl SignedActionDispatcher for RecordingDispatcher {
    fn task(&self, task: SignedActionTask) -> PostCommit {
        PostCommit::SendTask(Box::new(RecordedTask(task, self.sent.clone())))
    }
}

/// The stored EML of each request document.
struct EmlStore(HashMap<Uuid, Vec<u8>>);

#[async_trait]
impl RevisionStore for EmlStore {
    async fn load(
        &self,
        _tx: &Transaction<'_>,
        _tenant_id: Uuid,
        _election_event_id: Uuid,
        document_id: Uuid,
    ) -> anyhow::Result<Vec<u8>> {
        self.0
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
        _content: &[u8],
    ) -> anyhow::Result<Uuid> {
        anyhow::bail!("an EML request stores no revision")
    }

    async fn link(
        &self,
        _tx: &Transaction<'_>,
        _tenant_id: Uuid,
        _election_event_id: Uuid,
        document_id: Uuid,
    ) -> anyhow::Result<Option<DocumentLink>> {
        Ok(self.0.contains_key(&document_id).then(|| DocumentLink {
            url: format!("https://documents.invalid/{document_id}"),
            name: Some("er.xml".into()),
        }))
    }
}

/// The Post's transmission configuration, or none.
struct Configured(Option<PostSbeis>);

#[async_trait]
impl SbeiDirectory for Configured {
    async fn post_sbeis(
        &self,
        _tx: &Transaction<'_>,
        _request: &SigningRequestRow,
    ) -> anyhow::Result<Option<PostSbeis>> {
        Ok(self.0.clone())
    }
}

/// One package in a tally session, kept in memory; records each build.
struct Packages {
    package: Mutex<MiruTransmissionPackageData>,
    eml: Vec<u8>,
    compressed: Vec<u8>,
    sbeis: Option<PostSbeis>,
    builds: Mutex<Vec<(Vec<ACMTrustee>, Vec<MiruSignature>)>>,
}

impl Packages {
    fn new(signing_request: &MiruSigningRequest, parts: &Parts, sbeis: Option<PostSbeis>) -> Self {
        Packages {
            package: Mutex::new(MiruTransmissionPackageData {
                election_id: "post".into(),
                area_id: "country".into(),
                servers: parts.servers.clone(),
                documents: vec![MiruDocument {
                    document_ids: MiruDocumentIds {
                        eml: "eml".into(),
                        xz: "xz".into(),
                        all_servers: "unsigned".into(),
                    },
                    transaction_id: "tx".into(),
                    servers_sent_to: vec![],
                    created_at: "2028-05-12T10:00:00+08:00".into(),
                    signatures: vec![],
                }],
                logs: vec![],
                threshold: i64::from(signing_request.required),
                signing_request: Some(signing_request.clone()),
            }),
            eml: parts.eml.clone(),
            compressed: parts.compressed.clone(),
            sbeis,
            builds: Mutex::new(vec![]),
        }
    }

    fn builds(&self) -> Vec<(Vec<ACMTrustee>, Vec<MiruSignature>)> {
        self.builds.lock().unwrap().clone()
    }

    fn current(&self) -> MiruTransmissionPackageData {
        self.package.lock().unwrap().clone()
    }
}

#[async_trait]
impl TransmissionPackages for Packages {
    async fn load(
        &self,
        _tx: &Transaction<'_>,
        _request: &SigningRequestRow,
        tally_session_id: &str,
    ) -> anyhow::Result<LoadedPackage> {
        Ok(LoadedPackage {
            tally_session_id: tally_session_id.into(),
            package: self.current(),
            eml: self.eml.clone(),
            compressed: self.compressed.clone(),
            sbeis: self.sbeis.clone(),
        })
    }

    async fn store_signed(
        &self,
        _tx: &Transaction<'_>,
        _request: &SigningRequestRow,
        _loaded: &LoadedPackage,
        members: Vec<ACMTrustee>,
        signatures: Vec<MiruSignature>,
    ) -> anyhow::Result<String> {
        self.builds
            .lock()
            .unwrap()
            .push((members, signatures.clone()));
        let mut package = self.package.lock().unwrap();
        let mut signed = package.documents[0].clone();
        signed.document_ids.all_servers = "signed".into();
        signed.signatures = signatures;
        package.documents.push(signed);
        Ok("signed".into())
    }
}

/// The effect of a transmission, on the in-memory packages.
struct TransmitEffects(Arc<Packages>);

#[async_trait]
impl SignedActionEffects for TransmitEffects {
    async fn run(
        &self,
        tx: &Transaction<'_>,
        request: &SigningRequestRow,
        approvals: &[SigningApprovalRow],
        progress: &EffectProgress,
    ) -> anyhow::Result<Value> {
        sign_package(tx, self.0.as_ref(), request, approvals, progress).await
    }
}

struct Transmission {
    w: World,
    request: MiruSigningRequest,
    parts: Parts,
    signatories: Vec<Signatory>,
    /// The Post's transmission configuration, when the preset has one.
    sbeis: Option<PostSbeis>,
    services: SigningServices,
    sent: Arc<Mutex<Vec<SignedActionTask>>>,
}

fn services_for(
    signatories: &[Signatory],
    documents: EmlStore,
    sbeis: Option<PostSbeis>,
    dispatcher: RecordingDispatcher,
) -> SigningServices {
    let certificates: Vec<&TestCert> = signatories.iter().map(|s| &s.certificate).collect();
    SigningServices {
        documents: Arc::new(DocumentSigners::new(
            Arc::new(EmlDocumentSigner::new(
                Arc::new(documents),
                Arc::new(Configured(sbeis)),
            )),
            Arc::new(NoDocumentSigner),
        )),
        ..signing::services(
            FakeVerifier::knowing(&certificates),
            executors(Arc::new(dispatcher)),
        )
    }
}

async fn transmission(preset: &Preset) -> Transmission {
    let w = world(preset.label).await;
    w.rule(ACTION, preset.required, RequesterSigning::NotAllowed, None)
        .await;
    let parts = Parts::new(preset.destinations, EML);
    let request = guard_package(&w, Some(&operator(&w)), &parts.to_sign(&w))
        .await
        .unwrap()
        .unwrap();
    let signatories = signatories(&w, usize::from(preset.required)).await;
    let sbeis = preset.configured.then(|| configuration(&signatories));
    let dispatcher = RecordingDispatcher::default();
    let sent = dispatcher.sent.clone();
    let services = services_for(
        &signatories,
        EmlStore(HashMap::from([(parts.eml_document_id, EML.to_vec())])),
        sbeis.clone(),
        dispatcher,
    );
    Transmission {
        w,
        request,
        parts,
        signatories,
        sbeis,
        services,
        sent,
    }
}

impl Transmission {
    fn id(&self) -> Uuid {
        self.request.id.parse().unwrap()
    }

    /// Signer `i` signs the payload and the EML with `services`.
    async fn sign_with(
        &self,
        services: &SigningServices,
        i: usize,
    ) -> SigningResult<ApproveOutcome> {
        let signatory = &self.signatories[i];
        let row = self.w.request(self.id()).await;
        let mut client = self.w.pool.get().await.unwrap();
        approve(
            &mut client,
            services,
            &signatory.caller,
            self.w.tenant,
            &ApproveInput {
                request_id: self.id(),
                chain_pem: vec![signatory.certificate.identity.pem.clone()],
                algorithm: SignatureAlgorithm::RsaPkcs1Sha256,
                payload_signature: signature_over(&row.canonical_payload),
                document_signature: Some(rsa_sign(&signatory.key, EML)),
                pdf_cms: None,
                revision: None,
            },
            at(1),
        )
        .await
    }

    async fn sign(&self, i: usize) -> SigningResult<ApproveOutcome> {
        self.sign_with(&self.services, i).await
    }

    async fn sign_all(&self) -> SignedActionTask {
        for i in 0..self.signatories.len() {
            self.sign(i).await.unwrap();
        }
        let sent = self.sent.lock().unwrap();
        assert_eq!(sent.len(), 1);
        sent[0]
    }

    async fn run(&self, effects: &dyn SignedActionEffects, task: &SignedActionTask) -> RunOutcome {
        let mut client = self.w.pool.get().await.unwrap();
        run_dispatched(&mut client, effects, &NoSeal, task)
            .await
            .unwrap()
    }

    async fn send_check(
        &self,
        package: &MiruTransmissionPackageData,
    ) -> Result<TransmissionSendCheck, TransmissionRefusal> {
        let mut client = self.w.pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        transmission_send_check(&tx, self.w.tenant, self.w.event, package)
            .await
            .unwrap()
    }
}

/// What a central server checks: the member's signature of the EML with
/// the member's public key.
fn verifies(member: &ACMTrustee, eml: &[u8]) -> bool {
    let key = PKey::public_key_from_pem(member.publickey.as_ref().unwrap().as_bytes()).unwrap();
    let signature = BASE64.decode(member.signature.as_ref().unwrap()).unwrap();
    let mut verifier = Verifier::new(MessageDigest::sha256(), &key).unwrap();
    verifier.update(eml).unwrap();
    verifier.verify(&signature).unwrap()
}

#[tokio::test]
async fn the_last_signature_builds_the_signed_package_once_and_sends_nothing() {
    for preset in &PRESETS {
        let label = preset.label;
        let required = preset.required;
        let t = transmission(preset).await;
        let packages = Arc::new(Packages::new(&t.request, &t.parts, t.sbeis.clone()));
        let effects = TransmitEffects(packages.clone());

        for i in 0..usize::from(required) - 1 {
            let outcome = t.sign(i).await.unwrap();
            assert_eq!(outcome.status, SigningRequestStatus::Waiting, "{label}");
        }
        assert!(
            t.sent.lock().unwrap().is_empty(),
            "{label}: dispatched early"
        );
        assert!(packages.builds().is_empty());
        // Short of its signatures, the package can't be sent.
        assert_eq!(
            t.send_check(&packages.current()).await,
            Err(TransmissionRefusal::NotSigned {
                code: Some(t.request.code.clone()),
                status: Some(SigningRequestStatus::Waiting),
            }),
            "{label}"
        );

        let last = t.sign(usize::from(required) - 1).await.unwrap();
        // The task builds the package; the request ran when it reports.
        assert_eq!(last.status, SigningRequestStatus::Completed, "{label}");
        let sent = t.sent.lock().unwrap().clone();
        assert_eq!(sent.len(), 1, "{label}");
        assert_eq!(sent[0].request_id, t.id());
        assert!(
            t.send_check(&packages.current()).await.is_err(),
            "{label}: sendable before it ran"
        );

        let run = t.run(&effects, &sent[0]).await;
        assert!(matches!(run, RunOutcome::Executed(_)), "{label}: {run:?}");
        // A second copy of the task builds nothing.
        assert_eq!(t.run(&effects, &sent[0]).await, RunOutcome::NotClaimed);

        let builds = packages.builds();
        assert_eq!(builds.len(), 1, "{label}");
        let (members, signatures) = &builds[0];
        assert_eq!(members.len(), usize::from(required), "{label}");
        for (i, member) in members.iter().enumerate() {
            // Who the central servers know, from the configuration in use.
            let (id, name) = match &t.sbeis {
                Some(sbeis) => {
                    let identity = &sbeis[&t.signatories[i].caller.username];
                    (identity.miru_id.clone(), identity.miru_name.clone())
                }
                None => (
                    t.signatories[i].caller.username.clone(),
                    t.signatories[i].caller.display_name.clone(),
                ),
            };
            assert_eq!((&member.id, &member.name), (&id, &name), "{label}");
            assert!(
                member
                    .publickey
                    .as_ref()
                    .unwrap()
                    .starts_with("-----BEGIN PUBLIC KEY-----"),
                "{label}"
            );
            assert!(verifies(member, EML), "{label}: member {i}");
            assert!(!verifies(member, b"other results"));
            assert_eq!(signatures[i].sbei_miru_id, member.id);
            assert_eq!(Some(&signatures[i].signature), member.signature.as_ref());
            assert_eq!(Some(&signatures[i].pub_key), member.publickey.as_ref());
            assert_eq!(
                signatures[i].certificate_fingerprint,
                t.signatories[i].certificate.identity.fingerprint_sha256
            );
        }
        // The 2025 wire format: exactly these keys.
        let wire = serde_json::to_value(&members[0]).unwrap();
        let mut keys: Vec<&String> = wire.as_object().unwrap().keys().collect();
        keys.sort();
        assert_eq!(keys, ["id", "name", "publickey", "signature"]);

        let row = t.w.request(t.id()).await;
        assert_eq!(row.status, SigningRequestStatus::Executed, "{label}");
        let result = row.execution_result.unwrap();
        assert_eq!(result["signatures"], json!(required));
        assert_eq!(result["eml_sha256"], json!(sha256_hex(EML)));
        assert_eq!(
            result["package_sha256"],
            json!(sha256_hex(&t.parts.compressed))
        );
        assert_eq!(result["all_servers_document_id"], json!("signed"));

        // Now it goes, the document it built, with the approvals' count.
        let signed = packages.current();
        assert_eq!(
            t.send_check(&signed).await,
            Ok(TransmissionSendCheck::Signed {
                count: i64::from(required)
            }),
            "{label}"
        );
        let steps = t.w.steps().await;
        assert_eq!(
            steps.last().map(String::as_str),
            Some("SigningActionExecuted")
        );
        t.w.assert_two_entries_per_step().await;
    }
}

#[tokio::test]
async fn only_the_package_the_request_built_is_sent() {
    let t = transmission(&PRESETS[1]).await;
    let packages = Arc::new(Packages::new(&t.request, &t.parts, None));
    let task = t.sign_all().await;
    t.run(&TransmitEffects(packages.clone()), &task).await;
    let signed = packages.current();
    assert!(t.send_check(&signed).await.is_ok());
    let not_it = Err(TransmissionRefusal::NotTheSignedPackage {
        code: t.request.code.clone(),
    });

    // A later document (a 2025 upload, another build): not the one signed.
    let mut later = signed.clone();
    let mut extra = later.documents.last().unwrap().clone();
    extra.document_ids.all_servers = "rebuilt".into();
    later.documents.push(extra);
    assert_eq!(t.send_check(&later).await, not_it);

    // The built document, with a signature that is not an approval's.
    let mut forged = signed.clone();
    forged.documents.last_mut().unwrap().signatures[0].signature = BASE64.encode(b"forged");
    assert_eq!(t.send_check(&forged).await, not_it);
    // ... or one signature short.
    let mut short = signed.clone();
    short.documents.last_mut().unwrap().signatures.pop();
    assert_eq!(t.send_check(&short).await, not_it);
}

/// Why `sign_package` refuses `packages`.
async fn refused(
    tx: &Transaction<'_>,
    request: &SigningRequestRow,
    approvals: &[SigningApprovalRow],
    packages: &Packages,
) -> String {
    sign_package(tx, packages, request, approvals, &EffectProgress::default())
        .await
        .unwrap_err()
        .to_string()
}

#[tokio::test]
async fn a_package_that_changed_after_signing_is_not_built() {
    let preset = &PRESETS[1];
    let t = transmission(preset).await;
    let task = t.sign_all().await;
    // The tally session now holds other results under the same request.
    let recount = Parts::new(preset.destinations, b"<EML>recount</EML>");
    let packages = Arc::new(Packages::new(&t.request, &recount, None));
    let run = t.run(&TransmitEffects(packages.clone()), &task).await;
    // A refusal: not tried again, nothing built.
    assert_eq!(run, RunOutcome::Failed("package-changed".into()));
    assert!(packages.builds().is_empty());
    let row = t.w.request(t.id()).await;
    assert_eq!(row.status, SigningRequestStatus::Failed);
    assert!(row.executed_at.is_none());

    let mut client = t.w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let approvals = list_signing_approvals(&tx, t.w.tenant, t.w.event, t.id())
        .await
        .unwrap();
    let request = get_signing_request(&tx, t.w.tenant, t.w.event, t.id())
        .await
        .unwrap()
        .unwrap();
    // Another request's package.
    let other = MiruSigningRequest {
        id: Uuid::new_v4().to_string(),
        ..t.request.clone()
    };
    let refusal = refused(
        &tx,
        &request,
        &approvals,
        &Packages::new(&other, &t.parts, None),
    )
    .await;
    assert!(refusal.starts_with("package-changed"), "{refusal}");
    // The compressed results are not the EML's.
    let mut tampered = Packages::new(&t.request, &t.parts, None);
    tampered.compressed = xz_compress(b"<EML>other</EML>").unwrap();
    let refusal = refused(&tx, &request, &approvals, &tampered).await;
    assert!(refusal.contains("compressed results"), "{refusal}");
    // The stored destinations are not the signed ones.
    let moved = Packages::new(&t.request, &t.parts, None);
    moved.package.lock().unwrap().servers = servers(&["somewhere-else"]);
    let refusal = refused(&tx, &request, &approvals, &moved).await;
    assert!(refusal.contains("destinations"), "{refusal}");
    // A signer the configuration doesn't know.
    let refusal = refused(
        &tx,
        &request,
        &approvals,
        &Packages::new(&t.request, &t.parts, Some(HashMap::new())),
    )
    .await;
    assert!(refusal.starts_with("unmapped-signer"), "{refusal}");
}

#[tokio::test]
async fn the_transmission_configuration_decides_who_signs() {
    let preset = &PRESETS[0];
    let t = transmission(preset).await;
    let store = || EmlStore(HashMap::from([(t.parts.eml_document_id, EML.to_vec())]));
    let refused_check = |result: SigningResult<ApproveOutcome>| match result {
        Err(SigningError::Refused { check, .. }) => check,
        other => panic!("expected a refusal, got {other:?}"),
    };
    // Sbei-0 is not in the configuration.
    let mut without_first = configuration(&t.signatories);
    without_first.remove(&t.signatories[0].caller.username);
    let services = services_for(
        &t.signatories,
        store(),
        Some(without_first),
        RecordingDispatcher::default(),
    );
    assert_eq!(
        refused_check(t.sign_with(&services, 0).await),
        CertificateCheckId::Registered
    );
    // A 2025 signature pinned another certificate to sbei-0, in OpenSSL's format.
    let mut pinned = configuration(&t.signatories);
    pinned
        .get_mut(&t.signatories[0].caller.username)
        .unwrap()
        .certificate_fingerprint = Some(format!("sha256 Fingerprint={}", "AB:".repeat(31) + "AB"));
    let services = services_for(
        &t.signatories,
        store(),
        Some(pinned.clone()),
        RecordingDispatcher::default(),
    );
    assert_eq!(
        refused_check(t.sign_with(&services, 0).await),
        CertificateCheckId::Registered
    );
    assert!(t.w.approvals(t.id()).await.is_empty());
    // Pinned to their own certificate, written the 2025 way: they sign.
    let own = &t.signatories[0].certificate.identity.fingerprint_sha256;
    let colons: Vec<String> = own
        .to_uppercase()
        .as_bytes()
        .chunks(2)
        .map(|pair| String::from_utf8(pair.to_vec()).unwrap())
        .collect();
    pinned
        .get_mut(&t.signatories[0].caller.username)
        .unwrap()
        .certificate_fingerprint = Some(format!("sha256 Fingerprint={}", colons.join(":")));
    let services = services_for(
        &t.signatories,
        store(),
        Some(pinned),
        RecordingDispatcher::default(),
    );
    assert_eq!(t.sign_with(&services, 0).await.unwrap().count, 1);
}

#[test]
fn fingerprints_compare_in_any_format() {
    let hex = "ab".repeat(32);
    assert_eq!(normalise_fingerprint(&hex), hex);
    assert_eq!(
        normalise_fingerprint(&format!("sha256 Fingerprint={}", "AB:".repeat(31) + "AB")),
        hex
    );
    assert_eq!(normalise_fingerprint(&"AB".repeat(32)), hex);
}

#[tokio::test]
async fn the_2025_upload_is_refused_while_signing_applies() {
    for preset in &PRESETS {
        let w = world(preset.label).await;
        let unsigned = MiruTransmissionPackageData {
            election_id: w.post.to_string(),
            area_id: w.area.to_string(),
            servers: servers(preset.destinations),
            documents: vec![],
            logs: vec![],
            threshold: 0,
            signing_request: None,
        };
        let refusal = |package: MiruTransmissionPackageData| {
            let w = w.clone();
            async move {
                let mut client = w.pool.get().await.unwrap();
                let tx = client.transaction().await.unwrap();
                upload_signature_refusal(&tx, w.tenant, w.event, &package)
                    .await
                    .unwrap()
            }
        };
        assert_eq!(refusal(unsigned.clone()).await, None, "{}", preset.label);
        // With the rule, no package takes one.
        w.rule(ACTION, preset.required, RequesterSigning::NotAllowed, None)
            .await;
        assert_eq!(
            refusal(unsigned.clone()).await,
            Some(TransmissionRefusal::SignedByRequest)
        );
        // A package whose request waits takes none, even once the rule is off.
        let parts = Parts::new(preset.destinations, EML);
        let waiting = guard_package(&w, Some(&operator(&w)), &parts.to_sign(&w))
            .await
            .unwrap()
            .unwrap();
        let with_request = MiruTransmissionPackageData {
            signing_request: Some(waiting.clone()),
            ..unsigned.clone()
        };
        w.execute(
            "DELETE FROM sequent_backend.signing_rule WHERE election_event_id = $1",
            &[&w.event],
        )
        .await;
        assert_eq!(
            refusal(with_request.clone()).await,
            Some(TransmissionRefusal::SignedByRequest),
            "{}",
            preset.label
        );
        // Once that request ended without running, the 2025 path is back.
        w.execute(
            "UPDATE sequent_backend.signing_request SET status = 'cancelled',
                 cancel_reason = 'by-operator' WHERE id = $1",
            &[&waiting.id.parse::<Uuid>().unwrap()],
        )
        .await;
        assert_eq!(refusal(with_request).await, None, "{}", preset.label);
    }
}

#[tokio::test]
async fn an_eml_signature_goes_with_the_stored_eml_only() {
    let t = transmission(&PRESETS[0]).await;
    // A stored document whose bytes aren't the signed ones: refused.
    let services = services_for(
        &t.signatories,
        EmlStore(HashMap::from([(
            t.parts.eml_document_id,
            b"tampered".to_vec(),
        )])),
        t.sbeis.clone(),
        RecordingDispatcher::default(),
    );
    assert!(t.sign_with(&services, 0).await.is_err());
    assert!(t.w.approvals(t.id()).await.is_empty());

    // Without the EML signature: refused before anything is checked.
    let row = t.w.request(t.id()).await;
    let signatory = &t.signatories[0];
    let mut client = t.w.pool.get().await.unwrap();
    let result = approve(
        &mut client,
        &t.services,
        &signatory.caller,
        t.w.tenant,
        &ApproveInput {
            request_id: t.id(),
            chain_pem: vec![signatory.certificate.identity.pem.clone()],
            algorithm: SignatureAlgorithm::RsaPkcs1Sha256,
            payload_signature: signature_over(&row.canonical_payload),
            document_signature: None,
            pdf_cms: None,
            revision: None,
        },
        at(1),
    )
    .await;
    assert!(
        matches!(result, Err(SigningError::Invalid { .. })),
        "{result:?}"
    );
    // With it, the signature counts.
    assert_eq!(t.sign(0).await.unwrap().count, 1);
}

#[tokio::test]
async fn the_send_follows_the_rule_and_the_request() {
    for preset in &PRESETS {
        let w = world(preset.label).await;
        let unsigned = MiruTransmissionPackageData {
            election_id: w.post.to_string(),
            area_id: w.area.to_string(),
            servers: servers(preset.destinations),
            documents: vec![],
            logs: vec![],
            threshold: 0,
            signing_request: None,
        };
        let check = |package: MiruTransmissionPackageData| {
            let w = w.clone();
            async move {
                let mut client = w.pool.get().await.unwrap();
                let tx = client.transaction().await.unwrap();
                transmission_send_check(&tx, w.tenant, w.event, &package)
                    .await
                    .unwrap()
            }
        };
        // No rule: as in 2025.
        assert_eq!(
            check(unsigned.clone()).await,
            Ok(TransmissionSendCheck::Unsigned),
            "{}",
            preset.label
        );
        // As in 2025, short of the Post's minimum: a typed refusal.
        let short = MiruTransmissionPackageData {
            threshold: i64::from(preset.required),
            ..unsigned.clone()
        };
        assert_eq!(
            check(short).await,
            Err(TransmissionRefusal::SignaturesShort(
                TransmissionSignaturesShort {
                    signatures: 0,
                    threshold: i64::from(preset.required),
                }
            )),
            "{}",
            preset.label
        );
        // With the rule, a package made before it has no request: refused.
        w.rule(ACTION, preset.required, RequesterSigning::NotAllowed, None)
            .await;
        assert_eq!(
            check(unsigned.clone()).await,
            Err(TransmissionRefusal::NotSigned {
                code: None,
                status: None
            }),
            "{}",
            preset.label
        );
        let parts = Parts::new(preset.destinations, EML);
        let waiting = guard_package(&w, Some(&operator(&w)), &parts.to_sign(&w))
            .await
            .unwrap()
            .unwrap();
        let with_request = MiruTransmissionPackageData {
            signing_request: Some(waiting.clone()),
            ..unsigned.clone()
        };
        let refusal = check(with_request).await.unwrap_err().to_string();
        assert!(refusal.contains(&waiting.code), "{refusal}");
    }
}

#[tokio::test]
async fn the_panel_links_the_eml_the_signers_sign() {
    let t = transmission(&PRESETS[0]).await;
    let row = t.w.request(t.id()).await;
    let signer = EmlDocumentSigner::new(
        Arc::new(EmlStore(HashMap::from([(
            t.parts.eml_document_id,
            EML.to_vec(),
        )]))),
        Arc::new(Configured(None)),
    );
    let mut client = t.w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let document = signer.panel_document(&tx, &row).await.unwrap();
    assert_eq!(
        document.url,
        Some(format!(
            "https://documents.invalid/{}",
            t.parts.eml_document_id
        ))
    );
    assert_eq!(document.name.as_deref(), Some("er.xml"));
    // An EML has no PDF revisions.
    assert_eq!(document.revision, None);
    // The bytes it serves are the ones the payload names.
    assert_eq!(
        signer.document_bytes(&tx, &row).await.unwrap(),
        Some(EML.to_vec())
    );
}

#[test]
fn packages_are_dated_in_the_events_zone_with_its_minutes() {
    use chrono::TimeZone as _;
    use sequent_core::types::date_time::TimeZone;
    use windmill::services::consolidation::signed_transmission_package::time_zone_at;
    let now = chrono::Utc.with_ymd_and_hms(2028, 1, 15, 12, 0, 0).unwrap();
    let offset = |zone: TimeZone| match zone {
        TimeZone::UTC => (0, 0),
        TimeZone::Offset(hours) => (hours * 60, 1),
        TimeZone::OffsetMinutes(minutes) => (minutes, 2),
    };
    assert_eq!(offset(time_zone_at(chrono_tz::Asia::Manila, now)), (480, 1));
    assert_eq!(
        offset(time_zone_at(chrono_tz::Asia::Kolkata, now)),
        (330, 2)
    );
    assert_eq!(
        offset(time_zone_at(chrono_tz::Asia::Kathmandu, now)),
        (345, 2)
    );
    assert_eq!(
        offset(time_zone_at(chrono_tz::Europe::Madrid, now)),
        (60, 1)
    );
}
