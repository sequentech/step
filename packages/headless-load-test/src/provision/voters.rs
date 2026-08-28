// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Voter provisioning: build a voters CSV and bulk-import it via
//! `import_users` — the same mutation/celery task
//! (`packages/windmill/src/tasks/import_users.rs`, `.../import_users_file`)
//! the admin-portal's Voters-tab import wizard uses. Chosen over the
//! per-voter `create_user`/`edit_user` GraphQL calls this module used to
//! make: it's one upload for the whole batch instead of two round trips per
//! voter, and — unlike `edit_user` — it sets a real, immediately-usable
//! password credential rather than a temporary one that would need a
//! required-action flow to complete.
//!
//! Deliberately does **not** set an `authorized-election-ids` attribute.
//! That's not how eligibility is normally established: the custom Keycloak
//! protocol mapper `AuthorizedElectionsUserAttributeMapper`
//! (`packages/keycloak-extensions/conditional-authenticators/src/main/java/sequent/keycloak/protocol/oidc/mappers/AuthorizedElectionsUserAttributeMapper.java:135-244`)
//! computes that claim at token-issuance time: if the user has no explicit
//! `authorized-election-ids` attribute, it looks up the voter's `area-id`
//! attribute against `sequent_backend_area_contest` (joined to
//! `contest.election_id`) and authorizes exactly the elections reachable
//! from that area — falling back to *every* election in the event only if
//! the area has none. Setting `area_name` in the CSV (which the importer
//! resolves to the `area-id` attribute via `get_areas_by_name`,
//! `packages/windmill/src/services/import/import_users.rs:698-712`) is
//! therefore both necessary and sufficient; a real voter export from this
//! platform leaves its own `authorized-election-ids` column blank for the
//! same reason.

use anyhow::{Context, Result};
use graphql_client::GraphQLQuery;

use crate::hasura::HasuraClient;
use crate::provision::tasks::poll_task_execution;
use crate::provision::upload::upload_document;
use crate::types::hasura::*;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/get_areas.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct GetAreas;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/get_elections.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct GetElections;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/import_users.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct ImportUsers;

#[derive(Debug, Clone)]
pub struct Area {
    pub id: String,
    pub name: String,
}

pub async fn get_areas(client: &HasuraClient, election_event_id: &str) -> Result<Vec<Area>> {
    let variables = get_areas::Variables {
        election_event_id: election_event_id.to_string(),
    };
    let data = client
        .data_or_bail::<GetAreas>(variables)
        .await
        .context("failed to fetch areas")?;
    // An area with no name can't be targeted by the CSV importer's
    // by-name lookup (`get_areas_by_name`), so it can't be assigned a
    // voter through this path either.
    Ok(data
        .sequent_backend_area
        .into_iter()
        .filter_map(|area| area.name.map(|name| Area { id: area.id, name }))
        .collect())
}

pub async fn get_election_ids(
    client: &HasuraClient,
    election_event_id: &str,
) -> Result<Vec<String>> {
    let variables = get_elections::Variables {
        election_event_id: election_event_id.to_string(),
    };
    let data = client
        .data_or_bail::<GetElections>(variables)
        .await
        .context("failed to fetch elections")?;
    Ok(data
        .sequent_backend_election
        .into_iter()
        .map(|election| election.id)
        .collect())
}

/// A voter provisioned for headless password-grant login. Deterministic so
/// a run's cast-vote traffic is reproducible.
#[derive(Debug, Clone)]
pub struct VoterCredential {
    pub username: String,
    pub password: String,
}

pub fn voter_credential(index: u32) -> VoterCredential {
    let username = format!("voter-{index}");
    let password = username.clone();
    VoterCredential { username, password }
}

/// Builds the voters CSV `import_users_file`
/// (`packages/windmill/src/services/import/import_users.rs:131-249`)
/// expects: `password` in plaintext, hashed server-side with a random salt
/// (`import_users.rs:756-765`), and `area_name` resolved to the `area-id`
/// attribute by name — not `area-id` directly, which would attempt the
/// same by-name lookup on the raw id and silently resolve to no area.
fn build_voters_csv(area_name: &str, count: u32) -> Result<(Vec<u8>, Vec<VoterCredential>)> {
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record(["username", "password", "area_name"])?;

    let mut voters = Vec::with_capacity(count as usize);
    for index in 0..count {
        let credential = voter_credential(index);
        writer.write_record([&credential.username, &credential.password, area_name])?;
        voters.push(credential);
    }
    writer.flush()?;

    let bytes = writer
        .into_inner()
        .context("failed to build the voters CSV")?;
    Ok((bytes, voters))
}

/// Provisions `count` voters into `election_event_id`'s `area_name` area in
/// one bulk CSV import, returning their credentials.
pub async fn provision_voters(
    client: &HasuraClient,
    http: &reqwest::Client,
    tenant_id: &str,
    election_event_id: &str,
    area_name: &str,
    count: u32,
) -> Result<Vec<VoterCredential>> {
    let (csv_bytes, voters) = build_voters_csv(area_name, count)?;

    let document_id = upload_document(
        client,
        http,
        "voters.csv",
        "text/csv",
        Some(election_event_id.to_string()),
        &csv_bytes,
    )
    .await
    .context("failed to upload the voters CSV")?;

    let import_variables = import_users::Variables {
        tenant_id: tenant_id.to_string(),
        election_event_id: Some(election_event_id.to_string()),
        document_id,
        sha256: None,
    };
    let imported = client
        .data_or_bail::<ImportUsers>(import_variables)
        .await
        .context("failed to start the voters import")?;
    let imported = imported
        .import_users
        .ok_or_else(|| anyhow::anyhow!("import_users returned no data"))?;

    poll_task_execution(client, &imported.task_execution.id)
        .await
        .context("voters import did not complete")?;

    Ok(voters)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voter_credentials_are_deterministic_and_distinct() {
        let a = voter_credential(0);
        let b = voter_credential(1);
        assert_eq!(a.username, "voter-0");
        assert_eq!(a.username, a.password);
        assert_ne!(a.username, b.username);

        let a_again = voter_credential(0);
        assert_eq!(a.username, a_again.username);
    }

    #[test]
    fn the_voters_csv_has_the_columns_the_importer_expects() {
        let (bytes, voters) = build_voters_csv("Ward 1", 3).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        let mut lines = text.lines();

        assert_eq!(lines.next(), Some("username,password,area_name"));
        assert_eq!(lines.next(), Some("voter-0,voter-0,Ward 1"));
        assert_eq!(lines.next(), Some("voter-1,voter-1,Ward 1"));
        assert_eq!(lines.next(), Some("voter-2,voter-2,Ward 1"));
        assert_eq!(lines.next(), None);
        assert_eq!(voters.len(), 3);
    }

    #[test]
    fn an_area_name_needing_quotes_round_trips() {
        // area names are free text and can contain commas — the CSV writer
        // must quote them, not just concatenate.
        let (bytes, _voters) = build_voters_csv("Ward 1, English Public", 1).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        let mut reader = csv::Reader::from_reader(text.as_bytes());
        let record = reader.records().next().unwrap().unwrap();
        assert_eq!(&record[2], "Ward 1, English Public");
    }
}
