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

/// The format of `ReceivedStatement::received_at`, which the ballot box signs.
const RECEIVED_AT_FORMAT: &str = "%Y-%m-%dT%H:%M:%S%.3fZ";

pub fn format_received_at(received_at: &DateTime<Utc>) -> String {
    received_at.format(RECEIVED_AT_FORMAT).to_string()
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

/// Marks the voter's received ballot as cast and returns it. `None` when the
/// ballot box has no such ballot still waiting to be cast.
#[instrument(skip_all, err)]
pub async fn mark_received_ballot_cast(
    hasura_transaction: &Transaction<'_>,
    scope: &ReceivedBallotScope<'_>,
) -> Result<Option<StoredReceivedBallot>> {
    let statement = hasura_transaction
        .prepare(&format!(
            r#"
                UPDATE sequent_backend.received_ballot
                SET status = $6, cast_at = now(), last_updated_at = now()
                WHERE
                    tenant_id = $1 AND
                    election_event_id = $2 AND
                    election_id = $3 AND
                    voter_id_string = $4 AND
                    ballot_hash = $5 AND
                    status = $7
                RETURNING {RECEIVED_BALLOT_COLUMNS};
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
                &ReceivedBallotStatus::Cast.to_string(),
                &ReceivedBallotStatus::Received.to_string(),
            ],
        )
        .await?
        .map(StoredReceivedBallot::try_from)
        .transpose()
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
