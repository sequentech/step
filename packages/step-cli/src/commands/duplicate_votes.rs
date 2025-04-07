// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::utils::keycloak::get_keyckloak_pool;
use crate::utils::read_config::load_external_config;
use anyhow::Result;
use clap::Args;
use serde_json::Value;
use windmill::services::database::get_hasura_pool;
use std::env;
use tokio_postgres::{Row, Transaction};
use uuid::Uuid;
use windmill::services::providers::transactions_provider::provide_hasura_transaction;
use deadpool_postgres::Client as DbClient;
#[derive(Args)]
#[command(about)]
pub struct DuplicateVotes {
    /// Working directory for input/output
    #[arg(long)]
    working_directory: String,

    #[arg(long)]
    num_votes: usize,
}

impl DuplicateVotes {
    /// Execute the rendering process
    pub fn run(&self) {
        let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
        match runtime.block_on(self.run_duplicate_votes(&self.working_directory, self.num_votes)) {
            Ok(_) => println!("Successfully duplicate vote"),
            Err(err) => eprintln!("Error! Failed to duplicate vote: {err:?}"),
        }
    }

    pub async fn run_duplicate_votes(
        &self,
        working_dir: &str,
        num_votes: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let config = load_external_config(working_dir)?;
        let realm_name = config.realm_name;

        let duplicate_votes_config = config.duplicate_votes;
        let row_id_to_clone = duplicate_votes_config.row_id_to_clone;

        let mut hasura_db_client: DbClient = get_hasura_pool()
    .await
    .get()
    .await
    .map_err(|e| anyhow::anyhow!("Error getting DB client: {:?}", e))?;

let hasura_transaction = hasura_db_client
    .transaction()
    .await
    .map_err(|e| anyhow::anyhow!("Error starting transaction: {:?}", e))?;

        let row = get_base_row(&hasura_transaction, row_id_to_clone).await?;
        let area_id = row.try_get::<_, Uuid>(3)?;
        let kc_client = get_keyckloak_pool()
            .await?
            .get()
            .await
            .map_err(|e| anyhow::anyhow!("Error getting hasura client: {}", e.to_string()))?;

            let keycloak_query = "\
            SELECT ue.id, ue.username, r.name AS realm_name \
            FROM user_entity AS ue \
            JOIN realm AS r ON ue.realm_id = r.id \
            JOIN user_attribute AS ua ON ue.id = ua.user_id \
            WHERE r.name = $1 \
              AND ua.name = $2 \
              AND ua.value = $3 \
            LIMIT $4 \
            OFFSET 0";

        let kc_rows = kc_client
            .query(keycloak_query, &[&realm_name,&"area-id", &area_id,  &(num_votes as i64)])
            .await?;
        let existing_user_ids: Vec<String> = kc_rows
            .iter()
            .filter_map(|row| row.get::<_, Option<String>>(0))
            .collect();
        println!("Number of existing user IDs::: {}", &existing_user_ids.len());


        insert_votes(&hasura_transaction, existing_user_ids.as_ref(), &row).await?;

        let _commit = hasura_transaction.commit().await .map_err(|e| anyhow::anyhow!("Error commiting hasura client: {}", e.to_string()))?;

        println!("Inserted {} duplicate votes.", existing_user_ids.len());
        Ok(())
    }
}

async fn get_base_row(hasura_transaction: &Transaction<'_>, row_id_to_clone: String) -> Result<Row> {
    let base_query = "\
    SELECT tenant_id, election_event_id, election_id, area_id, annotations, content, cast_ballot_signature, ballot_id \
        FROM sequent_backend.cast_vote WHERE id = $1";
    let base_row = hasura_transaction
        .query_opt(base_query, &[&Uuid::parse_str(&row_id_to_clone)?])
        .await
        .map_err(|e| {
            anyhow::anyhow!(
                "Error querying base row: {}",
                e.to_string()
            )
        })?;
        if(base_row.is_none()) {
            return Err(anyhow::anyhow!("No row found with ID: {}", row_id_to_clone));
        }
        let row = base_row.unwrap();
    Ok(row)
}

async fn insert_votes(
    hasura_transaction: &Transaction<'_>,
    existing_user_ids: &Vec<String>,
    row: &Row,
) -> Result<()> {
    let tenant_id = row.try_get::<_, Uuid>(0)?;
    let election_event_id = row.try_get::<_, Uuid>(1)?;
    let election_id = row.try_get::<_, Uuid>(2)?;
    let area_id = row.try_get::<_, Uuid>(3)?;
    let annotations: Value = row.get(4);
    let content: String = row.get::<_, &str>(5).to_string();
    let cast_ballot_signature: Vec<u8> = row.get(6);
    let ballot_id: String = row.get::<_, &str>(7).to_string();

    let batch_size = env::var("DEFAULT_SQL_BATCH_SIZE")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1000);

    let row_param_count = 9; // Each row has 9 parameters.

    for batch in existing_user_ids.chunks(batch_size) {
        let total_params = batch.len() * row_param_count;

        // Preallocate for efficiency
        let mut query = String::with_capacity(100 + total_params * 3);
        query.push_str("INSERT INTO sequent_backend.cast_vote (voter_id_string, election_id, tenant_id, area_id, annotations, content, cast_ballot_signature, election_event_id, ballot_id) VALUES ");

        let mut params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
            Vec::with_capacity(total_params);

        let mut placeholders = Vec::with_capacity(batch.len());

        for (i, uid) in batch.iter().enumerate() {
            let start = i * row_param_count + 1;
            let placeholder = (start..start + row_param_count)
                .map(|idx| format!("${}", idx))
                .collect::<Vec<_>>()
                .join(", ");

            placeholders.push(format!("({})", placeholder));

            // Push parameters
            params.push(uid);
            params.push(&election_id);
            params.push(&tenant_id);
            params.push(&area_id);
            params.push(&annotations);
            params.push(&content);
            params.push(&cast_ballot_signature);
            params.push(&election_event_id);
            params.push(&ballot_id);
        }

        query.push_str(&placeholders.join(", "));

        hasura_transaction.execute(query.as_str(), &params).await?;
    }

    Ok(())
}
