// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Reports that need signatures: a held report starts its request and
//! keeps how it is released; the latest signed revision is released where
//! the unsigned report would have gone (wrapped in its password as it
//! would have been) only once the request runs, never before; the e-mail a
//! scheduled report holds back goes after that commit, once, and a failed
//! send is kept, not retried. The tally holds the election returns of each
//! Post and country.
//!
//! Each test commits its own tenant, since the approve step owns its
//! transactions, and runs under two configurations. Numbers and labels
//! come from the configuration.

#[path = "support/schema.rs"]
mod schema;
#[path = "support/signing.rs"]
mod signing;

use async_trait::async_trait;
use deadpool_postgres::Transaction;
use lopdf::content::{Content, Operation};
use lopdf::{dictionary, Document, Object, Stream};
use sequent_core::signing::{
    CancelReason, DocumentKind, DocumentRevisionState, RequesterSigning, SigningAction,
    SigningRequestStatus,
};
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Value};
use signing::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use windmill::postgres::signing::*;
use windmill::postgres::signing_document_revision::{
    insert_document_revision, NewDocumentRevision,
};
use windmill::postgres::signing_report_release::{
    get_report_release, report_awaits_signatures, set_result_pdf, ReleaseEncryption, ReleaseTarget,
    ReleasedDocument, ReportEmail, ReportRelease,
};
use windmill::services::signing::actions::reports::{
    held_report_requests, hold_tally_report, mail_held_report, release_report,
    start_held_tally_reports, start_report_signing, sweep_held_mails, tally_subject_key, HeldMail,
    ReportPublisher, ReportRenderer, ReportToSign, TallyReport, TallyRequester,
};
use windmill::services::signing::actions::transmission::sha256_hex;
use windmill::services::signing::actions::{
    executors, run_dispatched, EffectProgress, EffectRefused, NoSeal, RunOutcome,
    SignedActionDispatcher, SignedActionEffects, SignedActionTask,
};
use windmill::services::signing::approve::{DocumentSigner, SigningServices};
use windmill::services::signing::certificates::CertificateIdentity;
use windmill::services::signing::executors::{PostCommit, PostCommitTask};
use windmill::services::signing::guard::SigningRequestSummary;
use windmill::services::signing::log::Actor;
use windmill::services::signing::pdf::{DocumentLink, RevisionStore, SigningBase};
use windmill::services::signing::{SigningCaller, SigningError, SigningResult};

/// Two configurations: a Post label, how many sign, whether the report is
/// mailed (a scheduled one) and how it is protected.
struct Preset {
    label: &'static str,
    required: u16,
    mailed: bool,
    encryption: ReleaseEncryption,
}

const PRESETS: [Preset; 2] = [
    Preset {
        label: "madrid-pe",
        required: 3,
        mailed: false,
        encryption: ReleaseEncryption::ConfiguredPassword,
    },
    Preset {
        label: "faculty-of-science",
        required: 2,
        mailed: true,
        encryption: ReleaseEncryption::NoEncryption,
    },
];

/// `sweep_held_mails` claims every tenant's held e-mails. The tests that
/// release a report with an e-mail, and so hold one it could claim (or mark
/// failed) before they mail or check it themselves, take turns with it.
static HELD_MAILS: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

const ACTION: SigningAction = SigningAction::GenerateReports;
const BASE: &[u8] = b"%PDF-1.7 base with its signature page";

fn signed_bytes(k: usize) -> Vec<u8> {
    [BASE, format!("\nsigned revision {k}").as_bytes()].concat()
}

/// The documents of the requests, in memory.
#[derive(Default)]
struct Documents(Mutex<HashMap<Uuid, Vec<u8>>>);

impl Documents {
    fn put(&self, id: Uuid, bytes: &[u8]) {
        self.0.lock().unwrap().insert(id, bytes.to_vec());
    }
}

#[async_trait]
impl RevisionStore for Documents {
    async fn load(
        &self,
        _tx: &Transaction<'_>,
        _tenant_id: Uuid,
        _election_event_id: Uuid,
        document_id: Uuid,
    ) -> anyhow::Result<Vec<u8>> {
        self.0
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
        let id = Uuid::new_v4();
        self.put(id, content);
        Ok(id)
    }

    async fn link(
        &self,
        _tx: &Transaction<'_>,
        _tenant_id: Uuid,
        _election_event_id: Uuid,
        _document_id: Uuid,
    ) -> anyhow::Result<Option<DocumentLink>> {
        Ok(None)
    }
}

/// Stands in for PR 7's PAdES side: each signature adds a signed revision,
/// stored in `documents`.
struct SignedRevisions(Arc<Documents>);

#[async_trait]
impl DocumentSigner for SignedRevisions {
    fn supports(&self, kind: DocumentKind) -> bool {
        kind != DocumentKind::Eml
    }

    async fn document_bytes(
        &self,
        _tx: &Transaction<'_>,
        _request: &SigningRequestRow,
    ) -> anyhow::Result<Option<Vec<u8>>> {
        Ok(None)
    }

    async fn embed(
        &self,
        tx: &Transaction<'_>,
        request: &SigningRequestRow,
        signer: &Actor,
        _certificate: &CertificateIdentity,
        _pdf_cms: Option<&[u8]>,
        _revision: Option<i32>,
    ) -> SigningResult<()> {
        let field: i64 = tx
            .query_one(
                "SELECT count(*) FROM sequent_backend.signing_document_revision
                 WHERE request_id = $1 AND state = 'signed'",
                &[&request.id],
            )
            .await?
            .get(0);
        let bytes = signed_bytes(field as usize + 1);
        let document_id = self
            .0
            .store(tx, request.tenant_id, request.election_event_id, "", &bytes)
            .await?;
        insert_document_revision(
            tx,
            &NewDocumentRevision {
                tenant_id: request.tenant_id,
                election_event_id: request.election_event_id,
                request_id: request.id,
                document_id: Some(document_id),
                sha256: sha256_hex(&bytes),
                state: DocumentRevisionState::Signed,
                prepared_for_user: Some(signer.user_id.clone()),
                byte_range: None,
                field_index: Some(field as i32),
                parent_sha256: None,
                digest_sha256: None,
                appearance: None,
            },
        )
        .await?;
        Ok(())
    }
}

/// Records the tasks the approve step sends.
#[derive(Default)]
struct RecordingDispatcher(Arc<Mutex<Vec<SignedActionTask>>>);

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
        PostCommit::SendTask(Box::new(RecordedTask(task, self.0.clone())))
    }
}

/// Records what a release publishes and mails; the mail can fail.
struct Publications {
    documents: Arc<Documents>,
    published: Mutex<Vec<(ReportRelease, Vec<u8>)>>,
    mailed: Mutex<Vec<(ReportEmail, ReleasedDocument, Vec<u8>)>>,
    mail_fails: bool,
}

impl Publications {
    fn new(documents: Arc<Documents>, mail_fails: bool) -> Self {
        Publications {
            documents,
            published: Mutex::new(vec![]),
            mailed: Mutex::new(vec![]),
            mail_fails,
        }
    }
}

#[async_trait]
impl ReportPublisher for Publications {
    async fn publish(
        &self,
        _tx: &Transaction<'_>,
        _request: &SigningRequestRow,
        release: &ReportRelease,
        pdf: &[u8],
    ) -> anyhow::Result<ReleasedDocument> {
        self.published
            .lock()
            .unwrap()
            .push((release.clone(), pdf.to_vec()));
        // A protected report is stored wrapped, as the real one is.
        let (stored, name, media_type) = match release.encryption {
            ReleaseEncryption::ConfiguredPassword => (
                [b"wrapped:".as_slice(), pdf].concat(),
                "report.epdf",
                "application/octet-stream",
            ),
            ReleaseEncryption::NoEncryption => (pdf.to_vec(), "report.pdf", "application/pdf"),
        };
        let document_id = match &release.target {
            ReleaseTarget::Report { document_id } => *document_id,
            ReleaseTarget::TallyResult { .. } => Uuid::new_v4(),
        };
        self.documents.put(document_id, &stored);
        Ok(ReleasedDocument {
            document_id,
            name: name.into(),
            media_type: media_type.into(),
            protected: release.encryption == ReleaseEncryption::ConfiguredPassword,
        })
    }

    async fn mail(
        &self,
        email: &ReportEmail,
        released: &ReleasedDocument,
        content: &[u8],
    ) -> anyhow::Result<()> {
        self.mailed
            .lock()
            .unwrap()
            .push((email.clone(), released.clone(), content.to_vec()));
        if self.mail_fails {
            anyhow::bail!("the e-mail provider is down")
        }
        Ok(())
    }
}

struct ReleaseEffects {
    documents: Arc<Documents>,
    publications: Arc<Publications>,
}

#[async_trait]
impl SignedActionEffects for ReleaseEffects {
    async fn run(
        &self,
        tx: &Transaction<'_>,
        request: &SigningRequestRow,
        _approvals: &[SigningApprovalRow],
        progress: &EffectProgress,
    ) -> anyhow::Result<Value> {
        release_report(
            tx,
            self.documents.as_ref(),
            self.publications.as_ref(),
            request,
            progress,
        )
        .await
    }
}

fn email() -> ReportEmail {
    ReportEmail {
        recipients: vec!["auditor@example.invalid".into()],
        subject: "Scheduled report".into(),
        plaintext_body: "Attached.".into(),
        html_body: None,
    }
}

fn to_sign(w: &World, preset: &Preset, document_id: Uuid) -> ReportToSign {
    let report_id = Uuid::new_v4();
    ReportToSign {
        tenant_id: w.tenant,
        election_event_id: w.event,
        election_id: Some(w.post),
        area_id: None,
        report_type: "PARTICIPATION_REPORT".into(),
        template_id: Some("template-2028".into()),
        subject_key: report_id.to_string(),
        base: SigningBase {
            action: ACTION,
            document_id,
            sha256: sha256_hex(BASE),
        },
        release: ReportRelease {
            report_id: Some(report_id),
            target: ReleaseTarget::Report {
                document_id: Uuid::new_v4(),
            },
            file_name: "participation.pdf".into(),
            is_public: false,
            encryption: preset.encryption,
            email: preset.mailed.then(email),
        },
    }
}

async fn start(
    w: &World,
    requester: &SigningCaller,
    report: &ReportToSign,
) -> SigningResult<SigningRequestSummary> {
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let outcome = start_report_signing(&tx, requester, report).await;
    if outcome.is_ok() {
        tx.commit().await.unwrap();
    }
    outcome
}

fn operator(w: &World) -> SigningCaller {
    caller("operator", &[Permissions::REPORT_READ], &[&w.label])
}

async fn release_row(
    w: &World,
    id: Uuid,
) -> windmill::postgres::signing_report_release::ReportReleaseRow {
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    get_report_release(&tx, w.tenant, w.event, id, false)
        .await
        .unwrap()
        .unwrap()
}

#[tokio::test]
async fn a_held_report_starts_its_request_and_keeps_its_release() {
    for preset in &PRESETS {
        let w = world(preset.label).await;
        w.rule(ACTION, preset.required, RequesterSigning::NotAllowed, None)
            .await;
        let base_id = Uuid::new_v4();
        let report = to_sign(&w, preset, base_id);
        let summary = start(&w, &operator(&w), &report).await.unwrap();
        assert_eq!(
            summary.required,
            i32::from(preset.required),
            "{}",
            preset.label
        );

        let row = w.request(summary.id).await;
        assert_eq!(row.action, ACTION);
        assert_eq!(row.status, SigningRequestStatus::Waiting);
        assert_eq!(
            row.subject,
            json!({
                "report_type": "PARTICIPATION_REPORT",
                "document_sha256": sha256_hex(BASE),
                "template_id": "template-2028",
            }),
            "{}",
            preset.label
        );
        assert_eq!(row.document_id, Some(base_id));
        assert_eq!((row.election_id, row.area_id), (Some(w.post), None));
        let release = release_row(&w, summary.id).await;
        assert_eq!(release.release, report.release, "{}", preset.label);
        assert_eq!(release.released_at, None);

        // The same report generated again answers the same waiting request,
        // whose release doesn't move.
        let mut again = report.clone();
        again.release.target = ReleaseTarget::Report {
            document_id: Uuid::new_v4(),
        };
        assert_eq!(start(&w, &operator(&w), &again).await.unwrap(), summary);
        assert_eq!(release_row(&w, summary.id).await.release, report.release);
        // A scheduled run skips the report while its request waits.
        let mut client = w.pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        assert!(report_awaits_signatures(
            &tx,
            w.tenant,
            w.event,
            report.release.report_id.unwrap()
        )
        .await
        .unwrap());
        assert!(
            !report_awaits_signatures(&tx, w.tenant, w.event, Uuid::new_v4())
                .await
                .unwrap()
        );
    }
}

#[tokio::test]
async fn a_report_needs_a_post_and_the_rule_it_was_generated_under() {
    let preset = &PRESETS[1];
    let w = world(preset.label).await;
    w.rule(ACTION, preset.required, RequesterSigning::NotAllowed, None)
        .await;
    let mut eventwide = to_sign(&w, preset, Uuid::new_v4());
    eventwide.election_id = None;
    assert!(matches!(
        start(&w, &operator(&w), &eventwide).await,
        Err(SigningError::Invalid { .. })
    ));
    // Another Post's operator can't start it.
    let elsewhere = caller("operator", &[Permissions::REPORT_READ], &["other-post"]);
    assert!(matches!(
        start(&w, &elsewhere, &to_sign(&w, preset, Uuid::new_v4())).await,
        Err(SigningError::Forbidden(_))
    ));
    // Without the rule (switched off meanwhile) nothing starts: the report
    // is generated again.
    let other = world("no-rule").await;
    assert!(matches!(
        start(
            &other,
            &operator(&other),
            &to_sign(&other, preset, Uuid::new_v4())
        )
        .await,
        Err(SigningError::Conflict(_))
    ));
}

/// A held report signed by `required` people, its request completed and its
/// task recorded.
struct Signed {
    w: World,
    report: ReportToSign,
    summary: SigningRequestSummary,
    documents: Arc<Documents>,
    task: SignedActionTask,
}

async fn signed(preset: &Preset, publications_before: Option<&Publications>) -> Signed {
    signed_to(preset, publications_before, None).await
}

/// [`signed`], released to `target` when given.
async fn signed_to(
    preset: &Preset,
    publications_before: Option<&Publications>,
    target: Option<ReleaseTarget>,
) -> Signed {
    let w = world(preset.label).await;
    w.rule(ACTION, preset.required, RequesterSigning::NotAllowed, None)
        .await;
    let documents = Arc::new(Documents::default());
    let mut report = to_sign(&w, preset, Uuid::new_v4());
    if let Some(target) = target {
        report.release.target = target;
    }
    let summary = start(&w, &operator(&w), &report).await.unwrap();
    let mut signers = vec![];
    for i in 0..usize::from(preset.required) {
        let user = format!("sbei-{i}");
        let certificate = cert(&user, &user, &user);
        w.register(&user, &certificate, None).await;
        signers.push((w.signer(&user, ACTION), certificate));
    }
    let dispatcher = RecordingDispatcher::default();
    let sent = dispatcher.0.clone();
    let certificates: Vec<&TestCert> = signers.iter().map(|(_, c)| c).collect();
    let services = SigningServices {
        documents: Arc::new(SignedRevisions(documents.clone())),
        ..signing::services(
            FakeVerifier::knowing(&certificates),
            executors(Arc::new(dispatcher)),
        )
    };
    let (last, first) = signers.split_last().unwrap();
    for (signer, certificate) in first {
        let outcome = w
            .sign(&services, signer, summary.id, certificate, at(1))
            .await
            .unwrap();
        assert_eq!(outcome.status, SigningRequestStatus::Waiting);
        // Short of its signatures: nothing to run, nothing released.
        assert!(sent.lock().unwrap().is_empty(), "{}", preset.label);
        if let Some(publications) = publications_before {
            assert!(publications.published.lock().unwrap().is_empty());
        }
    }
    let outcome = w
        .sign(&services, &last.0, summary.id, &last.1, at(1))
        .await
        .unwrap();
    assert_eq!(outcome.status, SigningRequestStatus::Completed);
    let task = {
        let sent = sent.lock().unwrap();
        assert_eq!(sent.len(), 1, "{}", preset.label);
        sent[0]
    };
    Signed {
        w,
        report,
        summary,
        documents,
        task,
    }
}

#[tokio::test]
async fn the_signed_report_is_released_once_its_request_ran_and_mailed_after() {
    let _turn = HELD_MAILS.lock().await;
    for preset in &PRESETS {
        let s = signed(preset, None).await;
        let publications = Arc::new(Publications::new(s.documents.clone(), false));
        let effects = ReleaseEffects {
            documents: s.documents.clone(),
            publications: publications.clone(),
        };
        let mut client = s.w.pool.get().await.unwrap();
        let run = run_dispatched(&mut client, &effects, &NoSeal, &s.task)
            .await
            .unwrap();
        assert!(matches!(run, RunOutcome::Executed(_)), "{run:?}");
        // A second copy of the task releases nothing more.
        assert_eq!(
            run_dispatched(&mut client, &effects, &NoSeal, &s.task)
                .await
                .unwrap(),
            RunOutcome::NotClaimed
        );

        // The latest signed revision, released where the report goes, with
        // the protection it was generated under.
        let latest = signed_bytes(usize::from(preset.required));
        let published = publications.published.lock().unwrap().clone();
        assert_eq!(
            published,
            vec![(s.report.release.clone(), latest.clone())],
            "{}",
            preset.label
        );
        // Nothing mailed inside the release.
        assert!(publications.mailed.lock().unwrap().is_empty());

        let row = s.w.request(s.summary.id).await;
        assert_eq!(row.status, SigningRequestStatus::Executed);
        let result = row.execution_result.unwrap();
        assert_eq!(
            result["sha256"],
            json!(sha256_hex(&latest)),
            "{}",
            preset.label
        );
        let ReleaseTarget::Report { document_id } = s.report.release.target else {
            unreachable!()
        };
        assert_eq!(result["document_id"], json!(document_id));
        assert!(release_row(&s.w, s.summary.id).await.released_at.is_some());

        // After the commit, the held e-mail goes once, with what was stored.
        let first = mail_held_report(
            &mut client,
            s.documents.as_ref(),
            publications.as_ref(),
            s.w.tenant,
            s.w.event,
            s.summary.id,
        )
        .await
        .unwrap();
        let second = mail_held_report(
            &mut s.w.pool.get().await.unwrap(),
            s.documents.as_ref(),
            publications.as_ref(),
            s.w.tenant,
            s.w.event,
            s.summary.id,
        )
        .await
        .unwrap();
        let mailed = publications.mailed.lock().unwrap().clone();
        if preset.mailed {
            assert_eq!(first, HeldMail::Sent);
            assert_eq!(mailed.len(), 1);
            assert_eq!(mailed[0].0, email());
            assert_eq!(
                mailed[0].2, latest,
                "the unprotected report is mailed as it is"
            );
            assert!(release_row(&s.w, s.summary.id).await.mailed_at.is_some());
        } else {
            assert_eq!(first, HeldMail::Nothing);
            assert!(mailed.is_empty());
        }
        assert_eq!(second, HeldMail::Nothing, "{}", preset.label);
    }
}

#[tokio::test]
async fn a_failed_mail_is_kept_and_not_tried_again() {
    let _turn = HELD_MAILS.lock().await;
    let preset = &PRESETS[1];
    let s = signed(preset, None).await;
    let publications = Arc::new(Publications::new(s.documents.clone(), true));
    let effects = ReleaseEffects {
        documents: s.documents.clone(),
        publications: publications.clone(),
    };
    let mut client = s.w.pool.get().await.unwrap();
    run_dispatched(&mut client, &effects, &NoSeal, &s.task)
        .await
        .unwrap();
    let outcome = mail_held_report(
        &mut client,
        s.documents.as_ref(),
        publications.as_ref(),
        s.w.tenant,
        s.w.event,
        s.summary.id,
    )
    .await
    .unwrap();
    assert!(matches!(outcome, HeldMail::Failed(_)), "{outcome:?}");
    let release = release_row(&s.w, s.summary.id).await;
    assert!(release.mail_error.is_some() && release.mailed_at.is_none());
    // The release stands; the failure is logged as an error.
    assert_eq!(
        s.w.request(s.summary.id).await.status,
        SigningRequestStatus::Executed
    );
    let errors: Vec<String> =
        s.w.outbox()
            .await
            .into_iter()
            .filter(|row| row.3 == "ERROR")
            .map(|row| row.5)
            .collect();
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("could not mail"), "{errors:?}");
    let logged_actor: Option<String> = client
        .query_one(
            "SELECT user_id FROM sequent_backend.signing_log_outbox
             WHERE tenant_id = $1 AND election_event_id = $2
                 AND statement_kind = 'SigningActionExecuted' AND entry = 0
                 AND body->'details'->>'step' = 'mail'",
            &[&s.w.tenant, &s.w.event],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(logged_actor, Some(format!("sbei-{}", preset.required - 1)));
    // Never tried again.
    assert_eq!(
        mail_held_report(
            &mut client,
            s.documents.as_ref(),
            publications.as_ref(),
            s.w.tenant,
            s.w.event,
            s.summary.id,
        )
        .await
        .unwrap(),
        HeldMail::Nothing
    );
    assert_eq!(publications.mailed.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn a_release_refuses_what_is_not_as_signed() {
    let preset = &PRESETS[0];
    let s = signed(preset, None).await;
    let request = s.w.request(s.summary.id).await;
    let publications = Publications::new(s.documents.clone(), false);
    let code = |error: anyhow::Error| {
        error
            .downcast_ref::<EffectRefused>()
            .map(|refused| refused.code.clone())
    };
    let mut client = s.w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    // The signed revision's bytes changed in the store.
    let latest_id: Uuid = tx
        .query_one(
            "SELECT document_id FROM sequent_backend.signing_document_revision
             WHERE request_id = $1 ORDER BY revision DESC LIMIT 1",
            &[&request.id],
        )
        .await
        .unwrap()
        .get(0);
    s.documents.put(latest_id, b"other bytes");
    let error = release_report(
        &tx,
        s.documents.as_ref(),
        &publications,
        &request,
        &EffectProgress::default(),
    )
    .await
    .unwrap_err();
    assert_eq!(code(error).as_deref(), Some("revision-changed"));
    // A document with fewer signatures than the request needs.
    tx.execute(
        "DELETE FROM sequent_backend.signing_document_revision
         WHERE request_id = $1 AND field_index = 0",
        &[&request.id],
    )
    .await
    .unwrap();
    let error = release_report(
        &tx,
        s.documents.as_ref(),
        &publications,
        &request,
        &EffectProgress::default(),
    )
    .await
    .unwrap_err();
    assert_eq!(code(error).as_deref(), Some("signatures-missing"));
    // No signed revision at all.
    tx.execute(
        "DELETE FROM sequent_backend.signing_document_revision WHERE request_id = $1",
        &[&request.id],
    )
    .await
    .unwrap();
    let error = release_report(
        &tx,
        s.documents.as_ref(),
        &publications,
        &request,
        &EffectProgress::default(),
    )
    .await
    .unwrap_err();
    assert_eq!(code(error).as_deref(), Some("no-signed-revision"));
    assert!(publications.published.lock().unwrap().is_empty());
}

/// A rendered one-page report.
fn report_pdf(text: &str) -> Vec<u8> {
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
            Operation::new("Tj", vec![Object::string_literal(text)]),
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

/// Makes `area` vote in `post`.
async fn votes_in(w: &World, post: Uuid, area: Uuid) {
    let contest = Uuid::new_v4();
    w.execute(
        "INSERT INTO sequent_backend.contest (id, tenant_id, election_event_id, election_id)
         VALUES ($1, $2, $3, $4)",
        &[&contest, &w.tenant, &w.event, &post],
    )
    .await;
    w.execute(
        "INSERT INTO sequent_backend.area_contest
             (id, tenant_id, election_event_id, area_id, contest_id)
         VALUES ($1, $2, $3, $4, $5)",
        &[&Uuid::new_v4(), &w.tenant, &w.event, &area, &contest],
    )
    .await;
}

/// The holds of an event: (election, area, request, error), oldest first.
async fn holds(w: &World) -> Vec<(Uuid, Option<Uuid>, Option<Uuid>, Option<String>)> {
    let client = w.pool.get().await.unwrap();
    client
        .query(
            "SELECT election_id, area_id, request_id, error FROM sequent_backend.signing_tally_hold
             WHERE election_event_id = $1 ORDER BY created_at, id",
            &[&w.event],
        )
        .await
        .unwrap()
        .into_iter()
        .map(|row| (row.get(0), row.get(1), row.get(2), row.get(3)))
        .collect()
}

#[tokio::test]
async fn the_tally_holds_the_election_returns_of_each_post_and_country() {
    const ER: SigningAction = SigningAction::GenerateElectionReturns;
    for preset in &PRESETS {
        let w = world(preset.label).await;
        w.rule(ER, preset.required, RequesterSigning::NotAllowed, None)
            .await;
        votes_in(&w, w.post, w.area).await;
        let documents = Documents::default();
        let results_event_id = Uuid::new_v4();
        let pdf = report_pdf("Election returns");
        // References link back to this exact tally and result document; a
        // transmitter sees only its package, never another country's.
        let tally_session_id = Uuid::new_v4();
        let keys_id = Uuid::new_v4();
        w.execute(
            "INSERT INTO sequent_backend.keys_ceremony
             (id, tenant_id, election_event_id, trustee_ids, threshold)
             VALUES ($1, $2, $3, '{}', 1)",
            &[&keys_id, &w.tenant, &w.event],
        )
        .await;
        let package = json!({"election_id": w.post, "area_id": w.area,
            "documents": [], "servers": [], "logs": [], "threshold": preset.required});
        let annotations = json!({"miru:tally-session-data": serde_json::to_string(&json!([
            {"election_id": w.post, "area_id": Uuid::new_v4(), "documents": ["other-country"]},
            package.clone()
        ])).unwrap()});
        w.execute(
            "INSERT INTO sequent_backend.tally_session
             (id, tenant_id, election_event_id, keys_ceremony_id, threshold, election_ids, annotations)
             VALUES ($1, $2, $3, $4, 1, $5, $6)",
            &[&tally_session_id, &w.tenant, &w.event, &keys_id, &vec![w.post], &annotations],
        ).await;
        w.execute(
            "INSERT INTO sequent_backend.results_event (id, tenant_id, election_event_id)
             VALUES ($1, $2, $3)",
            &[&results_event_id, &w.tenant, &w.event],
        )
        .await;
        w.execute(
            "INSERT INTO sequent_backend.tally_session_execution
             (id, tenant_id, election_event_id, current_message_id, tally_session_id, results_event_id)
             VALUES ($1, $2, $3, 0, $4, $5)",
            &[&Uuid::new_v4(), &w.tenant, &w.event, &tally_session_id, &results_event_id],
        ).await;
        w.execute(
            "INSERT INTO sequent_backend.results_election
             (id, tenant_id, election_event_id, results_event_id, election_id, total_voters_percent, documents)
             VALUES ($1, $2, $3, $4, $5, 0, $6)",
            &[&Uuid::new_v4(), &w.tenant, &w.event, &results_event_id, &w.post,
              &json!({"json": "post-results.json"})],
        ).await;
        w.execute(
            "INSERT INTO sequent_backend.results_election_area
             (id, tenant_id, election_event_id, results_event_id, election_id, area_id, name, documents)
             VALUES ($1, $2, $3, $4, $5, $6, 'Country', $7)",
            &[&Uuid::new_v4(), &w.tenant, &w.event, &results_event_id, &w.post, &w.area,
              &json!({"json": "country-results.json"})],
        ).await;
        // Who ran the tally; a configuration of their own per preset.
        let executer = TallyRequester {
            user_id: format!("{}-tally-executer", preset.label),
            username: format!("{}-executer", preset.label),
        };
        let hold = |area_id: Option<Uuid>| TallyReport {
            tenant_id: w.tenant,
            election_event_id: w.event,
            results_event_id,
            action: ER,
            report_type: "ELECTORAL_RESULTS",
            election_id: w.post,
            area_id,
            file_name: "report.pdf",
            pdf: &pdf,
            requester: &executer,
        };

        // Lock order: the tally's transaction holds the tally session's row
        // when it holds its reports, so holding takes no signing lock. With
        // another transaction holding the event's signing lock, it still
        // completes.
        let mut signing = w.pool.get().await.unwrap();
        let signing_tx = signing.transaction().await.unwrap();
        lock_signing_event(&signing_tx, w.tenant, w.event)
            .await
            .unwrap();
        let mut client = w.pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        for area_id in [None, Some(w.area), Some(Uuid::new_v4())] {
            tokio::time::timeout(
                std::time::Duration::from_secs(10),
                hold_tally_report(&tx, &documents, &hold(area_id)),
            )
            .await
            .expect("holding a report waited for the signing lock")
            .unwrap();
        }
        tx.commit().await.unwrap();
        signing_tx.rollback().await.unwrap();
        // Nothing starts before the tally committed.
        assert!(holds(&w).await.iter().all(|hold| hold.2.is_none()));

        // After the commit, in a transaction that takes the signing lock
        // first, the requests start.
        let mut client = w.pool.get().await.unwrap();
        assert_eq!(
            start_held_tally_reports(&mut client, w.tenant, w.event)
                .await
                .unwrap(),
            2,
            "{}",
            preset.label
        );
        let started = holds(&w).await;
        let mut read_client = w.pool.get().await.unwrap();
        let read = read_client.transaction().await.unwrap();
        let signer = w.signer("report-signer", ER);
        let found = held_report_requests(&read, &signer, w.tenant, w.event)
            .await
            .unwrap();
        assert_eq!(found.len(), 2);
        for entry in &found {
            assert!(started[..2]
                .iter()
                .any(|hold| hold.2 == Some(entry.request_id)));
            assert_eq!(entry.report_type, "ELECTORAL_RESULTS");
            assert_eq!(entry.results_event_id, Some(results_event_id));
            assert_eq!(entry.tally_session_id, Some(tally_session_id));
            assert_eq!(
                entry.results_document_id.as_deref(),
                Some(if entry.area_id.is_some() {
                    "country-results.json"
                } else {
                    "post-results.json"
                })
            );
            assert!(entry.transmission_package.is_none());
            assert!(serde_json::to_value(entry)
                .unwrap()
                .get("transmission_package")
                .is_none());
        }
        let transmitter = caller(
            "report-signer",
            &[
                Permissions::SIGN_GENERATE_ELECTION_RETURNS,
                Permissions::MIRU_SEND,
            ],
            &[preset.label],
        );
        let transmitted = held_report_requests(&read, &transmitter, w.tenant, w.event)
            .await
            .unwrap();
        assert_eq!(transmitted.len(), 2);
        for entry in transmitted {
            assert_eq!(
                entry.transmission_package,
                entry.area_id.map(|_| package.clone())
            );
        }
        let unlabelled = caller(
            "unlabelled",
            &[Permissions::SIGN_GENERATE_ELECTION_RETURNS],
            &[],
        );
        assert!(held_report_requests(&read, &unlabelled, w.tenant, w.event)
            .await
            .unwrap()
            .is_empty());
        let other_action = w.signer("other-action", SigningAction::GenerateReports);
        assert!(
            held_report_requests(&read, &other_action, w.tenant, w.event)
                .await
                .unwrap()
                .is_empty()
        );
        let stranger = caller("stranger", &[], &[]);
        assert!(matches!(
            held_report_requests(&read, &stranger, w.tenant, w.event).await,
            Err(SigningError::Forbidden(_))
        ));
        assert!(
            held_report_requests(&read, &signer, Uuid::new_v4(), w.event)
                .await
                .unwrap()
                .is_empty()
        );
        read.rollback().await.unwrap();
        // A country that doesn't vote in the Post keeps the reason.
        assert!(started[2].2.is_none());
        assert!(
            started[2]
                .3
                .as_deref()
                .is_some_and(|error| error.contains("doesn't vote in the Post")),
            "{started:?}"
        );
        for (election_id, area_id, request_id, _) in &started[..2] {
            let row = w.request(request_id.unwrap()).await;
            assert_eq!(
                (row.election_id, row.area_id),
                (Some(*election_id), *area_id)
            );
            assert_eq!(row.required, i32::from(preset.required));
            assert!(row.scope_key.ends_with(&tally_subject_key(
                "ELECTORAL_RESULTS",
                results_event_id,
                *area_id
            )));
            // The tally's executer started it.
            assert_eq!(
                (
                    row.requested_by.as_str(),
                    row.requested_by_username.as_str()
                ),
                (executer.user_id.as_str(), executer.username.as_str())
            );
            // What signers sign is the report with its signature page.
            let base = documents
                .0
                .lock()
                .unwrap()
                .get(&row.document_id.unwrap())
                .cloned()
                .unwrap();
            assert!(base.len() > pdf.len());
            assert_eq!(row.document_sha256, Some(sha256_hex(&base)));
            assert_eq!(
                release_row(&w, row.id).await.release.target,
                ReleaseTarget::TallyResult {
                    results_event_id,
                    election_id: w.post,
                    area_id: *area_id,
                    report_type: "ELECTORAL_RESULTS".into(),
                },
                "{}",
                preset.label
            );
        }
        // Started once: nothing left to start.
        assert_eq!(
            start_held_tally_reports(&mut client, w.tenant, w.event)
                .await
                .unwrap(),
            0
        );

        // A recount's results are signed by a request of their own.
        let country_request = started[1].2.unwrap();
        let tx = client.transaction().await.unwrap();
        let recounted = report_pdf("Election returns, recounted");
        hold_tally_report(
            &tx,
            &documents,
            &TallyReport {
                results_event_id: Uuid::new_v4(),
                pdf: &recounted,
                ..hold(Some(w.area))
            },
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        assert_eq!(
            start_held_tally_reports(&mut client, w.tenant, w.event)
                .await
                .unwrap(),
            1
        );
        let recount = holds(&w).await.last().unwrap().2.unwrap();
        assert_ne!(recount, country_request);
        let superseded = w.request(country_request).await;
        assert_eq!(superseded.status, SigningRequestStatus::Cancelled);
        assert_eq!(superseded.cancel_reason, Some(CancelReason::PayloadChanged));
        // A recount of this country does not replace the Post-wide report.
        assert_eq!(
            w.request(started[0].2.unwrap()).await.status,
            SigningRequestStatus::Waiting
        );
        assert_eq!(
            w.request(recount).await.status,
            SigningRequestStatus::Waiting
        );

        // An old hold retried after the recount cannot revive stale returns or
        // cancel the newer request. This models a delayed worker retry.
        w.execute(
            "UPDATE sequent_backend.signing_tally_hold
             SET request_id = NULL, error = NULL WHERE request_id = $1",
            &[&country_request],
        )
        .await;
        assert_eq!(
            start_held_tally_reports(&mut client, w.tenant, w.event)
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            w.request(recount).await.status,
            SigningRequestStatus::Waiting
        );

        // A request about to run for the same results never takes a newer
        // base: the hold keeps the reason instead.
        w.execute(
            "UPDATE sequent_backend.signing_request SET status = 'completed',
                 completed_at = clock_timestamp() WHERE id = $1",
            &[&recount],
        )
        .await;
        let recount_results = holds(&w).await;
        let tx = client.transaction().await.unwrap();
        let other = report_pdf("Election returns, recounted again");
        let recount_event: Uuid = tx
            .query_one(
                "SELECT results_event_id FROM sequent_backend.signing_tally_hold
                 WHERE request_id = $1",
                &[&recount],
            )
            .await
            .unwrap()
            .get(0);
        hold_tally_report(
            &tx,
            &documents,
            &TallyReport {
                results_event_id: recount_event,
                pdf: &other,
                ..hold(Some(w.area))
            },
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        assert_eq!(
            start_held_tally_reports(&mut client, w.tenant, w.event)
                .await
                .unwrap(),
            0
        );
        let last = holds(&w).await.last().unwrap().clone();
        assert_eq!(holds(&w).await.len(), recount_results.len() + 1);
        assert!(last.2.is_none());
        assert!(
            last.3
                .as_deref()
                .is_some_and(|error| error.contains("signs another document")),
            "{last:?}"
        );
    }
}

#[tokio::test]
async fn a_released_tally_report_becomes_its_results_rows_pdf() {
    let w = world("madrid-pe").await;
    let results_event_id = Uuid::new_v4();
    w.execute(
        "INSERT INTO sequent_backend.results_event (id, tenant_id, election_event_id)
         VALUES ($1, $2, $3)",
        &[&results_event_id, &w.tenant, &w.event],
    )
    .await;
    w.execute(
        "INSERT INTO sequent_backend.results_election (
             id, tenant_id, election_event_id, results_event_id, election_id,
             total_voters_percent, documents)
         VALUES ($1, $2, $3, $4, $5, 0, $6)",
        &[
            &Uuid::new_v4(),
            &w.tenant,
            &w.event,
            &results_event_id,
            &w.post,
            &json!({"json": "results.json"}),
        ],
    )
    .await;
    w.execute(
        "INSERT INTO sequent_backend.results_election_area (
             id, tenant_id, election_event_id, results_event_id, election_id, area_id, name)
         VALUES ($1, $2, $3, $4, $5, $6, 'Country')",
        &[
            &Uuid::new_v4(),
            &w.tenant,
            &w.event,
            &results_event_id,
            &w.post,
            &w.area,
        ],
    )
    .await;
    let (post_pdf, country_pdf) = (Uuid::new_v4(), Uuid::new_v4());
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    set_result_pdf(
        &tx,
        w.tenant,
        w.event,
        results_event_id,
        w.post,
        None,
        post_pdf,
    )
    .await
    .unwrap();
    set_result_pdf(
        &tx,
        w.tenant,
        w.event,
        results_event_id,
        w.post,
        Some(w.area),
        country_pdf,
    )
    .await
    .unwrap();
    let post: Value = tx
        .query_one(
            "SELECT documents FROM sequent_backend.results_election WHERE results_event_id = $1",
            &[&results_event_id],
        )
        .await
        .unwrap()
        .get(0);
    // The PDF joins the row's other documents.
    assert_eq!(
        post,
        json!({"json": "results.json", "pdf": post_pdf.to_string()})
    );
    let country: Value = tx
        .query_one(
            "SELECT documents FROM sequent_backend.results_election_area
             WHERE results_event_id = $1",
            &[&results_event_id],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(country, json!({"pdf": country_pdf.to_string()}));
    // A results row that is gone: the release fails rather than losing it.
    assert!(set_result_pdf(
        &tx,
        w.tenant,
        w.event,
        Uuid::new_v4(),
        w.post,
        None,
        post_pdf
    )
    .await
    .is_err());
}

/// Renders a report's HTML as the one-page PDF of its text, counting.
#[derive(Default)]
struct FakeRenderer(Mutex<Vec<String>>);

impl ReportRenderer for FakeRenderer {
    fn render(&self, html: String) -> anyhow::Result<Vec<u8>> {
        self.0.lock().unwrap().push(html.clone());
        Ok(report_pdf(&html))
    }
}

/// A tally's working folder as velvet leaves it: HTML renderings and JSON
/// results, no PDFs.
fn velvet_folder(post: Uuid, areas: &[Uuid]) -> tempfile::TempDir {
    let base = tempfile::tempdir().unwrap();
    let reports = base.path().join("output/velvet-generate-reports");
    let write = |folder: &std::path::Path, name: &str| {
        std::fs::create_dir_all(folder).unwrap();
        std::fs::write(folder.join("report.html"), format!("<h1>{name}</h1>")).unwrap();
        std::fs::write(folder.join("report.json"), "{}").unwrap();
    };
    write(&reports, "event");
    let post_folder = reports.join(format!("election__{post}"));
    write(&post_folder, "post");
    write(&post_folder.join("contest__c1"), "contest");
    for area in areas {
        write(
            &post_folder.join(format!("area__{area}")),
            &format!("country {area}"),
        );
    }
    base
}

fn files_named(base: &std::path::Path, extension: &str) -> usize {
    walkdir::WalkDir::new(base)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().and_then(|e| e.to_str()) == Some(extension))
        .count()
}

#[tokio::test]
async fn the_tally_renders_and_holds_its_reports_and_keeps_no_unsigned_copy() {
    use windmill::services::ceremonies::result_documents::{hold_tally_reports, TallyPost};
    const ER: SigningAction = SigningAction::GenerateElectionReturns;
    for preset in &PRESETS {
        let w = world(preset.label).await;
        votes_in(&w, w.post, w.area).await;
        let missing_country = Uuid::new_v4();
        w.execute(
            "INSERT INTO sequent_backend.area (id, tenant_id, election_event_id, name)
             VALUES ($1, $2, $3, 'Unrendered')",
            &[&missing_country, &w.tenant, &w.event],
        )
        .await;
        votes_in(&w, w.post, missing_country).await;
        let folder = velvet_folder(w.post, &[w.area]);
        let posts = [TallyPost {
            election_id: w.post.to_string(),
            area_ids: vec![w.area.to_string(), missing_country.to_string()],
        }];
        let executer = TallyRequester {
            user_id: "executer-id".into(),
            username: "executer".into(),
        };
        let documents = Documents::default();
        let renderer = FakeRenderer::default();
        let results_event_id = Uuid::new_v4();
        let run = |w: &World| {
            let w = w.clone();
            let (posts, folder, executer, documents, renderer) = (
                &posts,
                folder.path().to_path_buf(),
                &executer,
                &documents,
                &renderer,
            );
            async move {
                let mut client = w.pool.get().await.unwrap();
                let tx = client.transaction().await.unwrap();
                let held = hold_tally_reports(
                    &tx,
                    posts,
                    &w.tenant.to_string(),
                    &w.event.to_string(),
                    &results_event_id.to_string(),
                    &folder,
                    &sequent_core::types::ceremonies::TallyType::ELECTORAL_RESULTS,
                    documents,
                    renderer,
                    executer,
                )
                .await
                .unwrap();
                tx.commit().await.unwrap();
                held
            }
        };

        // Without the rule, the tally's renderings stay as they are.
        assert_eq!(run(&w).await, 0, "{}", preset.label);
        assert_eq!(files_named(folder.path(), "html"), 4);
        assert!(renderer.0.lock().unwrap().is_empty());

        // With it, the Post's and the country's reports are rendered and
        // held; a country without a rendering is logged and stays held.
        w.rule(ER, preset.required, RequesterSigning::NotAllowed, None)
            .await;
        assert_eq!(run(&w).await, 2, "{}", preset.label);
        assert_eq!(
            renderer.0.lock().unwrap().clone(),
            vec![
                "<h1>post</h1>".to_string(),
                format!("<h1>country {}</h1>", w.area)
            ]
        );
        // No unsigned copy stays: no HTML or PDF of any report; the JSON stays.
        assert_eq!(files_named(folder.path(), "html"), 0);
        assert_eq!(files_named(folder.path(), "pdf"), 0);
        assert_eq!(files_named(folder.path(), "json"), 4);
        let held = holds(&w).await;
        assert_eq!(
            held.iter().map(|hold| (hold.0, hold.1)).collect::<Vec<_>>(),
            vec![(w.post, None), (w.post, Some(w.area))]
        );
        let mut client = w.pool.get().await.unwrap();
        assert_eq!(
            start_held_tally_reports(&mut client, w.tenant, w.event)
                .await
                .unwrap(),
            2
        );
    }
}

#[tokio::test]
async fn a_tally_report_is_released_to_its_results_row() {
    let _turn = HELD_MAILS.lock().await;
    let preset = &PRESETS[1];
    let target = ReleaseTarget::TallyResult {
        results_event_id: Uuid::new_v4(),
        election_id: Uuid::new_v4(),
        area_id: None,
        report_type: "INITIALIZATION_REPORT".into(),
    };
    let s = signed_to(preset, None, Some(target.clone())).await;
    let publications = Arc::new(Publications::new(s.documents.clone(), false));
    let effects = ReleaseEffects {
        documents: s.documents.clone(),
        publications: publications.clone(),
    };
    let mut client = s.w.pool.get().await.unwrap();
    let run = run_dispatched(&mut client, &effects, &NoSeal, &s.task)
        .await
        .unwrap();
    assert!(matches!(run, RunOutcome::Executed(_)), "{run:?}");
    let published = publications.published.lock().unwrap().clone();
    assert_eq!(published.len(), 1);
    assert_eq!(published[0].0.target, target);
    assert_eq!(published[0].1, signed_bytes(usize::from(preset.required)));
}

#[tokio::test]
async fn the_sweeper_mails_what_no_task_did_and_fails_stale_sends() {
    let _turn = HELD_MAILS.lock().await;
    let preset = &PRESETS[1];
    let s = signed(preset, None).await;
    let publications = Arc::new(Publications::new(s.documents.clone(), false));
    let effects = ReleaseEffects {
        documents: s.documents.clone(),
        publications: publications.clone(),
    };
    let mut client = s.w.pool.get().await.unwrap();
    run_dispatched(&mut client, &effects, &NoSeal, &s.task)
        .await
        .unwrap();
    // The task stopped before mailing: the sweeper mails it, once.
    assert_eq!(
        sweep_held_mails(&mut client, s.documents.as_ref(), publications.as_ref())
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        sweep_held_mails(&mut client, s.documents.as_ref(), publications.as_ref())
            .await
            .unwrap(),
        0
    );
    assert_eq!(publications.mailed.lock().unwrap().len(), 1);

    // A send claimed long ago that never reported back: failed, logged, not
    // sent again.
    let t = signed(preset, None).await;
    let mut client = t.w.pool.get().await.unwrap();
    run_dispatched(&mut client, &effects, &NoSeal, &t.task)
        .await
        .unwrap();
    t.w.execute(
        "UPDATE sequent_backend.signing_report_release
         SET mail_started_at = clock_timestamp() - interval '1 hour' WHERE request_id = $1",
        &[&t.summary.id],
    )
    .await;
    assert_eq!(
        sweep_held_mails(&mut client, t.documents.as_ref(), publications.as_ref())
            .await
            .unwrap(),
        0
    );
    let release = release_row(&t.w, t.summary.id).await;
    assert!(release.mail_error.is_some() && release.mailed_at.is_none());
    assert_eq!(publications.mailed.lock().unwrap().len(), 1);
    let errors =
        t.w.outbox()
            .await
            .into_iter()
            .filter(|row| row.3 == "ERROR")
            .count();
    assert_eq!(errors, 1);
}
