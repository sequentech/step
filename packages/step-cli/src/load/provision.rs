// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Provision through existing Rust API clients, using typed results rather than CLI output.
//! The coordinator runs this module in a private-log subprocess because legacy API
//! clients print diagnostics. Neither those logs nor administrator tokens reach workers.
use super::{
    census,
    config::{Settings, UploadMode},
    files,
    input::{self, Event, Input},
};
use crate::{commands, utils::read_config::refresh_and_save_token};
use anyhow::{bail, ensure, Context, Result};
use serde_json::{json, Value};
use std::{
    fs::File,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

/// Convert legacy boxed client errors at the boundary to the CLI's error type.
fn api<T>(result: std::result::Result<T, Box<dyn std::error::Error>>) -> Result<T> {
    result.map_err(|error| anyhow::anyhow!(error.to_string()))
}

/// Resolve operator paths against their YAML file, independently of working directory.
pub fn resolve(path: &Option<PathBuf>, base: &Path) -> Option<PathBuf> {
    path.as_ref().map(|path| {
        if path.is_absolute() {
            path.clone()
        } else {
            base.join(path)
        }
    })
}

/// Reduce an exported fixture to one eligible contest and configure exact username login.
pub fn fixture(mut event: Value, settings: &Settings) -> Result<Value> {
    let area = event["areas"]
        .as_array()
        .and_then(|a| a.first())
        .context("Fixture needs an area")?
        .clone();
    let link = event["area_contests"]
        .as_array()
        .context("Fixture needs area-contest links")?
        .iter()
        .find(|item| item["area_id"] == area["id"])
        .context("Area has no contest")?
        .clone();
    let contest = event["contests"]
        .as_array()
        .context("Fixture needs contests")?
        .iter()
        .find(|item| item["id"] == link["contest_id"])
        .context("Missing linked contest")?
        .clone();
    let election = event["elections"]
        .as_array()
        .context("Fixture needs elections")?
        .iter()
        .find(|item| item["id"] == contest["election_id"])
        .context("Missing contest election")?
        .clone();
    let candidates: Vec<_> = event["candidates"]
        .as_array()
        .context("Fixture needs candidates")?
        .iter()
        .filter(|item| item["contest_id"] == contest["id"])
        .cloned()
        .collect();
    event["areas"] = json!([area]);
    event["area_contests"] = json!([link]);
    event["contests"] = json!([contest]);
    event["elections"] = json!([election]);
    event["candidates"] = json!(candidates);
    for key in ["election_event", "elections", "contests", "candidates"] {
        let items: Vec<&mut Value> = if key == "election_event" {
            vec![&mut event[key]]
        } else {
            event[key]
                .as_array_mut()
                .context("Expected fixture array")?
                .iter_mut()
                .collect()
        };
        for item in items {
            let object = item.as_object_mut().context("Expected fixture object")?;
            object.insert("annotations".into(), json!({}));
            for (key, value) in object {
                if key.ends_with("_document_id") {
                    *value = Value::Null;
                }
            }
        }
    }
    event["contests"][0]["min_votes"] = json!(1);
    event["contests"][0]["max_votes"] = json!(1);
    let alias = format!("Synthetic load {}", uuid::Uuid::new_v4());
    event["election_event"]["alias"] = json!(alias);
    event["election_event"]["presentation"]["i18n"]["en"]["alias"] = json!(alias);
    let origin = settings.target.portal_url.trim_end_matches('/');
    event["election_event"]["presentation"]["logo_url"] = json!(format!("{origin}/favicon.svg"));
    let realm = &mut event["keycloak_event_realm"];
    realm["passwordPolicy"] = json!(format!(
        "hashAlgorithm(pbkdf2-sha256) and hashIterations({})",
        settings.workload.hash_iterations
    ));
    if let Some(configs) = realm["authenticatorConfig"].as_array_mut() {
        for config in configs {
            if config["config"].get("matchAttributes").is_some() {
                config["config"]["matchAttributes"] = json!("username");
            }
        }
    }
    if let Some(localizations) = realm["localizationTexts"].as_object_mut() {
        for messages in localizations.values_mut() {
            messages["loginCustomCss"] = json!("");
        }
    }
    for client in realm["clients"]
        .as_array_mut()
        .context("Fixture needs Keycloak clients")?
    {
        if client["clientId"] == settings.workload.client_id {
            client["rootUrl"] = json!(origin);
            client["baseUrl"] = json!(origin);
            client["redirectUris"] = json!([format!("{origin}/*")]);
            client["webOrigins"] = json!([origin]);
        }
    }
    Ok(event)
}

/// Import one CSV at a time, publishing checkpoints only after confirmed server completion.
fn import_census(input: &Input, directory: &Path) -> Result<()> {
    census::generate(input, directory)?;
    for shard in 0..input.shards() {
        api(refresh_and_save_token())?;
        api(commands::import_voters::import_voters(
            &input.event.election_event_id,
            directory
                .join(format!("{shard:06}.csv"))
                .to_str()
                .context("CSV path must be UTF-8")?,
            matches!(input.settings.target.upload_mode, UploadMode::Local),
        ))?;
        files::create(&directory.join(format!("{shard:06}.imported")))?.sync_all()?;
    }
    Ok(())
}

/// Provision an event or import a fresh range into an explicitly selected existing event.
pub fn setup(settings_path: &Path, base: &Path, output: &Path, assets: &Path) -> Result<()> {
    let settings = Settings::read(settings_path)?;
    let session = api(refresh_and_save_token())?;
    ensure!(
        session.tenant_id == settings.target.tenant_id
            && session.endpoint_url == settings.target.graphql_url
            && session.keycloak_url == settings.target.keycloak_url,
        "CLI administrator session does not match the workload target"
    );
    let existing = resolve(&settings.preparation.existing_event, base);
    if let Some(existing) = existing {
        let prior: Value = files::read(&existing)?;
        ensure!(
            prior["tenant_id"] == settings.target.tenant_id,
            "Existing event belongs to another tenant"
        );
        let event: Event = serde_json::from_value(prior)?;
        let input = input::protocol(&settings, event);
        input.validate()?;
        import_census(&input, &output.join("census"))?;
        return input.save(&output.join("config.json"));
    }
    let template = resolve(&settings.preparation.template, base)
        .unwrap_or_else(|| assets.join("packages/voting-load/fixtures/election.json"));
    let fixture = fixture(files::read(&template)?, &settings)?;
    let fixture_path = output.join("fixture.json");
    files::save(&fixture_path, &fixture)?;
    let event_id = api(commands::import_election_event::import(
        fixture_path
            .to_str()
            .context("Fixture path must be UTF-8")?,
        matches!(settings.target.upload_mode, UploadMode::Local),
    ))?;
    files::save(
        &output.join("setup-state.json"),
        &json!({"election_event_id":event_id}),
    )?;
    let export = output.join("export");
    files::directory(&export)?;
    api(commands::export_election_event::export_election_event(
        &event_id,
        export.to_str().context("Export path must be UTF-8")?,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
    ))?;
    let mut archive = zip::ZipArchive::new(File::open(export.join("election_event_export.zip"))?)?;
    let names: Vec<_> = archive
        .file_names()
        .filter(|name| name.ends_with(".json"))
        .map(str::to_owned)
        .collect();
    ensure!(
        names.len() == 1,
        "Expected exactly one exported event configuration"
    );
    let imported: Value = serde_json::from_reader(archive.by_name(&names[0])?)?;
    ensure!(
        imported["election_event"]["id"] == event_id
            && imported["election_event"]["tenant_id"] == settings.target.tenant_id,
        "Export scope mismatch"
    );
    let event = Event {
        election_event_id: event_id.clone(),
        election_id: imported["elections"][0]["id"]
            .as_str()
            .context("Export has no election ID")?
            .into(),
        area_name: imported["areas"][0]["name"]
            .as_str()
            .context("Export has no area name")?
            .into(),
        realm: format!("tenant-{}-event-{event_id}", settings.target.tenant_id),
        login_url: format!(
            "{}/tenant/{}/event/{event_id}/login",
            settings.target.portal_url.trim_end_matches('/'),
            settings.target.tenant_id
        ),
    };
    let input = input::protocol(&settings, event);
    input.validate()?;
    input.save(&output.join("config.json"))?;
    import_census(&input, &output.join("census"))?;
    let ceremony = api(commands::start_key_ceremony::start_ceremony(
        &event_id,
        settings.preparation.threshold.try_into()?,
        None,
        None,
        true,
    ))?;
    files::save(
        &output.join("setup-state.json"),
        &json!({"election_event_id":event_id,"key_ceremony_id":ceremony}),
    )?;
    let started = Instant::now();
    loop {
        api(refresh_and_save_token())?;
        let status = api(
            crate::utils::trustees::get_ceremony_status::get_keys_ceremony_status(
                &event_id, &ceremony,
            ),
        )?;
        match status.as_deref() {
            Some("SUCCESS") => break,
            Some("CANCELLED" | "FAILED") => bail!("Automatic key ceremony failed"),
            _ => ensure!(
                started.elapsed()
                    < Duration::from_secs(settings.preparation.ceremony_timeout_seconds),
                "Automatic key ceremony timed out"
            ),
        }
        std::thread::sleep(Duration::from_secs(
            settings.preparation.poll_interval_seconds,
        ));
    }
    let publication = api(commands::publish_changes::publish_changes(&event_id, None))?;
    if let Some(writer) = resolve(&settings.preparation.publication_preparer, base) {
        let log = files::create(&output.join("publication.log"))?;
        ensure!(
            Command::new(writer)
                .args([&settings.target.tenant_id, &event_id, &publication])
                .stdout(Stdio::from(log.try_clone()?))
                .stderr(Stdio::from(log))
                .status()?
                .success(),
            "Publication preparation failed; inspect private publication.log"
        );
    }
    use sequent_core::ballot::{VotingStatus, VotingStatusChannel};
    api(
        commands::update_event_voting_status::update_event_voting_status(
            &event_id,
            &VotingStatus::OPEN,
            &Some(VotingStatusChannel::ONLINE),
        ),
    )?;
    files::save(
        &output.join("setup-state.json"),
        &json!({"election_event_id":event_id,"key_ceremony_id":ceremony,"ready":true}),
    )
}
