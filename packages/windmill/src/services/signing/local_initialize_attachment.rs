// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Private phases for the existing localhost Admin Portal. The installation
//! constructs the scope and public pins from protected state. Only the
//! responsible officer's verified Portal session may review or attach.
//! See migration 2250 for the two Portal actions and exact installation hook.
#![cfg(all(
    feature = "felix3-local-initialize",
    not(feature = "operational-signing-demo-fixture")
))]

use super::certificates::{
    key_usage_check, operational_revocation_until, validity_check, verified_path,
    CertificateIdentity, StoredCrl,
};
use super::log::Actor;
use super::operational_initialize::{prepare_initialize_signing, InitializeSigningScope};
use super::staff_certificates::{register_staff_certificate, StaffCertificateRegistrationInput};
use super::SigningCaller;
use crate::services::audit_log::{self, AuditActor, AuditOrigin};
use crate::tasks::audit_log_outbox::kick_audit_log_outbox;
use anyhow::{anyhow, ensure, Context, Result};
use chrono::{DateTime, Duration, Utc};
use deadpool_postgres::{Pool, Transaction};
use openssl::nid::Nid;
use openssl::sha::sha256;
use openssl::x509::{CrlStatus, X509Crl, X509Req, X509};
use sequent_core::services::jwt::{decode_permission_labels, JwtClaims};
use sequent_core::signing::SignatureAlgorithm;
use sequent_core::types::permissions::Permissions;
use serde::Serialize;
use serde_json::json;
use std::future::Future;
use uuid::Uuid;
use x509_parser::{
    certification_request::X509CertificationRequest, parse_x509_certificate, prelude::FromDer,
};

/// The exact 2200 binding. Installation-owned, never deserialized from HTTP.
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct Binding {
    pub installation: Uuid,
    pub tenant: Uuid,
    pub event: Uuid,
    pub post: Uuid,
    pub publication: Uuid,
    pub request: Uuid,
    pub official: Uuid,
    pub payload_sha256: String,
}

pub struct LocalAttachment {
    bound: Binding,
    attempt: Uuid,
    enrollment: Uuid,
    csr_sha256: String,
    spki_sha256: String,
    portal_client: String,
    registry_login: String,
    authority: Pool,
    registry: Pool,
}

/// Protected per-key issuance handoff. Only the genuine public leaf is
/// per-key material. The event's approved issuer and current CRL live in DB.
pub struct IssuedLeaf {
    pub der: Vec<u8>,
    pub leaf_sha256: String,
    pub issuer: Uuid,
    pub root_sha256: String,
    pub crl_sha256: String,
}

/// In-process result of a live, separately authenticated nominee check.
/// No bearer, serialization, public constructor or permission boolean.
pub struct CurrentNominee {
    bound: Binding,
    attempt: Uuid,
    labels: Vec<String>,
    username: String,
    until: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct AttachmentEvidence {
    pub enrollment: Uuid,
    pub certificate: Uuid,
    pub leaf_sha256: String,
    pub spki_sha256: String,
    pub attached_at: DateTime<Utc>,
}

fn hash(data: &[u8]) -> String {
    hex::encode(sha256(data))
}

fn fingerprint(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

fn until_claim_expiry(claims: &JwtClaims) -> Result<DateTime<Utc>> {
    DateTime::from_timestamp(claims.exp.try_into()?, 0).context("invalid Portal expiry")
}

impl LocalAttachment {
    #[allow(clippy::too_many_arguments)]
    pub fn from_installation(
        portal_origin: &str,
        device_profile: &str,
        bound: Binding,
        attempt: Uuid,
        enrollment: Uuid,
        csr_sha256: String,
        spki_sha256: String,
        portal_client: String,
        registry_login: String,
        authority: Pool,
        registry: Pool,
    ) -> Result<Self> {
        ensure!(portal_origin == "http://localhost:3002" && device_profile == "softhsm-test");
        let ids = [
            bound.installation,
            bound.tenant,
            bound.event,
            bound.post,
            bound.publication,
            bound.request,
            bound.official,
            attempt,
            enrollment,
        ];
        ensure!(ids.iter().all(|id| !id.is_nil()));
        ensure!(
            fingerprint(&bound.payload_sha256)
                && fingerprint(&csr_sha256)
                && fingerprint(&spki_sha256)
        );
        ensure!(
            !portal_client.is_empty()
                && !registry_login.is_empty()
                && registry_login != "felix3_local_attachment_officer"
        );
        Ok(Self {
            bound,
            attempt,
            enrollment,
            csr_sha256,
            spki_sha256,
            portal_client,
            registry_login,
            authority,
            registry,
        })
    }

    // Called only with claims from the normal verified bearer guard. The
    // guard owns RS256, audience and realm key validation for this install.
    fn actor(&self, claims: &JwtClaims, permission: Permissions) -> Result<Uuid> {
        sequent_core::services::authorization::authorize(
            claims,
            true,
            Some(self.bound.tenant.to_string()),
            vec![],
        )
        .map_err(|_| anyhow!("Portal authorization unavailable"))?;
        let id = Uuid::parse_str(&claims.sub)?;
        ensure!(
            !id.is_nil()
                && claims.hasura_claims.user_id == id.to_string()
                && claims.hasura_claims.tenant_id == self.bound.tenant.to_string()
                && claims.iss
                    == format!("http://localhost:18090/realms/tenant-{}", self.bound.tenant)
                && claims.azp == self.portal_client
                && claims.typ == "Bearer"
                && claims
                    .hasura_claims
                    .allowed_roles
                    .contains(&permission.to_string())
                && until_claim_expiry(claims)? > Utc::now()
        );
        Ok(id)
    }

    /// Invoke in the nominee's own normal session. Pass the existing
    /// LocalInitializePost::verify operation for this same transaction.
    /// The host must return a fresh decision for each officer action.
    pub async fn current_nominee<F, Checked>(
        &self,
        tx: &Transaction<'_>,
        claims: &JwtClaims,
        verify_local_post: F,
    ) -> Result<CurrentNominee>
    where
        F: FnOnce(InitializeSigningScope) -> Checked,
        Checked: Future<Output = Result<()>>,
    {
        let observed = Utc::now();
        ensure!(self.actor(claims, Permissions::SIGN_INITIALIZE_VOTING)? == self.bound.official);
        let open = prepare_initialize_signing(
            tx,
            &SigningCaller::from_claims(claims),
            self.bound.tenant,
            self.bound.request,
        )
        .await
        .map_err(|_| anyhow!("Initialize request unavailable"))?;
        ensure!(
            open.tenant() == self.bound.tenant
                && open.event() == self.bound.event
                && open.election() == Some(self.bound.post)
                && open.request() == self.bound.request
                && open.actor() == self.bound.official
                && open.payload_sha256() == self.bound.payload_sha256
        );
        verify_local_post(open).await?;
        let until = until_claim_expiry(claims)?.min(observed + Duration::seconds(30));
        ensure!(Utc::now() < until);
        Ok(CurrentNominee {
            bound: self.bound.clone(),
            attempt: self.attempt,
            labels: decode_permission_labels(claims),
            username: claims
                .preferred_username
                .clone()
                .unwrap_or_else(|| claims.sub.clone()),
            until,
        })
    }

    fn officer(
        &self,
        claims: &JwtClaims,
        nominee: &CurrentNominee,
    ) -> Result<(Uuid, DateTime<Utc>)> {
        let id = self.actor(claims, Permissions::SIGNING_CERTIFICATES_REGISTER)?;
        let auth_time = claims.auth_time.context("officer step-up required")?;
        let authenticated = DateTime::from_timestamp(auth_time, 0)
            .context("invalid officer authentication time")?;
        let now = Utc::now();
        ensure!(
            nominee.bound == self.bound
                && nominee.attempt == self.attempt
                && nominee.until > now
                && id != self.bound.official
                && claims.acr == Permissions::GOLD.to_string()
                && authenticated <= now
                && now < authenticated + Duration::seconds(60)
        );
        Ok((id, authenticated))
    }

    fn check_csr(&self, pem: &str) -> Result<Vec<u8>> {
        ensure!(pem.len() <= 16384 && hash(pem.as_bytes()) == self.csr_sha256);
        let request = X509Req::from_pem(pem.as_bytes())?;
        ensure!(request.to_pem()? == pem.as_bytes());
        let public = request.public_key()?;
        let ec = public.ec_key()?;
        ensure!(ec.group().curve_name() == Some(Nid::X9_62_PRIME256V1));
        ec.check_key()?;
        ensure!(request.verify(&public)?);
        let der = request.to_der()?;
        let (rest, parsed) = X509CertificationRequest::from_der(&der)
            .map_err(|_| anyhow!("invalid public request"))?;
        let info = &parsed.certification_request_info;
        let spki = info.subject_pki.raw;
        let subject = request.subject_name().entries().collect::<Vec<_>>();
        ensure!(
            rest.is_empty()
                && parsed.signature_algorithm.algorithm.to_id_string() == "1.2.840.10045.4.3.2"
        );
        ensure!(
            request.version() == 0 && info.iter_attributes().next().is_none() && subject.len() == 2
        );
        ensure!(
            subject[0].object().nid() == Nid::COMMONNAME
                && subject[0].data().as_slice() == self.bound.official.to_string().as_bytes()
                && subject[1].object().nid() == Nid::ORGANIZATIONALUNITNAME
                && subject[1].data().as_slice() == self.bound.tenant.to_string().as_bytes()
        );
        ensure!(
            spki.len() == 91
                && info.subject_pki.subject_public_key.unused_bits == 0
                && info.subject_pki.subject_public_key.data.first() == Some(&4)
                && spki == public.public_key_to_der()?.as_slice()
                && hash(spki) == self.spki_sha256
        );
        Ok(spki.to_vec())
    }

    /// Responsible officer's read-only Portal review. A positive result is
    /// a public request and conveys no certificate readiness.
    pub async fn review_csr(&self, claims: &JwtClaims, nominee: CurrentNominee) -> Result<String> {
        let (officer_id, authenticated) = self.officer(claims, &nominee)?;
        let binding = serde_json::to_value(&self.bound)?;
        let officer_labels = decode_permission_labels(claims);
        let db = self.authority.get().await?;
        let row = db
            .query_opt(
                "SELECT enrollment,spki,csr FROM felix3_initialize_attachment.review_csr(
                jsonb_populate_record(NULL::felix3_initialize_setup.binding,$1),$2,$3,$4,$5,$6)",
                &[
                    &binding,
                    &self.attempt,
                    &officer_id,
                    &officer_labels,
                    &nominee.labels,
                    &authenticated,
                ],
            )
            .await?
            .context("public request unavailable")?;
        ensure!(row.try_get::<_, Uuid>("enrollment")? == self.enrollment);
        ensure!(row.try_get::<_, String>("spki")? == self.spki_sha256);
        let csr: String = row.try_get("csr")?;
        self.check_csr(&csr)?;
        self.officer(claims, &nominee)?;
        Ok(csr)
    }

    /// Explicit per-key officer action. Both pools connect over the installed
    /// Unix socket. No directory lookup, issuer import or device invocation.
    pub async fn register_and_attach(
        &self,
        claims: &JwtClaims,
        nominee: CurrentNominee,
        issued: &IssuedLeaf,
        reviewed_csr: &str,
        reviewed_leaf: &str,
    ) -> Result<AttachmentEvidence> {
        let (officer_id, authenticated) = self.officer(claims, &nominee)?;
        ensure!(
            reviewed_csr == self.csr_sha256
                && reviewed_leaf == issued.leaf_sha256
                && !issued.issuer.is_nil()
                && fingerprint(&issued.root_sha256)
                && fingerprint(&issued.crl_sha256)
                && (1..=32768).contains(&issued.der.len())
                && hash(&issued.der) == issued.leaf_sha256
        );
        let leaf = X509::from_der(&issued.der)?;
        ensure!(leaf.to_der()? == issued.der);
        let leaf_pem = String::from_utf8(leaf.to_pem()?)?;
        let pins = json!({
            "enrollment": self.enrollment,
            "csr_sha256": self.csr_sha256,
            "spki_sha256": self.spki_sha256,
            "leaf_sha256": issued.leaf_sha256,
            "issuer": issued.issuer,
            "root_sha256": issued.root_sha256,
            "crl_sha256": issued.crl_sha256,
        });
        let binding = serde_json::to_value(&self.bound)?;
        let officer_labels = decode_permission_labels(claims);
        // Reserve the writer before holding verification locks. A reply-loss
        // recovery returns without opening a transaction on this connection.
        let mut writer = self.registry.get().await?;
        let mut verifier = self.authority.get().await?;
        let check = verifier.transaction().await?;
        let material = check.query_opt(
            "SELECT csr,root_pem,crl_der,url,fetched_at,deadline,stamp,certificate,attached_at
             FROM felix3_initialize_attachment.inspect(
                 jsonb_populate_record(NULL::felix3_initialize_setup.binding,$1),$2,$3,$4,$5,$6,$7)",
            &[&binding, &self.attempt, &officer_id, &officer_labels, &nominee.labels, &authenticated, &pins],
        ).await?.context("issuance unavailable")?;
        let csr_spki = self.check_csr(&material.try_get::<_, String>("csr")?)?;
        let (rest, wire) =
            parse_x509_certificate(&issued.der).map_err(|_| anyhow!("invalid issued leaf"))?;
        ensure!(
            rest.is_empty()
                && !wire.tbs_certificate.is_ca()
                && wire.public_key().raw == csr_spki.as_slice()
        );
        let root_pem: String = material.try_get("root_pem")?;
        let root = X509::from_pem(root_pem.as_bytes())?;
        ensure!(hash(&root.to_der()?) == issued.root_sha256);
        let root_key = root.public_key()?;
        ensure!(
            root.issuer_name().to_der()? == root.subject_name().to_der()?
                && root.verify(&root_key)?
        );
        let identity = CertificateIdentity::of(&leaf, &[])?;
        ensure!(
            identity.fingerprint_sha256 == issued.leaf_sha256
                && identity.spki_sha256 == self.spki_sha256
                && identity.key_algorithm == Some(SignatureAlgorithm::EcdsaP256Sha256)
                && key_usage_check(&leaf, &identity)?.ok
        );
        let path =
            verified_path(&leaf, &[], &[root])?.map_err(|_| anyhow!("untrusted staff issuer"))?;
        ensure!(path.len() == 2 && hash(&path[1].to_der()?) == issued.root_sha256);
        let der: Vec<u8> = material.try_get("crl_der")?;
        ensure!(hash(&der) == issued.crl_sha256);
        let signed_list = X509Crl::from_der(&der)?;
        ensure!(
            signed_list.issuer_name().to_der()? == path[1].subject_name().to_der()?
                && signed_list.verify(&root_key)?
                && matches!(
                    signed_list.get_by_serial(leaf.serial_number()),
                    CrlStatus::NotRevoked
                )
        );
        let (tail, list) =
            x509_parser::parse_x509_crl(&der).map_err(|_| anyhow!("invalid staff CRL"))?;
        ensure!(
            tail.is_empty()
                && list.extensions().iter().all(|ext| !matches!(
                    ext.oid.to_id_string().as_str(),
                    "2.5.29.27" | "2.5.29.28"
                ))
        );
        let crl = StoredCrl {
            url: material.try_get("url")?,
            der: Some(der),
            fetched_at: material.try_get("fetched_at")?,
        };
        let now = Utc::now();
        ensure!(validity_check(&path, now)?.ok);
        let until = operational_revocation_until(&path, &[crl], now)
            .context("current signed CRL required")?
            .min(material.try_get::<_, DateTime<Utc>>("deadline")?)
            .min(identity.not_after)
            .min(CertificateIdentity::of(&path[1], &[])?.not_after)
            .min(nominee.until)
            .min(until_claim_expiry(claims)?);
        ensure!(Utc::now() < until);
        let evidence = |certificate, attached_at| AttachmentEvidence {
            enrollment: self.enrollment,
            certificate,
            leaf_sha256: issued.leaf_sha256.clone(),
            spki_sha256: self.spki_sha256.clone(),
            attached_at,
        };
        if let Some(certificate) = material.try_get::<_, Option<Uuid>>("certificate")? {
            let receipt = evidence(certificate, material.try_get("attached_at")?);
            check.commit().await?;
            return Ok(receipt);
        }
        let stamp: String = material.try_get("stamp")?;
        let tx = writer.transaction().await?;
        let socket = tx.query_one(
            "SELECT session_user::text AS login, pg_backend_pid() AS pid, pg_current_xact_id()::text AS xid,
                (inet_client_addr() IS NULL AND (system_user LIKE 'peer:%') IS TRUE AND current_user=session_user
                 AND current_setting('transaction_isolation')='read committed') AS local_peer",
            &[],
        ).await?;
        let login: String = socket.try_get("login")?;
        let pid: i32 = socket.try_get("pid")?;
        let xid: String = socket.try_get("xid")?;
        ensure!(socket.try_get::<_, bool>("local_peer")? && login == self.registry_login);
        let admitted = check
            .query_one(
                "SELECT felix3_initialize_attachment.admit(
                jsonb_populate_record(NULL::felix3_initialize_setup.binding,$1),$2,$3,$4,$5,$6,$7,
                $8,$9,$10::text::name,$11,$12::text::xid8)",
                &[
                    &binding,
                    &self.attempt,
                    &officer_id,
                    &officer_labels,
                    &nominee.labels,
                    &authenticated,
                    &pins,
                    &stamp,
                    &until,
                    &login,
                    &pid,
                    &xid,
                ],
            )
            .await?;
        ensure!(admitted.try_get::<_, bool>(0)?);
        check.commit().await?;
        self.officer(claims, &nominee)?;
        ensure!(until > Utc::now());
        let audit_actor = AuditActor {
            user_id: Some(officer_id.to_string()),
            username: claims.preferred_username.clone(),
            role: Some(claims.hasura_claims.default_role.clone()),
        };
        audit_log::act_for(&tx, &audit_actor, AuditOrigin::Harvest, self.bound.tenant).await?;
        let officer = Actor {
            user_id: officer_id.to_string(),
            username: claims
                .preferred_username
                .clone()
                .unwrap_or_else(|| officer_id.to_string()),
        };
        let input = StaffCertificateRegistrationInput {
            tenant_id: self.bound.tenant,
            election_event_id: self.bound.event,
            election_id: Some(self.bound.post),
            holder: Actor {
                user_id: self.bound.official.to_string(),
                username: nominee.username,
            },
            holder_name: None,
            linked_to: None,
            chain_pem: vec![leaf_pem, root_pem],
        };
        let registered = register_staff_certificate(&tx, &input, &officer, None, Utc::now())
            .await?
            .map_err(|_| anyhow!("staff registration refused"))?;
        let settled: Option<DateTime<Utc>> = tx
            .query_one(
                "SELECT felix3_initialize_attachment.finish($1,$2)",
                &[&self.attempt, &registered.id],
            )
            .await?
            .try_get(0)?;
        let receipt = evidence(registered.id, settled.context("attachment refused")?);
        tx.commit().await?;
        kick_audit_log_outbox();
        Ok(receipt)
    }
}
