// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Fixtures of the signing core tests: a committed tenant with one event and
//! one Post per test (the approve step owns its transactions), callers, test
//! certificates and a certificate verifier that checks the payload signature
//! and the registration the way the real one does, without a PKI. Include
//! it after `schema`: `#[path = "support/signing.rs"] mod signing;`.

#![allow(dead_code)]

use super::schema;
use async_trait::async_trait;
use chrono::{DateTime, Duration, SubsecRound, TimeZone, Utc};
use deadpool_postgres::{Pool, Transaction};
use sequent_core::signing::{
    CertificateCheckId, CertificateCheckResult, DocumentKind, RequesterSigning, RevocationStatus,
    SignatureAlgorithm, SigningAction, SigningRequirement, SigningRule,
    StaffCertificateRegistration,
};
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock};
use strum::IntoEnumIterator;
use uuid::Uuid;
use windmill::postgres::signing::*;
use windmill::services::signing::approve::{
    approve, ApproveInput, ApproveOutcome, DocumentSigner, NoDocumentSigner, SigningServices,
};
use windmill::services::signing::certificates::{
    CertificateIdentity, CertificateVerification, CertificateVerificationInput,
    CertificateVerifier, RegistrationState,
};
use windmill::services::signing::executors::{
    ExecutionOutcome, SigningExecutor, SigningExecutorRegistry,
};
use windmill::services::signing::guard::{
    guard_at, GuardOutcome, GuardRequest, RequestScope, SigningRequestSummary,
};
use windmill::services::signing::requests::{SigningExportStore, StoredExport};
use windmill::services::signing::{SigningCaller, SigningResult};

pub fn sha(text: &str) -> String {
    hex::encode(Sha256::digest(text.as_bytes()))
}

/// `minute` minutes after the binary started: the time a request is
/// started at and a certificate is checked at. Expiry follows the
/// database's clock; a test makes a request overdue with
/// [`World::make_overdue`].
pub fn at(minute: u32) -> DateTime<Utc> {
    static BASE: OnceLock<DateTime<Utc>> = OnceLock::new();
    let base = BASE.get_or_init(|| Utc::now().trunc_subsecs(0));
    *base + Duration::minutes(i64::from(minute))
}

/// An election's presentation naming it.
pub fn post_presentation(name: &str) -> Value {
    json!({ "i18n": { "en": { "name": name } } })
}

/// One committed tenant, event, Post (election) and country (area).
#[derive(Clone)]
pub struct World {
    pub pool: Pool,
    pub tenant: Uuid,
    pub event: Uuid,
    pub post: Uuid,
    pub area: Uuid,
    /// The Post's permission label.
    pub label: String,
}

/// A new world whose Post carries `label`.
pub async fn world(label: &str) -> World {
    let pool = schema::pool().await;
    let tenant = Uuid::new_v4();
    let event = Uuid::new_v4();
    let post = Uuid::new_v4();
    let area = Uuid::new_v4();
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
        &[&tenant, &format!("tenant-{tenant}")],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol)
         VALUES ($1, $2, 'RSA256')",
        &[&event, &tenant],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.election (id, tenant_id, election_event_id, presentation, permission_label)
         VALUES ($1, $2, $3, $4, $5)",
        &[&post, &tenant, &event, &post_presentation(&format!("Post {label}")), &label],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.area (id, tenant_id, election_event_id, name)
         VALUES ($1, $2, $3, 'Country')",
        &[&area, &tenant, &event],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    World {
        pool,
        tenant,
        event,
        post,
        area,
        label: label.to_owned(),
    }
}

impl World {
    /// A signer of `action` for this world's Post.
    pub fn signer(&self, user: &str, action: SigningAction) -> SigningCaller {
        caller(user, &[action.sign_permission()], &[&self.label])
    }

    /// The Post's scope for `action`.
    pub fn scope(&self, action: SigningAction) -> RequestScope {
        RequestScope {
            tenant_id: self.tenant,
            election_event_id: self.event,
            election_id: Some(self.post),
            area_id: None,
            trustee_id: None,
            subject_key: Some(action.to_string()),
        }
    }

    /// Saves a rule, committed.
    pub async fn rule(
        &self,
        action: SigningAction,
        signatures: u16,
        requester: RequesterSigning,
        expires_minutes: Option<u32>,
    ) {
        let mut client = self.pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let current = get_signing_rule(&tx, self.tenant, self.event, action)
            .await
            .unwrap()
            .map(|row| row.rule.revision)
            .unwrap_or(0);
        upsert_signing_rule(
            &tx,
            self.tenant,
            self.event,
            &SigningRule {
                action,
                requirement: SigningRequirement::Required,
                signatures,
                requester_signing: requester,
                expires_minutes,
                revision: 0,
            },
            current,
            "configuration-manager",
            Some("Configuration Manager"),
        )
        .await
        .unwrap()
        .unwrap();
        tx.commit().await.unwrap();
    }

    /// Runs the guard at `now`, committed.
    pub async fn guard(
        &self,
        caller: &SigningCaller,
        action: SigningAction,
        subject: Value,
        now: DateTime<Utc>,
    ) -> SigningResult<GuardOutcome> {
        self.guard_request(
            caller,
            &GuardRequest {
                action,
                scope: self.scope(action),
                subject,
                document: None,
                config_revision: None,
            },
            now,
        )
        .await
    }

    pub async fn guard_request(
        &self,
        caller: &SigningCaller,
        request: &GuardRequest,
        now: DateTime<Utc>,
    ) -> SigningResult<GuardOutcome> {
        let mut client = self.pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let outcome = guard_at(&tx, caller, request, now).await;
        if outcome.is_ok() {
            tx.commit().await.unwrap();
        }
        outcome
    }

    /// Starts a request that needs signatures.
    pub async fn start(
        &self,
        caller: &SigningCaller,
        action: SigningAction,
        subject: Value,
        now: DateTime<Utc>,
    ) -> SigningRequestSummary {
        match self.guard(caller, action, subject, now).await.unwrap() {
            GuardOutcome::SigningRequired(summary) => summary,
            GuardOutcome::Proceed => panic!("{action} needs no signatures"),
        }
    }

    /// Moves a request's time limit into the past.
    pub async fn make_overdue(&self, id: Uuid) {
        self.pool
            .get()
            .await
            .unwrap()
            .execute(
                "UPDATE sequent_backend.signing_request
                 SET expires_at = clock_timestamp() - interval '1 second' WHERE id = $1",
                &[&id],
            )
            .await
            .unwrap();
    }

    /// Runs `sql` with `params` on its own connection.
    pub async fn execute(&self, sql: &str, params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) {
        self.pool
            .get()
            .await
            .unwrap()
            .execute(sql, params)
            .await
            .unwrap();
    }

    pub async fn request(&self, id: Uuid) -> SigningRequestRow {
        let mut client = self.pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        get_signing_request(&tx, self.tenant, self.event, id)
            .await
            .unwrap()
            .unwrap()
    }

    pub async fn approvals(&self, id: Uuid) -> Vec<SigningApprovalRow> {
        let mut client = self.pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        list_signing_approvals(&tx, self.tenant, self.event, id)
            .await
            .unwrap()
    }

    /// Registers `certificate` to `user`, committed; `linked_to` for a
    /// Security Officer's link to the holder's other account.
    pub async fn register(&self, user: &str, certificate: &TestCert, linked_to: Option<&str>) {
        let mut client = self.pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let identity = &certificate.identity;
        insert_staff_certificate(
            &tx,
            &NewStaffCertificate {
                tenant_id: self.tenant,
                election_event_id: self.event,
                user_id: user.to_owned(),
                username: user.to_owned(),
                election_id: Some(self.post),
                fingerprint_sha256: identity.fingerprint_sha256.clone(),
                spki_sha256: identity.spki_sha256.clone(),
                holder_sha256: identity.holder_sha256.clone(),
                serial: identity.serial.clone(),
                subject: identity.subject.clone(),
                issuer: identity.issuer.clone(),
                not_before: identity.not_before,
                not_after: identity.not_after,
                pem: identity.pem.clone(),
                registration: if linked_to.is_some() {
                    StaffCertificateRegistration::SecurityOfficer
                } else {
                    StaffCertificateRegistration::FirstUse
                },
                linked_to: linked_to.map(str::to_owned),
                registered_by: user.to_owned(),
                user_display_name: Some(format!("{user} display")),
                registered_by_name: None,
            },
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
    }

    /// `caller` signs request `id` with `certificate` at `now`.
    pub async fn sign(
        &self,
        services: &SigningServices,
        caller: &SigningCaller,
        id: Uuid,
        certificate: &TestCert,
        now: DateTime<Utc>,
    ) -> SigningResult<ApproveOutcome> {
        let request = self.request(id).await;
        let mut client = self.pool.get().await.unwrap();
        approve(
            &mut client,
            services,
            caller,
            self.tenant,
            &ApproveInput {
                request_id: id,
                chain_pem: vec![certificate.identity.pem.clone()],
                algorithm: SignatureAlgorithm::RsaPkcs1Sha256,
                payload_signature: signature_over(&request.canonical_payload),
                // What a document action also signs; a fake document side takes it.
                document_signature: (request.action.document() == DocumentKind::Eml)
                    .then(|| vec![1]),
                pdf_cms: (request.action.document() == DocumentKind::Pdf).then(|| vec![1]),
                revision: (request.action.document() == DocumentKind::Pdf).then_some(1),
            },
            now,
        )
        .await
    }

    /// The outbox rows of the event: (step, kind, event type, log type,
    /// user, description), in order.
    pub async fn outbox(&self) -> Vec<(Uuid, String, String, String, Option<String>, String)> {
        let client = self.pool.get().await.unwrap();
        client
            .query(
                "SELECT step_id, statement_kind, event_type, log_type, user_id,
                        body->>'description'
                 FROM sequent_backend.signing_log_outbox
                 WHERE tenant_id = $1 AND election_event_id = $2 ORDER BY id",
                &[&self.tenant, &self.event],
            )
            .await
            .unwrap()
            .into_iter()
            .map(|row| {
                (
                    row.get(0),
                    row.get(1),
                    row.get(2),
                    row.get(3),
                    row.get(4),
                    row.get(5),
                )
            })
            .collect()
    }

    /// The kinds of the steps logged, one per step, in order.
    pub async fn steps(&self) -> Vec<String> {
        let rows = self.outbox().await;
        let mut steps = vec![];
        let mut seen = HashSet::new();
        for (step, kind, ..) in rows {
            if seen.insert(step) {
                steps.push(kind);
            }
        }
        steps
    }

    /// Every step staged exactly a USER and a SYSTEM entry.
    pub async fn assert_two_entries_per_step(&self) {
        let rows = self.outbox().await;
        let mut per_step: HashMap<Uuid, Vec<String>> = HashMap::new();
        for (step, _, event_type, ..) in &rows {
            per_step.entry(*step).or_default().push(event_type.clone());
        }
        assert!(!per_step.is_empty());
        for (step, entries) in per_step {
            assert_eq!(entries, ["USER", "SYSTEM"], "step {step}");
        }
    }
}

/// A caller with `roles` and permission `labels`.
pub fn caller(user: &str, roles: &[Permissions], labels: &[&str]) -> SigningCaller {
    SigningCaller {
        user_id: user.to_owned(),
        username: format!("{user}-name"),
        display_name: format!("{user} display"),
        roles: roles.iter().map(ToString::to_string).collect(),
        labels: labels.iter().map(|label| label.to_string()).collect(),
        auth_time: Some(at(0)),
        trustee: None,
    }
}

/// What the fake verifier accepts as the signature of a payload.
pub fn signature_over(canonical_payload: &str) -> Vec<u8> {
    format!("signed:{}", sha(canonical_payload)).into_bytes()
}

/// A certificate whose certificate, key and holder hashes derive from the
/// given labels. Its PEM is what identifies it to [`FakeVerifier`].
#[derive(Debug, Clone)]
pub struct TestCert {
    pub identity: CertificateIdentity,
}

pub fn cert(certificate: &str, key: &str, holder: &str) -> TestCert {
    cert_with_pem(
        certificate,
        key,
        holder,
        &format!("test-certificate:{certificate}"),
    )
}

pub fn cert_with_pem(certificate: &str, key: &str, holder: &str, pem: &str) -> TestCert {
    TestCert {
        identity: CertificateIdentity {
            fingerprint_sha256: sha(&format!("certificate {certificate}")),
            spki_sha256: sha(&format!("key {key}")),
            holder_sha256: sha(&format!("holder {holder}")),
            serial: "01".into(),
            subject: format!("CN={holder}"),
            common_name: holder.to_owned(),
            issuer: "CN=Staff CA".into(),
            not_before: Utc.with_ymd_and_hms(2028, 1, 1, 0, 0, 0).unwrap(),
            not_after: Utc.with_ymd_and_hms(2029, 1, 1, 0, 0, 0).unwrap(),
            pem: pem.to_owned(),
            chain_pem: pem.to_owned(),
            key_algorithm: Some(SignatureAlgorithm::RsaPkcs1Sha256),
        },
    }
}

/// A self-signed certificate for `common_name`, for what the panel reads
/// from an approval's PEM.
pub fn real_pem(common_name: &str) -> String {
    use openssl::asn1::Asn1Time;
    use openssl::hash::MessageDigest;
    use openssl::pkey::PKey;
    use openssl::rsa::Rsa;
    use openssl::x509::{X509NameBuilder, X509};
    let key = PKey::from_rsa(Rsa::generate(2048).unwrap()).unwrap();
    let mut name = X509NameBuilder::new().unwrap();
    name.append_entry_by_text("O", "Test Organization").unwrap();
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
    String::from_utf8(builder.build().to_pem().unwrap()).unwrap()
}

/// Knows a set of certificates by PEM. Every check passes except: an
/// unknown PEM (`trusted-issuer`), a scripted refusal, a certificate not
/// registered to the signer (`registered`), and a payload signature that is
/// not [`signature_over`] the request's canonical payload (`signature`).
#[derive(Default)]
pub struct FakeVerifier {
    certificates: Vec<TestCert>,
    refusals: HashMap<String, CertificateCheckId>,
    /// A certificate nobody registered registers on its first use.
    first_use: bool,
}

impl FakeVerifier {
    pub fn knowing(certificates: &[&TestCert]) -> Self {
        FakeVerifier {
            certificates: certificates.iter().map(|c| (*c).clone()).collect(),
            refusals: HashMap::new(),
            first_use: false,
        }
    }

    pub fn registering_on_first_use(mut self) -> Self {
        self.first_use = true;
        self
    }

    pub fn refusing(mut self, certificate: &TestCert, check: CertificateCheckId) -> Self {
        self.refusals
            .insert(certificate.identity.pem.clone(), check);
        self
    }
}

#[async_trait]
impl CertificateVerifier for FakeVerifier {
    async fn verify(
        &self,
        tx: &Transaction<'_>,
        input: &CertificateVerificationInput<'_>,
    ) -> anyhow::Result<CertificateVerification> {
        let pem = input.chain_pem.first().cloned().unwrap_or_default();
        let Some(known) = self
            .certificates
            .iter()
            .find(|certificate| certificate.identity.pem == pem)
        else {
            return Ok(CertificateVerification {
                checks: vec![CertificateCheckResult {
                    id: CertificateCheckId::TrustedIssuer,
                    ok: false,
                    detail: Some("unknown certificate".into()),
                }],
                certificate: None,
                revocation_status: RevocationStatus::Unchecked,
                registration: RegistrationState::NotRegistered,
                other_holder: None,
            });
        };
        let identity = known.identity.clone();
        let registered = find_active_staff_certificates_by_fingerprint(
            tx,
            input.request.tenant_id,
            input.request.election_event_id,
            &identity.fingerprint_sha256,
        )
        .await?;
        let registration = match registered
            .iter()
            .find(|row| row.user_id == input.signer.user_id)
        {
            Some(row) => RegistrationState::Registered(row.clone()),
            None if self.first_use && registered.is_empty() => RegistrationState::FirstUse,
            None => RegistrationState::NotRegistered,
        };
        let signed = input.signatures.as_ref().is_some_and(|signatures| {
            signatures.payload_signature == signature_over(&input.request.canonical_payload)
        });
        let refused = self.refusals.get(&pem).copied();
        let checks = CertificateCheckId::iter()
            .map(|id| {
                let ok = match id {
                    CertificateCheckId::Registered => {
                        !matches!(registration, RegistrationState::NotRegistered)
                    }
                    CertificateCheckId::Signature => signed,
                    _ => refused != Some(id),
                };
                CertificateCheckResult {
                    id,
                    ok,
                    detail: (!ok).then(|| format!("{id} failed")),
                }
            })
            .collect();
        Ok(CertificateVerification {
            checks,
            certificate: Some(identity),
            revocation_status: RevocationStatus::Checked,
            registration,
            other_holder: None,
        })
    }
}

/// The approve step's services with `verifier` and `executors`.
pub fn services(
    verifier: FakeVerifier,
    executors: Vec<Arc<dyn SigningExecutor>>,
) -> SigningServices {
    SigningServices {
        verifier: Arc::new(verifier),
        executors: executors
            .into_iter()
            .fold(SigningExecutorRegistry::default(), |registry, executor| {
                registry.with(executor)
            }),
        documents: Arc::new(NoDocumentSigner),
        exports: Arc::new(RowOnlyExports),
    }
}

/// [`services`] with a document side that takes every signature.
pub fn services_taking_documents(
    verifier: FakeVerifier,
    executors: Vec<Arc<dyn SigningExecutor>>,
) -> SigningServices {
    SigningServices {
        documents: Arc::new(AcceptingDocuments),
        ..services(verifier, executors)
    }
}

/// Takes every document signature; the document is a few fixed bytes.
pub struct AcceptingDocuments;

#[async_trait]
impl DocumentSigner for AcceptingDocuments {
    fn supports(&self, _kind: DocumentKind) -> bool {
        true
    }

    async fn document_bytes(
        &self,
        _tx: &Transaction<'_>,
        _request: &SigningRequestRow,
    ) -> anyhow::Result<Option<Vec<u8>>> {
        Ok(Some(b"document".to_vec()))
    }

    async fn embed(
        &self,
        _tx: &Transaction<'_>,
        _request: &SigningRequestRow,
        _signer: &windmill::services::signing::log::Actor,
        _certificate: &CertificateIdentity,
        _pdf_cms: Option<&[u8]>,
        _revision: Option<i32>,
    ) -> SigningResult<()> {
        Ok(())
    }
}

/// Keeps an export as a document row only, with no object storage.
pub struct RowOnlyExports;

#[async_trait]
impl SigningExportStore for RowOnlyExports {
    async fn store(
        &self,
        tx: &Transaction<'_>,
        tenant_id: Uuid,
        election_event_id: Uuid,
        file_name: &str,
        content: &[u8],
    ) -> anyhow::Result<StoredExport> {
        let id = Uuid::new_v4();
        tx.execute(
            "INSERT INTO sequent_backend.document (id, tenant_id, election_event_id, name, media_type, size)
             VALUES ($1, $2, $3, $4, 'text/csv', $5)",
            &[&id, &tenant_id, &election_event_id, &file_name, &(content.len() as i64)],
        )
        .await?;
        Ok(StoredExport {
            document_id: id.to_string(),
            url: None,
        })
    }
}

/// A subject for `action`.
pub fn subject(n: u32) -> Value {
    json!({ "channel": format!("channel-{n}") })
}

/// What a [`FakeExecutor`] does when it runs.
#[derive(Debug, Clone, PartialEq)]
pub enum FakeBehaviour {
    Succeed(Value),
    Fail(String),
    /// Writes into the transaction before failing, to show the savepoint
    /// undoes it.
    WriteThenFail(String),
}

/// An executor that counts its runs and records what it saw.
#[derive(Debug)]
pub struct FakeExecutor {
    action: SigningAction,
    behaviour: FakeBehaviour,
    runs: std::sync::Mutex<Vec<(Uuid, usize)>>,
}

impl FakeExecutor {
    pub fn new(action: SigningAction, behaviour: FakeBehaviour) -> Self {
        FakeExecutor {
            action,
            behaviour,
            runs: std::sync::Mutex::new(vec![]),
        }
    }

    /// Each run: the request and how many approvals it had.
    pub fn runs(&self) -> Vec<(Uuid, usize)> {
        self.runs.lock().unwrap().clone()
    }
}

#[async_trait]
impl SigningExecutor for FakeExecutor {
    fn action(&self) -> SigningAction {
        self.action
    }

    async fn execute(
        &self,
        tx: &Transaction<'_>,
        request: &SigningRequestRow,
        approvals: &[SigningApprovalRow],
    ) -> anyhow::Result<ExecutionOutcome> {
        self.runs
            .lock()
            .unwrap()
            .push((request.id, approvals.len()));
        match &self.behaviour {
            FakeBehaviour::Succeed(result) => Ok(ExecutionOutcome::Executed {
                result: Some(result.clone()),
                post_commit: vec![],
            }),
            FakeBehaviour::Fail(message) => anyhow::bail!("{message}"),
            FakeBehaviour::WriteThenFail(message) => {
                tx.execute(
                    "UPDATE sequent_backend.signing_request SET open_failures = open_failures + 100
                     WHERE id = $1",
                    &[&request.id],
                )
                .await?;
                anyhow::bail!("{message}")
            }
        }
    }
}

/// The Keycloak tables the signers query reads, in a schema of this
/// transaction only, which it puts first on the search path.
pub async fn keycloak_tables(tx: &Transaction<'_>) {
    let schema = format!("kc_{}", Uuid::new_v4().simple());
    tx.batch_execute(&format!(
        "CREATE SCHEMA {schema};
         SET LOCAL search_path = {schema};
         CREATE TABLE realm (id varchar(36) PRIMARY KEY, name varchar(255));
         CREATE TABLE keycloak_role (id varchar(36) PRIMARY KEY, name varchar(255),
             realm_id varchar(255), client_role boolean NOT NULL DEFAULT false);
         CREATE TABLE composite_role (composite varchar(36), child_role varchar(36));
         CREATE TABLE keycloak_group (id varchar(36) PRIMARY KEY, name varchar(255),
             parent_group varchar(36) NOT NULL DEFAULT ' ', realm_id varchar(36));
         CREATE TABLE group_role_mapping (role_id varchar(36), group_id varchar(36));
         CREATE TABLE user_entity (id varchar(36) PRIMARY KEY, username varchar(255),
             first_name varchar(255), last_name varchar(255), enabled boolean NOT NULL,
             realm_id varchar(255), service_account_client_link varchar(255));
         CREATE TABLE user_role_mapping (role_id varchar(255), user_id varchar(36));
         CREATE TABLE user_group_membership (group_id varchar(36), user_id varchar(36));
         CREATE TABLE user_attribute (name varchar(255), value text, user_id varchar(36));
         CREATE TABLE group_attribute (name varchar(255), value varchar(255), group_id varchar(36));"
    ))
    .await
    .unwrap();
}
