// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use tempfile::NamedTempFile;
use uuid::Uuid;
use windmill::repositories::ballots::postgres::HasuraBallotRepository;
use windmill::repositories::ballots::BallotRepository;

mod support;

use support::postgres::shared_pool;

#[tokio::test]
async fn export_area_ballots_returns_all_voters_when_each_has_one_vote() -> Result<()> {
    assert_export_case(
        &[
            VoteSeed::new("voter-a", "{\"ballot\":\"alpha\"}", "2026-01-01T09:00:00Z"),
            VoteSeed::new("voter-b", "{\"ballot\":\"beta\"}", "2026-01-01T09:05:00Z"),
        ],
        &[row("voter-a", "{\"ballot\":\"alpha\"}"), row("voter-b", "{\"ballot\":\"beta\"}")],
    )
    .await
}

#[tokio::test]
async fn export_area_ballots_returns_only_latest_vote_per_voter() -> Result<()> {
    assert_export_case(
        &[
            VoteSeed::new(
                "voter-a",
                "{\"ballot\":\"alpha-old\"}",
                "2026-01-01T09:00:00Z",
            ),
            VoteSeed::new(
                "voter-a",
                "{\"ballot\":\"alpha-new\"}",
                "2026-01-01T09:10:00Z",
            ),
            VoteSeed::new(
                "voter-b",
                "{\"ballot\":\"beta-old\"}",
                "2026-01-01T09:01:00Z",
            ),
            VoteSeed::new(
                "voter-b",
                "{\"ballot\":\"beta-new\"}",
                "2026-01-01T09:11:00Z",
            ),
        ],
        &[
            row("voter-a", "{\"ballot\":\"alpha-new\"}"),
            row("voter-b", "{\"ballot\":\"beta-new\"}"),
        ],
    )
    .await
}

struct BallotRepositoryHarness {
    tenant_id: Uuid,
    election_event_id: Uuid,
    area_id: Uuid,
    election_id: Uuid,
}

#[derive(Clone, Copy)]
struct VoteSeed<'a> {
    voter_id_string: &'a str,
    content: &'a str,
    created_at: &'a str,
}

impl<'a> VoteSeed<'a> {
    const fn new(voter_id_string: &'a str, content: &'a str, created_at: &'a str) -> Self {
        Self {
            voter_id_string,
            content,
            created_at,
        }
    }
}

impl BallotRepositoryHarness {
    async fn new() -> Result<Self> {
        Ok(Self {
            tenant_id: Uuid::new_v4(),
            election_event_id: Uuid::new_v4(),
            area_id: Uuid::new_v4(),
            election_id: Uuid::new_v4(),
        })
    }

    async fn seed_votes(&self, transaction: &Transaction<'_>, votes: &[VoteSeed<'_>]) -> Result<()> {
        self.seed_reference_rows(transaction).await?;

        for vote in votes {
            insert_cast_vote(
                transaction,
                self.tenant_id,
                self.election_event_id,
                self.area_id,
                self.election_id,
                vote.voter_id_string,
                vote.content,
                timestamp(vote.created_at)?,
            )
            .await?;
        }

        Ok(())
    }

    async fn seed_reference_rows(&self, transaction: &Transaction<'_>) -> Result<()> {
        transaction
            .execute(
                r#"
                INSERT INTO sequent_backend.tenant (id, slug)
                VALUES ($1, $2)
                "#,
                &[&self.tenant_id, &"tenant-test"],
            )
            .await?;

        transaction
            .execute(
                r#"
                INSERT INTO sequent_backend.election_event (
                    id,
                    tenant_id,
                    encryption_protocol
                )
                VALUES ($1, $2, $3)
                "#,
                &[
                    &self.election_event_id,
                    &self.tenant_id,
                    &"test-protocol",
                ],
            )
            .await?;

        transaction
            .execute(
                r#"
                INSERT INTO sequent_backend.election (
                    id,
                    tenant_id,
                    election_event_id
                )
                VALUES ($1, $2, $3)
                "#,
                &[&self.election_id, &self.tenant_id, &self.election_event_id],
            )
            .await?;

        transaction
            .execute(
                r#"
                INSERT INTO sequent_backend.area (
                    id,
                    tenant_id,
                    election_event_id
                )
                VALUES ($1, $2, $3)
                "#,
                &[&self.area_id, &self.tenant_id, &self.election_event_id],
            )
            .await?;

        Ok(())
    }

    async fn export_rows(&self, transaction: &Transaction<'_>) -> Result<Vec<(String, String)>> {
        let repository = HasuraBallotRepository::new(&transaction);
        let output_file = NamedTempFile::new()?;

        repository
            .export_area_ballots(
                &self.tenant_id.to_string(),
                &self.election_event_id.to_string(),
                &self.area_id.to_string(),
                &self.election_id.to_string(),
                output_file.path(),
            )
            .await?;

        let mut rows = read_exported_rows(output_file.path())?;
        rows.sort();

        Ok(rows)
    }
}

async fn assert_export_case(
    votes: &[VoteSeed<'_>],
    expected_rows: &[(String, String)],
) -> Result<()> {
    let harness = BallotRepositoryHarness::new().await?;
    let pool = shared_pool().await?;
    let mut client = pool.get().await?;
    let transaction = client.transaction().await?;

    harness.seed_votes(&transaction, votes).await?;

    assert_eq!(harness.export_rows(&transaction).await?, expected_rows);

    transaction.rollback().await?;

    Ok(())
}

fn row(voter_id_string: &str, content: &str) -> (String, String) {
    (voter_id_string.to_string(), content.to_string())
}

async fn insert_cast_vote(
    transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    area_id: Uuid,
    election_id: Uuid,
    voter_id_string: &str,
    content: &str,
    created_at: DateTime<Utc>,
) -> Result<()> {
    transaction
        .execute(
            r#"
            INSERT INTO sequent_backend.cast_vote (
                tenant_id,
                election_event_id,
                election_id,
                area_id,
                voter_id_string,
                content,
                created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            &[
                &tenant_id,
                &election_event_id,
                &election_id,
                &area_id,
                &voter_id_string,
                &content,
                &created_at,
            ],
        )
        .await?;

    Ok(())
}

fn read_exported_rows(path: &std::path::Path) -> Result<Vec<(String, String)>> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_path(path)?;

    reader
        .records()
        .map(|record| {
            let record = record?;
            Ok((record[0].to_string(), record[1].to_string()))
        })
        .collect()
}

fn timestamp(value: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(value)?.with_timezone(&Utc))
}