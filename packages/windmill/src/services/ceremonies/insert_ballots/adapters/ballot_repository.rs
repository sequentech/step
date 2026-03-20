// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::ceremonies::insert_ballots::ports::BallotRepository;
use crate::services::database::get_hasura_pool;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use futures::TryStreamExt;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::{copy, AsyncWriteExt, BufWriter};
use tokio_util::io::StreamReader;
use tracing::{debug, info, instrument};

/// Reads area ballots from the Hasura (PostgreSQL) database by streaming a COPY query into a
/// CSV file.
pub struct PostgresBallotRepository;

#[async_trait]
impl BallotRepository for PostgresBallotRepository {
    #[instrument(skip(self, output_file), err)]
    async fn find_area_ballots(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        area_id: &str,
        election_id: &str,
        output_file: &PathBuf,
    ) -> Result<()> {
        let mut client = get_hasura_pool()
            .await
            .get()
            .await
            .with_context(|| "Error acquiring hasura connection pool")?;
        let transaction = client
            .transaction()
            .await
            .with_context(|| "Error acquiring hasura transaction")?;

        // COPY does not support parameters so we have to add them using format
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
            .map_err(|err| anyhow!("Could not create/open temporary file for tokio: {err}"))?;

        let copy_out_query = format!("COPY ({}) TO STDOUT WITH (FORMAT CSV)", areas_statement);
        let mut writer = BufWriter::new(tokio_temp_file);

        debug!("copy_out_query: {copy_out_query}");

        let reader = transaction.copy_out(&copy_out_query).await?;

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
}
