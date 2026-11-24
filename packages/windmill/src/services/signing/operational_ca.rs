// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Independent operational enrollment assessment. Trust comes only from
//! already configured event staff issuers and signed current CRLs. No user
//! supplied chain and no fetched URLs or enrollment/trust/credential changes.

use super::certificates::{
    key_usage_check, operational_revocation_until, parse_chain, validity_check, verified_path,
    CertificateIdentity, StoredCrl,
};
use anyhow::Result;
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use sequent_core::signing::SignatureAlgorithm;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceCaProbe {
    pub tenant: Uuid,
    pub event: Uuid,
    pub request: Uuid,
    pub official: Uuid,
    pub challenge: Uuid,
    pub fence: Uuid,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoldDeviceCaProbe {
    pub scope: DeviceCaProbe,
    pub binding_sha256: String,
}

/// The CA uses the same installed-device domain as approval and release. An
/// ordinary build cannot even select the disposable software provider.
pub use super::operational_device::InstalledDevice as RequiredCaDevice;

#[derive(Deserialize)]
struct Binding {
    tenant_id: Uuid,
    event_id: Uuid,
    request_id: Uuid,
    actor: String,
    challenge_id: Uuid,
    action: String,
    purpose: String,
    key_id: String,
    key_version: i32,
    cert_fingerprint: String,
    cert_spki: String,
}

#[derive(Deserialize)]
struct CurrentCrl {
    url: String,
    der_hex: String,
    fetched_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct PositiveCaAssessment {
    pub scope: DeviceCaProbe,
    pub key_id: String,
    pub key_version: i32,
    pub certificate_id: Uuid,
    pub profile: &'static str,
    pub leaf_der_b64: String,
    pub cert_fingerprint: String,
    pub cert_spki: String,
    pub assessed_at: i64,
    pub usable_until: i64,
    pub ca_version: String,
}

fn canonical_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Invoke only with a device peer verified by a dedicated private mTLS
/// endpoint, scoped through immutable tenant/profile provisioning. The
/// independent authority DB login is read-only and cannot sign/settle,
/// create or reserve attempts, or change enrollment and anchor tables.
pub async fn assess_reserved_ca(
    tx: &Transaction<'_>,
    pinned_tenant: Uuid,
    profile: RequiredCaDevice,
    probe: &DeviceCaProbe,
) -> Result<Option<PositiveCaAssessment>> {
    assess_with_ca_scope(tx, pinned_tenant, profile, probe, None).await
}

pub async fn assess_gold_reserved_ca(
    tx: &Transaction<'_>,
    pinned_tenant: Uuid,
    probe: &GoldDeviceCaProbe,
) -> Result<Option<PositiveCaAssessment>> {
    if !canonical_hash(&probe.binding_sha256) {
        return Ok(None);
    }
    assess_with_ca_scope(tx, pinned_tenant, RequiredCaDevice::YubiHsm, &probe.scope,
        Some(&probe.binding_sha256)).await
}

async fn assess_with_ca_scope(
    tx: &Transaction<'_>, pinned_tenant: Uuid, profile: RequiredCaDevice,
    probe: &DeviceCaProbe, gold_binding: Option<&str>,
) -> Result<Option<PositiveCaAssessment>> {
    if probe.tenant != pinned_tenant
        || [
            probe.tenant,
            probe.event,
            probe.request,
            probe.official,
            probe.challenge,
            probe.fence,
        ]
        .iter()
        .any(Uuid::is_nil)
    {
        return Ok(None);
    }

    let actor = probe.official.to_string();
    let profile = profile.as_str();
    let (query, argument) = if let Some(hash) = gold_binding {
        ("SELECT binding,certificate_id,chain_pem,grant_until,ca_version,anchors_pem,revocation_lists,false
          FROM sequent_backend.operational_gold_get_ca($1,$2,$3,$4,$5,$6,$7)", hash)
    } else {
        ("SELECT binding,certificate_id,chain_pem,grant_until,ca_version,anchors_pem,revocation_lists,locally_revoked
          FROM sequent_backend.operational_initialize_get_ca_enrollment($1,$2,$3,$4,$5,$6,$7)", profile)
    };
    let Some(row) = tx
        .query_opt(query,
            &[
                &probe.tenant,
                &probe.event,
                &probe.request,
                &actor,
                &probe.challenge,
                &probe.fence,
                &argument,
            ],
        )
        .await?
    else {
        return Ok(None);
    };
    let binding: Binding = serde_json::from_value(row.try_get(0)?)?;
    let certificate_id: Uuid = row.try_get(1)?;
    let chain: Vec<String> = row.try_get(2)?;
    let grant_until: DateTime<Utc> = row.try_get(3)?;
    let version: String = row.try_get(4)?;
    let anchor_pem: Vec<String> = row.try_get(5)?;
    let lists: Vec<CurrentCrl> = serde_json::from_value(row.try_get(6)?)?;
    let locally_revoked: bool = row.try_get(7)?;
    if (
        binding.tenant_id,
        binding.event_id,
        binding.request_id,
        binding.challenge_id,
    ) != (probe.tenant, probe.event, probe.request, probe.challenge)
        || (gold_binding.is_none()
            && (binding.action != "initialize-voting" || binding.purpose != "approval-payload"))
        || binding.actor != actor
        || binding.key_id != format!("official/{}/{}", probe.tenant, actor)
        || binding.key_version <= 0
        || !canonical_hash(&binding.cert_fingerprint)
        || !canonical_hash(&binding.cert_spki)
        || !canonical_hash(&version)
        || certificate_id.is_nil()
        || locally_revoked
        || grant_until <= Utc::now()
    {
        return Ok(None);
    }

    // The leaf and every helper chain cert are server-owned in the exact
    // active record. Helpers cannot act as trust anchors.
    let chain = parse_chain(&chain)?;
    let Some(leaf) = chain.first() else {
        return Ok(None);
    };
    let identity = CertificateIdentity::of(leaf, &chain)?;
    if identity.fingerprint_sha256 != binding.cert_fingerprint
        || identity.spki_sha256 != binding.cert_spki
        || identity.key_algorithm != Some(SignatureAlgorithm::EcdsaP256Sha256)
        || !key_usage_check(leaf, &identity)?.ok
    {
        return Ok(None);
    }
    let anchors: Vec<_> = anchor_pem
        .iter()
        .map(|pem| openssl::x509::X509::from_pem(pem.as_bytes()))
        .collect::<Result<_, _>>()?;
    let Ok(path) = verified_path(leaf, &chain[1..], &anchors)? else {
        return Ok(None);
    };
    let crls: Vec<_> = lists
        .into_iter()
        .map(|entry| {
            Ok(StoredCrl {
                url: entry.url,
                der: Some(hex::decode(entry.der_hex)?),
                fetched_at: entry.fetched_at,
            })
        })
        .collect::<Result<_, hex::FromHexError>>()?;
    let now = Utc::now();
    if !validity_check(&path, now)?.ok {
        return Ok(None);
    }
    let Some(mut until) = operational_revocation_until(&path, &crls, now) else {
        return Ok(None);
    };
    // Bound the lease by the earliest certificate expiry and the exact
    // existing device reservation, not issuer/browser-suggested lifetimes.
    until = until.min(grant_until);
    for certificate in &path {
        until = until.min(CertificateIdentity::of(certificate, &[])?.not_after);
    }
    let current: Option<String> = if gold_binding.is_some() {
        tx.query_opt(
            "SELECT ca_version FROM sequent_backend.operational_gold_get_ca($1,$2,$3,$4,$5,$6,$7)",
            &[&probe.tenant, &probe.event, &probe.request, &actor, &probe.challenge, &probe.fence, &argument],
        ).await?.map(|row| row.try_get(0)).transpose()?
    } else {
        Some(tx.query_one(
            "SELECT sequent_backend.operational_initialize_ca_version($1,$2,$3,$4)",
            &[&probe.tenant, &probe.event, &binding.cert_fingerprint, &binding.cert_spki],
        ).await?.try_get(0)?)
    };
    let Some(current) = current else { return Ok(None); };
    if current != version || until.timestamp() <= Utc::now().timestamp() {
        return Ok(None);
    }
    Ok(Some(PositiveCaAssessment {
        scope: probe.clone(),
        key_id: binding.key_id,
        key_version: binding.key_version,
        certificate_id,
        profile,
        leaf_der_b64: STANDARD.encode(leaf.to_der()?),
        cert_fingerprint: binding.cert_fingerprint,
        cert_spki: binding.cert_spki,
        assessed_at: now.timestamp(),
        usable_until: until.timestamp(),
        ca_version: current,
    }))
}
