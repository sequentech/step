// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::election::get_elections;
use crate::services::electoral_log_board::{
    get_election_board, get_event_board, get_protocol_manager_secret_path,
};
use crate::services::export::export_bulletin_boards::*;
use crate::services::import::import_users::HEADER_RE;
use crate::services::vault;
use anyhow::{anyhow, Context, Result};
use base64::engine::general_purpose;
use base64::Engine;
use csv::StringRecord;
use deadpool_postgres::Transaction;
use std::collections::HashMap;
use std::collections::HashSet;
use tempfile::NamedTempFile;
use tracing::{info, instrument};

#[instrument]
fn get_board_name_for_event_or_election(
    tenant_id: &str,
    election_event_id: &str,
    election_id: Option<String>,
    slug: &str,
) -> String {
    if let Some(election_id) = election_id.clone() {
        get_election_board(tenant_id, &election_id, slug)
    } else {
        get_event_board(tenant_id, election_event_id, slug)
    }
}

#[instrument(err, skip(replacement_map))]
pub async fn import_protocol_manager_keys(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    temp_file: NamedTempFile,
    replacement_map: HashMap<String, String>,
    boards: &[sequent_core::types::protocol_board::ProtocolBoard],
) -> Result<()> {
    let elections = get_elections(hasura_transaction, tenant_id, election_event_id).await?;
    let mut keys_map: HashMap<Option<String>, String> = HashMap::new();
    let mut restored_native = HashSet::new();
    let separator = b',';

    // Read the first line of the file to get the columns
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(separator)
        .from_reader(temp_file);

    let headers = match rdr.headers() {
        Ok(headers) => headers.clone(),
        Err(err) => {
            return Err(anyhow!("Error reading CSV headers from voters file: {err}"));
        }
    };

    // Validate headers
    info!("headers: {headers:?}");
    for header in headers.iter() {
        if !HEADER_RE.is_match(header) {
            return Err(anyhow!(
                "CSV Header contains characters not allowed: {header}"
            ));
        }
    }
    for result in rdr.records() {
        let record = match result {
            Ok(record) => record,
            Err(err) => {
                return Err(anyhow!("Error reading CSV record: {err}"));
            }
        };
        let fields: Vec<String> = record.iter().map(|val| val.to_string()).collect();

        if fields.len() < 2 {
            return Err(anyhow!(
                "Missing fields, required at least 2 but got {}",
                fields.len()
            ));
        }

        if let Some(source_name) = fields[0].strip_prefix("protocol:") {
            if let Some(remapped) = native_manager_board(source_name, &replacement_map, boards)? {
                let name = protocol_board::BoardName::parse(remapped)?;
                let manager = protocol_board::parse_manager_key(&fields[1])?;
                let board = boards
                    .iter()
                    .find(|board| board.name == remapped && board.parent_id.is_none())
                    .context("native manager key has no archived DKG board")?;
                protocol_board::SignedConfiguration::parse(&board.manager_message)?
                    .verify_manager_key(&manager)?;
                crate::services::protocol_board::save_manager_key(
                    hasura_transaction,
                    tenant_id,
                    election_event_id,
                    &name,
                    &manager,
                )
                .await?;
                restored_native.insert(remapped.to_owned());
            }
            continue;
        }
        let election_id = fields[0].clone();
        let new_election_id = if election_id.trim().len() > 0 {
            Some(
                replacement_map
                    .get(&election_id)
                    .ok_or(anyhow!("Can't find election id in replacement map"))?
                    .clone(),
            )
        } else {
            None
        };

        let value = fields[1].clone();
        keys_map.insert(new_election_id, value);
    }

    require_native_manager_keys(boards, &restored_native)?;

    // insert event protocol manager keys
    let slug = std::env::var("ENV_SLUG").with_context(|| "missing env var ENV_SLUG")?;
    if let Some(value) = keys_map.get(&None).cloned() {
        let board_name = get_event_board(tenant_id, election_event_id, &slug);
        let protocol_manager_key = get_protocol_manager_secret_path(&board_name);
        vault::save_secret(
            hasura_transaction,
            tenant_id,
            Some(election_event_id),
            &protocol_manager_key,
            &value,
        )
        .await
        .context("protocol manager secret not saved")?
    } else {
        return Err(anyhow!("Missing event protocol manager keys"));
    }

    // insert elections protocol managerkeys
    for election in elections {
        if let Some(value) = keys_map.get(&Some(election.id.clone())).cloned() {
            let board_name = get_election_board(tenant_id, &election.id, &slug);
            let protocol_manager_key = get_protocol_manager_secret_path(&board_name);
            vault::save_secret(
                hasura_transaction,
                tenant_id,
                Some(election_event_id),
                &protocol_manager_key,
                &value,
            )
            .await
            .context("protocol manager secret not saved")?
        } else {
            return Err(anyhow!(
                "Missing election protocol manager keys for election"
            ));
        }
    }

    Ok(())
}

fn native_manager_board<'a>(
    source_name: &str,
    replacement_map: &'a HashMap<String, String>,
    boards: &[sequent_core::types::protocol_board::ProtocolBoard],
) -> Result<Option<&'a str>> {
    // Earlier selective exports accidentally included every native manager
    // key even when board metadata was omitted. Leave those unused secrets out.
    if boards.is_empty() {
        return Ok(None);
    }
    let remapped = replacement_map
        .get(source_name)
        .context("missing native board name replacement")?;
    anyhow::ensure!(
        boards
            .iter()
            .any(|board| board.name == *remapped && board.parent_id.is_none()),
        "native manager key has no archived DKG board"
    );
    Ok(Some(remapped))
}

fn require_native_manager_keys(
    boards: &[sequent_core::types::protocol_board::ProtocolBoard],
    restored: &HashSet<String>,
) -> Result<()> {
    for board in boards.iter().filter(|board| board.parent_id.is_none()) {
        anyhow::ensure!(
            restored.contains(&board.name),
            "Missing native protocol manager key for DKG board {}",
            board.name
        );
    }
    Ok(())
}

#[cfg(test)]
mod archive_tests {
    use super::*;
    use sequent_core::types::protocol_board::ProtocolBoard;

    fn dkg() -> ProtocolBoard {
        serde_json::from_value(serde_json::json!({
            "id":"board", "tenant_id":"tenant", "election_event_id":"event",
            "keys_ceremony_id":"ceremony", "name":"dkg_new", "parent_id":null,
            "manager_message":[], "created_at":null, "tally_session_id":null,"batch":null
        }))
        .unwrap()
    }

    #[test]
    fn old_selective_archives_ignore_unneeded_native_private_keys() {
        assert_eq!(
            native_manager_board("dkg_original", &HashMap::new(), &[]).unwrap(),
            None
        );
    }

    #[test]
    fn every_archived_dkg_requires_its_private_manager_key() {
        let board = dkg();
        assert!(require_native_manager_keys(&[board.clone()], &HashSet::new()).is_err());
        assert!(require_native_manager_keys(
            &[board.clone()],
            &HashSet::from(["unrelated".into()])
        )
        .is_err());
        assert!(require_native_manager_keys(&[board], &HashSet::from(["dkg_new".into()])).is_ok());
        assert!(require_native_manager_keys(&[], &HashSet::new()).is_ok());
    }

    #[test]
    fn native_private_keys_require_their_exported_dkg_metadata() {
        let replacements = HashMap::from([("dkg_original".into(), "dkg_new".into())]);
        assert_eq!(
            native_manager_board("dkg_original", &replacements, &[dkg()]).unwrap(),
            Some("dkg_new")
        );
        assert!(native_manager_board("unlisted", &replacements, &[dkg()])
            .unwrap_err()
            .to_string()
            .contains("missing native board name replacement"));
        let mut child = dkg();
        child.parent_id = Some("parent".into());
        assert!(native_manager_board("dkg_original", &replacements, &[child]).is_err());
    }
}

/// Validate every canonical message against its archived immutable configuration
/// before publishing any of them under the remapped application board names.
pub async fn import_bulletin_boards(
    tenant_id: &str,
    election_event_id: &str,
    temp_file: NamedTempFile,
    replacement_map: HashMap<String, String>,
    boards: &[sequent_core::types::protocol_board::ProtocolBoard],
) -> Result<()> {
    let mut reader = csv::Reader::from_reader(temp_file);
    anyhow::ensure!(
        reader.headers()?.iter().eq(PROTOCOL_ARCHIVE_HEADERS),
        "unsupported protocol archive format"
    );
    let mut validated = Vec::new();
    let parents = boards
        .iter()
        .filter(|row| row.parent_id.is_none())
        .map(|row| {
            Ok((
                row.id.as_str(),
                protocol_board::SignedConfiguration::parse(&row.manager_message)?,
            ))
        })
        .collect::<Result<HashMap<_, _>>>()?;
    for record in reader.records() {
        let record = record?;
        anyhow::ensure!(record.len() == 2, "invalid protocol archive record");
        let source = record.get(0).context("missing board name")?;
        let name = replacement_map
            .get(source)
            .context("unknown archived board name")?;
        let row = boards
            .iter()
            .find(|row| row.name == *name)
            .context("missing archived board lineage")?;
        anyhow::ensure!(
            row.tenant_id == tenant_id && row.election_event_id == election_event_id,
            "board archive scope mismatch"
        );
        let cfg = parents
            .get(row.parent_id.as_deref().unwrap_or(&row.id))
            .context("missing DKG parent")?;
        let bytes = general_purpose::STANDARD
            .decode(record.get(1).context("missing canonical message")?)?;
        cfg.verify_message(&bytes)?;
        validated.push((protocol_board::BoardName::parse(name)?, bytes));
    }
    for (name, bytes) in validated {
        crate::services::protocol_board::board_handle(&name)?
            .import_message(&bytes)
            .await?;
    }
    Ok(())
}
