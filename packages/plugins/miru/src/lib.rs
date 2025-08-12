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
    services::create_transmission_package::create_transmission_package_service,
};
use bindings::exports::plugins_manager::common::plugin_common::Guest as PluginCommonGuest;
use bindings::plugins_manager::jwt::authorization::authorize;
use core::result::Result::{self, Ok};
use sequent_core::types::permissions::Permissions;
use serde_json::Value;

struct Component;

impl Guest for Component {
    fn create_transmission_package(input: String) -> Result<(), String> {
        // let parsed_data: Value = match serde_json::from_str(&input) {
        //     Ok(value) => value,
        //     Err(e) => {
        //         return Err(format!("Error parsed input as json value: {}", e));
        //     }
        // };

        // let claims_value: &Value = match parsed_data.get("claims") {
        //     Some(value) => value,
        //     None => {
        //         return Err("Error get claims".to_string());
        //     }
        // };

        // let claims_str: &str = match claims_value.as_str() {
        //     Some(s) => s,
        //     None => {
        //         return Err("Error parsed claims as str".to_string());
        //     }
        // };

        // let original_perms: Vec<Permissions> = vec![Permissions::ADMIN_USER];

        // let perm_strings: Vec<String> = original_perms.iter().map(|p| p.to_string()).collect();
        // let res = authorize(
        //     claims_str,
        //     true,
        //     Some("90505c8a-23a9-4cdf-a26b-4e19f6a097d5"),
        //     perm_strings.as_slice(),
        // );
        // if let Err(e) = res {
        //     return Err(format!("Error authorizing: {}", e));
        // }

        match create_hasura_transaction() {
            Ok(_) => {
                let tenant_id = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";
                let election_id = "881c759b-7d52-4440-a46b-893ed3bc91b0";
                let area_id = "c06ce4ce-8957-4504-b0a8-75ec70b02335";
                let tally_session_id = "7e1c1b80-179d-440b-9c45-2797fc04450c";
                let force = false;

                let _ = create_transmission_package_service(
                    tenant_id,
                    election_id,
                    area_id,
                    tally_session_id,
                    force,
                );
            }
            Err(e) => return Err(format!("Error creating hasura transaction: {}", e)),
        };

        Ok(())
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
            tasks: vec!["create-transmission-package".to_string()],
        }
    }
}

export!(Component with_types_in bindings);
