// SPDX-FileCopyrightText: 2025 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#[allow(warnings)]
mod bindings;

use crate::bindings::plugins_manager::common::types::Manifest;
use bindings::exports::plugins_manager::common::plugin_common::Guest as PluginCommonGuest;

struct Component;

impl PluginCommonGuest for Component {
    fn get_manifest() -> Manifest {
        Manifest {
            plugin_name: "miru".to_string(),
            hooks: vec![],
            routes: vec![],
            tasks: vec![],
        }
    }
}

bindings::export!(Component with_types_in bindings);
