// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{ensure, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::types::hasura::core::BallotPublication;
use sequent_core::{ballot::BallotStyle, serialization::deserialize_with_path::deserialize_str};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use tempfile::NamedTempFile;
use tracing::instrument;
use uuid::Uuid;

use crate::services::ballot_styles::ballot_style::{ElectionEventConfig, EVENT_CONFIG_FILE_NAME};
use crate::services::documents::upload_and_return_public_event_document;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BallotDesign {
    ballot_publication_id: String,
    ballot_styles: Vec<BallotStyle>,
    /// Older archives contain only the publication ID and ballot EMLs.
    #[serde(default)]
    publication: Option<BallotPublication>,
}

fn remap_publications(
    data: &str,
    tenant: &str,
    event: &str,
    replacements: &HashMap<String, String>,
) -> Result<Vec<BallotDesign>> {
    let originals: Vec<BallotDesign> = deserialize_str(data)?;
    let mut replacements = replacements.clone();
    // These IDs do not appear in the event schema; allocate them once for this archive.
    for design in &originals {
        for id in std::iter::once(&design.ballot_publication_id)
            .chain(design.ballot_styles.iter().map(|style| &style.id))
        {
            Uuid::parse_str(id).context("invalid publication or style ID")?;
            replacements
                .entry(id.clone())
                .or_insert_with(|| Uuid::new_v4().to_string());
        }
    }
    fn remap(value: &mut JsonValue, replacements: &HashMap<String, String>) {
        match value {
            JsonValue::String(id) => {
                if let Some(replacement) = replacements.get(id) {
                    *id = replacement.clone();
                }
            }
            JsonValue::Array(values) => values
                .iter_mut()
                .for_each(|value| remap(value, replacements)),
            JsonValue::Object(values) => values
                .values_mut()
                .for_each(|value| remap(value, replacements)),
            _ => {}
        }
    }
    let mut value = serde_json::to_value(originals)?;
    remap(&mut value, &replacements);
    let mut designs: Vec<BallotDesign> = serde_json::from_value(value)?;
    for design in &mut designs {
        for style in &design.ballot_styles {
            ensure!(
                style.tenant_id == tenant && style.election_event_id == event,
                "publication style scope mismatch"
            );
            ensure!(
                replacements.values().any(|id| id == &style.election_id)
                    && replacements.values().any(|id| id == &style.area_id),
                "publication style references an election or area outside the imported bundle"
            );
        }
        let publication = design.publication.get_or_insert_with(|| BallotPublication {
            id: design.ballot_publication_id.clone(),
            tenant_id: tenant.into(),
            election_event_id: event.into(),
            labels: None,
            annotations: None,
            created_at: None,
            deleted_at: None,
            created_by_user_id: None,
            is_generated: Some(!design.ballot_styles.is_empty()),
            election_ids: Some(
                design
                    .ballot_styles
                    .iter()
                    .map(|style| style.election_id.clone())
                    .collect::<std::collections::BTreeSet<_>>()
                    .into_iter()
                    .collect(),
            ),
            published_at: None,
            election_id: None,
        });
        ensure!(
            publication.id == design.ballot_publication_id
                && publication.tenant_id == tenant
                && publication.election_event_id == event,
            "publication archive scope mismatch"
        );
        ensure!(
            publication.published_at.is_none() || publication.is_generated == Some(true),
            "published archive is missing generated content"
        );
        // Immutable files must be rebuilt under the new tenant/event/publication scope.
        if let Some(annotations) = publication
            .annotations
            .as_mut()
            .and_then(JsonValue::as_object_mut)
        {
            annotations.remove(crate::services::ballot_styles::publication_files::FILES_ANNOTATION);
        }
    }
    Ok(designs)
}

#[instrument(err, skip(replacement_map))]
pub async fn import_ballot_publications(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    temp_file: NamedTempFile,
    replacement_map: HashMap<String, String>,
) -> Result<()> {
    let mut file = File::open(temp_file)?;
    let mut data_str = String::new();
    file.read_to_string(&mut data_str)?;
    let designs = remap_publications(&data_str, tenant_id, election_event_id, &replacement_map)?;
    for design in designs {
        let publication = design
            .publication
            .context("missing restored publication metadata")?;
        crate::postgres::ballot_publication::restore_ballot_publication(
            hasura_transaction,
            &publication,
        )
        .await?;
        for style in design.ballot_styles {
            crate::postgres::ballot_style::insert_ballot_style(
                hasura_transaction,
                &style.id,
                tenant_id,
                election_event_id,
                &style.election_id,
                &style.area_id,
                Some(serde_json::to_string(&style)?),
                None,
                &publication.id,
            )
            .await?;
        }
        if publication.is_generated == Some(true) {
            crate::services::ballot_styles::publication_files::prepare_publication_files(
                hasura_transaction,
                tenant_id,
                election_event_id,
                &publication.id,
            )
            .await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod archive_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn native_publications_remap_platform_ids_and_preserve_the_signed_key_domain() {
        let ids: Vec<_> = (0..8).map(|_| Uuid::new_v4().to_string()).collect();
        let replacements: HashMap<_, _> = ids[..5]
            .iter()
            .map(|id| (id.clone(), Uuid::new_v4().to_string()))
            .collect();
        let archive = json!([{
            "ballot_publication_id": ids[5],
            "publication": {
                "id": ids[5], "tenant_id": ids[0], "election_event_id": ids[1],
                "is_generated": true, "published_at": "2026-01-01T00:00:00Z",
                "election_ids": [ids[2]],
                "annotations": {"ballot_files_v1": "old storage", "label": "preserve"}
            },
            "ballot_styles": [{
                "id": ids[6], "tenant_id": ids[0], "election_event_id": ids[1],
                "election_id": ids[2], "area_id": ids[3], "contests": [],
                "public_key": {"public_key": "native-key", "is_demo": false,
                    "keys_ceremony_id": ids[4], "configuration_id": "9876543210"}
            }]
        }]);
        let restored = remap_publications(
            &archive.to_string(),
            &replacements[&ids[0]],
            &replacements[&ids[1]],
            &replacements,
        )
        .unwrap();
        let [design] = <[_; 1]>::try_from(restored).unwrap();
        let publication = design.publication.unwrap();
        assert_ne!(publication.id, ids[5]);
        assert!(Uuid::parse_str(&publication.id).is_ok());
        assert_eq!(
            publication.election_ids,
            Some(vec![replacements[&ids[2]].clone()])
        );
        assert!(publication.published_at.is_some());
        assert_eq!(
            publication.annotations.as_ref().unwrap()["label"],
            "preserve"
        );
        assert!(publication
            .annotations
            .as_ref()
            .unwrap()
            .get("ballot_files_v1")
            .is_none());
        let [style] = <[_; 1]>::try_from(design.ballot_styles).unwrap();
        assert_ne!(style.id, ids[6]);
        assert_eq!(style.tenant_id, replacements[&ids[0]]);
        assert_eq!(style.election_event_id, replacements[&ids[1]]);
        assert_eq!(style.election_id, replacements[&ids[2]]);
        assert_eq!(style.area_id, replacements[&ids[3]]);
        let key = style.public_key.unwrap();
        assert_eq!(key.public_key, "native-key");
        assert_eq!(key.keys_ceremony_id, Some(replacements[&ids[4]].clone()));
        assert_eq!(key.configuration_id.as_deref(), Some("9876543210"));

        let mut legacy = archive.clone();
        legacy[0].as_object_mut().unwrap().remove("publication");
        assert!(remap_publications(
            &legacy.to_string(),
            &replacements[&ids[0]],
            &replacements[&ids[1]],
            &replacements
        )
        .unwrap()[0]
            .publication
            .is_some());
        let mut foreign = archive.clone();
        foreign[0]["ballot_styles"][0]["election_event_id"] = json!(ids[7]);
        let error = remap_publications(
            &foreign.to_string(),
            &replacements[&ids[0]],
            &replacements[&ids[1]],
            &replacements,
        )
        .unwrap_err();
        assert!(error
            .to_string()
            .contains("publication style scope mismatch"));
        let mut foreign_area = archive.clone();
        foreign_area[0]["ballot_styles"][0]["area_id"] = json!(ids[7]);
        let error = remap_publications(
            &foreign_area.to_string(),
            &replacements[&ids[0]],
            &replacements[&ids[1]],
            &replacements,
        )
        .unwrap_err();
        assert!(error.to_string().contains("outside the imported bundle"));
    }
}

/// Imports the election event config file,
/// This file contains the election event presentation and is created during publication.
#[instrument(err, skip(replacement_map))]
pub async fn import_election_event_config_file(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    temp_file: NamedTempFile,
    replacement_map: HashMap<String, String>,
) -> Result<()> {
    let mut file = File::open(temp_file)?;
    let mut data_str = String::new();
    file.read_to_string(&mut data_str)?;
    let original_data: ElectionEventConfig = deserialize_str(&data_str)?;

    let new_id = Uuid::new_v4();

    let new_election_event_config = ElectionEventConfig {
        id: new_id.to_string(),
        tenant_id: tenant_id.to_string(),
        election_event_id: election_event_id.to_string(),
        election_event_presentation: original_data.election_event_presentation,
    };

    let config_json = serde_json::to_string(&new_election_event_config)?;

    // Write to temp file
    let mut temp_file = NamedTempFile::new()?;
    temp_file.write_all(config_json.as_bytes())?;

    let temp_file_path = temp_file.path().to_string_lossy().to_string();
    let file_size = config_json.len() as u64;

    // Upload to S3 public bucket with election_event_id in path
    let _document = upload_and_return_public_event_document(
        hasura_transaction,
        &temp_file_path,
        file_size,
        "application/json",
        tenant_id,
        election_event_id,
        EVENT_CONFIG_FILE_NAME,
        Some(new_id.to_string()),
    )
    .await?;

    Ok(())
}
