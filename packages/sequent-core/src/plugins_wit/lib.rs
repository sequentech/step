// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/// Wasmtime component bindings for the plugin interface.
///
/// This module provides the generated bindings for the WASM plugin interface,
/// including plugin registration, manifest types, and plugin route definitions.
/// Used by the plugin manager and plugin loader to interact with plugins at runtime.
///
/// Referenced in: `plugin_manager.rs` (for `Manifest`, `PluginRoute`), `plugin.rs` (for `Manifest`, `PluginInterface`).
pub mod plugin_bindings {
    #![allow(missing_docs)]
    wasmtime::component::bindgen!({
        path: "src/plugins_wit/plugin/plugin-world.wit",
        world: "plugins-manager:common/plugin",
        imports: {
            default: async,
        },
        exports: {
            default: async,
        },
        require_store_data_send: true,
    });
}

/// Wasmtime component bindings for the transactions manager interface.
///
/// This module provides the generated bindings for the WASM transactions manager,
/// enabling plugins to perform transactional operations via the host.
/// Used by the plugin database manager and plugin logic to coordinate transactional plugin calls.
///
/// Referenced in: `plugin_db_manager.rs`, `plugin.rs` (for transaction host and linker).
pub mod transactions_manager_bindings {
    #![allow(missing_docs)]
    wasmtime::component::bindgen!({
        path: "src/plugins_wit/transaction/transaction-world.wit",
        world: "transactions-manager",
        imports: {
            default: async,
        },
        exports: {
            default: async,
        },
        require_store_data_send: true,
    });
}

/// Wasmtime component bindings for the JWT authorization interface.
///
/// This module provides the generated bindings for JWT-based authorization,
/// allowing plugins to perform authorization checks and interact with JWT claims.
/// Used by the plugin system to add authorization logic to the plugin linker and host.
///
/// Referenced in: `plugin.rs` (for `add_auth_to_linker`, `HostAuth`).
pub mod authorization_bindings {
    #![allow(missing_docs)]
    wasmtime::component::bindgen!({
        path: "src/plugins_wit/jwt/jwt-world.wit",
        world: "jwt",
        imports: {
            default: async,
        },
        exports: {
            default: async,
        },
        require_store_data_send: true,
    });
}
