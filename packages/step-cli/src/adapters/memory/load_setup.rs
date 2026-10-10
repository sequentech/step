// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::load::{config::Settings, files, input::Input};
use crate::ports::load_setup::{PreparationEnvironment, Provisioning};
use crate::types::config::ConfigData;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::{
    collections::{HashMap, VecDeque},
    fs::File,
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{Duration, Instant},
};
use uuid::Uuid;

pub(crate) const EVENT: &str = "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb";
pub(crate) const ELECTION: &str = "cccccccc-cccc-4ccc-8ccc-cccccccccccc";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub(crate) enum Operation {
    Session,
    Import,
    Export,
    Voters,
    Start,
    Status,
    Publish,
    Open,
}

#[derive(Default)]
pub(crate) struct ProvisioningState {
    pub failures: HashMap<Operation, &'static str>,
    pub sessions: usize,
    pub fail_session: Option<usize>,
    pub imported_event: Option<(Value, bool)>,
    pub exported_events: Vec<String>,
    pub voter_imports: Vec<(String, String, bool)>,
    pub fail_voter_import: Option<usize>,
    pub ceremonies: Vec<(String, i64)>,
    pub statuses: VecDeque<Option<String>>,
    pub published_events: Vec<String>,
    pub open_events: Vec<String>,
}
impl ProvisioningState {
    fn check(&self, op: Operation) -> Result<()> {
        if let Some(error) = self.failures.get(&op) {
            return Err(anyhow!("{error}"));
        }
        Ok(())
    }
}

pub(crate) struct MemoryProvisioning {
    pub settings: Settings,
    pub exports: Vec<(String, Value)>,
    pub state: Mutex<ProvisioningState>,
}
impl MemoryProvisioning {
    pub fn new(settings: &Settings) -> Self {
        Self {
            settings: settings.clone(),
            exports: vec![(
                "event.json".into(),
                json!({
                    "election_event": {"id":EVENT,"tenant_id":settings.target.tenant_id},
                    "elections":[{"id":ELECTION}],"areas":[{"name":"District North"}]
                }),
            )],
            state: Mutex::new(ProvisioningState::default()),
        }
    }
    pub fn fail(&self, operation: Operation, message: &'static str) {
        self.state
            .lock()
            .unwrap()
            .failures
            .insert(operation, message);
    }
}
impl Provisioning for MemoryProvisioning {
    fn session(&self) -> Result<ConfigData> {
        let mut state = self.state.lock().unwrap();
        state.check(Operation::Session)?;
        state.sessions += 1;
        if state.fail_session == Some(state.sessions) {
            return Err(anyhow!("refresh failed"));
        }
        Ok(ConfigData {
            endpoint_url: self.settings.target.graphql_url.clone(),
            tenant_id: self.settings.target.tenant_id.clone(),
            keycloak_url: self.settings.target.keycloak_url.clone(),
            auth_token: "private-admin-token".into(),
            refresh_token: "private-refresh-token".into(),
            client_id: "admin-cli".into(),
            client_secret: String::new(),
            username: "synthetic-admin".into(),
        })
    }
    fn import_event(&self, path: &str, local: bool) -> Result<String> {
        let mut state = self.state.lock().unwrap();
        state.check(Operation::Import)?;
        state.imported_event = Some((files::read(Path::new(path))?, local));
        Ok(EVENT.into())
    }
    fn export_event(&self, event: &str, directory: &str) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        state.check(Operation::Export)?;
        let mut archive = zip::ZipWriter::new(File::create(
            Path::new(directory).join("election_event_export.zip"),
        )?);
        for (name, value) in &self.exports {
            archive.start_file(name, zip::write::SimpleFileOptions::default())?;
            archive.write_all(serde_json::to_string(value)?.as_bytes())?;
        }
        archive.finish()?;
        state.exported_events.push(event.into());
        Ok(())
    }
    fn import_voters(&self, event: &str, path: &str, local: bool) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        state.check(Operation::Voters)?;
        if state.fail_voter_import == Some(state.voter_imports.len() + 1) {
            return Err(anyhow!("voter import failed"));
        }
        let content = std::fs::read_to_string(path)?;
        state.voter_imports.push((event.into(), content, local));
        Ok(())
    }
    fn start_ceremony(&self, event: &str, threshold: i64) -> Result<String> {
        let mut state = self.state.lock().unwrap();
        state.check(Operation::Start)?;
        state.ceremonies.push((event.into(), threshold));
        Ok("ceremony-1".into())
    }
    fn ceremony_status(&self, event: &str, ceremony: &str) -> Result<Option<String>> {
        assert_eq!((event, ceremony), (EVENT, "ceremony-1"));
        let mut state = self.state.lock().unwrap();
        state.check(Operation::Status)?;
        Ok(state
            .statuses
            .pop_front()
            .unwrap_or_else(|| Some("SUCCESS".into())))
    }
    fn publish(&self, event: &str) -> Result<String> {
        let mut state = self.state.lock().unwrap();
        state.check(Operation::Publish)?;
        state.published_events.push(event.into());
        Ok("publication-1".into())
    }
    fn open_online_voting(&self, event: &str) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        state.check(Operation::Open)?;
        state.open_events.push(event.into());
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PreparedPublication {
    pub writer: PathBuf,
    pub log: PathBuf,
    pub tenant: String,
    pub event: String,
    pub publication: String,
}
pub(crate) struct EnvironmentState {
    pub now: Instant,
    pub sleeps: Vec<Duration>,
    pub generated_count: Option<usize>,
    pub prepared: Vec<PreparedPublication>,
    pub census_error: Option<&'static str>,
    pub preparation_error: Option<&'static str>,
}
pub(crate) struct MemoryPreparation(pub Mutex<EnvironmentState>);
impl Default for MemoryPreparation {
    fn default() -> Self {
        Self(Mutex::new(EnvironmentState {
            now: Instant::now(),
            sleeps: vec![],
            generated_count: None,
            prepared: vec![],
            census_error: None,
            preparation_error: None,
        }))
    }
}
impl PreparationEnvironment for MemoryPreparation {
    fn fixture_id(&self) -> Uuid {
        Uuid::from_u128(42)
    }
    fn generate_census(&self, input: &Input, directory: &Path) -> Result<()> {
        let mut state = self.0.lock().unwrap();
        if let Some(error) = state.census_error {
            return Err(anyhow!("{error}"));
        }
        files::claim_directory(directory)?;
        for shard in 0..input.shards() {
            let (start, count) = input.bounds(shard)?;
            let mut out = files::create(&directory.join(format!("{shard:06}.csv")))?;
            writeln!(out, "username")?;
            for index in start..start + count as u64 {
                writeln!(out, "{}{index}", input.settings.workload.username_prefix)?;
            }
        }
        state.generated_count = Some(input.settings.workload.count);
        Ok(())
    }
    fn now(&self) -> Instant {
        self.0.lock().unwrap().now
    }
    fn sleep(&self, duration: Duration) {
        let mut state = self.0.lock().unwrap();
        state.now += duration;
        state.sleeps.push(duration);
    }
    fn prepare_publication(
        &self,
        writer: &Path,
        log: &Path,
        tenant: &str,
        event: &str,
        publication: &str,
    ) -> Result<()> {
        let mut state = self.0.lock().unwrap();
        if let Some(error) = state.preparation_error {
            return Err(anyhow!("{error}"));
        }
        state.prepared.push(PreparedPublication {
            writer: writer.into(),
            log: log.into(),
            tenant: tenant.into(),
            event: event.into(),
            publication: publication.into(),
        });
        Ok(())
    }
}
