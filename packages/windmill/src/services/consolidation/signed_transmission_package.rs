// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Transmission packages as their signing reads and rebuilds them: the
//! tally session's annotations, read and written under the session's row
//! lock, and the package's stored documents.

use super::create_transmission_package_service::{
    generate_all_servers_document, update_transmission_package_annotations,
};
use super::eml_generator::ValidateAnnotations;
use super::eml_types::ACMTrustee;
use super::logs::sign_transmission_package_log;
use super::send_transmission_package_service::get_latest_miru_document;
use crate::postgres::area::get_area_by_id;
use crate::postgres::election::get_election_by_id;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::signing::SigningRequestRow;
use crate::postgres::tally_session::{get_tally_session_by_id, lock_tally_session_for_update};
use crate::services::reports::generation::ReportRequester;
use crate::services::signing::actions::transmission::{
    LoadedPackage, PostSbeis, SbeiDirectory, SbeiIdentity, TransmissionPackages,
};
use crate::services::signing::pdf::{RevisionStore, S3RevisionStore};
use crate::services::time_zones::{event_time_zone, offset_at};
use crate::types::miru_plugin::{
    MiruDocument, MiruDocumentIds, MiruSignature, MiruTallySessionData,
};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use chrono_tz::Tz;
use deadpool_postgres::Transaction;
use sequent_core::ballot::Annotations;
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::services::translations::Name;
use sequent_core::types::date_time::TimeZone;
use sequent_core::types::hasura::core::TallySession;
use std::collections::HashMap;
use tracing::instrument;
use uuid::Uuid;

/// A tally session locked for an update of its transmission packages, with
/// its annotations and packages as they stand under the lock.
pub struct LockedTransmissionData {
    pub tally_session: TallySession,
    pub annotations: Annotations,
    pub packages: MiruTallySessionData,
}

/// Locks the tally session's row (`FOR UPDATE`) and reads its packages
/// under the lock: every writer of the packages calls it first, so none
/// writes back a stale list.
#[instrument(skip(hasura_transaction), err)]
pub async fn lock_transmission_data(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
) -> Result<LockedTransmissionData> {
    lock_tally_session_for_update(
        hasura_transaction,
        tenant_id,
        election_event_id,
        tally_session_id,
    )
    .await?;
    let tally_session = get_tally_session_by_id(
        hasura_transaction,
        tenant_id,
        election_event_id,
        tally_session_id,
    )
    .await
    .context("Error fetching the tally session")?;
    let annotations: Annotations = tally_session
        .annotations
        .clone()
        .map(deserialize_value)
        .transpose()?
        .unwrap_or_default();
    let packages = tally_session.get_annotations().unwrap_or_default();
    Ok(LockedTransmissionData {
        tally_session,
        annotations,
        packages,
    })
}

/// The zone transmission packages are dated in: the event's primary.
#[derive(Debug, Clone)]
pub struct TransmissionZone {
    /// For the package's log lines and document times.
    pub zone: Tz,
    /// Its offset at the package's time, as the EML and ACM print it.
    pub offset: TimeZone,
}

/// The event's [`TransmissionZone`] at `now`.
pub async fn transmission_zone(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    now: chrono::DateTime<Utc>,
) -> Result<TransmissionZone> {
    let zone = event_time_zone(
        hasura_transaction,
        Uuid::parse_str(tenant_id).context("Error parsing the tenant id")?,
        Uuid::parse_str(election_event_id).context("Error parsing the election event id")?,
    )
    .await?;
    Ok(TransmissionZone {
        zone,
        offset: offset_at(zone, now),
    })
}

/// The bytes of a stored document of the request's event.
async fn document_bytes(
    hasura_transaction: &Transaction<'_>,
    request: &SigningRequestRow,
    document_id: &str,
) -> Result<Vec<u8>> {
    let document_id = Uuid::parse_str(document_id)
        .with_context(|| format!("Error parsing document id {document_id}"))?;
    S3RevisionStore
        .load(
            hasura_transaction,
            request.tenant_id,
            request.election_event_id,
            document_id,
        )
        .await
}

/// Where a transmission request's package is.
struct Place {
    tenant_id: String,
    election_event_id: String,
    election_id: String,
    area_id: String,
}

fn place(request: &SigningRequestRow) -> Result<Place> {
    Ok(Place {
        tenant_id: request.tenant_id.to_string(),
        election_event_id: request.election_event_id.to_string(),
        election_id: request
            .election_id
            .ok_or_else(|| anyhow!("A transmission request names its Post"))?
            .to_string(),
        area_id: request
            .area_id
            .ok_or_else(|| anyhow!("A transmission request names its country"))?
            .to_string(),
    })
}

/// The Post's people in the event's `miru:sbei-users` annotation, for the
/// country's members (`miru:area-trustee-users`) of the Post's election.
#[derive(Debug, Clone, Copy, Default)]
pub struct AnnotatedSbeis;

#[async_trait]
impl SbeiDirectory for AnnotatedSbeis {
    async fn post_sbeis(
        &self,
        hasura_transaction: &Transaction<'_>,
        request: &SigningRequestRow,
    ) -> Result<Option<PostSbeis>> {
        let at = place(request)?;
        let election_event =
            get_election_event_by_id(hasura_transaction, &at.tenant_id, &at.election_event_id)
                .await?;
        let sbei_users = election_event.get_annotations_or_empty_values()?.sbei_users;
        if sbei_users.is_empty() {
            return Ok(None);
        }
        let election = get_election_by_id(
            hasura_transaction,
            &at.tenant_id,
            &at.election_event_id,
            &at.election_id,
        )
        .await?
        .ok_or_else(|| anyhow!("Election not found"))?;
        let area = get_area_by_id(hasura_transaction, &at.tenant_id, &at.area_id)
            .await?
            .ok_or_else(|| anyhow!("Can't find area {}", at.area_id))?;
        let election_id = election.get_annotations_or_empty_values()?.election_id;
        let area_sbeis = area.get_annotations_or_empty_values()?.sbei_ids;
        Ok(Some(
            sbei_users
                .into_iter()
                .filter(|sbei| {
                    area_sbeis.contains(&sbei.miru_id) && sbei.miru_election_id == election_id
                })
                .map(|sbei| {
                    (
                        sbei.username,
                        SbeiIdentity {
                            miru_id: sbei.miru_id,
                            miru_name: sbei.miru_name,
                            certificate_fingerprint: sbei.certificate_fingerprint,
                        },
                    )
                })
                .collect::<HashMap<_, _>>(),
        ))
    }
}

/// The packages of the tally sessions' annotations, with their documents
/// in object storage.
#[derive(Debug, Clone, Copy, Default)]
pub struct StoredPackages;

#[async_trait]
impl TransmissionPackages for StoredPackages {
    #[instrument(skip_all, err)]
    async fn load(
        &self,
        hasura_transaction: &Transaction<'_>,
        request: &SigningRequestRow,
        tally_session_id: &str,
    ) -> Result<LoadedPackage> {
        let at = place(request)?;
        let locked = lock_transmission_data(
            hasura_transaction,
            &at.tenant_id,
            &at.election_event_id,
            tally_session_id,
        )
        .await?;
        let package = locked
            .packages
            .into_iter()
            .find(|data| data.area_id == at.area_id && data.election_id == at.election_id)
            .ok_or_else(|| anyhow!("The tally session has no package for this Post and country"))?;
        let document = get_latest_miru_document(&package.documents)
            .ok_or_else(|| anyhow!("The transmission package has no document"))?;
        let eml = document_bytes(hasura_transaction, request, &document.document_ids.eml).await?;
        let compressed =
            document_bytes(hasura_transaction, request, &document.document_ids.xz).await?;
        let sbeis = AnnotatedSbeis
            .post_sbeis(hasura_transaction, request)
            .await?;
        Ok(LoadedPackage {
            tally_session_id: tally_session_id.to_owned(),
            package,
            eml,
            compressed,
            sbeis,
        })
    }

    #[instrument(skip_all, err)]
    async fn store_signed(
        &self,
        hasura_transaction: &Transaction<'_>,
        request: &SigningRequestRow,
        loaded: &LoadedPackage,
        members: Vec<ACMTrustee>,
        signatures: Vec<MiruSignature>,
    ) -> Result<String> {
        let at = place(request)?;
        let now_utc = Utc::now();
        let zone = transmission_zone(
            hasura_transaction,
            &at.tenant_id,
            &at.election_event_id,
            now_utc,
        )
        .await?;
        let now_local = now_utc.with_timezone(&zone.zone);
        let election_event =
            get_election_event_by_id(hasura_transaction, &at.tenant_id, &at.election_event_id)
                .await?;
        let election = get_election_by_id(
            hasura_transaction,
            &at.tenant_id,
            &at.election_event_id,
            &at.election_id,
        )
        .await?
        .ok_or_else(|| anyhow!("Election not found"))?;
        let area = get_area_by_id(hasura_transaction, &at.tenant_id, &at.area_id)
            .await?
            .ok_or_else(|| anyhow!("Can't find area {}", at.area_id))?;
        let area_annotations = area.get_annotations()?;
        // The row is locked since `load`: these are the packages as they stand.
        let tally_session = get_tally_session_by_id(
            hasura_transaction,
            &at.tenant_id,
            &at.election_event_id,
            &loaded.tally_session_id,
        )
        .await?;
        let tally_annotations: Annotations = tally_session
            .annotations
            .clone()
            .map(deserialize_value)
            .transpose()?
            .unwrap_or_default();

        let eml = String::from_utf8(loaded.eml.clone()).context("The EML is not UTF-8")?;
        // The package names its EML by the uppercase hex SHA-256.
        let eml_hash =
            crate::services::signing::actions::transmission::sha256_hex(&loaded.eml).to_uppercase();
        let mut package = loaded.package.clone();
        let election_name = election.get_name(&election.get_default_language());
        let area_name = area.name.clone().unwrap_or_default();
        for member in &members {
            package.logs.push(sign_transmission_package_log(
                &now_local,
                &at.election_id,
                &election_name,
                &at.area_id,
                &area_name,
                &member.id,
            ));
        }
        let latest = get_latest_miru_document(&package.documents)
            .ok_or_else(|| anyhow!("The transmission package has no document"))?;
        // The destinations the package was made for, which its subject names.
        let all_servers = generate_all_servers_document(
            hasura_transaction,
            &eml_hash,
            &eml,
            loaded.compressed.clone(),
            &package.servers,
            &area_annotations,
            &election_event.get_annotations()?,
            &at.election_event_id,
            &at.tenant_id,
            zone.offset,
            now_utc,
            members,
            &package.logs,
            &election.get_annotations()?,
            &latest.transaction_id,
            &ReportRequester::default(),
        )
        .await?;
        package.documents.push(MiruDocument {
            document_ids: MiruDocumentIds {
                eml: latest.document_ids.eml.clone(),
                xz: latest.document_ids.xz.clone(),
                all_servers: all_servers.id.clone(),
            },
            transaction_id: latest.transaction_id.clone(),
            servers_sent_to: vec![],
            created_at: now_local.to_rfc3339(),
            signatures,
        });
        update_transmission_package_annotations(
            hasura_transaction,
            &at.tenant_id,
            &at.election_event_id,
            &loaded.tally_session_id,
            &at.area_id,
            &at.election_id,
            tally_session.get_annotations()?,
            package,
            tally_annotations,
        )
        .await?;
        Ok(all_servers.id)
    }
}
