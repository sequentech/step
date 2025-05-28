// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::utils::keycloak::get_keyckloak_pool;
use crate::utils::read_config::load_external_config;
use anyhow::Result;
use clap::Args;
use serde_json::Value;
use std::env;
use tokio_postgres::Transaction;
use uuid::Uuid;
use windmill::services::providers::transactions_provider::provide_hasura_transaction;
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

        let kc_client = get_keyckloak_pool()
            .await?
            .get()
            .await
            .map_err(|e| anyhow::anyhow!("Error getting hasura client: {}", e.to_string()))?;

        let keycloak_query = "\
            SELECT ue.id as id, ua.value as area_id FROM user_entity AS ue \
            JOIN realm AS r ON ue.realm_id = r.id \
            JOIN user_attribute as ua ON ua.user_id = ue.id \
            WHERE r.name = $1 \
            AND ua.name = 'area-id' \
            LIMIT $2 OFFSET 0";

        // SELECT ue.id, ua."value" FROM user_entity AS ue 
        // JOIN realm AS r ON ue.realm_id = r.id 
        // JOIN user_attribute as ua ON ua.user_id = ue.id
        // WHERE r.name = 'tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5-event-8d112040-cc69-49ae-a09b-98ac7240c603'
        // AND ua.name = 'area-id'
        // LIMIT 1000 
        // OFFSET 0;

        let kc_rows = kc_client
            .query(keycloak_query, &[&realm_name, &(num_votes as i64)])
            .await?;
        let existing_user_ids: Vec<(String, Uuid)> = kc_rows
            .iter()
            .map(|row| -> Result<(String, Uuid)> {
                let id = row.try_get("id")?;
                let area_id = Uuid::parse_str(row.try_get("area_id")?)?;
                Ok((id, area_id))
            })
            .collect::<Result<Vec<(String, Uuid)>>>()?;
        println!("Number of existing user IDs::: {}", existing_user_ids.len());

        provide_hasura_transaction(|hasura_transaction| {
            let existing_user_ids = existing_user_ids.clone();
            let row_id_to_clone = row_id_to_clone.clone();

            Box::pin(async move {
                insert_votes(hasura_transaction, existing_user_ids, row_id_to_clone).await
            })
        })
        .await?;

        println!("Inserted {} duplicate votes.", &existing_user_ids.len());
        Ok(())
    }
}

async fn insert_votes(
    hasura_transaction: &Transaction<'_>,
    existing_user_ids: Vec<(String, Uuid)>,
    row_id_to_clone: String,
) -> Result<()> {
    let base_query = "\
    SELECT tenant_id, election_event_id, election_id, area_id, annotations, content, cast_ballot_signature, ballot_id \
        FROM sequent_backend.cast_vote WHERE id = $1";
    let base_row = hasura_transaction
        .query_opt(base_query, &[&Uuid::parse_str(&row_id_to_clone)?])
        .await?;
    if base_row.is_none() {
        println!("No row found to clone.");
        return Ok(());
    }
    let row = base_row.unwrap();
    let tenant_id = row.try_get::<_, Uuid>(0)?;
    let election_event_id = row.try_get::<_, Uuid>(1)?;
    let election_id = row.try_get::<_, Uuid>(2)?;
    // let area_id = row.try_get::<_, Uuid>(3)?;
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

        for (i, (uid, area_id)) in batch.iter().enumerate() {
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
            params.push(area_id);
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
