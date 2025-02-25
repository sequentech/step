// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::candidate::insert_candidates;
use crate::postgres::contest::export_contests;
use crate::services::tasks_execution::*;
use crate::{
    postgres::document::get_document,
    services::{database::get_hasura_pool, documents::get_document_as_temp_file},
};
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Client as DbClient;
use encoding_rs::WINDOWS_1252;
use encoding_rs_io::DecodeReaderBytesBuilder;
use sequent_core::ballot::ContestPresentation;
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::types::hasura::core::Contest;
use sequent_core::types::hasura::core::{Candidate, TasksExecution};
use sequent_core::util::integrity_check::{integrity_check, HashFileVerifyError};
use std::io::BufReader;
use std::io::Seek;
use tracing::{event, info, instrument, Level};
use uuid::Uuid;

#[instrument(ret)]
fn get_political_party_extension(political_party: &str) -> String {
    // Mapping of numbers to political parties
    let party_map = vec![
        ("1", "\\N"),
        ("2", "AKBAYAN"),
        ("3", "AKSYON"),
        ("4", "ABC"),
        ("5", "ANAD"),
        ("6", "ANG KAPATIRAN"),
        ("7", "ANAKPAWIS"),
        ("8", "APP"),
        ("9", "BAGUMBAYAN"),
        ("10", "BP"),
        ("11", "BAYAN MUNA"),
        ("12", "BIGKIS"),
        ("13", "BUHAY"),
        ("14", "BUKLOD"),
        ("15", "CDP"),
        ("16", "DPP"),
        ("17", "FFP"),
        ("18", "\\N"),
        ("19", "KAMPI"),
        ("20", "KASOSYO"),
        ("21", "KDP"),
        ("22", "KATIPUNAN"),
        ("23", "BAGO"),
        ("24", "KBL"),
        ("25", "LDP"),
        ("26", "KKK"),
        ("27", "LAKAS CMD"),
        ("28", "LP"),
        ("29", "NP"),
        ("30", "NAD"),
        ("31", "NUP"),
        ("32", "NPC"),
        ("33", "OSME—A"),
        ("34", "PDP LABAN"),
        ("35", "PDSP"),
        ("36", "PLM"),
        ("37", "PNP"),
        ("38", "PDR"),
        ("39", "PRP"),
        ("40", "PGRP"),
        ("41", "PMP"),
        ("42", "\\N"),
        ("43", "SJS"),
        ("44", "\\N"),
        ("45", "UNA"),
        ("46", "WPP"),
        ("47", "KM NGAYON NA"),
        ("48", "SBP"),
        ("49", "ALL FISH"),
        ("50", "ACP"),
        ("51", "1AAA PARTY"),
        ("52", "GP"),
        ("53", "HNP"),
        ("54", "1STP"),
        ("55", "BISDAK"),
        ("56", "AI"),
        ("57", "APP"),
        ("58", "AAB"),
        ("59", "AKB"),
        ("60", "AKMA-PTM"),
        ("61", "ALAYON"),
        ("62", "ABP"),
        ("63", "BHW"),
        ("64", "\\N"),
        ("65", "BIAG"),
        ("66", "DA"),
        ("67", "DAMGO"),
        ("68", "EDSA"),
        ("69", "IPP"),
        ("70", "KABAKA"),
        ("71", "KUSGANO"),
        ("72", "METRO"),
        ("73", "\\N"),
        ("74", "\\N"),
        ("75", "PBB"),
        ("76", "MUSHAWARA"),
        ("77", "PARTIDO"),
        ("78", "BISAYA"),
        ("79", "\\N"),
        ("80", "\\N"),
        ("81", "\\N"),
        ("82", "1- ANG EDUKASYON"),
        ("83", "UNIDO"),
        ("84", "UBJP"),
        ("85", "UGP"),
        ("86", "\\N"),
        ("87", "AIM PARTY"),
        ("88", "AZaP"),
        ("89", "\\N"),
        ("90", "\\N"),
        ("91", "AMM"),
        ("92", "ATUN"),
        ("93", "BAKUD"),
        ("94", "BILEG"),
        ("95", "\\N"),
        ("96", "\\N"),
        ("97", "BPP"),
        ("98", "\\N"),
        ("99", "CATAPAT"),
        ("100", "CARD"),
        ("101", "CMIP"),
        ("102", "CCA PARTY"),
        ("103", "CORAL"),
        ("104", "LIHOK COTABATO"),
        ("105", "ALYANSA"),
        ("106", "DILC"),
        ("107", "\\N"),
        ("108", "\\N"),
        ("109", "HUGPONG"),
        ("110", "\\N"),
        ("111", "INA"),
        ("112", "LAPIANG K"),
        ("113", "KAMBILAN"),
        ("114", "KABATAK"),
        ("115", "KATIG-UBAN"),
        ("116", "KABACA"),
        ("117", "KSN"),
        ("118", "KDT"),
        ("119", "KUSUG"),
        ("120", "KUSGAN"),
        ("121", "KB"),
        ("122", "KDO"),
        ("123", "\\N"),
        ("124", "\\N"),
        ("125", "LAPIAN"),
        ("126", "BALANE"),
        ("127", "\\N"),
        ("128", "LINGKOD -TAGUIG PARTY"),
        ("129", "\\N"),
        ("130", "MRP"),
        ("131", "\\N"),
        ("132", "MMM"),
        ("133", "PARTIDO SANDUGO"),
        ("134", "\\N"),
        ("135", "MRP"),
        ("136", "NKP"),
        ("137", "\\N"),
        ("138", "\\N"),
        ("139", "\\N"),
        ("140", "\\N"),
        ("141", "\\N"),
        ("142", "PANDAYAN PARTY"),
        ("143", "\\N"),
        ("144", "PAK"),
        ("145", "BALIKATAN"),
        ("146", "\\N"),
        ("147", "\\N"),
        ("148", "\\N"),
        ("149", "\\N"),
        ("150", "\\N"),
        ("151", "PM"),
        ("152", "\\N"),
        ("153", "\\N"),
        ("154", "PAMBATANGUE—O"),
        ("155", "\\N"),
        ("156", "TAGUIG-PATEROS ACTION TEAM"),
        ("157", "PPP"),
        ("158", "PADER"),
        ("159", "PCM"),
        ("160", "PEOPLE'S ELA"),
        ("161", "PCNP"),
        ("162", "PINATUBO PARTY"),
        ("163", "\\N"),
        ("164", "\\N"),
        ("165", "\\N"),
        ("166", "\\N"),
        ("167", "SAMAHNA"),
        ("168", "SARRO"),
        ("169", "SPP"),
        ("170", "SZP"),
        ("171", "\\N"),
        ("172", "\\N"),
        ("173", "UGYON CAPIZ"),
        ("174", "\\N"),
        ("175", "UAM"),
        ("176", "PARTIDO NG PAGBABAGO"),
        ("177", "\\N"),
        ("178", "UNA"),
        ("179", "LGBT"),
        ("180", "IND"),
        ("181", "PDDS"),
        ("182", "PFP"),
        ("183", "WPP"),
        ("184", "UMP"),
        ("185", "MKBYN"),
        ("186", "MKMAZA"),
        ("187", "PAZ"),
        ("188", "KB"),
    ];

    // Convert the vector to a HashMap for efficient lookup
    let party_map: std::collections::HashMap<&str, &str> = party_map.into_iter().collect();

    // Check if the input or the mapped value is "\N"
    match party_map.get(political_party) {
        Some(&party_name) if party_name != "\\N" && political_party != "\\N" => {
            party_name.to_string()
        }
        _ => "IND".to_string(),
    }
}

#[instrument(ret, err, skip(contests))]
fn get_contest_from_postcode(contests: &Vec<Contest>, postcode: &str) -> Result<Option<String>> {
    // Mapping of postcodes to contest names
    let contest_map = vec![
        ("1", "PRESIDENT"),
        ("2", "VICE-PRESIDENT"),
        ("3", "SENATOR"),
        ("4", "PROVINCIAL GOVERNOR"),
        ("5", "PROVINCIAL VICE-GOVERNOR"),
        ("6", "MEMBER"),
        ("7", "MEMBER"),
        ("8", "MAYOR"),
        ("9", "VICE-MAYOR"),
        ("10", "COUNCILOR"),
        ("11", "PARTY LIST"),
        ("12", "REGIONAL GOVERNOR"),
        ("13", "REGIONAL VICE-GOVERNOR"),
        ("14", "MEMBER"),
        ("15", "PUNONG BARANGAY"),
        ("16", "MEMBER"),
        ("17", "CHAIRPERSON"),
        ("18", "MEMBER"),
    ];

    // Convert the vector to a HashMap for efficient lookup
    let contest_map: std::collections::HashMap<&str, &str> = contest_map.into_iter().collect();

    // Get the contest name from the map
    if let Some(&contest_name) = contest_map.get(postcode) {
        // Find the contest with the matching alias
        for contest in contests {
            if let Some(alias) = contest.alias.clone() {
                if alias == contest_name.to_string() {
                    return Ok(Some(contest.id.clone()));
                }
            }
            if let Some(presentation) = contest.presentation.clone() {
                let contest_presentation: ContestPresentation = deserialize_value(presentation)?;
                if let Some(i18n) = contest_presentation.i18n.clone() {
                    if let Some(en) = i18n.get("en") {
                        if let Some(en_alias_opt) = en.get("alias").clone() {
                            if en_alias_opt.clone().unwrap_or("".to_string())
                                == contest_name.to_string()
                            {
                                return Ok(Some(contest.id.clone()));
                            }
                        }
                    }
                }
            }
        }
    }

    // If no match found, return the first contest by default
    Ok(None)
}

#[instrument(err)]
pub async fn import_candidates_task(
    tenant_id: String,
    election_event_id: String,
    document_id: String,
    task_execution: TasksExecution,
    sha256: Option<String>,
) -> Result<()> {
    let mut hasura_db_client: DbClient = match get_hasura_pool().await.get().await {
        Ok(client) => client,
        Err(err) => {
            update_fail(&task_execution, "Failed to get Hasura DB pool").await?;
            return Err(anyhow!("Error getting Hasura DB pool: {}", err));
        }
    };

    let hasura_transaction = match hasura_db_client.transaction().await {
        Ok(transaction) => transaction,
        Err(err) => {
            update_fail(&task_execution, "Failed to start Hasura transaction").await?;
            return Err(anyhow!("Error starting Hasura transaction: {err}"));
        }
    };

    let document = match get_document(&hasura_transaction, &tenant_id, None, &document_id).await {
        Ok(Some(document)) => document,
        Ok(None) => {
            update_fail(&task_execution, "Document not found").await?;
            return Err(anyhow!("Document not found"));
        }
        Err(err) => {
            update_fail(&task_execution, "Error obtaining the document").await?;
            return Err(anyhow!("Error obtaining the document: {err:?}"));
        }
    };

    let contests = match export_contests(&hasura_transaction, &tenant_id, &election_event_id).await
    {
        Ok(contests) => contests,
        Err(err) => {
            update_fail(&task_execution, "Document not found").await?;
            return Err(anyhow!("Error obtaining the contests: {err:?}"));
        }
    };

    let mut temp_file = match get_document_as_temp_file(&tenant_id, &document).await {
        Ok(temp_file) => temp_file,
        Err(err) => {
            update_fail(&task_execution, "Document not found").await?;
            return Err(anyhow!("Error obtaining the tmp document: {err:?}"));
        }
    };

    temp_file.rewind()?;

    match sha256 {
        Some(hash) if !hash.is_empty() => match integrity_check(&temp_file, hash) {
            Ok(_) => {
                info!("Hash verified !");
            }
            Err(HashFileVerifyError::HashMismatch(input_hash, gen_hash)) => {
                let err_str = format!("Failed to verify the integrity: Hash of voters file: {gen_hash} does not match with the input hash: {input_hash}");
                update_fail(&task_execution, &err_str).await?;
                return Err(anyhow!(err_str));
            }
            Err(err) => {
                let err_str = format!("Failed to verify the integrity: {err:?}");
                update_fail(&task_execution, &err_str).await?;
                return Err(anyhow!(err_str));
            }
        },
        _ => {
            info!("No hash provided, skipping integrity check");
        }
    }

    let reader = BufReader::new(temp_file.as_file());

    // Decode the file using the specified encoding
    let transcoded_reader = DecodeReaderBytesBuilder::new()
        .encoding(Some(WINDOWS_1252)) // Use WINDOWS_1252 for encoding conversion
        .build(reader);

    // Read the first line of the file to get the columns
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b',')
        .has_headers(false)
        .from_reader(transcoded_reader);

    let mut candidates: Vec<Candidate> = vec![];
    for result in rdr.records() {
        match result.with_context(|| "Error reading CSV record") {
            Ok(record) => {
                event!(Level::INFO, "result {:?}", record);
                let name_on_ballot = record.get(26).unwrap_or("Candidate").to_string();
                let political_party = record.get(7).unwrap_or("\\N").to_string();
                let postcode = record.get(2).unwrap_or("1").to_string();

                let ext = get_political_party_extension(&political_party);
                let contest_id_opt = get_contest_from_postcode(&contests, &postcode)?;
                let Some(contest_id) = contest_id_opt else {
                    continue;
                };
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
            Err(err) => {
                event!(Level::ERROR, "Error reading CSV record: {:?}", err);
                update_fail(&task_execution, "Error reading CSV record").await?;
                return Err(anyhow!("Error reading CSV record: {}", err));
            }
        }
    }

    match insert_candidates(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &candidates,
    )
    .await
    {
        Ok(_) => (),
        Err(err) => {
            update_fail(&task_execution, "Error inserting candidates to db").await?;
            return Err(anyhow!("Inserting candidates failed: {:?}", err));
        }
    }

    match hasura_transaction.commit().await {
        Ok(_) => (),
        Err(err) => {
            update_fail(&task_execution, "Error updating db").await?;
            return Err(anyhow!("Commit failed: {}", err));
        }
    };

    update_complete(&task_execution, Some(document_id.clone()))
        .await
        .context("Failed to update task execution status to COMPLETED")?;

    Ok(())
}
