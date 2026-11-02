// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Transmitting the results of a Post and country with the signatures of
//! its people.
//!
//! With the transmit-results rule `Required`, creating a transmission
//! package starts a signing request for it ([`guard_transmission_package`]).
//! Each signer signs the payload and the package's EML bytes in the browser.
//! When the last signature arrives, the package is built once more, now with
//! the signatures in the members list the central servers verify
//! (`ACMTrustee {id, signature, publickey, name}`, as the 2025 server-side
//! signatures were), by [`sign_package`]. Sending stays an explicit step,
//! which [`transmission_send_check`] allows only for the package the request
//! built. Without the rule nothing here applies and the 2025 flow stays.

use super::super::guard::{
    effective_rule, GuardOutcome, GuardRequest, RequestScope, SigningDocument,
};
use super::super::{SigningCaller, SigningError, SigningResult};
use super::{gate, refuse, EffectProgress};
use crate::postgres::signing::{
    get_signing_request, list_signing_approvals, SigningApprovalRow, SigningRequestRow,
};
use crate::services::consolidation::eml_types::ACMTrustee;
use crate::services::consolidation::send_transmission_package_service::{
    check_transmission_signatures_against, get_latest_miru_document, TransmissionSignaturesShort,
};
use crate::services::consolidation::xz_compress::xz_decompress;
use crate::services::signing::configuration::event_transmission_threshold;
use crate::types::miru_plugin::{
    MiruCcsServer, MiruSignature, MiruSigningRequest, MiruTransmissionPackageData,
};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use deadpool_postgres::Transaction;
use openssl::x509::X509;
use sequent_core::signing::{SigningAction, SigningRequestStatus, TransmitResultsSubject};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fmt;
use tracing::instrument;
use uuid::Uuid;

const ACTION: SigningAction = SigningAction::TransmitResults;

/// Lowercase hex SHA-256.
pub fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// One request per tally session and country.
pub fn transmission_subject_key(tally_session_id: &str, area_id: Uuid) -> String {
    format!("{tally_session_id}|{area_id}")
}

/// The destinations of a package, as its subject names them: their names,
/// sorted, without duplicates.
pub fn destination_names(servers: &[MiruCcsServer]) -> Vec<String> {
    let mut names: Vec<String> = servers.iter().map(|server| server.name.clone()).collect();
    names.sort();
    names.dedup();
    names
}

/// A transmission package as it is signed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageToSign<'a> {
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    /// The Post.
    pub election_id: Uuid,
    /// The country.
    pub area_id: Uuid,
    pub tally_session_id: &'a str,
    /// The id the EML is stored under, the document the signers sign.
    pub eml_document_id: Uuid,
    pub eml: &'a [u8],
    /// The compressed results every destination receives.
    pub package: &'a [u8],
    pub servers: &'a [MiruCcsServer],
}

impl PackageToSign<'_> {
    pub fn subject(&self) -> TransmitResultsSubject {
        TransmitResultsSubject::new(
            self.tally_session_id.to_owned(),
            sha256_hex(self.package),
            sha256_hex(self.eml),
            destination_names(self.servers),
        )
    }

    pub fn guard_request(&self) -> Result<GuardRequest> {
        Ok(GuardRequest {
            action: ACTION,
            scope: RequestScope {
                tenant_id: self.tenant_id,
                election_event_id: self.election_event_id,
                election_id: Some(self.election_id),
                area_id: Some(self.area_id),
                trustee_id: None,
                subject_key: Some(transmission_subject_key(
                    self.tally_session_id,
                    self.area_id,
                )),
            },
            subject: serde_json::to_value(self.subject())
                .context("Error writing the transmission subject")?,
            document: Some(SigningDocument {
                document_id: Some(self.eml_document_id),
                sha256: sha256_hex(self.eml),
            }),
            config_revision: None,
        })
    }
}

/// The signing request a new transmission package waits for: `None` when
/// the event's rule needs no signatures (the package is sent as in 2025).
/// The requester must reach the Post. Call it before the tally session's
/// row is locked: the request takes the event's signing lock first.
#[instrument(skip(hasura_transaction, package), err)]
pub async fn guard_transmission_package(
    hasura_transaction: &Transaction<'_>,
    requester: Option<&SigningCaller>,
    package: &PackageToSign<'_>,
) -> SigningResult<Option<MiruSigningRequest>> {
    let Some(requester) = requester else {
        // A package queued before signing existed names no requester: it
        // can only be made while the rule needs no signatures.
        let rule = effective_rule(
            hasura_transaction,
            package.tenant_id,
            package.election_event_id,
            ACTION,
        )
        .await?;
        return match rule.is_required() {
            true => Err(SigningError::Forbidden(
                "This package needs signatures, and its creator is unknown: create it again."
                    .into(),
            )),
            false => Ok(None),
        };
    };
    let outcome = gate(
        hasura_transaction,
        requester,
        ACTION,
        package.tenant_id,
        package.election_event_id,
        || async { Ok(package.guard_request()?) },
    )
    .await?;
    Ok(match outcome {
        GuardOutcome::Proceed => None,
        GuardOutcome::SigningRequired(summary) => Some(MiruSigningRequest {
            id: summary.id.to_string(),
            code: summary.code,
            required: summary.required,
        }),
    })
}

/// A person of the Post in its transmission configuration
/// (`miru:sbei-users`): who the central servers know them as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SbeiIdentity {
    pub miru_id: String,
    pub miru_name: String,
    /// The certificate a 2025 signature pinned to them, in any of the
    /// formats it was written in.
    pub certificate_fingerprint: Option<String>,
}

/// The Post's people in its transmission configuration, by username.
pub type PostSbeis = HashMap<String, SbeiIdentity>;

/// Where the Post's transmission configuration is read.
#[async_trait]
pub trait SbeiDirectory: Send + Sync {
    /// `None` when the event has no transmission configuration of its
    /// people: anyone who may sign signs under their username.
    async fn post_sbeis(
        &self,
        hasura_transaction: &Transaction<'_>,
        request: &SigningRequestRow,
    ) -> Result<Option<PostSbeis>>;
}

/// A SHA-256 certificate fingerprint as lowercase hex, whether written as
/// OpenSSL prints it (`sha256 Fingerprint=AB:CD:…`) or as hex.
pub fn normalise_fingerprint(fingerprint: &str) -> String {
    let value = fingerprint
        .rsplit_once('=')
        .map_or(fingerprint, |(_, value)| value);
    value
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .collect::<String>()
        .to_ascii_lowercase()
}

/// Why a signer can't sign the Post's transmission, if they can't: they
/// are not in its transmission configuration, or a 2025 signature pinned
/// another certificate to them.
pub fn sbei_refusal(
    sbeis: Option<&PostSbeis>,
    username: &str,
    fingerprint_sha256: &str,
) -> Option<&'static str> {
    let sbeis = sbeis?;
    let Some(identity) = sbeis.get(username) else {
        return Some(
            "Only the Post's members in its transmission configuration sign its transmission.",
        );
    };
    match &identity.certificate_fingerprint {
        Some(pinned)
            if normalise_fingerprint(pinned) != normalise_fingerprint(fingerprint_sha256) =>
        {
            Some("The transmission configuration pins another certificate to you.")
        }
        _ => None,
    }
}

/// The public key of a PEM certificate, as a PEM `PUBLIC KEY` (the
/// members list's `publickey`).
pub fn public_key_pem(certificate_pem: &str) -> Result<String> {
    let certificate = X509::from_pem(certificate_pem.as_bytes())
        .context("Error reading the approval's certificate")?;
    let key = certificate
        .public_key()
        .context("Error reading the certificate's key")?;
    String::from_utf8(
        key.public_key_to_pem()
            .context("Error writing the certificate's key")?,
    )
    .context("Error writing the certificate's key")
}

/// The package's members and signatures from the approvals, in the order
/// they were signed; the signature is the approval's EML signature. With a
/// transmission configuration, a member is who the central servers know
/// (its id and name), and an approval outside it is refused; without one,
/// a member is their username and display name.
pub fn package_members(
    approvals: &[SigningApprovalRow],
    sbeis: Option<&PostSbeis>,
) -> Result<(Vec<ACMTrustee>, Vec<MiruSignature>)> {
    let mut approvals: Vec<&SigningApprovalRow> = approvals.iter().collect();
    approvals.sort_by(|a, b| a.signed_at.cmp(&b.signed_at).then(a.id.cmp(&b.id)));
    let mut members = vec![];
    let mut signatures = vec![];
    for approval in approvals {
        let document_signature = approval.document_signature.as_ref().ok_or_else(|| {
            anyhow!(
                "The signature of {} has no document signature",
                approval.username
            )
        })?;
        let signature = BASE64.encode(document_signature);
        let publickey = public_key_pem(&approval.certificate_pem)?;
        let (id, name) = match sbeis {
            Some(sbeis) => {
                let identity = sbeis.get(&approval.username).ok_or_else(|| {
                    refuse(
                        "unmapped-signer",
                        format!(
                            "{} is not in the Post's transmission configuration",
                            approval.username
                        ),
                    )
                })?;
                (identity.miru_id.clone(), identity.miru_name.clone())
            }
            None => (
                approval.username.clone(),
                approval
                    .display_name
                    .clone()
                    .unwrap_or_else(|| approval.username.clone()),
            ),
        };
        members.push(ACMTrustee {
            id: id.clone(),
            signature: Some(signature.clone()),
            publickey: Some(publickey.clone()),
            name,
        });
        signatures.push(MiruSignature {
            sbei_miru_id: id,
            pub_key: publickey,
            signature,
            certificate_fingerprint: approval.fingerprint_sha256.clone(),
        });
    }
    Ok((members, signatures))
}

/// A transmission package as it stands, read under the tally session's
/// row lock.
#[derive(Debug, Clone)]
pub struct LoadedPackage {
    pub tally_session_id: String,
    pub package: MiruTransmissionPackageData,
    /// The latest document's EML.
    pub eml: Vec<u8>,
    /// The latest document's compressed results.
    pub compressed: Vec<u8>,
    /// The Post's people in its transmission configuration.
    pub sbeis: Option<PostSbeis>,
}

/// Where transmission packages are kept: the tally session's annotations,
/// and their documents.
#[async_trait]
pub trait TransmissionPackages: Send + Sync {
    /// The package of the request's tally session, Post and country. Locks
    /// the tally session's row until the transaction ends.
    async fn load(
        &self,
        hasura_transaction: &Transaction<'_>,
        request: &SigningRequestRow,
        tally_session_id: &str,
    ) -> Result<LoadedPackage>;

    /// Builds the package once more, for its stored destinations, with
    /// `members`, and keeps it as its latest document with `signatures`.
    /// Sends nothing. Answers the new `all_servers` document id.
    async fn store_signed(
        &self,
        hasura_transaction: &Transaction<'_>,
        request: &SigningRequestRow,
        loaded: &LoadedPackage,
        members: Vec<ACMTrustee>,
        signatures: Vec<MiruSignature>,
    ) -> Result<String>;
}

/// The effect of an executed transmit-results request: the package it
/// signed, built once with its signatures. Before anything leaves the
/// database it checks that the package is the one signed (its request, its
/// EML and compressed results, its destinations) and that every signer is
/// in the Post's transmission configuration; otherwise it refuses.
#[instrument(skip_all, fields(request_id = %request.id), err)]
pub async fn sign_package(
    hasura_transaction: &Transaction<'_>,
    packages: &dyn TransmissionPackages,
    request: &SigningRequestRow,
    approvals: &[SigningApprovalRow],
    progress: &EffectProgress,
) -> Result<Value> {
    let subject: TransmitResultsSubject = serde_json::from_value(request.subject.clone())
        .context("Error reading the transmission subject")?;
    let loaded = packages
        .load(hasura_transaction, request, &subject.tally_session_id)
        .await?;
    let changed = |what: &str| {
        refuse(
            "package-changed",
            format!(
                "The transmission package changed since request {} was signed: {what}",
                request.code
            ),
        )
    };
    let request_id = request.id.to_string();
    if loaded
        .package
        .signing_request
        .as_ref()
        .map(|signing| signing.id.as_str())
        != Some(request_id.as_str())
    {
        return Err(changed("it waits for another request"));
    }
    let eml_sha256 = sha256_hex(&loaded.eml);
    if eml_sha256 != subject.eml_sha256 || Some(&eml_sha256) != request.document_sha256.as_ref() {
        return Err(changed("its EML"));
    }
    if sha256_hex(&loaded.compressed) != subject.package_sha256
        || xz_decompress(&loaded.compressed).ok().as_deref() != Some(loaded.eml.as_slice())
    {
        return Err(changed("its compressed results"));
    }
    if destination_names(&loaded.package.servers) != subject.destinations() {
        return Err(changed("its destinations"));
    }
    if approvals.len() < usize::try_from(request.required).unwrap_or(usize::MAX) {
        return Err(refuse(
            "signatures-missing",
            format!("Signing request {} lacks signatures", request.code),
        ));
    }
    let (members, signatures) = package_members(approvals, loaded.sbeis.as_ref())?;

    progress.reach_outside();
    let all_servers_document_id = packages
        .store_signed(hasura_transaction, request, &loaded, members, signatures)
        .await?;
    Ok(json!({
        "all_servers_document_id": all_servers_document_id,
        "package_sha256": subject.package_sha256,
        "eml_sha256": subject.eml_sha256,
        "signatures": approvals.len(),
    }))
}

/// Why a transmission package can't be sent, or why the 2025 signature
/// upload is refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransmissionRefusal {
    /// The rule needs signatures and the package's request didn't run;
    /// with the request's code and status, when it has one.
    NotSigned {
        code: Option<String>,
        status: Option<SigningRequestStatus>,
    },
    /// The package's latest document is not the one its request built.
    NotTheSignedPackage { code: String },
    /// The package's signatures come from its signing request.
    SignedByRequest,
    /// Without signing, the package has fewer 2025 signatures than it
    /// needs.
    SignaturesShort(TransmissionSignaturesShort),
}

impl fmt::Display for TransmissionRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransmissionRefusal::NotSigned {
                code: Some(code),
                status: Some(status),
            } => write!(
                f,
                "The transmission package can't be sent before it is signed: signing request {code} is {status}"
            ),
            TransmissionRefusal::NotSigned { .. } => write!(
                f,
                "The transmission package can't be sent before it is signed: create it again to start its signing request"
            ),
            TransmissionRefusal::NotTheSignedPackage { code } => write!(
                f,
                "The transmission package is not the one signing request {code} signed: create it again"
            ),
            TransmissionRefusal::SignedByRequest => write!(
                f,
                "This transmission package is signed through its signing request, not by uploading a certificate"
            ),
            TransmissionRefusal::SignaturesShort(short) => short.fmt(f),
        }
    }
}

impl std::error::Error for TransmissionRefusal {}

/// Whether a package can be sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransmissionSendCheck {
    /// Its signing request ran and built it: it carries `count`
    /// signatures, the approvals'.
    Signed { count: i64 },
    /// No signatures are needed: the 2025 checks apply.
    Unsigned,
}

/// The signatures of a package's latest document, as a comparable list.
fn signature_set(signatures: &[MiruSignature]) -> Vec<(String, String)> {
    let mut set: Vec<(String, String)> = signatures
        .iter()
        .map(|signature| (signature.signature.clone(), signature.pub_key.clone()))
        .collect();
    set.sort();
    set
}

/// Whether `package` can be sent: once its request ran, only the document
/// that request built, with exactly its approvals' signatures; refused
/// while the event's rule needs signatures it doesn't have, or while its
/// request waits; as in 2025 otherwise (no rule, and no request or one that
/// ended without running).
#[instrument(skip(hasura_transaction, package), err)]
pub async fn transmission_send_check(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    package: &MiruTransmissionPackageData,
) -> Result<std::result::Result<TransmissionSendCheck, TransmissionRefusal>> {
    let request = match &package.signing_request {
        Some(signing) => {
            let id = Uuid::parse_str(&signing.id).context("Error reading the request id")?;
            get_signing_request(hasura_transaction, tenant_id, election_event_id, id).await?
        }
        None => None,
    };
    if let Some(request) = request
        .as_ref()
        .filter(|request| request.status == SigningRequestStatus::Executed)
    {
        let not_it = Err(TransmissionRefusal::NotTheSignedPackage {
            code: request.code.clone(),
        });
        let built = request
            .execution_result
            .as_ref()
            .and_then(|result| result.get("all_servers_document_id"))
            .and_then(Value::as_str);
        let Some(latest) = get_latest_miru_document(&package.documents) else {
            return Ok(not_it);
        };
        if built != Some(latest.document_ids.all_servers.as_str()) {
            return Ok(not_it);
        }
        let approvals =
            list_signing_approvals(hasura_transaction, tenant_id, election_event_id, request.id)
                .await?;
        let mut expected = vec![];
        for approval in &approvals {
            let Some(signature) = &approval.document_signature else {
                return Ok(not_it);
            };
            expected.push((
                BASE64.encode(signature),
                public_key_pem(&approval.certificate_pem)?,
            ));
        }
        expected.sort();
        if signature_set(&latest.signatures) != expected {
            return Ok(not_it);
        }
        return Ok(Ok(TransmissionSendCheck::Signed {
            count: approvals.len() as i64,
        }));
    }
    let rule = effective_rule(hasura_transaction, tenant_id, election_event_id, ACTION).await?;
    let waiting = request.as_ref().is_some_and(|request| {
        matches!(
            request.status,
            SigningRequestStatus::Waiting | SigningRequestStatus::Completed
        )
    });
    if rule.is_required() || waiting {
        return Ok(Err(TransmissionRefusal::NotSigned {
            code: request.as_ref().map(|request| request.code.clone()),
            status: request.as_ref().map(|request| request.status),
        }));
    }
    // As in 2025: a minimum of uploaded signatures (-1: none). A package
    // made without a signing request takes the event's effective threshold;
    // one whose request ended without running keeps the Post's, stored with
    // it.
    let threshold = if package.signing_request.is_none() {
        event_transmission_threshold(
            hasura_transaction,
            tenant_id,
            election_event_id,
            package.threshold,
        )
        .await?
    } else {
        package.threshold
    };
    if let Err(short) = check_transmission_signatures_against(package, threshold) {
        return Ok(Err(TransmissionRefusal::SignaturesShort(short)));
    }
    Ok(Ok(TransmissionSendCheck::Unsigned))
}

/// Why a 2025 signature upload is refused: the event's rule needs
/// signatures, or the package's signing request is waiting, about to run or
/// ran. A package whose request ended without running (cancelled, expired,
/// failed) takes uploads again while the rule is off.
#[instrument(skip(hasura_transaction, package), err)]
pub async fn upload_signature_refusal(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    package: &MiruTransmissionPackageData,
) -> Result<Option<TransmissionRefusal>> {
    if effective_rule(hasura_transaction, tenant_id, election_event_id, ACTION)
        .await?
        .is_required()
    {
        return Ok(Some(TransmissionRefusal::SignedByRequest));
    }
    let Some(signing) = &package.signing_request else {
        return Ok(None);
    };
    let id = Uuid::parse_str(&signing.id).context("Error reading the request id")?;
    let live = get_signing_request(hasura_transaction, tenant_id, election_event_id, id)
        .await?
        .is_some_and(|request| {
            matches!(
                request.status,
                SigningRequestStatus::Waiting
                    | SigningRequestStatus::Completed
                    | SigningRequestStatus::Executed
            )
        });
    Ok(live.then_some(TransmissionRefusal::SignedByRequest))
}
