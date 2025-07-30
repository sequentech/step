// SPDX-FileCopyrightText: 2025 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#[allow(warnings)]
mod bindings;
mod services;

use crate::bindings::plugins_manager::{
    common::types::{Manifest, PluginRoute},
    transactions_manager::transaction::create_hasura_transaction,
};
use bindings::exports::plugins_manager::common::plugin_common::Guest as PluginCommonGuest;
use bindings::plugins_manager::jwt::authorization::authorize;
use bindings::Guest;
use core::result::Result;
use sequent_core::types::permissions::Permissions;
use serde::Serialize;
use serde_json::{to_string, Value};

struct Component;

#[derive(Serialize)]
struct CreateTransmissionPackageOutput {
    pub data: String,
}

async fn test_test() -> Result<String, String> {
    Ok("{}".to_string())
}

impl Guest for Component {
    fn create_transmission_package(input: String) -> Result<String, String> {
        let parsed_data: Value = match serde_json::from_str(&input) {
            Ok(value) => value,
            Err(e) => {
                return Err(format!("Error parsed input as json value: {}", e));
            }
        };

        let claims_value: &Value = match parsed_data.get("claims") {
            Some(value) => value,
            None => {
                return Err("Error get claims".to_string());
            }
        };

        let claims_str: &str = match claims_value.as_str() {
            Some(s) => s,
            None => {
                return Err("Error parsed claims as str".to_string());
            }
        };

        let original_perms: Vec<Permissions> = vec![Permissions::ADMIN_USER];

        let perm_strings: Vec<String> = original_perms.iter().map(|p| p.to_string()).collect();
        let res = authorize(
            claims_str,
            true,
            Some("90505c8a-23a9-4cdf-a26b-4e19f6a097d5"),
            perm_strings.as_slice(),
        );
        if let Err(e) = res {
            return Err(format!("Error authorizing: {}", e));
        }

        // match create_hasura_transaction() {
        //     Ok(hasura_transaction) => {
        //         let tenant_id = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";
        //         let election_id = "election-id";
        //         let area_id = "area-id";
        //         let tally_session_id = "tally-session-id";
        //         let force = false;

        //         create_transmission_package_service(
        //             tenant_id,
        //             election_id,
        //             area_id,
        //             tally_session_id,
        //             force,
        //         )
        //     }
        //     Err(e) => Err(format!("Error creating hasura transaction: {}", e)),
        // }
        Ok("".to_string())
    }

    fn test() -> Result<String, String> {
        wit_bindgen_rt::async_support::block_on(async {
            let transmission_package_bytes = std::fs::read("transmission_package_path")
                .map_err(|e| format!("Error reading transmission package: {}", e))?;

            let _res = test_test()
                .await
                .map_err(|e| format!("Error testing transmission package: {}", e))?;

            let output = CreateTransmissionPackageOutput {
                data: "Test function executed successfully".to_string(),
            };
            Ok(serde_json::to_string(&output).unwrap())
        })
    }
}

impl PluginCommonGuest for Component {
    fn get_manifest() -> Manifest {
        Manifest {
            plugin_name: "miru".to_string(),
            hooks: vec!["create-transmission-package".to_string()],
            routes: vec![PluginRoute {
                path: "/miru/create-transmission-package".to_string(),
                handler: "create-transmission-package".to_string(),
                process_as_task: true,
            }],
            tasks: vec![],
        }
    }
}

bindings::export!(Component with_types_in bindings);
