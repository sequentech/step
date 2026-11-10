// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! WebAssembly plugins, loaded from the `plugins/` prefix of the public bucket.
//!
//! The plugin runtime is compiled only with the `plugins` feature; it is not
//! part of the evaluated TOE. Without the feature, `plugin_manager` keeps only
//! the entry points that the start-up and the plugin task call: start-up loads
//! nothing, and no plugin hook, route or task can be called.

#[cfg(feature = "plugins")]
pub mod plugin;
#[cfg(feature = "plugins")]
pub mod plugin_db_manager;
#[cfg(feature = "plugins")]
pub mod plugin_manager;
#[cfg(feature = "plugins")]
pub mod plugins_hooks;

#[cfg(not(feature = "plugins"))]
pub mod plugin_manager {
    use anyhow::{anyhow, Result};

    /// Why no plugin can be called in this build.
    pub const PLUGINS_DISABLED: &str =
        "Plugins are not part of this build (feature `plugins` is disabled)";

    /// The plugin runtime, which this build does not have: the type has no
    /// values, so none of its methods can run.
    pub enum PluginManager {}

    impl PluginManager {
        /// Never runs: there is no plugin manager to call it on.
        pub async fn execute_task(&self, _task: &str, _input_json: String) -> Result<()> {
            match *self {}
        }
    }

    /// Refuses: this build has no plugin manager.
    pub async fn get_plugin_manager() -> Result<&'static PluginManager> {
        Err(anyhow!(PLUGINS_DISABLED))
    }

    /// Loads nothing: this build reads no plugin from the bucket.
    pub async fn init_plugin_manager() -> Result<()> {
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        // Compiles only while `PluginManager` has no values.
        const _: fn(&PluginManager) -> ! = |manager| match *manager {};

        #[tokio::test]
        async fn this_build_loads_no_plugin_and_has_none_to_call() {
            // Needs no bucket configuration: nothing is read.
            init_plugin_manager().await.unwrap();
            let refusal = get_plugin_manager().await.err().unwrap();
            assert_eq!(refusal.to_string(), PLUGINS_DISABLED);
        }
    }
}
