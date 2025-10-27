// SPDX-FileCopyrightText: 2025 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#[allow(warnings)]
mod bindings;
mod services;

use crate::{
    bindings::{
        export,
        plugins_manager::{
            common::types::{Manifest, PluginRoute},
            transactions_manager::transaction::create_hasura_transaction,
        },
        Guest,
    },
    services::{
        create_transmission_package::create_transmission_package_service,
        send_transmission_package::send_transmission_package_service,
        upload_signature_service::upload_transmission_package_signature_service,
    },
};
use bindings::exports::plugins_manager::common::plugin_common::Guest as PluginCommonGuest;
use bindings::plugins_manager::jwt::authorization::authorize;
use sequent_core::{
    plugins::Plugins, serialization::deserialize_with_path::deserialize_str,
    types::permissions::Permissions,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

struct Component;
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CreateTransmissionPackageInput {
    tenant_id: String,
    election_id: String,
    area_id: String,
    tally_session_id: String,
    claims: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SendTransmissionPackageInput {
    tenant_id: String,
    election_id: String,
    area_id: String,
    tally_session_id: String,
    claims: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UploadSignatureInput {
    election_id: String,
    area_id: String,
    tally_session_id: String,
    document_id: String,
    password: String,
    claims: String,
}

impl Guest for Component {
    fn create_transmission_package(input: String) -> Result<(), String> {
        let data: CreateTransmissionPackageInput =
            deserialize_str::<CreateTransmissionPackageInput>(&input)
                .map_err(|e| format!("Error deserializing input: {}", e))?;

        let original_perms: Vec<Permissions> = vec![Permissions::MIRU_CREATE];
        let claims_str: &str = &data.claims.clone();
        let tenant_id = data.tenant_id.clone();
        let election_id = data.election_id.clone();
        let area_id = data.area_id.clone();
        let tally_session_id = data.tally_session_id.clone();

        let perm_strings: Vec<String> = original_perms.iter().map(|p| p.to_string()).collect();
        let auth_res = authorize(claims_str, true, Some(&tenant_id), perm_strings.as_slice());
        if let Err(e) = auth_res {
            return Err(format!("Error authorizing: {}", e));
        }

        match create_hasura_transaction() {
            Ok(_) => {
                match create_transmission_package_service(
                    &tenant_id,
                    &election_id,
                    &area_id,
                    &tally_session_id,
                    false,
                ) {
                    Ok(_) => {}
                    Err(e) => return Err(format!("Error creating transmission package: {}", e)),
                }
            }
            Err(e) => return Err(format!("Error creating hasura transaction: {}", e)),
        };

        Ok(())
    }

    fn send_transmission_package(input: String) -> Result<(), String> {
        let data: SendTransmissionPackageInput =
            deserialize_str::<SendTransmissionPackageInput>(&input)
                .map_err(|e| format!("Error deserializing input: {}", e))?;

        let original_perms: Vec<Permissions> = vec![Permissions::MIRU_CREATE];
        let claims_str: &str = &data.claims.clone();
        let tenant_id = data.tenant_id.clone();
        let election_id = data.election_id.clone();
        let area_id = data.area_id.clone();
        let tally_session_id = data.tally_session_id.clone();

        let perm_strings: Vec<String> = original_perms.iter().map(|p| p.to_string()).collect();
        let auth_res = authorize(claims_str, true, Some(&tenant_id), perm_strings.as_slice());
        if let Err(e) = auth_res {
            return Err(format!("Error authorizing: {}", e));
        }

        match create_hasura_transaction() {
            Ok(_) => {
                match send_transmission_package_service(
                    &tenant_id,
                    &election_id,
                    &area_id,
                    &tally_session_id,
                ) {
                    Ok(_) => {}
                    Err(e) => return Err(format!("Error creating transmission package: {}", e)),
                }
            }
            Err(e) => return Err(format!("Error creating hasura transaction: {}", e)),
        };

        Ok(())
    }

    fn upload_transmission_package_signature(input: String) -> Result<(), String> {
        let data: UploadSignatureInput = deserialize_str::<UploadSignatureInput>(&input)
            .map_err(|e| format!("Error deserializing input: {}", e))?;

        let original_perms: Vec<Permissions> = vec![Permissions::MIRU_CREATE];
        let claims_str: &str = &data.claims.clone();
        let election_id = data.election_id.clone();
        let area_id = data.area_id.clone();
        let tally_session_id = data.tally_session_id.clone();
        let document_id = data.document_id.clone();
        let password = data.password.clone();

        let perm_strings: Vec<String> = original_perms.iter().map(|p| p.to_string()).collect();
        let claims: Value = serde_json::from_str(claims_str)
            .map_err(|e| format!("Failed to parse JWT claims JSON: {}", e))?;

        let hasura_claims = claims
            .get("https://hasura.io/jwt/claims")
            .ok_or_else(|| "Missing 'https://hasura.io/jwt/claims' field.".to_string())?;
        let tenant_id_value = hasura_claims
            .get("x-hasura-tenant-id")
            .ok_or_else(|| "Missing 'x-hasura-tenant-id' field in Hasura claims.".to_string())?;
        let tenant_id = tenant_id_value
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "'x-hasura-tenant-id' is not a string.".to_string())?;

        let auth_res = authorize(claims_str, true, Some(&tenant_id), perm_strings.as_slice());
        if let Err(e) = auth_res {
            return Err(format!("Error authorizing: {}", e));
        }

        let username = claims.get("preferred_username").and_then(|v| v.as_str());

        let Some(username) = username.clone() else {
            return Err(format!("missing username in claims"));
        };

        println!("[Plugin Guest] Uploading transmission package signature for tenant_id: {}, election_id: {}, area_id: {}, tally_session_id: {}, document_id: {}, username: {}",
            tenant_id, election_id, area_id, tally_session_id, document_id, username);

        match create_hasura_transaction() {
            Ok(_) => {
                match upload_transmission_package_signature_service(
                    &tenant_id,
                    &election_id,
                    &area_id,
                    &tally_session_id,
                    &username,
                    &document_id,
                    &password,
                ) {
                    Ok(_) => {}
                    Err(e) => {
                        return Err(format!(
                            "Error uploading transmission package signature: {}",
                            e
                        ))
                    }
                }
            }
            Err(e) => return Err(format!("Error creating hasura transaction: {}", e)),
        };
        Ok(())
    }
}

impl PluginCommonGuest for Component {
    fn get_manifest() -> Manifest {
        Manifest {
            plugin_name: Plugins::MIRU.to_string(),
            hooks: vec![
                "create-transmission-package".to_string(),
                "send-transmission-package".to_string(),
                "upload-transmission-package-signature".to_string(),
            ],
            routes: vec![
                PluginRoute {
                    path: "/miru/create-transmission-package".to_string(),
                    handler: "create-transmission-package".to_string(),
                    process_as_task: true,
                },
                PluginRoute {
                    path: "/miru/send-transmission-package".to_string(),
                    handler: "send-transmission-package".to_string(),
                    process_as_task: true,
                },
                PluginRoute {
                    path: "/miru/upload-signature".to_string(),
                    handler: "upload-transmission-package-signature".to_string(),
                    process_as_task: true,
                },
            ],
            tasks: vec![],
        }
    }
}

export!(Component with_types_in bindings);
