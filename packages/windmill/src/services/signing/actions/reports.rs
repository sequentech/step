// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Election returns and other reports that need signatures.
//!
//! With the action's rule `Required`, a report gets its signature page and
//! is held: kept as a signing document only (`pdf::SigningBase`), not
//! downloadable, printable or mailed. The election returns and the
//! initialization report are held where they are produced, by the tally
//! (per Post, and per Post and country for the election returns); the other
//! reports where the Reports tab generates them. [`start_report_signing`]
//! starts the request and keeps how the report will be released. Each
//! signer adds a PAdES revision; once the request runs, [`release_report`]
//! releases the latest signed revision where the unsigned report would have
//! gone, wrapped in its configured password as it would have been, and
//! [`mail_held_report`] sends the e-mail it held back, after that commit and
//! once.

use super::super::guard::{GuardOutcome, GuardRequest, RequestScope, SigningDocument};
use super::super::log::SystemOutcome;
use super::super::pdf::{
    latest_signed_document, report_signature_page, tally_signing_action, RevisionStore, SigningBase,
};
use super::super::requests::{cancel_request, last_signer, stage_request_step};
use super::super::{SigningCaller, SigningError, SigningResult};
use super::transmission::sha256_hex;
use super::{gate, refuse, EffectProgress, EFFECT_PARTIAL};
use crate::postgres::document::{get_document, set_document_annotations};
use crate::postgres::reports::{get_reports_by_election_event_id, ReportType};
use crate::postgres::signing::{
    get_signing_request, list_waiting_signing_requests, lock_signing_event,
    lock_waiting_signing_request, SigningRequestRow,
};
use crate::postgres::signing_document_revision::list_document_revisions;
use crate::postgres::signing_report_release::{
    area_votes_in_post, claim_report_mail, fail_stale_mail_claims, finish_report_mail,
    finish_tally_hold, get_report_release, insert_report_release, insert_tally_hold,
    list_events_with_pending_tally_holds, list_unmailed_releases, lock_pending_tally_holds,
    mark_report_released, set_result_pdf, tally_hold_was_superseded, ReleaseEncryption,
    ReleaseTarget, ReleasedDocument, ReportEmail, ReportRelease, TallyHoldRow,
};
use crate::services::ceremonies::encrypter::encrypt_directory_contents_sql;
use crate::services::consolidation::aes_256_cbc_encrypt::encrypt_file_aes_256_cbc;
use crate::services::document_password::save_password;
use crate::services::documents::upload_and_return_document_with_annotations;
use crate::services::providers::email_sender::{Attachment, EmailSender};
use crate::services::reports::generation::{
    attach_report_manifest, manifest_of, manifest_of_document, released_manifest, ReportRequester,
};
use crate::services::reports_vault::get_report_secret_key;
use crate::services::signing::guard::SigningRequestSummary;
use crate::services::vault;
use crate::tasks::signing_log_outbox::kick_signing_log_outbox;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::{Client, Transaction};
use electoral_log::messages::newtypes::SigningStatementKind;
use sequent_core::election_config::archive::Artifact;
use sequent_core::election_config::manifest::ReportManifest;
use sequent_core::signing::{
    CancelReason, DocumentKind, DocumentRevisionState, DocumentSubject, SigningAction, SigningScope,
};
use sequent_core::types::hasura::core::DocumentAnnotations;
use sequent_core::util::temp_path::{generate_temp_file, get_file_size};
use serde::Serialize;
use serde_json::{json, Value};
use std::str::FromStr;
use tracing::{instrument, warn};
use uuid::Uuid;

const PDF_MEDIA_TYPE: &str = "application/pdf";
/// How an encrypted report is attached, as an unsigned one is.
const ENCRYPTED_MEDIA_TYPE: &str = "application/octet-stream";

/// Who starts a request nobody started by hand: the cron's executer of a
/// scheduled report, or the tally. It reaches every Post and holds no
/// sign permission, so it never signs ([`SigningCaller::system`]).
pub fn system_requester(name: &str) -> SigningCaller {
    SigningCaller::system(name)
}

/// Who started the tally whose reports are held: their requests'
/// requester, so the requester rule applies to them as to anyone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TallyRequester {
    pub user_id: String,
    pub username: String,
}

/// Renders a report's HTML as a PDF, as the tally renders its reports.
pub trait ReportRenderer: Send + Sync {
    fn render(&self, html: String) -> Result<Vec<u8>>;
}

/// Headless Chromium with the tally's PDF options.
pub struct ChromiumReportRenderer {
    pub options: Option<sequent_core::types::templates::PrintToPdfOptionsLocal>,
}

impl ReportRenderer for ChromiumReportRenderer {
    fn render(&self, html: String) -> Result<Vec<u8>> {
        sequent_core::services::pdf::sync::PdfRenderer::render_pdf(
            html,
            self.options
                .as_ref()
                .map(|options| options.to_print_to_pdf_options()),
        )
        .map_err(|error| anyhow!("Error rendering the report: {error:?}"))
    }
}

/// A report held for its signatures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportToSign {
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    /// The Post the report is of.
    pub election_id: Option<Uuid>,
    /// The country, for the election returns.
    pub area_id: Option<Uuid>,
    pub report_type: String,
    pub template_id: Option<String>,
    /// What one request stands for within the Post: a configured report, or
    /// a tally's report of the Post or a country of it. A newer one for the
    /// same key replaces the waiting request.
    pub subject_key: String,
    /// The report's base: its rendering with the signature page.
    pub base: SigningBase,
    pub release: ReportRelease,
}

/// The subject key of a tally's report of a Post or a country of it: one
/// request per results (a recount's results get their own request).
pub fn tally_subject_key(
    report_type: &str,
    results_event_id: Uuid,
    area_id: Option<Uuid>,
) -> String {
    match area_id {
        Some(area_id) => format!("tally|{report_type}|{results_event_id}|{area_id}"),
        None => format!("tally|{report_type}|{results_event_id}|post"),
    }
}

impl ReportToSign {
    pub fn subject(&self) -> DocumentSubject {
        DocumentSubject {
            report_type: self.report_type.clone(),
            document_sha256: self.base.sha256.clone(),
            template_id: self.template_id.clone(),
        }
    }

    pub fn guard_request(&self) -> SigningResult<GuardRequest> {
        let election_id = self.election_id.ok_or_else(|| {
            SigningError::bad_input("A report that needs signatures is generated for a Post.")
        })?;
        let area_id = match self.base.action.scope() {
            SigningScope::PostAndCountry => self.area_id,
            _ => None,
        };
        Ok(GuardRequest {
            action: self.base.action,
            scope: RequestScope {
                tenant_id: self.tenant_id,
                election_event_id: self.election_event_id,
                election_id: Some(election_id),
                area_id,
                trustee_id: None,
                subject_key: Some(self.subject_key.clone()),
            },
            subject: serde_json::to_value(self.subject())
                .context("Error writing the report subject")?,
            document: Some(SigningDocument {
                document_id: Some(self.base.document_id),
                sha256: self.base.sha256.clone(),
            }),
            config_revision: None,
        })
    }
}

/// Starts the signing request of a held report, and keeps how it will be
/// released. The country of a report of a Post and country is one of that
/// Post's.
/// Refuses when the rule no longer needs signatures: the report was held,
/// so it is generated again.
#[instrument(skip(hasura_transaction, report), err)]
pub async fn start_report_signing(
    hasura_transaction: &Transaction<'_>,
    requester: &SigningCaller,
    report: &ReportToSign,
) -> SigningResult<SigningRequestSummary> {
    if report.base.action.document() != DocumentKind::Pdf {
        return Err(SigningError::bad_input(format!(
            "A {} request signs no report.",
            report.base.action
        )));
    }
    let request = report.guard_request()?;
    // A country named with the election returns is one of the Post's.
    if let (Some(election_id), Some(area_id)) = (request.scope.election_id, request.scope.area_id) {
        if !area_votes_in_post(
            hasura_transaction,
            report.tenant_id,
            report.election_event_id,
            election_id,
            area_id,
        )
        .await?
        {
            return Err(SigningError::bad_input(
                "The country doesn't vote in the Post.",
            ));
        }
    }
    let outcome = gate(
        hasura_transaction,
        requester,
        report.base.action,
        report.tenant_id,
        report.election_event_id,
        || async { Ok(request) },
    )
    .await?;
    let summary = match outcome {
        GuardOutcome::SigningRequired(summary) => summary,
        GuardOutcome::Proceed => {
            return Err(SigningError::Conflict(
                "The report's signing rule changed while it was generated: generate it again."
                    .into(),
            ))
        }
    };
    insert_report_release(
        hasura_transaction,
        report.tenant_id,
        report.election_event_id,
        summary.id,
        &report.release,
    )
    .await?;
    Ok(summary)
}

/// Where a released report goes, and how the held e-mail is sent.
#[async_trait]
pub trait ReportPublisher: Send + Sync {
    /// Stores the signed `pdf` where the unsigned report would have gone,
    /// wrapped in its configured password when it has one.
    async fn publish(
        &self,
        hasura_transaction: &Transaction<'_>,
        request: &SigningRequestRow,
        release: &ReportRelease,
        pdf: &[u8],
    ) -> Result<ReleasedDocument>;

    /// Sends the held e-mail with the released document's bytes.
    async fn mail(
        &self,
        email: &ReportEmail,
        released: &ReleasedDocument,
        content: &[u8],
    ) -> Result<()>;
}

/// The documents service, the report passwords and the e-mail provider.
#[derive(Debug, Clone, Copy, Default)]
pub struct StoredReports;

/// The hash manifest the document held for `request`'s signatures was
/// stored with: it names the configuration its release is stamped with.
/// `None` for a report of an event not imported from a signed one.
async fn held_manifest(
    hasura_transaction: &Transaction<'_>,
    request: &SigningRequestRow,
) -> Result<Option<ReportManifest>> {
    let Some(document_id) = request.document_id else {
        return Ok(None);
    };
    let document = get_document(
        hasura_transaction,
        &request.tenant_id.to_string(),
        Some(request.election_event_id.to_string()),
        &document_id.to_string(),
    )
    .await?;
    manifest_of_document(
        document
            .as_ref()
            .and_then(|document| document.annotations.as_ref()),
    )
}

/// `name` with the extension of an encrypted report.
fn encrypted_name(name: &str) -> String {
    let stem = name.strip_suffix(".pdf").unwrap_or(name);
    format!("{stem}.epdf")
}

#[async_trait]
impl ReportPublisher for StoredReports {
    async fn publish(
        &self,
        hasura_transaction: &Transaction<'_>,
        request: &SigningRequestRow,
        release: &ReportRelease,
        pdf: &[u8],
    ) -> Result<ReleasedDocument> {
        let tenant_id = request.tenant_id.to_string();
        let event_id = request.election_event_id.to_string();
        let file = generate_temp_file("signed-report-", ".pdf")
            .context("Error creating the signed report's file")?;
        std::fs::write(file.path(), pdf).context("Error writing the signed report")?;
        let path = file.path().to_string_lossy().to_string();
        let held = held_manifest(hasura_transaction, request).await?;
        match &release.target {
            ReleaseTarget::Report { document_id } => {
                let document_id = document_id.to_string();
                let mut annotations = DocumentAnnotations::default();
                let (upload_path, name, media_type, _encrypted) = match release.encryption {
                    ReleaseEncryption::NoEncryption => {
                        (path, release.file_name.clone(), PDF_MEDIA_TYPE, None)
                    }
                    ReleaseEncryption::ConfiguredPassword => {
                        let report_id = release
                            .report_id
                            .ok_or_else(|| anyhow!("An encrypted report names its report"))?;
                        let password = vault::read_secret(
                            hasura_transaction,
                            &tenant_id,
                            Some(&event_id),
                            &get_report_secret_key(
                                &tenant_id,
                                &event_id,
                                Some(report_id.to_string()),
                            ),
                        )
                        .await?
                        .ok_or_else(|| anyhow!("Encryption password not found"))?;
                        let encrypted = generate_temp_file("signed-report-", ".epdf")
                            .context("Error creating the encrypted report's file")?
                            .into_temp_path();
                        let encrypted_path = encrypted.to_string_lossy().to_string();
                        encrypt_file_aes_256_cbc(&path, &encrypted_path, &password)?;
                        // Downloads find the password the file was wrapped with.
                        let password_secret_id = save_password(
                            hasura_transaction,
                            &tenant_id,
                            Some(&event_id),
                            &document_id,
                            &password,
                        )
                        .await?;
                        annotations
                            .access
                            .get_or_insert_with(Default::default)
                            .password_secret_id = Some(password_secret_id);
                        (
                            encrypted_path,
                            encrypted_name(&release.file_name),
                            ENCRYPTED_MEDIA_TYPE,
                            Some(encrypted),
                        )
                    }
                };
                // The hash manifest of the report as it is released: the
                // signed file, and the encrypted one when that is stored.
                if let Some(held) = &held {
                    let signed = Artifact {
                        name: release.file_name.clone(),
                        bytes: pdf.to_vec(),
                    };
                    let stored = match release.encryption {
                        ReleaseEncryption::NoEncryption => None,
                        ReleaseEncryption::ConfiguredPassword => Some(Artifact {
                            name: name.clone(),
                            bytes: std::fs::read(&upload_path)
                                .context("Error reading the encrypted report")?,
                        }),
                    };
                    attach_report_manifest(
                        hasura_transaction,
                        &tenant_id,
                        &event_id,
                        &released_manifest(held, signed, stored)?,
                        &ReportRequester::default(),
                        &mut annotations,
                    )
                    .await?;
                }
                let size = get_file_size(&upload_path)
                    .map_err(|error| anyhow!("Error reading the report's size: {error}"))?;
                upload_and_return_document_with_annotations(
                    hasura_transaction,
                    &upload_path,
                    size,
                    PDF_MEDIA_TYPE,
                    &tenant_id,
                    Some(event_id.clone()),
                    &name,
                    Some(document_id.clone()),
                    release.is_public,
                    &annotations,
                )
                .await
                .context("Error uploading the signed report")?;
                Ok(ReleasedDocument {
                    document_id: Uuid::parse_str(&document_id)?,
                    name,
                    media_type: media_type.into(),
                    protected: release.encryption == ReleaseEncryption::ConfiguredPassword,
                })
            }
            ReleaseTarget::TallyResult {
                results_event_id,
                election_id,
                area_id,
                report_type,
            } => {
                // Wrapped as the tally wraps its reports.
                let report_type = ReportType::from_str(report_type)
                    .map_err(|_| anyhow!("Unknown report type {report_type}"))?;
                let all_reports =
                    get_reports_by_election_event_id(hasura_transaction, &tenant_id, &event_id)
                        .await?;
                let wrapped_path = encrypt_directory_contents_sql(
                    hasura_transaction,
                    &tenant_id,
                    &event_id,
                    None,
                    report_type,
                    &path,
                    &all_reports,
                )
                .await?;
                let protected = wrapped_path != path;
                let upload_path = wrapped_path;
                // The hash manifest of the file that is stored, wrapped or
                // not.
                let mut annotations = DocumentAnnotations::default();
                if let Some(held) = &held {
                    let stored = Artifact {
                        name: release.file_name.clone(),
                        bytes: std::fs::read(&upload_path)
                            .context("Error reading the signed report")?,
                    };
                    attach_report_manifest(
                        hasura_transaction,
                        &tenant_id,
                        &event_id,
                        &released_manifest(held, stored, None)?,
                        &ReportRequester::default(),
                        &mut annotations,
                    )
                    .await?;
                }
                let size = get_file_size(&upload_path)
                    .map_err(|error| anyhow!("Error reading the report's size: {error}"))?;
                let document = upload_and_return_document_with_annotations(
                    hasura_transaction,
                    &upload_path,
                    size,
                    PDF_MEDIA_TYPE,
                    &tenant_id,
                    Some(event_id.clone()),
                    &release.file_name,
                    None,
                    false,
                    &annotations,
                )
                .await
                .context("Error uploading the signed report")?;
                let document_id = Uuid::parse_str(&document.id)?;
                set_result_pdf(
                    hasura_transaction,
                    request.tenant_id,
                    request.election_event_id,
                    *results_event_id,
                    *election_id,
                    *area_id,
                    document_id,
                )
                .await?;
                Ok(ReleasedDocument {
                    document_id,
                    name: release.file_name.clone(),
                    media_type: PDF_MEDIA_TYPE.into(),
                    protected,
                })
            }
        }
    }

    async fn mail(
        &self,
        email: &ReportEmail,
        released: &ReleasedDocument,
        content: &[u8],
    ) -> Result<()> {
        EmailSender::new()
            .await
            .map_err(|error| anyhow!("Error getting the e-mail sender: {error:?}"))?
            .send(
                email.recipients.clone(),
                email.subject.clone(),
                email.plaintext_body.clone(),
                email.html_body.clone(),
                vec![Attachment {
                    filename: released.name.clone(),
                    mimetype: released.media_type.clone(),
                    content: content.to_vec(),
                }],
            )
            .await
            .map_err(|error| anyhow!("Error sending the signed report: {error:?}"))
    }
}

/// The effect of an executed report request: the latest signed revision is
/// released where the unsigned report would have gone. Before anything
/// leaves the database it checks the release, that the document carries
/// every signature the request needs and that its bytes are the signed
/// ones; otherwise it refuses. The held e-mail waits for the commit
/// ([`mail_held_report`]). The result names the signed PDF's SHA-256.
#[instrument(skip_all, fields(request_id = %request.id), err)]
pub async fn release_report(
    hasura_transaction: &Transaction<'_>,
    store: &dyn RevisionStore,
    publisher: &dyn ReportPublisher,
    request: &SigningRequestRow,
    progress: &EffectProgress,
) -> Result<Value> {
    let (tenant_id, event_id) = (request.tenant_id, request.election_event_id);
    let release = get_report_release(hasura_transaction, tenant_id, event_id, request.id, true)
        .await?
        .ok_or_else(|| {
            refuse(
                "no-release",
                format!("Signing request {} releases no report", request.code),
            )
        })?;
    let latest = latest_signed_document(hasura_transaction, request)
        .await
        .map_err(|error| anyhow!("{error}"))?
        .filter(|latest| latest.state == DocumentRevisionState::Signed)
        .ok_or_else(|| {
            refuse(
                "no-signed-revision",
                format!("Signing request {} has no signed document", request.code),
            )
        })?;
    if let (Some(released_at), Some(released)) = (release.released_at, &release.released) {
        // Released before: the same result again.
        return Ok(json!({
            "document_id": released.document_id,
            "sha256": latest.sha256,
            "revision": latest.revision,
            "released_at": released_at,
        }));
    }
    let signed = list_document_revisions(hasura_transaction, tenant_id, event_id, request.id)
        .await?
        .iter()
        .filter(|revision| revision.state == DocumentRevisionState::Signed)
        .count();
    if signed != usize::try_from(request.required).unwrap_or(usize::MAX) {
        return Err(refuse(
            "signatures-missing",
            format!(
                "The document of signing request {} carries {signed} of {} signatures",
                request.code, request.required
            ),
        ));
    }
    let document_id = latest
        .document_id
        .ok_or_else(|| anyhow!("The signed revision has no document"))?;
    let pdf = store
        .load(hasura_transaction, tenant_id, event_id, document_id)
        .await?;
    if sha256_hex(&pdf) != latest.sha256 {
        return Err(refuse(
            "revision-changed",
            "The signed revision's bytes are not the signed ones",
        ));
    }

    progress.reach_outside();
    let released = publisher
        .publish(hasura_transaction, request, &release.release, &pdf)
        .await?;
    mark_report_released(
        hasura_transaction,
        tenant_id,
        event_id,
        request.id,
        &released,
    )
    .await?;
    Ok(json!({
        "document_id": released.document_id,
        "sha256": latest.sha256,
        "revision": latest.revision,
        "mail": release.release.email.is_some(),
        "protected": released.protected,
    }))
}

/// What sending a released report's held e-mail did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeldMail {
    /// No e-mail held, not released yet, or sent (or tried) before.
    Nothing,
    Sent,
    /// The send failed; it is kept and not tried again.
    Failed(String),
}

/// Sends the e-mail a released report held back, after the release
/// committed, at most once: the row is claimed first, and a failed send is
/// recorded (and logged as an error, `effect-partial`) instead of being
/// tried again.
#[instrument(skip(client, store, publisher), err)]
pub async fn mail_held_report(
    client: &mut Client,
    store: &dyn RevisionStore,
    publisher: &dyn ReportPublisher,
    tenant_id: Uuid,
    election_event_id: Uuid,
    request_id: Uuid,
) -> Result<HeldMail> {
    let transaction = client.transaction().await?;
    let claimed = claim_report_mail(&transaction, tenant_id, election_event_id, request_id).await?;
    let release = get_report_release(
        &transaction,
        tenant_id,
        election_event_id,
        request_id,
        false,
    )
    .await?;
    transaction.commit().await?;
    let (true, Some(release)) = (claimed, release) else {
        return Ok(HeldMail::Nothing);
    };
    let (Some(email), Some(released)) = (&release.release.email, &release.released) else {
        return Ok(HeldMail::Nothing);
    };

    let read = client.transaction().await?;
    let content = store
        .load(&read, tenant_id, election_event_id, released.document_id)
        .await;
    read.rollback().await?;
    let sent = match content {
        Ok(content) => publisher.mail(email, released, &content).await,
        Err(error) => Err(error),
    };
    let failure = sent.as_ref().err().map(|error| format!("{error:#}"));

    let transaction = client.transaction().await?;
    lock_signing_event(&transaction, tenant_id, election_event_id).await?;
    finish_report_mail(
        &transaction,
        tenant_id,
        election_event_id,
        request_id,
        failure.as_deref(),
    )
    .await?;
    if let Some(failure) = &failure {
        warn!(%request_id, "the signed report was released but not mailed: {failure}");
        if let Some(request) =
            get_signing_request(&transaction, tenant_id, election_event_id, request_id).await?
        {
            stage_request_step(
                &transaction,
                &request,
                SigningStatementKind::SigningActionExecuted,
                last_signer(&transaction, &request).await?,
                SystemOutcome::Error,
                format!(
                    "Released the signed report of signing request {} but could not mail it",
                    request.code
                ),
                json!({ "code": EFFECT_PARTIAL, "step": "mail" }),
            )
            .await?;
        }
    }
    transaction.commit().await?;
    if failure.is_some() {
        kick_signing_log_outbox();
    }
    Ok(match failure {
        Some(failure) => HeldMail::Failed(failure),
        None => HeldMail::Sent,
    })
}

/// Whether `action` releases a held report.
pub fn releases_a_report(action: SigningAction) -> bool {
    matches!(
        action,
        SigningAction::GenerateElectionReturns | SigningAction::GenerateReports
    )
}

/// A Post's (or a Post and country's) report the tally produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TallyReport<'a> {
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    pub results_event_id: Uuid,
    pub action: SigningAction,
    /// The tally's report type (`ELECTORAL_RESULTS`, `INITIALIZATION_REPORT`).
    pub report_type: &'a str,
    pub election_id: Uuid,
    pub area_id: Option<Uuid>,
    /// The file name the results row stores it under.
    pub file_name: &'a str,
    pub pdf: &'a [u8],
    pub requester: &'a TallyRequester,
    /// The hash manifest of the report's folder, for an event imported from
    /// a signed configuration: its release is stamped with what it names.
    pub configuration: Option<&'a ReportManifest>,
}

/// The annotations of the document kept to be signed: the hash manifest of
/// that file, under the configuration its folder's manifest names.
pub fn held_annotations(
    configuration: &ReportManifest,
    file_name: &str,
    base: &[u8],
) -> Result<DocumentAnnotations> {
    let written = manifest_of(
        &configuration.report_type,
        &configuration.configuration,
        &[Artifact {
            name: file_name.to_string(),
            bytes: base.to_vec(),
        }],
    )?;
    Ok(DocumentAnnotations {
        report_manifest: Some(serde_json::to_value(&written.manifest)?),
        ..Default::default()
    })
}

/// Holds a tally's report of a Post (or a Post and country) whose action
/// needs signatures: its signature page is added, the result is kept as a
/// signing document only and the hold is recorded. Its request starts
/// after the tally's transaction commits ([`start_held_tally_reports`]).
///
/// **Lock order.** The tally's transaction holds locks on the tally
/// session's row (its executions reference it) before it gets here, while
/// every signing step takes the event's signing lock before any row lock.
/// So this step takes no signing lock and starts no request: it only
/// writes rows of its own. Once the request runs, the signed report becomes
/// the PDF of its results row.
#[instrument(skip_all, fields(report_type = report.report_type), err)]
pub async fn hold_tally_report(
    hasura_transaction: &Transaction<'_>,
    store: &dyn RevisionStore,
    report: &TallyReport<'_>,
) -> Result<Uuid> {
    let base = report_signature_page(
        hasura_transaction,
        report.tenant_id,
        report.election_event_id,
        report.action,
        report.pdf,
    )
    .await?
    .ok_or_else(|| anyhow!("{} no longer needs signatures", report.action))?;
    let stem = report
        .file_name
        .strip_suffix(".pdf")
        .unwrap_or(report.file_name);
    let to_sign = format!("{stem}-to-sign.pdf");
    let base_document_id = store
        .store(
            hasura_transaction,
            report.tenant_id,
            report.election_event_id,
            &to_sign,
            &base,
        )
        .await?;
    if let Some(configuration) = report.configuration {
        set_document_annotations(
            hasura_transaction,
            report.tenant_id,
            report.election_event_id,
            base_document_id,
            &held_annotations(configuration, &to_sign, &base)?,
        )
        .await?;
    }
    let id = Uuid::new_v4();
    insert_tally_hold(
        hasura_transaction,
        &TallyHoldRow {
            id,
            tenant_id: report.tenant_id,
            election_event_id: report.election_event_id,
            results_event_id: report.results_event_id,
            action: report.action.to_string(),
            report_type: report.report_type.to_owned(),
            election_id: report.election_id,
            area_id: report.area_id,
            file_name: report.file_name.to_owned(),
            base_document_id,
            base_sha256: sha256_hex(&base),
            requested_by: report.requester.user_id.clone(),
            requested_by_username: report.requester.username.clone(),
            request_id: None,
            error: None,
        },
    )
    .await?;
    Ok(id)
}

/// The request a held tally report waits for.
fn held_report(hold: &TallyHoldRow) -> Result<ReportToSign> {
    let action = SigningAction::from_str(&hold.action)
        .map_err(|_| anyhow!("Unknown signing action {}", hold.action))?;
    Ok(ReportToSign {
        tenant_id: hold.tenant_id,
        election_event_id: hold.election_event_id,
        election_id: Some(hold.election_id),
        area_id: hold.area_id,
        report_type: hold.report_type.clone(),
        template_id: None,
        subject_key: tally_subject_key(&hold.report_type, hold.results_event_id, hold.area_id),
        base: SigningBase {
            action,
            document_id: hold.base_document_id,
            sha256: hold.base_sha256.clone(),
        },
        release: ReportRelease {
            report_id: None,
            target: ReleaseTarget::TallyResult {
                results_event_id: hold.results_event_id,
                election_id: hold.election_id,
                area_id: hold.area_id,
                report_type: hold.report_type.clone(),
            },
            file_name: hold.file_name.clone(),
            is_public: false,
            // Wrapped at the release as the tally wraps its reports.
            encryption: ReleaseEncryption::NoEncryption,
            email: None,
        },
    })
}

/// Cancels an earlier tally's waiting report for the same Post, country and
/// report type. The caller already holds the event's signing lock. Keep this
/// outside the new request's savepoint: a failed start must not leave
/// superseded returns available to sign and release.
async fn cancel_superseded_tally_reports(
    transaction: &Transaction<'_>,
    caller: &SigningCaller,
    report: &ReportToSign,
) -> Result<usize> {
    let ReleaseTarget::TallyResult {
        results_event_id,
        election_id,
        area_id,
        report_type,
    } = &report.release.target
    else {
        return Ok(0);
    };
    let waiting = list_waiting_signing_requests(
        transaction,
        report.tenant_id,
        report.election_event_id,
        report.base.action,
    )
    .await?;
    let mut cancelled = 0;
    for candidate in waiting {
        let Some(release) = get_report_release(
            transaction,
            report.tenant_id,
            report.election_event_id,
            candidate.id,
            false,
        )
        .await?
        else {
            continue;
        };
        let superseded = matches!(
            &release.release.target,
            ReleaseTarget::TallyResult {
                results_event_id: previous_results,
                election_id: previous_post,
                area_id: previous_country,
                report_type: previous_type,
            } if previous_results != results_event_id
                && previous_post == election_id
                && previous_country == area_id
                && previous_type == report_type
        );
        if !superseded {
            continue;
        }
        if let Some(request) = lock_waiting_signing_request(
            transaction,
            report.tenant_id,
            report.election_event_id,
            report.base.action,
            &candidate.scope_key,
        )
        .await?
        {
            cancel_request(
                transaction,
                &request,
                CancelReason::PayloadChanged,
                caller.actor(),
                Some("The tally was recounted, so the report changed."),
            )
            .await?;
            cancelled += 1;
        }
    }
    Ok(cancelled)
}

/// Starts the requests of an event's held tally reports, oldest first, in
/// one transaction that takes the event's signing lock before any row lock,
/// each in its own savepoint so one Post's failure doesn't stop the others.
/// A failure is logged as an error and the report stays held: a passing
/// failure leaves it for the sweeper to start again; a refusal (its rule
/// was switched off, its Post is gone) or a request that signs another
/// document keeps the reason. How many started.
#[instrument(skip(client), err)]
pub async fn start_held_tally_reports(
    client: &mut Client,
    tenant_id: Uuid,
    election_event_id: Uuid,
) -> Result<usize> {
    let mut transaction = client.transaction().await?;
    lock_signing_event(&transaction, tenant_id, election_event_id).await?;
    let holds = lock_pending_tally_holds(&transaction, tenant_id, election_event_id).await?;
    let mut started = 0;
    let mut cancelled = 0;
    for hold in holds {
        if tally_hold_was_superseded(&transaction, &hold).await? {
            finish_tally_hold(
                &transaction,
                hold.id,
                None,
                Some("The tally was recounted, so the report changed."),
            )
            .await?;
            continue;
        }
        let report = held_report(&hold)?;
        let requester = system_requester(&hold.requested_by_username);
        let requester = SigningCaller {
            user_id: hold.requested_by.clone(),
            ..requester
        };
        cancelled += cancel_superseded_tally_reports(&transaction, &requester, &report).await?;
        let savepoint = transaction.savepoint("tally_hold").await?;
        let outcome = start_report_signing(&savepoint, &requester, &report).await;
        // The request must sign this base, never an older one's.
        let outcome = match outcome {
            Ok(summary) => {
                match get_signing_request(&savepoint, tenant_id, election_event_id, summary.id)
                    .await
                {
                    Ok(Some(row)) if row.document_sha256.as_ref() == Some(&hold.base_sha256) => {
                        Ok(summary)
                    }
                    Ok(_) => Err(SigningError::Conflict(format!(
                        "Signing request {} signs another document",
                        summary.code
                    ))),
                    Err(error) => Err(SigningError::Internal(error)),
                }
            }
            Err(error) => Err(error),
        };
        match outcome {
            Ok(summary) => {
                savepoint.commit().await?;
                finish_tally_hold(&transaction, hold.id, Some(summary.id), None).await?;
                started += 1;
            }
            Err(SigningError::Internal(error)) => {
                savepoint.rollback().await?;
                tracing::error!(
                    hold_id = %hold.id,
                    "a held tally report's request didn't start; it stays held: {error:#}"
                );
            }
            Err(refusal) => {
                savepoint.rollback().await?;
                tracing::error!(
                    hold_id = %hold.id,
                    "a held tally report's request can't start; it stays held: {refusal}"
                );
                finish_tally_hold(&transaction, hold.id, None, Some(&refusal.to_string())).await?;
            }
        }
    }
    transaction.commit().await?;
    if started > 0 || cancelled > 0 {
        kick_signing_log_outbox();
    }
    Ok(started)
}

/// [`start_held_tally_reports`] for every event with held tally reports
/// whose request hasn't started (a tally whose own start failed).
#[instrument(skip(client), err)]
pub async fn start_all_held_tally_reports(client: &mut Client) -> Result<usize> {
    let events = {
        let transaction = client.transaction().await?;
        list_events_with_pending_tally_holds(&transaction).await?
    };
    let mut started = 0;
    for (tenant_id, election_event_id) in events {
        match start_held_tally_reports(client, tenant_id, election_event_id).await {
            Ok(count) => started += count,
            Err(error) => warn!(%election_event_id, "held tally reports not started: {error:?}"),
        }
    }
    Ok(started)
}

/// Safe references to a report's signing request, without its document or payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HeldReportRequest {
    pub request_id: Uuid,
    pub code: String,
    pub report_type: String,
    pub election_id: Option<Uuid>,
    pub area_id: Option<Uuid>,
    pub report_id: Option<Uuid>,
    pub results_event_id: Option<Uuid>,
    pub tally_session_id: Option<Uuid>,
    pub results_document_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transmission_package: Option<Value>,
    pub status: sequent_core::signing::SigningRequestStatus,
}

/// Permissions that can discover report requests, still limited to the
/// requests the caller may open and the Posts their labels reach.
pub fn reads_report_requests(caller: &SigningCaller) -> bool {
    use sequent_core::types::permissions::Permissions;
    [
        Permissions::SIGNING_REQUESTS_READ,
        Permissions::SIGN_GENERATE_ELECTION_RETURNS,
        Permissions::SIGN_GENERATE_REPORTS,
        Permissions::REPORT_READ,
        Permissions::MIRU_CREATE,
        Permissions::MIRU_SEND,
    ]
    .into_iter()
    .any(|permission| caller.has(permission))
}

pub async fn held_report_requests(
    transaction: &Transaction<'_>,
    caller: &SigningCaller,
    tenant_id: Uuid,
    election_event_id: Uuid,
) -> SigningResult<Vec<HeldReportRequest>> {
    use sequent_core::types::permissions::Permissions;
    if !reads_report_requests(caller) {
        return Err(SigningError::Forbidden(
            "You can't read held report requests.".into(),
        ));
    }
    let rows = transaction
        .query(
            "SELECT request.*, release.report_id, release.results_event_id,
             COALESCE(release.report_type,
                 request.canonical_payload::jsonb #>> '{subject,report_type}') AS held_report_type,
             execution.tally_session_id AS held_tally_session_id,
             execution.annotations AS held_tally_annotations,
             CASE WHEN release.area_id IS NULL THEN post_result.documents::jsonb->>'json'
                 ELSE country_result.documents::jsonb->>'json' END AS held_results_document_id
         FROM sequent_backend.signing_request request
         JOIN sequent_backend.signing_report_release release ON release.request_id = request.id
             AND release.tenant_id = request.tenant_id
             AND release.election_event_id = request.election_event_id
         LEFT JOIN LATERAL (
             SELECT step.tally_session_id, tally.annotations
             FROM sequent_backend.tally_session_execution step
             JOIN sequent_backend.tally_session tally ON tally.id = step.tally_session_id
                 AND tally.tenant_id = request.tenant_id
                 AND tally.election_event_id = request.election_event_id
             WHERE step.tenant_id = request.tenant_id AND step.results_event_id = release.results_event_id
             ORDER BY step.created_at DESC LIMIT 1
         ) execution ON true
         LEFT JOIN sequent_backend.results_election post_result
             ON post_result.tenant_id = request.tenant_id
             AND post_result.results_event_id = release.results_event_id
             AND post_result.election_id = request.election_id
             AND release.area_id IS NULL
         LEFT JOIN sequent_backend.results_election_area country_result
             ON country_result.tenant_id = request.tenant_id
             AND country_result.results_event_id = release.results_event_id
             AND country_result.election_id = request.election_id
             AND country_result.area_id = release.area_id
         WHERE request.tenant_id = $1 AND request.election_event_id = $2
             AND request.action IN ('generate-election-returns', 'generate-reports')
             AND request.status IN ('waiting', 'completed', 'executed')
         ORDER BY request.created_at DESC, request.id",
            &[&tenant_id, &election_event_id],
        )
        .await
        .context("Error reading held report requests")?;
    let mut found = Vec::new();
    for row in rows {
        let request = SigningRequestRow::try_from(row.clone())?;
        if !caller.reaches(request.permission_label.as_deref())
            || !(caller.user_id == request.requested_by
                || caller.has(Permissions::SIGNING_REQUESTS_READ)
                || caller.has(request.action.sign_permission()))
        {
            continue;
        }
        let transmission_package = if caller.has(Permissions::MIRU_CREATE)
            || caller.has(Permissions::MIRU_SEND)
        {
            let annotations: Option<Value> = row.try_get("held_tally_annotations")?;
            let packages = annotations
                .as_ref()
                .and_then(|value| value.get("miru:tally-session-data"))
                .and_then(Value::as_str)
                .and_then(|text| serde_json::from_str::<Value>(text).ok());
            let post = request.election_id.map(|id| id.to_string());
            let country = request.area_id.map(|id| id.to_string());
            packages
                .as_ref()
                .and_then(Value::as_array)
                .and_then(|packages| {
                    packages.iter().find(|package| {
                        post.is_some()
                            && country.is_some()
                            && package.get("election_id").and_then(Value::as_str) == post.as_deref()
                            && package.get("area_id").and_then(Value::as_str) == country.as_deref()
                    })
                })
                .cloned()
        } else {
            None
        };
        found.push(HeldReportRequest {
            request_id: request.id,
            code: request.code,
            report_type: row
                .try_get::<_, Option<String>>("held_report_type")?
                .ok_or_else(|| anyhow!("A held report request names its report type"))?,
            election_id: request.election_id,
            area_id: request.area_id,
            report_id: row.try_get("report_id")?,
            results_event_id: row.try_get("results_event_id")?,
            tally_session_id: row.try_get("held_tally_session_id")?,
            results_document_id: row.try_get("held_results_document_id")?,
            transmission_package,
            status: request.status,
        });
    }
    Ok(found)
}

/// Whether a report type is one the tally produces and holds, and its rule
/// needs signatures now: then it is never generated or released elsewhere.
pub async fn held_by_the_tally(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    report_type: &ReportType,
) -> Result<bool> {
    match tally_signing_action(report_type) {
        Some(action) => Ok(super::super::guard::effective_rule(
            hasura_transaction,
            tenant_id,
            election_event_id,
            action,
        )
        .await?
        .is_required()),
        None => Ok(false),
    }
}

/// How long a claimed send of a held e-mail may take before it is taken
/// for failed.
pub const MAIL_CLAIM_SECONDS: i64 = 900;

/// The held e-mails the tasks didn't send: released reports whose e-mail no
/// send claimed are mailed now (once, as [`mail_held_report`] does); a
/// claimed send that never reported back is marked failed and logged as an
/// error (`effect-partial`), never sent again. How many were mailed.
#[instrument(skip(client, store, publisher), err)]
pub async fn sweep_held_mails(
    client: &mut Client,
    store: &dyn RevisionStore,
    publisher: &dyn ReportPublisher,
) -> Result<usize> {
    let transaction = client.transaction().await?;
    let stale = fail_stale_mail_claims(&transaction, MAIL_CLAIM_SECONDS).await?;
    let unmailed = list_unmailed_releases(&transaction).await?;
    transaction.commit().await?;
    for (tenant_id, election_event_id, request_id) in stale {
        let transaction = client.transaction().await?;
        lock_signing_event(&transaction, tenant_id, election_event_id).await?;
        if let Some(request) =
            get_signing_request(&transaction, tenant_id, election_event_id, request_id).await?
        {
            tracing::error!(%request_id, "a held e-mail's send never reported back");
            stage_request_step(
                &transaction,
                &request,
                SigningStatementKind::SigningActionExecuted,
                last_signer(&transaction, &request).await?,
                SystemOutcome::Error,
                format!(
                    "Released the signed report of signing request {} but its e-mail may not have been sent",
                    request.code
                ),
                json!({ "code": EFFECT_PARTIAL, "step": "mail" }),
            )
            .await?;
        }
        transaction.commit().await?;
        kick_signing_log_outbox();
    }
    let mut mailed = 0;
    for (tenant_id, election_event_id, request_id) in unmailed {
        match mail_held_report(
            client,
            store,
            publisher,
            tenant_id,
            election_event_id,
            request_id,
        )
        .await
        {
            Ok(HeldMail::Sent) => mailed += 1,
            Ok(_) => {}
            Err(error) => tracing::error!(%request_id, "a held e-mail wasn't sent: {error:?}"),
        }
    }
    Ok(mailed)
}
