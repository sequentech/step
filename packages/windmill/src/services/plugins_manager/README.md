<!--
SPDX-FileCopyrightText: 2025 Sequent Legal <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only

-->
# Windmill Plugin Manager Guide

This document explains how to use the Plugin Manager system in Windmill, including how to implement hooks, compile plugins, and upload them to S3. It also covers the use of WIT files from sequent-core when creating new plugins.

## Overview

The Plugin Manager allows you to dynamically load, manage, and invoke WebAssembly (WASM) plugins at runtime. Plugins can register hooks, routes, and tasks, and interact with the host system via defined interfaces.

**Relevant files:**
- `plugin_manager.rs`: Core logic for loading, registering, and calling plugins, hooks, routes, and tasks.
- `plugin.rs`: Defines the Plugin struct and dynamic hook invocation.
- `plugin_db_manager.rs`: Provides database transaction management for plugins.
- `plugins_hooks.rs`: Where you implement Rust-side logic for each hook.
- `packages/sequent-core/src/plugins.rs`: define existing plugins.

---

## 1. Implementing Hooks

Each hook that you want to expose from the plugins must have a corresponding implementation in `plugins_hooks.rs`.

- Define the hook in the `PluginHooks` trait in `plugins_hooks.rs`.
- Implement the hook for `PluginManager` in the same file.
- The plugin manager will call these hooks when requested by plugins.

**Example:**
```rust
#[async_trait]
pub trait PluginHooks {
    async fn my_hook(&self, arg1: i32, arg2: String) -> Result<String>;
}

#[async_trait]
impl PluginHooks for PluginManager {
    async fn my_hook(&self, arg1: i32, arg2: String) -> Result<String> {
        // Your implementation Here have to use self.call_hook_dynamic.
    }
}
```
**Calling a Hook from the Plugin Manager**
To call a hook (e.g., `my_hook`) from your application code, use the following pattern:

```rust
use windmill::services::plugins_manager::plugin_manager::{self, PluginHooks, PluginManager};
// ...
let plugin_manager = plugin_manager::get_plugin_manager()
    .await
    .map_err(|e| (Status::InternalServerError, e.to_string()))?;

// ---- when  my_hook has its own implementation in plugins_hooks.rs  ----
let res = plugin_manager.my_hook(42, "example input".to_string())
    .await
    .map_err(|e| (Status::InternalServerError, e.to_string()))?;
// ...
```
---

## 2. Creating a New Plugin

When creating a new plugin under `packages/plugins`, you must:

1.  **Use WIT files from sequent-core**
    * Reference the WIT files in `sequent-core` when `plugins-manager:common/plugin-common` is mendatory in order to add plugin to the system
    * Example:
      ```world wit
      import plugins-manager:transactions-manager/transaction;
      export plugins-manager:common/plugin-common; 
      ```
2.  **Implement your plugin logic in Rust (or another supported language).**
    * **Generate Bindings:** After modifying your plugin's WIT files (`wit/world.wit` etc.) or if the `sequent-core` WIT files change, you **must** run `cargo component bindings` in your plugin's directory (`packages/plugins/my_plugin/`). This command generates/updates the Rust bindings in your `src/lib.rs` (or other designated output) file, allowing your Rust code to interact with the defined interfaces.

3. **add plugin name to `packages/sequent-core/src/plugins.rs` Plugins enum**
  - should be the same name as writting in cargo.toml under package name.
  
## 3. Compile your plugin to the `wasm32-wasip2` target.

* Example build command:
    ```sh
    cargo build --target wasm32-wasip2
    ```
    * **Tip:** To simplify your build command, you can configure Cargo to always build for `wasm32-wasip2` by default for your plugin. Create a `.cargo` folder in your plugin's root directory (e.g., `packages/plugins/my_plugin/.cargo/`) and add a `config.toml` file with the following content:

    ```toml
    # packages/plugins/my_plugin/.cargo/config.toml
    [build]
    target = "wasm32-wasip2"
    ```
    With this configuration in place, you can simply run `cargo build` (or `cargo build --release`) and it will automatically compile for the `wasm32-wasip2` target.

1.  **Upload the resulting `.wasm` file to S3.**
    * Use the provided script: `.devcontainer/scripts/upload_plugins_to_s3.sh`
    * The script will upload all plugins in `packages/plugins/*/rust-local-target/wasm32-wasip2/debug/{plugin_name}.wasm` to the S3 bucket under the `plugins/` prefix.
---

## 3. Plugin Loading and Invocation

- The Plugin Manager (`plugin_manager.rs`) loads all plugins from S3 at startup.
- It registers their hooks, routes, and tasks based on their manifest.
- When a hook is called, the manager dispatches the call to all plugins that registered for that hook.
- When a route or task is called, the manager dispatches to the appropriate plugin(s).

---

## 4. Example Plugin Directory Structure

```
packages/plugins/my_plugin/
├── .cargo/
│   └── config.toml
├── Cargo.toml
├── src/
│   └── lib.rs
├── wit/
│   └── world.wit
└── rust-local-target/
    └── wasm32-wasip2/
        └── debug/
            └── my_plugin.wasm
```

---

For more details, see the source files:
- `packages/windmill/src/services/plugins_manager/plugins_hooks.rs`
- `packages/windmill/src/services/plugins_manager/plugin_db_manager.rs`
- `packages/windmill/src/services/plugins_manager/plugin.rs`
- `packages/windmill/src/services/plugins_manager/plugin_manager.rs` 