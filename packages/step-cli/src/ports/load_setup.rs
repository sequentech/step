// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::load::input::Input;
use crate::types::config::ConfigData;
use anyhow::Result;
use std::{
    path::Path,
    time::{Duration, Instant},
};
use uuid::Uuid;

/// Administrator operations used while preparing one finite voting workload.
pub(crate) trait Provisioning {
    fn session(&self) -> Result<ConfigData>;
    fn import_event(&self, path: &str, local: bool) -> Result<String>;
    fn export_event(&self, event: &str, directory: &str) -> Result<()>;
    fn import_voters(&self, event: &str, path: &str, local: bool) -> Result<()>;
    fn start_ceremony(&self, event: &str, threshold: i64) -> Result<String>;
    fn ceremony_status(&self, event: &str, ceremony: &str) -> Result<Option<String>>;
    fn publish(&self, event: &str) -> Result<String>;
    fn open_online_voting(&self, event: &str) -> Result<()>;
}

/// Local resources whose real implementations involve randomness, secrets,
/// waiting, or an operator-supplied publication preparation process.
pub(crate) trait PreparationEnvironment {
    fn fixture_id(&self) -> Uuid;
    fn generate_census(&self, input: &Input, directory: &Path) -> Result<()>;
    fn now(&self) -> Instant;
    fn sleep(&self, duration: Duration);
    fn prepare_publication(
        &self,
        writer: &Path,
        log: &Path,
        tenant: &str,
        event: &str,
        publication: &str,
    ) -> Result<()>;
}
