// SPDX-FileCopyrightText: 2024 Sequent Tech <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::utils::{
    get_opt_date, get_opt_json_value_item, get_replaced_id, get_string_or_null_item, process_uuids,
};
use crate::postgres::ballot_publication::insert_many_ballot_publications;
use crate::types::documents::EDocuments;
use anyhow::{anyhow, Context, Result};
use csv::StringRecord;
use deadpool_postgres::Transaction;
use regex::Regex;
use sequent_core::services::date::ISO8601;
use sequent_core::types::hasura::core::BallotPublication;
use sequent_core::{ballot::BallotStyle, serialization::deserialize_with_path::deserialize_str};
use std::collections::HashMap;
use std::fs::File;
use tempfile::NamedTempFile;
use tracing::{info, instrument};

#[instrument(err, skip_all)]
async fn process_ballot_publications_file(
    hasura_transaction: &Transaction<'_>,
    temp_file: &NamedTempFile,
    tenant_id: &str,
    election_event_id: &str,
    replacement_map: HashMap<String, String>,
) -> Result<()> {
    let file = File::open(temp_file)?;
    let mut rdr = csv::Reader::from_reader(file);

    let mut publications: Vec<BallotPublication> = Vec::new();

    for result in rdr.records() {
        let record = result.map_err(|e| anyhow!("Error reading CSV record: {e:?}"))?;

        let ballot_publication_id_ = get_replaced_id(&record, 0, &replacement_map).await?;

        let labels = get_opt_json_value_item(&record, 3).await?;
        let annotations = get_opt_json_value_item(&record, 4).await?;
        let created_at = get_opt_date(&record, 5).await?;
        let deleted_at = get_opt_date(&record, 6).await?;
        let created_by_user_id = get_string_or_null_item(&record, 7).await?;
        let is_generated = record.get(8).map(|s| s.parse::<bool>().ok()).flatten();
        let election_ids = process_uuids(record.get(9), replacement_map.clone()).await?;
        let published_at = get_opt_date(&record, 10).await?;
        let election_id = get_string_or_null_item(&record, 11).await?;
        let new_election_id = match election_id {
            Some(election_id) => Some(get_replaced_id(&record, 11, &replacement_map).await?),
            None => None,
        };

        let ballot_publication = BallotPublication {
            id: ballot_publication_id_,
            tenant_id: tenant_id.to_string(),
            election_event_id: election_event_id.to_string(),
            labels,
            annotations,
            created_at,
            deleted_at,
            created_by_user_id,
            is_generated,
            election_ids,
            published_at,
            election_id: new_election_id,
        };
        publications.push(ballot_publication);
    }

    let _ = insert_many_ballot_publications(hasura_transaction, publications)
        .await
        .map_err(|err| anyhow!("Error at insert_many_ballot_publications {:?}", err))?;

    Ok(())
}

#[instrument(err, skip(replacement_map))]
pub async fn import_ballot_publications(
    hasura_transaction: &Transaction<'_>,
    temp_file: &NamedTempFile,
    file_name: String,
    tenant_id: &str,
    election_event_id: &str,
    replacement_map: HashMap<String, String>,
) -> Result<()> {
    if file_name == EDocuments::PUBLICATIONS.to_file_name().to_string() {
        process_ballot_publications_file(
            hasura_transaction,
            temp_file,
            tenant_id,
            election_event_id,
            replacement_map.clone(),
        )
        .await?;
    }
    Ok(())
}
