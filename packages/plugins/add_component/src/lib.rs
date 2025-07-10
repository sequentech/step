// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#[allow(warnings)]
mod bindings;

use crate::bindings::docs::transactions_manager::transaction::{
    commit_hasura_transaction, create_hasura_transaction, execute_hasura_query,
};
use bindings::Guest;

struct Component;

impl Guest for Component {
    fn add(x: u32, y: u32) -> String {
        let sql = format!("INSERT INTO sequent_backend.document (name) VALUES ('test');");
        let _ = create_hasura_transaction();
        let exec_query = match execute_hasura_query(&sql) {
            Ok(r) => r,
            Err(e) => e,
        };
        let _ = commit_hasura_transaction();
        let a = x + y;
        format!("{} + {} = {}. Result: {}", x, y, a, exec_query)
    }

    fn add_route(data: String) -> String {
        serde_json::json!({
            "data": data,
            "res": "Route called successfully!"
        })
        .to_string()
    }

    fn get_manifest() -> String {
        serde_json::json!({
            "plugin_name": "adder",
            "hooks": ["add"],
            "routes": [
                {
                    "path": "/adder/add",
                    "handler": "add-route", // This is the name of the function that will handle the route as written in the .wit file
                }
            ]
        })
        .to_string()
    }
}

bindings::export!(Component with_types_in bindings);
