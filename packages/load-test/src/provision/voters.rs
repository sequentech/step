// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Voter provisioning: `create_user` (no password) + `edit_user` (sets it),
//! the same two mutations `step-cli create-voter` + `update-voter` use
//! (`packages/step-cli/src/commands/create_voter.rs`,
//! `.../update_voter.rs`).
//!
//! Sets `area-id` **and** `authorized-election-ids` attributes.
//! `authorize_voter_election`
//! (`packages/sequent-core/src/services/authorization.rs:96-108`) requires
//! both on the voter's JWT to allow casting; `step-cli create-voter` only
//! sets `area-id`, since it's never used to provision a voter that then
//! logs in and casts its own vote via password grant.

use anyhow::{Context, Result};
use graphql_client::GraphQLQuery;
use serde_json::json;

use crate::hasura::HasuraClient;
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
    query_path = "src/graphql/create_user.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct CreateUser;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/edit_user.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct EditUser;

pub async fn get_area_ids(client: &HasuraClient, election_event_id: &str) -> Result<Vec<String>> {
    let variables = get_areas::Variables {
        election_event_id: election_event_id.to_string(),
    };
    let data = client
        .data_or_bail::<GetAreas>(variables)
        .await
        .context("failed to fetch areas")?;
    Ok(data
        .sequent_backend_area
        .into_iter()
        .map(|area| area.id)
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

pub async fn provision_voter(
    client: &HasuraClient,
    tenant_id: &str,
    election_event_id: &str,
    area_id: &str,
    election_ids: &[String],
    credential: &VoterCredential,
) -> Result<()> {
    let attributes = json!({
        "area-id": [area_id],
        "authorized-election-ids": election_ids,
    });

    let create_variables = create_user::Variables {
        tenant_id: tenant_id.to_string(),
        election_event_id: Some(election_event_id.to_string()),
        user: create_user::KeycloakUser2 {
            attributes: Some(attributes),
            email: None,
            email_verified: None,
            enabled: Some(true),
            first_name: None,
            groups: None,
            id: None,
            last_name: None,
            username: Some(credential.username.clone()),
        },
    };
    let created = client
        .data_or_bail::<CreateUser>(create_variables)
        .await
        .with_context(|| format!("failed to create voter `{}`", credential.username))?;
    let user_id = created.create_user.id.ok_or_else(|| {
        anyhow::anyhow!(
            "create_user returned no id for voter `{}`",
            credential.username
        )
    })?;

    let edit_variables = edit_user::Variables {
        body: edit_user::EditUsersInput {
            attributes: None,
            election_event_id: Some(election_event_id.to_string()),
            email: None,
            enabled: None,
            first_name: None,
            groups: None,
            last_name: None,
            password: Some(credential.password.clone()),
            temporary: Some(false),
            tenant_id: tenant_id.to_string(),
            user_id,
            username: None,
        },
    };
    client
        .data_or_bail::<EditUser>(edit_variables)
        .await
        .with_context(|| format!("failed to set password for voter `{}`", credential.username))?;

    Ok(())
}

pub async fn provision_voters(
    client: &HasuraClient,
    tenant_id: &str,
    election_event_id: &str,
    area_id: &str,
    election_ids: &[String],
    count: u32,
) -> Result<Vec<VoterCredential>> {
    let mut voters = Vec::with_capacity(count as usize);
    for index in 0..count {
        let credential = voter_credential(index);
        provision_voter(
            client,
            tenant_id,
            election_event_id,
            area_id,
            election_ids,
            &credential,
        )
        .await?;
        voters.push(credential);
    }
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
}
