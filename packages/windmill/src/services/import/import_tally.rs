use crate::types::documents::ETallyDocuments;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use std::fs::File;
use tempfile::NamedTempFile;
use tracing::{info, instrument};

#[instrument(err, skip(hasura_transaction, temp_file))]
async fn process_tally_event_results_file(
    hasura_transaction: &Transaction<'_>,
    temp_file: &NamedTempFile,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<()> {
    let file = File::open(temp_file)?;
    let separator = b',';

    let mut rdr = csv::Reader::from_reader(file);

    // let mut reports: Vec<_> = Vec::new();
    println!("rdr:: {:?}", &rdr);

    for result in rdr.records() {
        let record = result.map_err(|e| anyhow!("Error reading CSV record: {e:?}"))?;
        println!("record:: {:?}", &record);
    }
    Ok(())
}

#[instrument(err, skip(hasura_transaction, temp_file))]
pub async fn process_tally_file(
    hasura_transaction: &Transaction<'_>,
    temp_file: &NamedTempFile,
    file_name: String,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<()> {
    if file_name == ETallyDocuments::RESULTS_EVENT.to_file_name().to_string() {
        process_tally_event_results_file(
            hasura_transaction,
            temp_file,
            election_event_id,
            tenant_id,
        )
        .await?;
    }

    Ok(())
}
