// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The signed close's seal hook (VOTE-FREEZE). The close itself created the
//! election's `pending` seals in the same transaction; this records on them
//! the Close voting request and who signed it.

use crate::postgres::ballot_box_seal::{
    list_for_elections, set_close_provenance, BallotBoxSeal, BallotBoxSealStatus, ClosedBy,
    ClosedBySigner,
};
use crate::postgres::signing::get_signing_area_name;
use crate::services::signing::actions::voting::{SealRecord, SealSummary};
use crate::services::signing::actions::SealRecordSink;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use serde::Serialize;
use uuid::Uuid;

/// The hash algorithm a seal summary states.
const SEAL_HASH_ALGORITHM: &str = "SHA-512";

/// One ballot box of a Close voting request's Post, as the signing panel
/// shows it: the seal row as it stands now. The boxes are sealed after the
/// close commits, and after the grace period when there is one, so the
/// request's stored result (its [`SealSummary`] list) can't name them.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ClosingBallotBox {
    pub area_id: Uuid,
    pub area_name: String,
    pub status: BallotBoxSealStatus,
    /// When the box is sealed: the close plus the grace period.
    pub grace_deadline: DateTime<Utc>,
    pub sealed_at: Option<DateTime<Utc>>,
    pub ballots: Option<i64>,
    pub hash_algorithm: &'static str,
    pub seal_hash: Option<String>,
}

impl ClosingBallotBox {
    pub fn from_seal(seal: &BallotBoxSeal, area_name: String) -> Self {
        ClosingBallotBox {
            area_id: seal.area_id,
            area_name,
            status: seal.status,
            grace_deadline: seal.grace_deadline,
            sealed_at: seal.sealed_at,
            ballots: seal.ballots_in_box,
            hash_algorithm: SEAL_HASH_ALGORITHM,
            seal_hash: seal.seal_hash.clone(),
        }
    }
}

/// The ballot boxes of `election_id` that the Close voting request
/// `request_id` closed, as the seal table has them now, with their
/// countries' names. None when the request closed only some channels: the
/// boxes are then closed, and sealed, by the close of the last one.
pub async fn closing_ballot_boxes(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    election_id: Uuid,
    request_id: Uuid,
) -> Result<Vec<ClosingBallotBox>> {
    let seals = list_for_elections(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        &[election_id],
    )
    .await?;
    let mut boxes = Vec::with_capacity(seals.len());
    for seal in seals
        .iter()
        .filter(|seal| seal.close_request_id == Some(request_id))
    {
        // The row names its country only once it is sealed.
        let area_name = match &seal.area_name {
            Some(name) => name.clone(),
            None => get_signing_area_name(
                hasura_transaction,
                tenant_id,
                election_event_id,
                seal.area_id,
            )
            .await?
            .unwrap_or_else(|| seal.area_id.to_string()),
        };
        boxes.push(ClosingBallotBox::from_seal(seal, area_name));
    }
    Ok(boxes)
}

/// Records a signed close on its election's pending seals.
#[derive(Debug, Clone, Copy)]
pub struct PendingSealSink {
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    /// The Close voting request that ran.
    pub request_id: Uuid,
}

#[async_trait]
impl SealRecordSink for PendingSealSink {
    async fn feed(
        &self,
        hasura_transaction: &Transaction<'_>,
        record: &SealRecord,
    ) -> Result<Vec<SealSummary>> {
        let election_id = record
            .election_id
            .ok_or_else(|| anyhow!("the seal record of request {} has no Post", self.request_id))?;
        let closed_by = ClosedBy::Signed {
            signers: Some(
                record
                    .signatures
                    .iter()
                    .map(|signature| ClosedBySigner {
                        name: signature.display_name.clone(),
                        certificate_sha256: signature.certificate_fingerprint.clone(),
                    })
                    .collect(),
            ),
            signing_code: Some(record.code.clone()).filter(|code| !code.is_empty()),
        };
        set_close_provenance(
            hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
            &election_id,
            Some(self.request_id),
            &closed_by,
        )
        .await?;
        // Boxes already sealed before this close (usually none).
        Ok(list_for_elections(
            hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
            &[election_id],
        )
        .await?
        .into_iter()
        .filter(|seal| {
            matches!(
                seal.status,
                BallotBoxSealStatus::Sealed | BallotBoxSealStatus::Published
            )
        })
        .map(|seal| SealSummary {
            area_id: Some(seal.area_id),
            area_name: None,
            hash_algorithm: SEAL_HASH_ALGORITHM.to_string(),
            hash: seal.seal_hash.unwrap_or_default(),
            ballots: seal
                .ballots_in_box
                .and_then(|ballots| u64::try_from(ballots).ok()),
            signed_by: None,
        })
        .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn seal(status: BallotBoxSealStatus) -> BallotBoxSeal {
        let at = Utc.with_ymd_and_hms(2026, 10, 8, 10, 0, 0).unwrap();
        BallotBoxSeal {
            id: Uuid::nil(),
            tenant_id: Uuid::nil(),
            election_event_id: Uuid::nil(),
            election_id: Uuid::nil(),
            area_id: Uuid::from_u128(7),
            status,
            closed_at: at,
            grace_deadline: at,
            close_request_id: None,
            closed_by: ClosedBy::Scheduled,
            sealed_at: None,
            ballots_in_box: None,
            ballots_counted: None,
            seal_hash: None,
            manifest: Some(vec![1]),
            signed_message: Some(vec![2]),
            failure_reason: None,
            log_entry_id: None,
            public_document_id: None,
            public_path: None,
            published_at: None,
            created_at: at,
            last_attempt_at: None,
            waiting_reason: None,
            failure_posted_at: None,
            election_name: None,
            area_name: None,
        }
    }

    #[test]
    fn a_pending_box_shows_its_deadline_and_no_seal() {
        let row = ClosingBallotBox::from_seal(&seal(BallotBoxSealStatus::Pending), "Spain".into());
        assert_eq!(row.area_name, "Spain");
        assert_eq!(row.status, BallotBoxSealStatus::Pending);
        assert_eq!(row.seal_hash, None);
        assert_eq!(row.ballots, None);
        let json = serde_json::to_value(&row).unwrap();
        assert_eq!(json["status"], "pending");
        assert_eq!(json["hash_algorithm"], "SHA-512");
        // The panel never carries the large columns.
        assert!(json.get("manifest").is_none());
        assert!(json.get("signed_message").is_none());
    }

    #[test]
    fn a_sealed_box_shows_its_hash_and_ballots() {
        let mut sealed = seal(BallotBoxSealStatus::Published);
        sealed.seal_hash = Some("ab".repeat(64));
        sealed.ballots_in_box = Some(1342);
        sealed.sealed_at = Some(sealed.closed_at);
        let row = ClosingBallotBox::from_seal(&sealed, "Spain".into());
        assert_eq!(row.seal_hash.as_deref(), Some("ab".repeat(64).as_str()));
        assert_eq!(row.ballots, Some(1342));
        assert_eq!(row.sealed_at, sealed.sealed_at);
    }
}
