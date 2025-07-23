// SPDX-FileCopyrightText: 2025 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#[allow(warnings)]
mod bindings;

use crate::bindings::plugins_manager::common::types::Manifest;
use bindings::exports::plugins_manager::common::plugin_common::Guest as PluginCommonGuest;
use bindings::plugins_manager::jwt::authorization::authorize;
use bindings::Guest;
use sequent_core::types::permissions::Permissions;
use serde_json::Value;
use core::result::Result;
use std::fmt::Display;

struct Component;

impl Guest for Component {
    fn create_transmission_package(input: String) -> Result<(), String> {
        let parsed_data: Value = match serde_json::from_str(&input) {
            Ok(value) => value,
            Err(e) => {
                return Err(format!("Error parsed input as json value: {}", e));
            }
        };

        let claims_value: &Value = match parsed_data.get("claims") {
            Some(value) => value,
            None => {
                return Err("Error get claims".to_string())  ;
            }
        };

        let claims_str: &str = match claims_value.as_str() {
            Some(s) => s,
            None => {
                return Err("Error parsed claims as str".to_string());
            }
        };

        let original_perms: Vec<Permissions> = vec![Permissions::TRUSTEE_CEREMONY];

        let perm_strings: Vec<String> = original_perms.iter().map(|p| p.to_string()).collect();
        let res = authorize(
            claims_str,
            true,
            Some("90505c8a-23a9-4cdf-a26b-4e19f6a097d5"),
            perm_strings.as_slice(),
        );
        match res {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Error creating transmission package: {}", e)),
        }
    }
}

impl PluginCommonGuest for Component {
    fn get_manifest() -> Manifest {
        Manifest {
            plugin_name: "miru".to_string(),
            hooks: vec!["create-transmission-package".to_string()],
            routes: vec![],
            tasks: vec![],
        }
    }
}

bindings::export!(Component with_types_in bindings);
