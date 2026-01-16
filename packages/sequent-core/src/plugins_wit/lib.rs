// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
pub mod plugin_bindings {
    wasmtime::component::bindgen!({
        path: "src/plugins_wit/plugin/plugin-world.wit",
        world: "plugins-manager:common/plugin",
        async: true,
    });
}

pub mod transactions_manager_bindings {
    wasmtime::component::bindgen!({
        path: "src/plugins_wit/transaction/transaction-world.wit",
        world: "transactions-manager",
        async: true,
    });
}

pub mod authorization_bindings {
    wasmtime::component::bindgen!({
        path: "src/plugins_wit/jwt/jwt-world.wit",
        world: "jwt",
        async: true,
    });
}
