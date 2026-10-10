// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Publication of an event imported from a signed configuration package:
//! each ballot style about to be published must be the design the package's
//! manifest approved. The digest is the one the Election Architect computed
//! before signing, by the same function, so an event edited after import is
//! refused here rather than published.

use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use electoral_log::messages::newtypes::{ConfigurationDesignDigest, PublishedConfiguration};
use sequent_core::ballot::BallotStyle;
use sequent_core::election_config::design::{
    ballot_design_digests, mismatches, DesignDigest, DesignKeys, DesignMismatch,
};
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use tracing::instrument;

use crate::postgres::area::get_event_areas;
use crate::postgres::ballot_style::get_publication_ballot_styles;
use crate::postgres::candidate::export_candidates;
use crate::postgres::configuration_packages::manifest_of_event;
use crate::postgres::contest::export_contests;
use crate::postgres::document::get_event_or_tenant_document_names;
use crate::postgres::election::get_elections;

/// What the check found for a publication of a signed configuration.
#[derive(Debug, Clone)]
pub struct DesignCheck {
    pub external_id: String,
    pub revision: u64,
    pub manifest_sha256: String,
    pub digests: Vec<DesignDigest>,
    pub mismatches: Vec<DesignMismatch>,
}

impl DesignCheck {
    /// What the publication's electoral log entry says of the configuration:
    /// its revision, the manifest SHA-256 and the digest of each design
    /// published.
    pub fn published(self) -> PublishedConfiguration {
        PublishedConfiguration {
            external_id: self.external_id,
            revision: self.revision,
            manifest_sha256: self.manifest_sha256,
            design_digests: self
                .digests
                .into_iter()
                .map(|digest| ConfigurationDesignDigest {
                    area: digest.area,
                    election: digest.election,
                    sha256: digest.sha256,
                })
                .collect(),
        }
    }
}

/// `None` for an event that was not imported from a signed package.
#[instrument(skip(hasura_transaction), err)]
pub async fn check_publication_designs(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    ballot_publication_id: &str,
) -> Result<Option<DesignCheck>> {
    let Some((manifest, manifest_sha256)) =
        manifest_of_event(hasura_transaction, tenant_id, election_event_id).await?
    else {
        return Ok(None);
    };

    let areas = get_event_areas(hasura_transaction, tenant_id, election_event_id).await?;
    let elections = get_elections(hasura_transaction, tenant_id, election_event_id).await?;
    let contests = export_contests(hasura_transaction, tenant_id, election_event_id).await?;
    let candidates = export_candidates(hasura_transaction, tenant_id, election_event_id).await?;
    let mut keys = DesignKeys::of_entities(&areas, &elections, &contests, &candidates)
        .map_err(|problem| anyhow!(problem.message))?;
    let image_ids: Vec<String> = candidates
        .iter()
        .filter_map(|candidate| candidate.image_document_id.clone())
        .collect();
    let image_names = get_event_or_tenant_document_names(
        hasura_transaction,
        tenant_id,
        election_event_id,
        &image_ids,
    )
    .await?;
    for (document_id, name) in &image_names {
        keys = keys.with_document(document_id, name);
    }

    let styles = get_publication_ballot_styles(
        hasura_transaction,
        tenant_id,
        election_event_id,
        ballot_publication_id,
        None,
    )
    .await?
    .into_iter()
    .filter_map(|row| row.ballot_eml)
    .map(|eml| deserialize_str::<BallotStyle>(&eml).map_err(|error| anyhow!("{error}")))
    .collect::<Result<Vec<_>>>()?;
    let digests =
        ballot_design_digests(&styles, &keys).map_err(|problem| anyhow!(problem.message))?;

    Ok(Some(DesignCheck {
        mismatches: mismatches(&manifest.content.ballot_designs, &digests),
        external_id: manifest.configuration.external_id.clone(),
        revision: manifest.configuration.revision,
        manifest_sha256,
        digests,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_publication_logs_the_manifest_and_every_design_it_published() {
        let design = |area: &str, byte: &str| DesignDigest {
            area: area.to_string(),
            election: "national".to_string(),
            sha256: byte.repeat(32),
        };
        let published = DesignCheck {
            external_id: "ov-2028".to_string(),
            revision: 8,
            manifest_sha256: "ab".repeat(32),
            digests: vec![design("Post 1", "cd"), design("Post 2", "ef")],
            mismatches: Vec::new(),
        }
        .published();

        assert_eq!(published.external_id, "ov-2028");
        assert_eq!(published.revision, 8);
        assert_eq!(published.manifest_sha256, "ab".repeat(32));
        let designs: Vec<_> = published
            .design_digests
            .iter()
            .map(|design| {
                (
                    design.area.as_str(),
                    design.election.as_str(),
                    design.sha256.clone(),
                )
            })
            .collect();
        assert_eq!(
            designs,
            vec![
                ("Post 1", "national", "cd".repeat(32)),
                ("Post 2", "national", "ef".repeat(32)),
            ]
        );
    }
}
