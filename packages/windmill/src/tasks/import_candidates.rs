// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::candidate::insert_candidates;
use crate::postgres::contest::export_contests;
use crate::{
    postgres::document::get_document,
    services::{database::get_hasura_pool, documents::get_document_as_temp_file},
};
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Client as DbClient;
use sequent_core::types::hasura::core::Candidate;
use sequent_core::types::hasura::core::Contest;
use std::io::Seek;
use uuid::Uuid;

fn get_political_party_extension(political_party: &str) -> String {
    political_party.to_string()
}

fn get_contest_from_postcode(contests: &Vec<Contest>, postcode: &str) -> String {
    postcode.to_string()
}

pub async fn import_candidates_task(
    tenant_id: String,
    election_event_id: String,
    document_id: String,
) -> Result<()> {
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|err| anyhow!("Error getting hasura db pool: {err}"))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|err| anyhow!("Error starting hasura transaction: {err}"))?;

    let document = get_document(&hasura_transaction, &tenant_id, None, &document_id)
        .await
        .with_context(|| "Error obtaining the document")?
        .ok_or(anyhow!("document not found"))?;

    let contests = export_contests(&hasura_transaction, &tenant_id, &election_event_id).await?;

    let mut temp_file = get_document_as_temp_file(&tenant_id, &document).await?;
    temp_file.rewind()?;
    // Read the first line of the file to get the columns
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b',')
        .has_headers(false)
        .from_reader(temp_file);

    let mut candidates: Vec<Candidate> = vec![];
    for result in rdr.records() {
        let record = result.with_context(|| "Error reading CSV record")?;
        let name_on_ballot = record.get(26).unwrap_or("Candidate").to_string();
        let political_party = record.get(7).unwrap_or("\\N").to_string();
        let postcode = record.get(2).unwrap_or("1").to_string();

        let ext = get_political_party_extension(&political_party);
        let contest_id = get_contest_from_postcode(&contests, &postcode);

        let candidate = Candidate {
            id: Uuid::new_v4().to_string(),
            tenant_id: tenant_id.clone(),
            election_event_id: election_event_id.clone(),
            contest_id: Some(contest_id),
            created_at: None,
            last_updated_at: None,
            labels: None,
            annotations: None,
            name: Some(format!("{name_on_ballot} ({ext})")),
            alias: None,
            description: None,
            r#type: None,
            presentation: None,
            is_public: Some(true),
            image_document_id: None,
        };
        candidates.push(candidate);
    }
    insert_candidates(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &candidates,
    )
    .await?;

    let _commit = hasura_transaction
        .commit()
        .await
        .map_err(|e| anyhow!("Commit failed: {}", e));

    Ok(())
}
