// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::load::{census, files, input::Input};
use crate::ports::load_setup::{PreparationEnvironment, Provisioning};
use crate::{commands, types::config::ConfigData, utils::read_config::refresh_and_save_token};
use anyhow::{ensure, Result};
use std::{
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use uuid::Uuid;

fn api<T>(result: std::result::Result<T, Box<dyn std::error::Error>>) -> Result<T> {
    result.map_err(|error| anyhow::anyhow!(error.to_string()))
}

pub(crate) struct AdministratorProvisioning;

impl Provisioning for AdministratorProvisioning {
    fn session(&self) -> Result<ConfigData> {
        api(refresh_and_save_token())
    }
    fn import_event(&self, path: &str, local: bool) -> Result<String> {
        api(commands::import_election_event::import(path, local))
    }
    fn export_event(&self, event: &str, directory: &str) -> Result<()> {
        api(commands::export_election_event::export_election_event(
            event, directory, false, false, false, false, false, false, false, false, false, false,
        ))
    }
    fn import_voters(&self, event: &str, path: &str, local: bool) -> Result<()> {
        api(commands::import_voters::import_voters(event, path, local))
    }
    fn start_ceremony(&self, event: &str, threshold: i64) -> Result<String> {
        api(commands::start_key_ceremony::start_ceremony(
            event, threshold, None, None, true,
        ))
    }
    fn ceremony_status(&self, event: &str, ceremony: &str) -> Result<Option<String>> {
        api(crate::utils::trustees::get_ceremony_status::get_keys_ceremony_status(event, ceremony))
    }
    fn publish(&self, event: &str) -> Result<String> {
        api(commands::publish_changes::publish_changes(event, None))
    }
    fn open_online_voting(&self, event: &str) -> Result<()> {
        use sequent_core::ballot::{VotingStatus, VotingStatusChannel};
        api(
            commands::update_event_voting_status::update_event_voting_status(
                event,
                &VotingStatus::OPEN,
                &Some(VotingStatusChannel::ONLINE),
            ),
        )?;
        Ok(())
    }
}

pub(crate) struct LocalPreparation;

impl PreparationEnvironment for LocalPreparation {
    fn fixture_id(&self) -> Uuid {
        Uuid::new_v4()
    }
    fn generate_census(&self, input: &Input, directory: &Path) -> Result<()> {
        census::generate(input, directory)
    }
    fn now(&self) -> Instant {
        Instant::now()
    }
    fn sleep(&self, duration: Duration) {
        std::thread::sleep(duration);
    }
    fn prepare_publication(
        &self,
        writer: &Path,
        log: &Path,
        tenant: &str,
        event: &str,
        publication: &str,
    ) -> Result<()> {
        let log = files::create(log)?;
        ensure!(
            Command::new(writer)
                .args([tenant, event, publication])
                .stdout(Stdio::from(log.try_clone()?))
                .stderr(Stdio::from(log))
                .status()?
                .success(),
            "Publication preparation failed; inspect private publication.log"
        );
        Ok(())
    }
}
