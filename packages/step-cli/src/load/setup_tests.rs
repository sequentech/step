// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::{
    config::{Settings, UploadMode},
    files,
    provision::setup_with,
};
use crate::adapters::memory::load_setup::*;
use anyhow::Result;
use serde_json::{json, Value};
use std::{fs, path::PathBuf, time::Duration};

struct Fixture {
    _directory: tempfile::TempDir,
    base: PathBuf,
    output: PathBuf,
    settings: Settings,
    api: MemoryProvisioning,
    environment: MemoryPreparation,
}
impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let base = directory.path().to_path_buf();
        let output = base.join("output");
        files::directory(&output).unwrap();
        let mut settings = Settings::default();
        settings.target.tenant_id = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa".into();
        settings.workload.count = 3;
        settings.workload.shard_size = 2;
        settings.workload.start = 10;
        settings.workload.username_prefix = "test-voter-".into();
        settings.preparation.template = Some("fixture.json".into());
        settings.preparation.threshold = 2;
        settings.preparation.ceremony_timeout_seconds = 2;
        settings.preparation.poll_interval_seconds = 1;
        fs::write(
            base.join("fixture.json"),
            include_str!("../../../voting-load/fixtures/election.json"),
        )
        .unwrap();
        Self {
            _directory: directory,
            base,
            output,
            api: MemoryProvisioning::new(&settings),
            settings,
            environment: MemoryPreparation::default(),
        }
    }
    fn run(&self) -> Result<()> {
        fs::write(
            self.base.join("settings.json"),
            serde_json::to_vec(&self.settings)?,
        )?;
        setup_with(
            &self.base.join("settings.json"),
            &self.base,
            &self.output,
            &self.base,
            &self.api,
            &self.environment,
        )
    }
    fn state(&self) -> Value {
        files::read(&self.output.join("setup-state.json")).unwrap()
    }
    fn existing(&mut self) {
        self.settings.preparation.existing_event = Some("existing.json".into());
        files::save(&self.base.join("existing.json"),&json!({
            "tenant_id":self.settings.target.tenant_id,"election_event_id":EVENT,"election_id":ELECTION,
            "realm":"existing-realm","area_name":"Existing district",
            "login_url":format!("{}/login",self.settings.target.portal_url.trim_end_matches('/'))
        })).unwrap();
    }
}

#[test]
fn invalid_settings_fail_before_refreshing_administrator_credentials() {
    let mut f = Fixture::new();
    f.settings.workload.count = 0;
    assert!(f.run().is_err());
    assert_eq!(f.api.state.lock().unwrap().sessions, 0);
    assert_eq!(fs::read_dir(&f.output).unwrap().count(), 0);
}

#[test]
fn every_administrator_target_must_match_before_importing() {
    for field in ["tenant", "graphql", "keycloak"] {
        let mut f = Fixture::new();
        match field {
            "tenant" => f.api.settings.target.tenant_id = "another-tenant".into(),
            "graphql" => f.api.settings.target.graphql_url = "https://other.invalid/graphql".into(),
            _ => f.api.settings.target.keycloak_url = "https://other.invalid/keycloak".into(),
        }
        assert_eq!(
            f.run().unwrap_err().to_string(),
            "CLI administrator session does not match the workload target"
        );
        assert!(f.api.state.lock().unwrap().imported_event.is_none());
        assert_eq!(fs::read_dir(&f.output).unwrap().count(), 0);
    }
}

#[test]
fn a_new_event_imports_each_shard_then_opens_only_online_voting() {
    let f = Fixture::new();
    f.run().unwrap();
    assert_eq!(
        f.state(),
        json!({"election_event_id":EVENT,"key_ceremony_id":"ceremony-1","ready":true})
    );
    let state = f.api.state.lock().unwrap();
    assert_eq!(state.exported_events, vec![EVENT]);
    assert_eq!(state.ceremonies, vec![(EVENT.into(), 2)]);
    assert_eq!(state.published_events, vec![EVENT]);
    assert_eq!(state.open_events, vec![EVENT]);
    assert_eq!(state.sessions, 4);
    assert_eq!(
        state
            .voter_imports
            .iter()
            .map(|(event, csv, local)| (event.as_str(), csv.as_str(), *local))
            .collect::<Vec<_>>(),
        vec![
            (EVENT, "username\ntest-voter-10\ntest-voter-11\n", true),
            (EVENT, "username\ntest-voter-12\n", true)
        ]
    );
    let (event, local) = state.imported_event.as_ref().unwrap();
    assert!(*local);
    assert_eq!(
        event["election_event"]["alias"],
        "Synthetic load 00000000-0000-0000-0000-00000000002a"
    );
    for shard in ["000000", "000001"] {
        assert!(f.output.join(format!("census/{shard}.imported")).exists());
    }
    let worker = fs::read_to_string(f.output.join("config.json")).unwrap();
    assert!(!worker.contains("private-admin-token"));
    assert!(!worker.contains("private-refresh-token"));
}

#[test]
fn exported_election_external_ids_survive_preparation_and_event_reuse() {
    for external in [
        None,
        Some(Value::Null),
        Some(json!("")),
        Some(json!("external-election-2026")),
    ] {
        let mut f = Fixture::new();
        if let Some(value) = external.clone() {
            f.api.exports[0].1["elections"][0]["external_id"] = value;
        }
        f.run().unwrap();
        let config_path = f.output.join("config.json");
        let prepared: Value = files::read(&config_path).unwrap();
        assert_eq!(prepared["election_id"], ELECTION);
        assert_eq!(
            prepared["election_external_id"],
            external.clone().unwrap_or(Value::Null)
        );

        let mut reused = Fixture::new();
        reused.settings.preparation.existing_event = Some(config_path);
        reused.settings.workload.start = 100;
        reused.run().unwrap();
        let input: Value = files::read(&reused.output.join("config.json")).unwrap();
        assert_eq!(input["election_id"], ELECTION);
        assert_eq!(
            input["election_external_id"],
            external.unwrap_or(Value::Null)
        );
        assert!(reused.api.state.lock().unwrap().imported_event.is_none());
    }
}

#[test]
fn direct_upload_mode_reaches_both_event_and_voter_imports() {
    let mut f = Fixture::new();
    f.settings.target.upload_mode = UploadMode::Direct;
    f.run().unwrap();
    let state = f.api.state.lock().unwrap();
    assert!(!state.imported_event.as_ref().unwrap().1);
    assert!(state.voter_imports.iter().all(|(_, _, local)| !local));
}

#[test]
fn an_existing_event_gets_new_voters_without_ceremony_publication_or_opening() {
    let mut f = Fixture::new();
    f.existing();
    f.run().unwrap();
    let state = f.api.state.lock().unwrap();
    assert!(state.imported_event.is_none());
    assert!(state.exported_events.is_empty());
    assert!(state.ceremonies.is_empty());
    assert!(state.published_events.is_empty());
    assert!(state.open_events.is_empty());
    assert_eq!(state.voter_imports.len(), 2);
    let input: Value = files::read(&f.output.join("config.json")).unwrap();
    assert_eq!(input["realm"], "existing-realm");
    assert_eq!(input["election_event_id"], EVENT);
    assert!(!f.output.join("setup-state.json").exists());
}

#[test]
fn an_existing_event_from_another_tenant_generates_no_census() {
    let mut f = Fixture::new();
    f.existing();
    let p = f.base.join("existing.json");
    let mut event: Value = files::read(&p).unwrap();
    event["tenant_id"] = json!("other-tenant");
    fs::write(p, serde_json::to_vec(&event).unwrap()).unwrap();
    assert_eq!(
        f.run().unwrap_err().to_string(),
        "Existing event belongs to another tenant"
    );
    assert!(!f.output.join("census").exists());
}

#[test]
fn an_existing_event_with_a_foreign_login_origin_is_rejected() {
    let mut f = Fixture::new();
    f.existing();
    let p = f.base.join("existing.json");
    let mut event: Value = files::read(&p).unwrap();
    event["login_url"] = json!("https://foreign.invalid/login");
    fs::write(p, serde_json::to_vec(&event).unwrap()).unwrap();
    assert_eq!(
        f.run().unwrap_err().to_string(),
        "Event login is outside the configured portal origin"
    );
    assert!(!f.output.join("census").exists());
}

#[test]
fn each_successful_shard_is_checkpointed_but_failed_shards_are_not() {
    for existing in [false, true] {
        let mut f = Fixture::new();
        if existing {
            f.existing();
        }
        f.api.state.lock().unwrap().fail_voter_import = Some(2);
        assert_eq!(f.run().unwrap_err().to_string(), "voter import failed");
        assert!(f.output.join("census/000000.imported").exists());
        assert!(!f.output.join("census/000001.imported").exists());
        assert_eq!(f.output.join("config.json").exists(), !existing);
        assert!(f.api.state.lock().unwrap().ceremonies.is_empty());
    }
}

#[test]
fn credentials_are_refreshed_before_each_shard_and_ceremony_poll() {
    for failed_session in [2, 3, 4] {
        let f = Fixture::new();
        f.api.state.lock().unwrap().fail_session = Some(failed_session);
        assert_eq!(f.run().unwrap_err().to_string(), "refresh failed");
        assert_eq!(
            f.api.state.lock().unwrap().voter_imports.len(),
            failed_session.saturating_sub(2).min(2)
        );
        assert!(f.api.state.lock().unwrap().published_events.is_empty());
    }
}

#[test]
fn exports_must_contain_exactly_one_json_configuration() {
    for count in [0, 2] {
        let mut f = Fixture::new();
        f.api.exports = (0..count)
            .map(|i| (format!("{i}.json"), json!({})))
            .collect();
        assert_eq!(
            f.run().unwrap_err().to_string(),
            "Expected exactly one exported event configuration"
        );
        assert_eq!(f.state(), json!({"election_event_id":EVENT}));
        assert!(!f.output.join("census").exists());
    }
}

#[test]
fn exports_must_match_both_the_imported_event_and_configured_tenant() {
    for field in ["id", "tenant_id"] {
        let mut f = Fixture::new();
        f.api.exports[0].1["election_event"][field] = json!("wrong-scope");
        assert_eq!(f.run().unwrap_err().to_string(), "Export scope mismatch");
        assert!(!f.output.join("config.json").exists());
    }
}

#[test]
fn missing_export_identities_do_not_create_worker_inputs() {
    for (field, message) in [
        ("elections", "Export has no election ID"),
        ("areas", "Export has no area name"),
    ] {
        let mut f = Fixture::new();
        f.api.exports[0].1[field] = json!([]);
        assert_eq!(f.run().unwrap_err().to_string(), message);
        assert!(!f.output.join("config.json").exists());
    }
}

#[test]
fn failed_and_cancelled_ceremonies_never_publish_or_open_voting() {
    for status in ["FAILED", "CANCELLED"] {
        let f = Fixture::new();
        f.api
            .state
            .lock()
            .unwrap()
            .statuses
            .push_back(Some(status.into()));
        assert_eq!(
            f.run().unwrap_err().to_string(),
            "Automatic key ceremony failed"
        );
        assert_eq!(
            f.state(),
            json!({"election_event_id":EVENT,"key_ceremony_id":"ceremony-1"})
        );
        assert!(f.api.state.lock().unwrap().published_events.is_empty());
        assert!(f.environment.0.lock().unwrap().sleeps.is_empty());
    }
}

#[test]
fn pending_ceremonies_time_out_at_the_exact_deadline() {
    let f = Fixture::new();
    f.api.state.lock().unwrap().statuses.extend([
        None,
        Some("IN_PROGRESS".into()),
        Some("UNKNOWN".into()),
    ]);
    assert_eq!(
        f.run().unwrap_err().to_string(),
        "Automatic key ceremony timed out"
    );
    assert_eq!(
        f.environment.0.lock().unwrap().sleeps,
        vec![Duration::from_secs(1); 2]
    );
    assert!(f.api.state.lock().unwrap().published_events.is_empty());
}

#[test]
fn success_status_is_accepted_even_on_the_deadline() {
    let f = Fixture::new();
    f.api.state.lock().unwrap().statuses.extend([
        None,
        Some("IN_PROGRESS".into()),
        Some("SUCCESS".into()),
    ]);
    f.run().unwrap();
    assert_eq!(f.state()["ready"], true);
    assert_eq!(f.environment.0.lock().unwrap().sleeps.len(), 2);
}

#[test]
fn infrastructure_errors_keep_the_last_completed_setup_checkpoint() {
    for operation in [
        Operation::Session,
        Operation::Import,
        Operation::Export,
        Operation::Voters,
        Operation::Start,
        Operation::Status,
        Operation::Publish,
        Operation::Open,
    ] {
        let f = Fixture::new();
        f.api.fail(operation, "infrastructure unavailable");
        assert_eq!(
            f.run().unwrap_err().to_string(),
            "infrastructure unavailable"
        );
        assert!(f.api.state.lock().unwrap().open_events.is_empty());
        let checkpoint = f.output.join("setup-state.json");
        if matches!(operation, Operation::Session | Operation::Import) {
            assert!(!checkpoint.exists());
        } else {
            assert!(f.state().get("ready").is_none());
        }
    }
}

#[test]
fn publication_preparation_receives_resolved_paths_and_scoped_identifiers() {
    let mut f = Fixture::new();
    f.settings.preparation.publication_preparer = Some("bin/prepare".into());
    f.run().unwrap();
    assert_eq!(
        f.environment.0.lock().unwrap().prepared,
        vec![PreparedPublication {
            writer: f.base.join("bin/prepare"),
            log: f.output.join("publication.log"),
            tenant: f.settings.target.tenant_id.clone(),
            event: EVENT.into(),
            publication: "publication-1".into(),
        }]
    );
}

#[test]
fn publication_preparation_failure_keeps_voting_closed_and_setup_incomplete() {
    let mut f = Fixture::new();
    f.settings.preparation.publication_preparer = Some("prepare".into());
    f.environment.0.lock().unwrap().preparation_error = Some("preparation unavailable");
    assert_eq!(f.run().unwrap_err().to_string(), "preparation unavailable");
    assert_eq!(f.api.state.lock().unwrap().published_events, vec![EVENT]);
    assert!(f.api.state.lock().unwrap().open_events.is_empty());
    assert!(f.state().get("ready").is_none());
}

#[test]
fn census_generation_failure_never_imports_voters_or_starts_a_ceremony() {
    let f = Fixture::new();
    f.environment.0.lock().unwrap().census_error = Some("census unavailable");
    assert_eq!(f.run().unwrap_err().to_string(), "census unavailable");
    let state = f.api.state.lock().unwrap();
    assert!(state.voter_imports.is_empty());
    assert!(state.ceremonies.is_empty());
}
