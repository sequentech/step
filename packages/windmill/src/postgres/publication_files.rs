// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Rows behind the private S3 objects of a ballot publication.

use anyhow::Result;
use deadpool_postgres::Transaction;
use futures::{Stream, TryStreamExt};
use serde_json::Value;
use uuid::Uuid;

/// A published, non-deleted ballot style in a voter's area, with the object
/// root of its publication and the live policy of its election.
#[derive(Clone, Debug, PartialEq)]
pub struct PublishedBallotStyle {
    pub id: Uuid,
    pub election_id: Uuid,
    pub publication_id: Uuid,
    pub root: Option<String>,
    pub status: Option<Value>,
    pub num_allowed_revotes: Option<i64>,
    pub voting_channels: Option<Value>,
}

pub async fn get_publication_annotations(
    hasura_transaction: &Transaction<'_>,
    tenant: Uuid,
    event: Uuid,
    publication: Uuid,
) -> Result<Option<Value>> {
    let row = hasura_transaction
        .query_one(
            "SELECT annotations FROM sequent_backend.ballot_publication WHERE tenant_id=$1 AND election_event_id=$2 AND id=$3",
            &[&tenant, &event, &publication],
        )
        .await?;
    Ok(row.get(0))
}

pub async fn get_publication_event(
    hasura_transaction: &Transaction<'_>,
    tenant: Uuid,
    event: Uuid,
) -> Result<Value> {
    let row = hasura_transaction
        .query_one(
            "SELECT jsonb_build_object('id', id, 'presentation', presentation, 'description', description) FROM sequent_backend.election_event WHERE tenant_id=$1 AND id=$2",
            &[&tenant, &event],
        )
        .await?;
    Ok(row.get(0))
}

pub async fn get_publication_elections(
    hasura_transaction: &Transaction<'_>,
    tenant: Uuid,
    event: Uuid,
    publication: Uuid,
) -> Result<Vec<Value>> {
    let elections = hasura_transaction
        .query(
            r#"
            SELECT jsonb_build_object('id', id,
                'tenant_id', tenant_id,
                'election_event_id', election_event_id,
                'annotations', annotations,
                'created_at', created_at,
                'description', description,
                'is_consolidated_ballot_encoding', is_consolidated_ballot_encoding,
                'labels', labels,
                'last_updated_at', last_updated_at,
                'presentation', presentation,
                'spoil_ballot_option', spoil_ballot_option)
            FROM sequent_backend.election
            WHERE tenant_id=$1
              AND election_event_id=$2
              AND id = ANY(SELECT unnest(election_ids)
            FROM sequent_backend.ballot_publication
            WHERE id=$3
              AND tenant_id=$1)
        "#,
            &[&tenant, &event, &publication],
        )
        .await?;
    Ok(elections.iter().map(|row| row.get(0)).collect())
}

/// Streams full styles: publication memory does not grow with the number of areas.
pub async fn stream_publication_styles(
    hasura_transaction: &Transaction<'_>,
    tenant: Uuid,
    event: Uuid,
    publication: Uuid,
) -> Result<impl Stream<Item = Result<Value>> + Send> {
    let rows = hasura_transaction
        .query_raw(
            r#"
            SELECT jsonb_build_object('id', id,
                'tenant_id', tenant_id,
                'election_event_id', election_event_id,
                'election_id', election_id,
                'area_id', area_id,
                'created_at', created_at,
                'last_updated_at', last_updated_at,
                'annotations', annotations,
                'labels', labels,
                'ballot_eml', ballot_eml,
                'ballot_signature', ballot_signature,
                'status', status,
                'deleted_at', deleted_at)
            FROM sequent_backend.ballot_style
            WHERE tenant_id=$1
              AND election_event_id=$2
              AND ballot_publication_id=$3
        "#,
            [&tenant, &event, &publication],
        )
        .await?;
    Ok(rows
        .map_ok(|row| row.get::<_, Value>(0))
        .map_err(anyhow::Error::from))
}

/// Adds or replaces one text annotation, keeping the others.
pub async fn merge_ballot_publication_annotation(
    hasura_transaction: &Transaction<'_>,
    tenant: Uuid,
    event: Uuid,
    publication: Uuid,
    key: &str,
    value: &str,
) -> Result<()> {
    hasura_transaction
        .execute(
            r#"
            UPDATE sequent_backend.ballot_publication
            SET annotations=COALESCE(annotations,'{}'::jsonb) || jsonb_build_object($4::text,$5::text)
            WHERE tenant_id=$1
              AND election_event_id=$2
              AND id=$3
        "#,
            &[&tenant, &event, &publication, &key, &value],
        )
        .await?;
    Ok(())
}

/// Newest publication first. `root_annotation` names the annotation that holds
/// each publication's object root.
pub async fn get_published_ballot_styles(
    hasura_transaction: &Transaction<'_>,
    tenant: Uuid,
    event: Uuid,
    area: Uuid,
    elections: &[Uuid],
    root_annotation: &str,
) -> Result<Vec<PublishedBallotStyle>> {
    let rows = hasura_transaction
        .query(
            r#"
            SELECT s.id, s.election_id, p.id AS publication_id, p.annotations->>$5::text AS root, e.status, e.num_allowed_revotes::bigint AS num_allowed_revotes, e.voting_channels
            FROM sequent_backend.ballot_style s
            JOIN sequent_backend.ballot_publication p ON p.id=s.ballot_publication_id
              AND p.tenant_id=s.tenant_id
              AND p.election_event_id=s.election_event_id
            JOIN sequent_backend.election e ON e.id=s.election_id
              AND e.tenant_id=s.tenant_id
              AND e.election_event_id=s.election_event_id
            WHERE s.tenant_id=$1
              AND s.election_event_id=$2
              AND s.area_id=$3
              AND s.election_id=ANY($4)
              AND s.deleted_at IS NULL
              AND p.published_at IS NOT NULL
              AND p.deleted_at IS NULL
            ORDER BY p.published_at DESC, s.election_id
        "#,
            &[&tenant, &event, &area, &elections, &root_annotation],
        )
        .await?;
    let mut styles = Vec::with_capacity(rows.len());
    for row in rows {
        styles.push(PublishedBallotStyle {
            id: row.get("id"),
            election_id: row.get("election_id"),
            publication_id: row.get("publication_id"),
            root: row.get("root"),
            status: row.get("status"),
            num_allowed_revotes: row.try_get("num_allowed_revotes")?,
            voting_channels: row.get("voting_channels"),
        });
    }
    Ok(styles)
}

/// `None` when the event does not exist; `Some(None)` when it has no status.
pub async fn get_election_event_status(
    hasura_transaction: &Transaction<'_>,
    tenant: Uuid,
    event: Uuid,
) -> Result<Option<Option<Value>>> {
    let row = hasura_transaction
        .query_opt(
            "SELECT status FROM sequent_backend.election_event WHERE tenant_id=$1 AND id=$2",
            &[&tenant, &event],
        )
        .await?;
    Ok(row.map(|row| row.get(0)))
}
