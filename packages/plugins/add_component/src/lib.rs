// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#[allow(warnings)]
mod bindings;

use crate::bindings::exec_query;
use bindings::Guest;

struct Component;

impl Guest for Component {
    fn add(x: u32, y: u32) -> String {
        let sql = format!("INSERT INTO logs (message) VALUES ('test');");
        let res = exec_query(&sql);
        let a = x + y;
        format!("{} + {} = {}. Result: {}", x, y, a, res)
    }

    fn get_manifest() -> String {
        serde_json::json!({
            "plugin_name": "adder",
            "hooks": ["add"],
            "routes": []
        })
        .to_string()
    }
}

bindings::export!(Component with_types_in bindings);
