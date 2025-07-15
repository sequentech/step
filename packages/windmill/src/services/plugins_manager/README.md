# Windmill Plugin Manager Guide

This document explains how to use the Plugin Manager system in Windmill, including how to implement hooks, compile plugins, and upload them to S3. It also covers the use of WIT files from sequent-core when creating new plugins.

## Overview

The Plugin Manager allows you to dynamically load, manage, and invoke WebAssembly (WASM) plugins at runtime. Plugins can register hooks, routes, and tasks, and interact with the host system via defined interfaces.

**Relevant files:**
- `plugin_manager.rs`: Core logic for loading, registering, and calling plugins, hooks, routes, and tasks.
- `plugin.rs`: Defines the Plugin struct and dynamic hook invocation.
- `plugin_db_manager.rs`: Provides database transaction management for plugins.
- `plugins_hooks.rs`: Where you implement Rust-side logic for each hook.

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
        // Your implementation 
    }
}
```
**Example: Calling a Hook from the Plugin Manager**
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

1. **Use WIT files from sequent-core**
   - Reference the WIT files in `sequent-core` (e.g., for transactions, plugin-common, etc.).
   - Example:
     ```wit
     import plugins-manager:transactions-manager/transaction;
     export plugins-manager:common/plugin-common;
     ```
2. **Implement your plugin logic in Rust (or another supported language).**
3. **Compile your plugin to the `wasm32-wasip2` target.**
   - Example build command:
     ```sh
     cargo build --target wasm32-wasip2 --release
     ```
4. **Upload the resulting `.wasm` file to S3.**
   - Use the provided script: `.devcontainer/scripts/upload_plugins_to_s3.sh`
   - The script will upload all plugins in `packages/plugins/*/rust-local-target/wasm32-wasip2/debug/` to the S3 bucket under the `plugins/` prefix.

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

## 5. Summary Checklist

- [ ] Create plugin in `packages/plugins/` using WIT files from sequent-core.
- [ ] Implement new hooks in `plugins_hooks.rs` and `PluginHooks` trait.
- [ ] Compile plugin to `wasm32-wasip2` target.
- [ ] Upload `.wasm` file to S3 using the provided script.
- [ ] Restart the host (windmill) to load new plugins.

---

For more details, see the source files:
- `packages/windmill/src/services/plugins_manager/plugins_hooks.rs`
- `packages/windmill/src/services/plugins_manager/plugin_db_manager.rs`
- `packages/windmill/src/services/plugins_manager/plugin.rs`
- `packages/windmill/src/services/plugins_manager/plugin_manager.rs` 