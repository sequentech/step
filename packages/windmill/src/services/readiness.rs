// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Compare authoritative event snapshots and record a successful readiness check.

use crate::domain::readiness::{
    self, ConfigurationEvidence, ElectionEvidence, EventEvidence, PublicationEvidence,
    ReadinessEvidence, ReadinessReport,
};
use crate::postgres::{
    area, ballot_box_seal, ballot_publication, configuration_packages, election, election_event,
    signing, voter_import,
};
use crate::services::audit_log::AuditActor;
use crate::services::audit_log_writer::delivery_id;
use crate::services::ballot_box_reads::get_event_ballot_box;
use crate::services::ballot_box_seal::{deadline, expected_ballot_boxes, published_ballot_styles};
use crate::services::ballot_styles::design_check::check_publication_designs;
use crate::services::database::get_keycloak_pool;
use crate::services::electoral_log::ElectoralLog;
use anyhow::{ensure, Context, Result};
use deadpool_postgres::Transaction;
use electoral_log::messages::message::Message;
use electoral_log::messages::newtypes::AuditStatementKind;
use electoral_log::messages::statement::StatementBody;
use sequent_core::ballot::{BallotBoxSealPolicy, ElectionStatus, LockedDown};
use sequent_core::election_config::package_verify::CertificateIdentity;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::hasura::core::VotingChannels;
use sequent_core::types::voter_import::VotersImportedEvidence;
use serde_json::Value;
use std::collections::BTreeMap;
use uuid::Uuid;
use voting_crypto::signing::VerificationKey;
use voting_crypto::RecordDeserialize;

/// Take the existing import/signing/publication locks in stable event order.
/// Every official state row stays locked through the final signed append.
pub async fn lock_events(
    tx: &Transaction<'_>,
    tenant: &str,
    official: &str,
    tested: &str,
) -> Result<()> {
    ensure!(
        official != tested,
        "The official and tested events must differ"
    );
    let tenant_uuid = Uuid::parse_str(tenant)?;
    let mut events = [Uuid::parse_str(official)?, Uuid::parse_str(tested)?];
    events.sort();
    for event in events {
        signing::lock_signing_event(tx, tenant_uuid, event).await?;
        voter_import::lock_voter_imports(tx, tenant, &event.to_string()).await?;
        ballot_publication::lock_publication_event(tx, tenant, &event.to_string()).await?;
        let row = tx.query_opt(
            "SELECT id FROM sequent_backend.election_event WHERE tenant_id = $1 AND id = $2 FOR SHARE",
            &[&tenant_uuid, &event],
        ).await?;
        ensure!(
            row.is_some(),
            "Election event not found in the authenticated tenant"
        );
        tx.query(
            "SELECT id FROM sequent_backend.election WHERE tenant_id = $1 AND election_event_id = $2 ORDER BY id FOR SHARE",
            &[&tenant_uuid, &event],
        ).await?;
    }
    Ok(())
}

fn status(value: Option<Value>) -> Result<Option<ElectionStatus>> {
    value
        .map(serde_json::from_value)
        .transpose()
        .context("Invalid voting status")
}

async fn configuration(
    tx: &Transaction<'_>,
    tenant: &str,
    event: &str,
) -> Result<Option<ConfigurationEvidence>> {
    let Some((manifest, manifest_sha256)) =
        configuration_packages::manifest_of_event(tx, tenant, event).await?
    else {
        return Ok(None);
    };
    let row = tx.query_one(
        "SELECT approvers FROM sequent_backend.configuration_package WHERE tenant_id = $1 AND election_event_id = $2 ORDER BY revision DESC LIMIT 1",
        &[&Uuid::parse_str(tenant)?, &Uuid::parse_str(event)?],
    ).await?;
    let approvers: Vec<CertificateIdentity> = serde_json::from_value(row.try_get(0)?)?;
    Ok(Some(ConfigurationEvidence {
        external_id: manifest.configuration.external_id,
        revision: manifest.configuration.revision,
        content_sha256: manifest.content_sha256,
        manifest_sha256,
        approvers: approvers
            .iter()
            .map(serde_json::to_string)
            .collect::<std::result::Result<_, _>>()?,
        areas: manifest
            .content
            .ballot_designs
            .iter()
            .map(|design| design.area.clone())
            .collect(),
    }))
}

/// File counts must describe the whole current registry, not just the last
/// additive CSV. Stable area names are the manifest's existing design keys.
async fn current_registry_counts(
    tx: &Transaction<'_>,
    tenant: &str,
    event: &str,
) -> Result<BTreeMap<String, u64>> {
    let names: BTreeMap<String, String> = area::get_event_areas(tx, tenant, event)
        .await?
        .into_iter()
        .filter_map(|area| area.name.map(|name| (area.id, name.trim().to_string())))
        .collect();
    let keycloak = get_keycloak_pool().await;
    let client = keycloak.get().await?;
    let realm = get_event_realm(tenant, event);
    registry_counts(&client, &realm, &names).await
}

/// The authoritative event census, one record per nonsystem identity. Missing,
/// unknown and ambiguous area assignments are retained under an empty key,
/// which a certified voter receipt can never accept.
pub async fn registry_counts(
    client: &deadpool_postgres::Client,
    realm: &str,
    names: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, u64>> {
    let rows = client
        .query(
            "SELECT area_id, count(*) FROM (
             SELECT u.id,
                    CASE WHEN count(DISTINCT a.value) = 1 THEN min(a.value) ELSE NULL END AS area_id
             FROM user_entity u
             JOIN realm r ON r.id = u.realm_id
             LEFT JOIN user_attribute a ON a.user_id = u.id AND a.name = $2
             WHERE r.name = $1 AND u.service_account_client_link IS NULL
             GROUP BY u.id
         ) AS census GROUP BY area_id",
            &[&realm, &sequent_core::types::keycloak::AREA_ID_ATTR_NAME],
        )
        .await
        .context("Could not count the current event registry")?;
    let mut counts = BTreeMap::new();
    for row in rows {
        let id: Option<String> = row.try_get(0)?;
        let name = id
            .as_ref()
            .and_then(|id| names.get(id))
            .map(|name| name.trim().to_string())
            .unwrap_or_default();
        *counts.entry(name).or_default() += u64::try_from(row.try_get::<_, i64>(1)?)?;
    }
    Ok(counts)
}

/// Every accepted ordinary ballot counts against an untouched official event,
/// including pending/rejected ballots and records awaiting log sequencing.
pub async fn ordinary_ballot_count(
    store: &electoral_log::adapters::postgres::PostgresStore,
    event: &str,
) -> Result<u64> {
    let count: i64 = store
        .client()
        .await?
        .query_one(
            "SELECT count(*) FROM ballot_box_ballot WHERE election_event_id = $1",
            &[&Uuid::parse_str(event)?],
        )
        .await?
        .try_get(0)?;
    Ok(u64::try_from(count)?)
}

async fn voter_receipt_delivered(
    tx: &Transaction<'_>,
    tenant: &str,
    store: &electoral_log::adapters::postgres::PostgresStore,
    board: &str,
    event: &str,
    evidence: &VotersImportedEvidence,
) -> Result<bool> {
    let id = Uuid::parse_str(&evidence.import_id)?;
    let Some(delivered) = store.delivered(board, &delivery_id(id)).await? else {
        return Ok(false);
    };
    let message = Message::record_deserialize(&delivered.message.message)?;
    let key =
        crate::services::electoral_log_board::get_board_signing_key(tx, tenant, Some(event), board)
            .await?;
    message
        .verify(&VerificationKey::from_sk(&key)?)
        .context("Voter-import evidence has an invalid electoral-log signature")?;
    if message.statement.head.event.0 != event {
        return Ok(false);
    }
    match message.statement.body {
        StatementBody::Audit(entry)
            if entry.kind == AuditStatementKind::VotersImported
                && entry.entry_id == evidence.import_id =>
        {
            let signed: VotersImportedEvidence = serde_json::from_str(&entry.details_json)?;
            Ok(signed == *evidence)
        }
        _ => Ok(false),
    }
}

pub async fn snapshot(tx: &Transaction<'_>, tenant: &str, event_id: &str) -> Result<EventEvidence> {
    let event = election_event::get_election_event_by_id(tx, tenant, event_id).await?;
    let presentation = event.get_presentation()?.unwrap_or_default();
    let elections = election::get_elections(tx, tenant, event_id).await?;
    let ids: Vec<String> = elections
        .iter()
        .map(|election| election.id.clone())
        .collect();
    let ballot_box = get_event_ballot_box(tx, tenant, event_id).await?;
    // Counts every stored status, including ballots not yet sequenced. All
    // current elections of this event are official; practice uses isolated
    // events until EMS-TEST-VOTING supplies an explicit designation contract.
    let ballot_count = ordinary_ballot_count(&ballot_box.store, event_id).await?;
    // Review-stage receipts are already stored ballots even before the voter
    // confirms casting. They cannot be evidence of an untouched official event.
    let received_ballot_count: i64 = ballot_box
        .store
        .client()
        .await?
        .query_one(
            "SELECT count(*) FROM ballot_box_received WHERE election_event_id = $1",
            &[&Uuid::parse_str(event_id)?],
        )
        .await?
        .try_get(0)?;
    let tally_count: i64 = tx.query_one(
        "SELECT count(*) FROM sequent_backend.tally_session WHERE tenant_id = $1 AND election_event_id = $2",
        &[&Uuid::parse_str(tenant)?, &Uuid::parse_str(event_id)?],
    ).await?.try_get(0)?;
    let voters = voter_import::latest_voter_import(tx, tenant, event_id).await?;
    let pending_import: bool = tx.query_one(
        "SELECT EXISTS (SELECT 1 FROM sequent_backend.tasks_execution WHERE tenant_id = $1 AND election_event_id = $2 AND type = 'IMPORT_USERS' AND execution_status = 'IN_PROGRESS')",
        &[&Uuid::parse_str(tenant)?, &Uuid::parse_str(event_id)?],
    ).await?.try_get(0)?;
    let voter_log_delivered = match &voters {
        Some(receipt) => {
            voter_receipt_delivered(
                tx,
                tenant,
                &ballot_box.store,
                &ballot_box.board,
                event_id,
                receipt,
            )
            .await?
        }
        None => false,
    };
    let current_voter_counts = current_registry_counts(tx, tenant, event_id).await?;
    let mut publication = None;
    // generated-but-unpublished rows have NULL published_at and must not win.
    let publication_row = tx.query_opt(
        "SELECT id::text FROM sequent_backend.ballot_publication WHERE tenant_id = $1 AND election_event_id = $2 AND deleted_at IS NULL AND published_at IS NOT NULL ORDER BY published_at DESC LIMIT 1",
        &[&Uuid::parse_str(tenant)?, &Uuid::parse_str(event_id)?],
    ).await?;
    if let Some(row) = publication_row {
        let id: String = row.try_get(0)?;
        let checked = check_publication_designs(tx, tenant, event_id, &id).await?;
        publication = Some(PublicationEvidence {
            publication_id: id,
            matches_manifest: checked
                .as_ref()
                .is_some_and(|check| check.mismatches.is_empty()),
            design_count: checked.map_or(0, |check| check.digests.len()),
        });
    }
    let styles = published_ballot_styles(tx, tenant, event_id, &ids).await?;
    let expected_boxes = expected_ballot_boxes(&styles);
    let uuids: Vec<Uuid> = ids
        .iter()
        .map(|id| Uuid::parse_str(id))
        .collect::<std::result::Result<_, _>>()?;
    let seals = ballot_box_seal::list_for_elections(
        tx,
        &Uuid::parse_str(tenant)?,
        &Uuid::parse_str(event_id)?,
        &uuids,
    )
    .await?;
    let sealed_boxes = seals
        .into_iter()
        .filter(|seal| {
            matches!(
                seal.status,
                ballot_box_seal::BallotBoxSealStatus::Sealed
                    | ballot_box_seal::BallotBoxSealStatus::Published
            ) && seal.seal_hash.as_ref().is_some_and(|hash| !hash.is_empty())
                && seal.sealed_at.is_some()
                && seal.log_record.is_some()
        })
        .map(|seal| (seal.election_id.to_string(), seal.area_id.to_string()))
        .collect();
    let mut all_channels_closed = !elections.is_empty();
    let mut election_evidence = Vec::new();
    for election in elections {
        let election_presentation = election
            .presentation
            .clone()
            .map(serde_json::from_value::<sequent_core::ballot::ElectionPresentation>)
            .transpose()
            .context("Invalid election presentation")?
            .unwrap_or_default();
        let status = status(election.status)?;
        let channels: VotingChannels = election
            .voting_channels
            .map(serde_json::from_value)
            .transpose()?
            .unwrap_or_default();
        all_channels_closed &= status.as_ref().is_some_and(|status| {
            deadline::seal_deadline(status, &channels, &election_presentation)
                .is_some_and(|(_, deadline)| deadline <= chrono::Utc::now())
        });
        election_evidence.push(ElectionEvidence {
            election_id: election.id,
            status,
        });
    }
    Ok(EventEvidence {
        event_id: event_id.into(),
        configuration: configuration(tx, tenant, event_id).await?,
        voters,
        voter_log_delivered,
        current_voter_counts,
        voter_import_in_progress: pending_import,
        status: status(event.status)?,
        elections: election_evidence,
        ballot_count,
        received_ballot_count: u64::try_from(received_ballot_count)?,
        tally_session_count: u64::try_from(tally_count)?,
        publication,
        seal_at_close: presentation.ballot_box_seal_policy() == BallotBoxSealPolicy::SEAL_AT_CLOSE,
        expected_boxes,
        sealed_boxes,
        all_channels_closed,
        locked_down: presentation.locked_down == Some(LockedDown::LOCKED_DOWN),
    })
}

/// The only write is the signed entry after the pure evaluator passes.
pub async fn verify(
    tx: &Transaction<'_>,
    tenant: &str,
    official: &str,
    tested: &str,
    actor: &AuditActor,
) -> Result<ReadinessReport> {
    let evidence = ReadinessEvidence {
        official: snapshot(tx, tenant, official).await?,
        tested: snapshot(tx, tenant, tested).await?,
    };
    let report = readiness::evaluate(evidence, chrono::Utc::now().to_rfc3339());
    record_if_passed(
        report,
        &EventRecorder {
            tx,
            tenant,
            official,
            actor,
        },
    )
    .await
}

struct EventRecorder<'a, 'tx> {
    tx: &'a Transaction<'tx>,
    tenant: &'a str,
    official: &'a str,
    actor: &'a AuditActor,
}

impl crate::ports::readiness::ReadinessRecorder for EventRecorder<'_, '_> {
    async fn record(&self, report: &ReadinessReport) -> Result<()> {
        let event =
            election_event::get_election_event_by_id(self.tx, self.tenant, self.official).await?;
        let board = crate::services::election_event_board::get_election_event_board(
            event.bulletin_board_reference,
        )
        .context("Official event has no log")?;
        let log = ElectoralLog::new(self.tx, self.tenant, Some(self.official), &board).await?;
        log.post_readiness_verified(self.official, report, self.actor)
            .await
    }
}

pub async fn record_if_passed(
    mut report: ReadinessReport,
    recorder: &impl crate::ports::readiness::ReadinessRecorder,
) -> Result<ReadinessReport> {
    if report.passed {
        report.log_entry_id = Some(Uuid::new_v4().to_string());
        recorder.record(&report).await?;
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct Recorder {
        entries: Mutex<Vec<Value>>,
        fail: bool,
    }

    impl crate::ports::readiness::ReadinessRecorder for Recorder {
        async fn record(&self, report: &ReadinessReport) -> Result<()> {
            ensure!(!self.fail, "The signed append failed");
            self.entries
                .lock()
                .unwrap()
                .push(serde_json::to_value(report)?);
            Ok(())
        }
    }

    fn report() -> ReadinessReport {
        readiness::evaluate(
            serde_json::from_value(readiness::tests::evidence()).unwrap(),
            "2026-10-14T10:00:00Z".into(),
        )
    }

    #[tokio::test]
    async fn failed_checks_write_nothing_and_passes_record_the_returned_evidence() {
        let recorder = Recorder {
            entries: Mutex::new(Vec::new()),
            fail: false,
        };
        let mut failed = report();
        failed.evidence.official.ballot_count = 1;
        let failed = readiness::evaluate(failed.evidence, failed.checked_at);
        assert!(!record_if_passed(failed, &recorder).await.unwrap().passed);
        assert!(recorder.entries.lock().unwrap().is_empty());
        let passed = record_if_passed(report(), &recorder).await.unwrap();
        assert!(passed.passed);
        assert!(passed.log_entry_id.is_some());
        assert_eq!(
            *recorder.entries.lock().unwrap(),
            [serde_json::to_value(&passed).unwrap()]
        );
    }

    #[tokio::test]
    async fn append_failure_is_never_a_successful_readiness_response() {
        let recorder = Recorder {
            entries: Mutex::new(Vec::new()),
            fail: true,
        };
        assert_eq!(
            record_if_passed(report(), &recorder)
                .await
                .unwrap_err()
                .to_string(),
            "The signed append failed"
        );
        assert!(recorder.entries.lock().unwrap().is_empty());
    }
}
