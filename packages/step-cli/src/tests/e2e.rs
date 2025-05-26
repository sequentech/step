// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::init_loadero::init_loadero_tests;
use crate::{
    commands::{
        complete_key_ceremony::complete_ceremony, configure::create_config, confirm_tally_ceremoney_key::confirm_key, create_voter::create_voter, import_election_event::import, publish_changes::publish_changes, start_key_ceremony::start_ceremony as start_key_ceremony, start_tally::start_ceremony as start_tally_ceremony, update_event_voting_status::update_event_voting_status, update_tally_status::update_status as update_tally_status, update_voter::edit_voter
    },
    types::tally::TallyExecutionStatus,
    utils::areas::get_areas::GetAreas,
};
use sequent_core::{ballot::VotingStatus, types::ceremonies::TallyType};
use std::{env, error::Error, fmt::format};

#[test]
fn run_e2e() -> Result<(), Box<dyn Error>> {
    // TODO: Add environment Variables in dev.env + beyond + gitops - Do this for the variables in this file + the variables in init_loadero.rs

    // Step 0 - Create Config
    let tenant_id = env::var("TENANT_ID")?;
    let hasura_endpoint = env::var("HASURA_ENDPOINT")?;
    let keycloak_url = env::var("KEYCLOAK_URL")?;
    let keycloak_user = env::var("KEYCLOAK_USER")?;
    let keycloak_password = env::var("KEYCLOAK_PASSWORD")?;
    let keycloak_client_id = env::var("KEYCLOAK_CLIENT_ID")?;
    let keycloak_client_secret = env::var("KEYCLOAK_CLIENT_SECRET")?;
    create_config(
        &hasura_endpoint,
        &keycloak_url,
        &keycloak_user,
        &keycloak_password,
        &keycloak_client_id,
        &keycloak_client_secret,
        &tenant_id,
    )?;

    // Step 1: Import Election event
    let test_event_path = env::var("TEST_ELECTION_EVENT_PATH").unwrap_or(
        "/workspaces/step/packages/step-cli/data/test-election-template.json".to_string(),
    );
    let election_event_id = import(test_event_path.as_str(), false)?;

    // Step 1.5: Create Voters if necessary (Currently 10 Voters in template)
    let voter_sim_number = env::var("VOTER_NUMBER").unwrap_or("10".to_string()); // Use number of voters or default to 10
    let voter_sim_number: u64 = voter_sim_number.parse()?;

    if voter_sim_number > 10 {
        let area_ids = GetAreas::get_area_ids(&election_event_id)?;
        let area_id = area_ids[0].clone();
        for i in 11..(voter_sim_number + 1) {
            let name = format!("test{}", i);
            let pass = format!("password{}", i);
            let user_id = create_voter(&election_event_id, &name, &name, &name, "")?;
            // Call Edit to update password and area id for voter
            edit_voter(
                &election_event_id,
                &user_id,
                "",
                "",
                "",
                "",
                &pass,
                &area_id,
                "",
                &true,
            )?;
        }
    }

    // Step 2: Start Key Ceremony
    let key_ceremony_id = start_key_ceremony(&election_event_id, 2, None, None)?;

    // Auth with trustee1
    let trustee1 = env::var("TRUSTEE_1").unwrap_or("trustee1".to_string());
    create_config(
        &hasura_endpoint,
        &keycloak_url,
        &trustee1,
        &trustee1,
        &keycloak_client_id,
        &keycloak_client_secret,
        &tenant_id,
    )?;
    // Complete Key Ceremony
    complete_ceremony(&election_event_id, &key_ceremony_id)?;
    // Auth with trustee2
    let trustee2 = env::var("TRUSTEE_2").unwrap_or("trustee1".to_string());
    create_config(
        &hasura_endpoint,
        &keycloak_url,
        &trustee2,
        &trustee2,
        &keycloak_client_id,
        &keycloak_client_secret,
        &tenant_id,
    )?;
    // Complete Key Ceremony
    complete_ceremony(&election_event_id, &key_ceremony_id)?;

    // Revert to user config
    create_config(
        &hasura_endpoint,
        &keycloak_url,
        &keycloak_user,
        &keycloak_password,
        &keycloak_client_id,
        &keycloak_client_secret,
        &tenant_id,
    )?;

    // Step 3: Start Election - Update Status
    let status = update_event_voting_status::VotingStatus::OPEN;
    update_event_voting_status(&election_event_id, &status)?;

    // Step 3.5 : Create Publication
    publish_changes(&election_event_id, None)?;

    // Step 4: Create + Run Loadero Test
    let voting_portal_domain = std::env::var("VOTING_PORTAL_DOMAIN")?;
    let voting_portal_url = format!(
        "{}/tenant/{}/event/{}/login",
        &voting_portal_domain, &tenant_id, &election_event_id
    );
    init_loadero_tests(&election_event_id, &voting_portal_url, voter_sim_number)?;

    // Step 5: Tally votes
    let tally_id = start_tally_ceremony(&election_event_id, None, TallyType::ELECTORAL_RESULTS.to_string().as_str())?;
    // Confirm trustee keys
    // Trustee 1
    create_config(
        &hasura_endpoint,
        &keycloak_url,
        &trustee1,
        &trustee1,
        &keycloak_client_id,
        &keycloak_client_secret,
        &tenant_id,
    )?;
    confirm_key(&election_event_id, &tally_id)?;
    // Trustee 2
    create_config(
        &hasura_endpoint,
        &keycloak_url,
        &trustee2,
        &trustee2,
        &keycloak_client_id,
        &keycloak_client_secret,
        &tenant_id,
    )?;
    confirm_key(&election_event_id, &tally_id)?;

    // Revert to user config
    create_config(
        &hasura_endpoint,
        &keycloak_url,
        &keycloak_user,
        &keycloak_password,
        &keycloak_client_id,
        &keycloak_client_secret,
        &tenant_id,
    )?;

    // Start Tally
    update_tally_status(
        &election_event_id,
        &tally_id,
        TallyExecutionStatus::IN_PROGRESS.to_string().as_str(),
    )?;

    Ok(())
}
