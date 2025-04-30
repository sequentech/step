// SPDX-FileCopyrightText: 2024 Sequent Tech <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::utils::{
    get_opt_base64_vec_item, get_opt_date, get_opt_json_value_item, get_replaced_id,
    get_string_or_null_item, process_uuids,
};
use crate::postgres::ballot_publication::insert_many_ballot_publications;
use crate::postgres::ballot_style::insert_many_ballot_styles;
use crate::types::documents::EDocuments;
use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use sequent_core::types::hasura::core::{BallotPublication, BallotStyle};
use std::collections::HashMap;
use std::fs::File;
use tempfile::NamedTempFile;
use tracing::instrument;
use uuid::Uuid;

#[instrument(err)]
async fn process_ballot_publications_file(
    hasura_transaction: &Transaction<'_>,
    temp_file: &NamedTempFile,
    tenant_id: &str,
    election_event_id: &str,
    replacement_map: HashMap<String, String>,
    executer_id: String,
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
        let created_by_user_id = executer_id.clone(); // replace with current executer id (the importer)
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
            created_by_user_id: Some(created_by_user_id),
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

#[instrument(err, skip_all)]
async fn process_ballot_styles_file(
    hasura_transaction: &Transaction<'_>,
    temp_file: &NamedTempFile,
    tenant_id: &str,
    election_event_id: &str,
    replacement_map: HashMap<String, String>,
) -> Result<()> {
    let file = File::open(temp_file)?;
    let mut rdr = csv::Reader::from_reader(file);

    let mut ballot_styles: Vec<BallotStyle> = Vec::new();

    for result in rdr.records() {
        let record = result.map_err(|e| anyhow!("Error reading CSV record: {e:?}"))?;

        let election_id = get_replaced_id(&record, 2, &replacement_map).await?;
        let area_id = get_string_or_null_item(&record, 3).await?;
        let new_area_id = match area_id {
            Some(area_id) => Some(get_replaced_id(&record, 3, &replacement_map).await?),
            None => None,
        };
        let created_at = get_opt_date(&record, 4).await?;
        let last_updated_at = get_opt_date(&record, 5).await?;
        let labels = get_opt_json_value_item(&record, 6).await?;
        let annotations = get_opt_json_value_item(&record, 7).await?;

        let ballot_eml = get_string_or_null_item(&record, 8).await?;
        let ballot_signature = get_opt_base64_vec_item(&record, 9).await?;
        let status = get_string_or_null_item(&record, 10).await?;
        let deleted_at = get_opt_date(&record, 12).await?;
        let ballot_publication_id = get_replaced_id(&record, 13, &replacement_map).await?;

        let ballot_style = BallotStyle {
            id: Uuid::new_v4().to_string(),
            tenant_id: tenant_id.to_string(),
            election_event_id: election_event_id.to_string(),
            area_id: new_area_id,
            created_at,
            last_updated_at,
            labels,
            annotations,
            ballot_eml,
            ballot_signature,
            status,
            deleted_at,
            ballot_publication_id,
            election_id,
        };
        ballot_styles.push(ballot_style);
    }
    let _ = insert_many_ballot_styles(hasura_transaction, ballot_styles)
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
    executer_id: String,
) -> Result<()> {
    if file_name == EDocuments::PUBLICATIONS.to_file_name().to_string() {
        process_ballot_publications_file(
            hasura_transaction,
            temp_file,
            tenant_id,
            election_event_id,
            replacement_map.clone(),
            executer_id,
        )
        .await?;
    } else if file_name == EDocuments::BALLOT_STYLE.to_file_name().to_string() {
        process_ballot_styles_file(
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
