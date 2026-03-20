// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::ceremonies::insert_ballots::ports::VoterRepository;
use crate::services::database::get_keycloak_pool;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use futures::TryStreamExt;
use sequent_core::types::keycloak::{AREA_ID_ATTR_NAME, AUTHORIZED_ELECTION_IDS_NAME};
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::{copy, AsyncWriteExt, BufWriter};
use tokio_util::io::StreamReader;
use tracing::{info, instrument};

const DELEGATE_TO_ATTR_NAME: &str = "delegate-vote-to";

/// Reads eligible voters from the Keycloak PostgreSQL database by streaming a COPY query into a
/// CSV file.
///
/// User-controlled values (`realm`, `area_id`, `election_alias`) are bound as `$1`/`$2`/`$3`
/// parameters in a parameterized `INSERT … SELECT` into a temporary table. The subsequent
/// `COPY … TO STDOUT` reads only from that table, so no user data is ever interpolated into SQL
/// text.
pub struct KeycloakVoterRepository;

#[async_trait]
impl VoterRepository for KeycloakVoterRepository {
    #[instrument(skip(self, output_file), err)]
    async fn find_eligible_voters(
        &self,
        realm: &str,
        area_id: &str,
        election_alias: &str,
        output_file: &PathBuf,
        delegated_voting_enabled: bool,
    ) -> Result<()> {
        let mut client = get_keycloak_pool()
            .await
            .get()
            .await
            .with_context(|| "Error acquiring keycloak connection pool")?;
        let transaction = client
            .transaction()
            .await
            .with_context(|| "Error acquiring keycloak transaction")?;

        // Create a temp table with the right schema for this run.
        let create_table_sql = if delegated_voting_enabled {
            "CREATE TEMP TABLE temp_eligible_voters (id TEXT, delegate_count BIGINT)"
        } else {
            "CREATE TEMP TABLE temp_eligible_voters (id TEXT)"
        };
        transaction
            .execute(create_table_sql, &[])
            .await
            .with_context(|| "Error creating temp voter table")?;

        // $1 = realm, $2 = area_id, $3 = election_alias.
        // Only internal constants (attribute key names) remain as SQL string literals — no
        // user-supplied value is ever interpolated into the query text.
        let insert_sql = if delegated_voting_enabled {
            format!(
                r#"
                INSERT INTO temp_eligible_voters
                SELECT
                    u.id,
                    (
                        SELECT COUNT(delegator.id)
                        FROM user_entity AS delegator
                        JOIN user_attribute AS ua_delegate
                            ON delegator.id = ua_delegate.user_id
                        WHERE
                            ua_delegate.name = '{DELEGATE_TO_ATTR_NAME}' AND
                            ua_delegate.value = u.username
                    ) AS delegate_count
                FROM user_entity AS u
                JOIN realm ra ON u.realm_id = ra.id
                LEFT JOIN user_attribute ua_area
                    ON u.id = ua_area.user_id AND ua_area.name = '{AREA_ID_ATTR_NAME}'
                LEFT JOIN user_attribute ua_elections
                    ON u.id = ua_elections.user_id AND ua_elections.name = '{AUTHORIZED_ELECTION_IDS_NAME}'
                WHERE
                    ra.name = $1 AND
                    u.enabled IS TRUE AND
                    ua_area.value = $2 AND
                    (ua_elections.value = $3 OR ua_elections.value IS NULL)
                GROUP BY u.id
                ORDER BY u.id
                "#
            )
        } else {
            format!(
                r#"
                INSERT INTO temp_eligible_voters
                SELECT u.id
                FROM user_entity AS u
                JOIN realm ra ON u.realm_id = ra.id
                LEFT JOIN user_attribute ua_area
                    ON u.id = ua_area.user_id AND ua_area.name = '{AREA_ID_ATTR_NAME}'
                LEFT JOIN user_attribute ua_elections
                    ON u.id = ua_elections.user_id AND ua_elections.name = '{AUTHORIZED_ELECTION_IDS_NAME}'
                WHERE
                    ra.name = $1 AND
                    u.enabled IS TRUE AND
                    ua_area.value = $2 AND
                    (ua_elections.value = $3 OR ua_elections.value IS NULL)
                GROUP BY u.id
                ORDER BY u.id
                "#
            )
        };

        let insert_stmt = transaction
            .prepare(&insert_sql)
            .await
            .with_context(|| "Error preparing voter insert statement")?;

        transaction
            .execute(&insert_stmt, &[&realm, &area_id, &election_alias])
            .await
            .with_context(|| "Error inserting eligible voters into temp table")?;

        // COPY from the temp table — no user data is present in this query string.
        let tokio_temp_file = File::create(output_file)
            .await
            .map_err(|err| anyhow!("Could not create/open temporary file for tokio: {err}"))?;

        let mut writer = BufWriter::new(tokio_temp_file);

        let reader = transaction
            .copy_out("COPY temp_eligible_voters TO STDOUT WITH (FORMAT CSV)")
            .await?;

        let adapt_pg_error_to_io_error = |pg_err: tokio_postgres::Error| {
            std::io::Error::new(std::io::ErrorKind::Other, pg_err.to_string())
        };
        let io_error_stream = reader.map_err(adapt_pg_error_to_io_error);

        let async_reader = StreamReader::new(io_error_stream);
        tokio::pin!(async_reader);

        let bytes_copied = copy(&mut async_reader, &mut writer).await?;

        info!("voters bytes_copied: {bytes_copied}");

        writer.flush().await?;

        Ok(())
    }
}
