// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use sequent_core::ballot_receipt::{ReceivedBallot, ReceivedStatement};
use strum_macros::{Display, EnumString};
use tokio_postgres::row::Row;
use tracing::instrument;
use uuid::Uuid;

/// The format of the times the ballot box signs: `ReceivedStatement::received_at`
/// and `CastReceiptStatement::cast_at`.
const SIGNED_TIME_FORMAT: &str = "%Y-%m-%dT%H:%M:%S%.3fZ";

pub fn format_received_at(received_at: &DateTime<Utc>) -> String {
    received_at.format(SIGNED_TIME_FORMAT).to_string()
}

pub fn format_cast_at(cast_at: &DateTime<Utc>) -> String {
    cast_at.format(SIGNED_TIME_FORMAT).to_string()
}

#[derive(Display, EnumString, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceivedBallotStatus {
    #[strum(serialize = "received")]
    Received,
    #[strum(serialize = "cast")]
    Cast,
    #[strum(serialize = "audited")]
    Audited,
}

pub struct ReceivedBallotScope<'a> {
    pub tenant_id: &'a Uuid,
    pub election_event_id: &'a Uuid,
    pub election_id: &'a Uuid,
    pub voter_id: &'a str,
    pub ballot_hash: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredReceivedBallot {
    pub id: Uuid,
    pub status: ReceivedBallotStatus,
    pub received: ReceivedBallot,
}

impl TryFrom<Row> for StoredReceivedBallot {
    type Error = anyhow::Error;
    fn try_from(item: Row) -> Result<Self> {
        let received_at: DateTime<Utc> = item.try_get("received_at")?;
        Ok(StoredReceivedBallot {
            id: item.try_get("id")?,
            status: item
                .try_get::<_, String>("status")?
                .parse()
                .map_err(|err| anyhow!("Invalid received ballot status: {err}"))?,
            received: ReceivedBallot {
                statement: ReceivedStatement {
                    tenant_id: item.try_get::<_, Uuid>("tenant_id")?.to_string(),
                    election_event_id: item.try_get::<_, Uuid>("election_event_id")?.to_string(),
                    election_id: item.try_get::<_, Uuid>("election_id")?.to_string(),
                    ballot_hash: item.try_get("ballot_hash")?,
                    voter_signing_pk: item.try_get("voter_signing_pk")?,
                    voter_ballot_signature: item.try_get("voter_ballot_signature")?,
                    received_at: format_received_at(&received_at),
                    key_id: item.try_get("key_id")?,
                },
                received_signature: item.try_get("received_signature")?,
                ballot_id: item.try_get("ballot_id")?,
            },
        })
    }
}

/// The cast the ballot box has stored for a received ballot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredCast {
    pub cast_at: DateTime<Utc>,
    pub cast_signature: String,
    pub cast_receipt_signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceivedBallotToCast {
    pub stored: StoredReceivedBallot,
    /// The voter-signed ballot, as it was received.
    pub content: String,
    /// `None` until the ballot is cast with a Cast signature.
    pub cast: Option<StoredCast>,
}

impl TryFrom<Row> for ReceivedBallotToCast {
    type Error = anyhow::Error;
    fn try_from(item: Row) -> Result<Self> {
        let cast = match (
            item.try_get::<_, Option<DateTime<Utc>>>("cast_at")?,
            item.try_get::<_, Option<String>>("cast_signature")?,
            item.try_get::<_, Option<String>>("cast_receipt_signature")?,
        ) {
            (Some(cast_at), Some(cast_signature), Some(cast_receipt_signature)) => {
                Some(StoredCast {
                    cast_at,
                    cast_signature,
                    cast_receipt_signature,
                })
            }
            _ => None,
        };
        Ok(ReceivedBallotToCast {
            content: item.try_get("content")?,
            cast,
            stored: StoredReceivedBallot::try_from(item)?,
        })
    }
}

const RECEIVED_BALLOT_COLUMNS: &str = "id, tenant_id, election_event_id, election_id, \
    ballot_id, ballot_hash, voter_signing_pk, voter_ballot_signature, received_at, key_id, \
    received_signature, status";

#[instrument(skip_all, err)]
pub async fn get_received_ballot(
    hasura_transaction: &Transaction<'_>,
    scope: &ReceivedBallotScope<'_>,
) -> Result<Option<StoredReceivedBallot>> {
    let statement = hasura_transaction
        .prepare(&format!(
            r#"
                SELECT {RECEIVED_BALLOT_COLUMNS}
                FROM sequent_backend.received_ballot
                WHERE
                    tenant_id = $1 AND
                    election_event_id = $2 AND
                    election_id = $3 AND
                    voter_id_string = $4 AND
                    ballot_hash = $5;
            "#
        ))
        .await?;
    hasura_transaction
        .query_opt(
            &statement,
            &[
                &scope.tenant_id,
                &scope.election_event_id,
                &scope.election_id,
                &scope.voter_id,
                &scope.ballot_hash,
            ],
        )
        .await?
        .map(StoredReceivedBallot::try_from)
        .transpose()
}

/// Stores a ballot with the receipt the ballot box signed for it. `None` when
/// a unique constraint refused the row: the voter has already sent this
/// ballot, or its Ballot ID is taken in the election event.
#[instrument(skip_all, err)]
pub async fn insert_received_ballot(
    hasura_transaction: &Transaction<'_>,
    scope: &ReceivedBallotScope<'_>,
    area_id: &Uuid,
    content: &str,
    received_at: &DateTime<Utc>,
    received: &ReceivedBallot,
) -> Result<Option<Uuid>> {
    let statement = hasura_transaction
        .prepare(
            r#"
                INSERT INTO sequent_backend.received_ballot
                    (tenant_id, election_event_id, election_id, area_id, voter_id_string,
                     ballot_id, ballot_hash, content, voter_signing_pk,
                     voter_ballot_signature, received_at, key_id, received_signature)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
                ON CONFLICT DO NOTHING
                RETURNING id;
            "#,
        )
        .await?;
    let row = hasura_transaction
        .query_opt(
            &statement,
            &[
                &scope.tenant_id,
                &scope.election_event_id,
                &scope.election_id,
                &area_id,
                &scope.voter_id,
                &received.ballot_id,
                &scope.ballot_hash,
                &content,
                &received.statement.voter_signing_pk,
                &received.statement.voter_ballot_signature,
                &received_at,
                &received.statement.key_id,
                &received.received_signature,
            ],
        )
        .await
        .map_err(|err| anyhow::Error::new(err).context("Error inserting received ballot"))?;

    row.map(|row| row.try_get("id").map_err(anyhow::Error::from))
        .transpose()
}

/// The voter's ballot the ballot box received under a Ballot ID, with what
/// casting it needs. `None` when that voter has no such ballot in the
/// election, whoever else may have one under that Ballot ID.
#[instrument(skip_all, err)]
pub async fn get_received_ballot_to_cast(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    election_id: &Uuid,
    voter_id: &str,
    ballot_id: &str,
) -> Result<Option<ReceivedBallotToCast>> {
    let statement = hasura_transaction
        .prepare(&format!(
            r#"
                SELECT {RECEIVED_BALLOT_COLUMNS}, content, cast_at, cast_signature,
                    cast_receipt_signature
                FROM sequent_backend.received_ballot
                WHERE
                    tenant_id = $1 AND
                    election_event_id = $2 AND
                    election_id = $3 AND
                    voter_id_string = $4 AND
                    ballot_id = $5;
            "#
        ))
        .await?;
    hasura_transaction
        .query_opt(
            &statement,
            &[
                &tenant_id,
                &election_event_id,
                &election_id,
                &voter_id,
                &ballot_id,
            ],
        )
        .await?
        .map(ReceivedBallotToCast::try_from)
        .transpose()
}

/// Marks a received ballot as cast, with the voter's Cast signature and the
/// receipt the ballot box signed for it. `false` when the ballot is no longer
/// waiting to be cast.
#[instrument(skip_all, err)]
pub async fn mark_received_ballot_cast(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    received_ballot_id: &Uuid,
    cast_at: &DateTime<Utc>,
    cast_signature: &str,
    cast_receipt_signature: &str,
) -> Result<bool> {
    let statement = hasura_transaction
        .prepare(
            r#"
                UPDATE sequent_backend.received_ballot
                SET status = $4, cast_at = $5, cast_signature = $6,
                    cast_receipt_signature = $7, last_updated_at = now()
                WHERE
                    tenant_id = $1 AND
                    election_event_id = $2 AND
                    id = $3 AND
                    status = $8;
            "#,
        )
        .await?;
    let updated = hasura_transaction
        .execute(
            &statement,
            &[
                &tenant_id,
                &election_event_id,
                &received_ballot_id,
                &ReceivedBallotStatus::Cast.to_string(),
                &cast_at,
                &cast_signature,
                &cast_receipt_signature,
                &ReceivedBallotStatus::Received.to_string(),
            ],
        )
        .await
        .map_err(|err| anyhow::Error::new(err).context("Error casting received ballot"))?;

    Ok(updated == 1)
}

/// `None` unless the ballot style is the one published for the voter's area
/// and election. Its `ballot_eml` is read only when `with_ballot_eml` asks
/// for it, since it is large.
#[instrument(skip(hasura_transaction), err)]
pub async fn get_published_ballot_eml(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    election_id: &Uuid,
    area_id: &Uuid,
    ballot_style_id: &Uuid,
    with_ballot_eml: bool,
) -> Result<Option<Option<String>>> {
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT CASE WHEN $6 THEN style.ballot_eml END AS ballot_eml
                FROM sequent_backend.ballot_style style
                JOIN sequent_backend.ballot_publication publication ON
                    publication.id = style.ballot_publication_id AND
                    publication.tenant_id = style.tenant_id AND
                    publication.election_event_id = style.election_event_id
                WHERE
                    style.id = $5 AND
                    style.tenant_id = $1 AND
                    style.election_event_id = $2 AND
                    style.election_id = $3 AND
                    style.area_id = $4 AND
                    style.deleted_at IS NULL AND
                    publication.deleted_at IS NULL AND
                    publication.published_at IS NOT NULL;
            "#,
        )
        .await?;
    let row = hasura_transaction
        .query_opt(
            &statement,
            &[
                &tenant_id,
                &election_event_id,
                &election_id,
                &area_id,
                &ballot_style_id,
                &with_ballot_eml,
            ],
        )
        .await?;

    Ok(row
        .map(|row| row.try_get::<_, Option<String>>("ballot_eml"))
        .transpose()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn the_signed_time_keeps_milliseconds_and_utc() {
        let received_at = Utc.timestamp_millis_opt(1_841_367_600_007).unwrap();

        assert_eq!(format_received_at(&received_at), "2028-05-08T03:00:00.007Z");
        assert_eq!(
            format_received_at(&Utc.timestamp_millis_opt(1_841_367_600_000).unwrap()),
            "2028-05-08T03:00:00.000Z"
        );
    }

    #[test]
    fn statuses_match_the_table_constraint() {
        for (status, stored) in [
            (ReceivedBallotStatus::Received, "received"),
            (ReceivedBallotStatus::Cast, "cast"),
            (ReceivedBallotStatus::Audited, "audited"),
        ] {
            assert_eq!(status.to_string(), stored);
            assert_eq!(stored.parse::<ReceivedBallotStatus>().unwrap(), status);
        }
    }
}
