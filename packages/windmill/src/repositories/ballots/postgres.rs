// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::database::PgConfig;
use crate::services::datafix::utils::{
    is_datafix_election_event_by_id, voted_via_not_internet_channel,
};
use crate::services::electoral_log::ElectoralLog;
use crate::services::sql_utils::escape_sql_literal;
use anyhow::{anyhow, Context, Result};
use chrono::NaiveDate;
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use futures::TryStreamExt;
use sequent_core::services::uuid_validation::parse_uuid_v4;
use sequent_core::types::keycloak::{User, VotesInfo};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use strand::signature::{StrandSignaturePk, StrandSignatureSk};
use tokio::fs::File;
use tokio::io::{copy, AsyncWriteExt, BufWriter};
use tokio_postgres::row::Row;
use tokio_util::io::StreamReader;
use tracing::{debug, info, instrument};
use uuid::Uuid;

use crate::repositories::ballots::BallotRepository;
use async_trait::async_trait;

/// Hasura-backed implementation of `BallotRepository`.
///
/// This adapter exports the ballots required for one contest batch into a CSV
/// artifact consumed later by the ballot processor.
pub struct HasuraBallotRepository<'a> {
    transaction: &'a Transaction<'a>,
}

impl<'a> HasuraBallotRepository<'a> {
    /// Creates a ballot repository bound to the provided Hasura transaction.
    pub fn new(transaction: &'a Transaction<'a>) -> Self {
        Self { transaction }
    }
}

#[async_trait]
impl BallotRepository for HasuraBallotRepository<'_> {
    async fn export_area_ballots(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        area_id: &str,
        election_id: &str,
        output_path: &Path,
    ) -> Result<()> {
        find_area_ballots(
            self.transaction,
            tenant_id,
            election_event_id,
            area_id,
            election_id,
            &output_path.to_path_buf(),
        )
        .await
    }
}

#[instrument(err)]
async fn find_area_ballots(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    area_id: &str,
    election_id: &str,
    output_file: &PathBuf,
) -> Result<()> {
    // COPY does not support parameters so we have to add them using format.
    // Validate as v4 UUIDs before interpolating into SQL.
    parse_uuid_v4(tenant_id)?;
    parse_uuid_v4(election_event_id)?;
    parse_uuid_v4(area_id)?;
    parse_uuid_v4(election_id)?;
    let tenant_id = escape_sql_literal(tenant_id);
    let election_event_id = escape_sql_literal(election_event_id);
    let area_id = escape_sql_literal(area_id);
    let election_id = escape_sql_literal(election_id);
    let areas_statement = format!(
        r#"
                    SELECT DISTINCT ON (election_id, voter_id_string)
                        voter_id_string,
                        content
                    FROM "sequent_backend".cast_vote
                    WHERE
                        tenant_id = '{tenant_id}' AND
                        election_event_id = '{election_event_id}' AND
                        area_id = '{area_id}' AND
                        election_id = '{election_id}'
                    ORDER BY election_id, voter_id_string, created_at DESC
                "#
    );

    let tokio_temp_file = File::create(output_file)
        .await
        .expect("Could not create/open temporary file for tokio");

    let copy_out_query = format!("COPY ({}) TO STDOUT WITH (FORMAT CSV)", areas_statement);
    let mut writer = BufWriter::new(tokio_temp_file);

    debug!("copy_out_query: {copy_out_query}");

    let reader = hasura_transaction.copy_out(&copy_out_query).await?;

    let adapt_pg_error_to_io_error = |pg_err: tokio_postgres::Error| {
        std::io::Error::new(std::io::ErrorKind::Other, pg_err.to_string())
    };
    let io_error_stream = reader.map_err(adapt_pg_error_to_io_error);

    let async_reader = StreamReader::new(io_error_stream);
    tokio::pin!(async_reader);

    let bytes_copied = copy(&mut async_reader, &mut writer).await?;

    info!("ballot bytes_copied: {bytes_copied}");

    writer.flush().await?;

    Ok(())
}
