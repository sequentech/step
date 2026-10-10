// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Operator lifecycle: validate, provision, freeze inputs, execute, then always report.
use super::{config::Settings, encryption, files, input::Input, provision, worker, Engine};
use anyhow::{bail, ensure, Context, Result};
use rayon::prelude::*;
use serde_json::{json, Value};
use std::{
    fs,
    io::{BufRead, BufReader},
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

/// Parse the same positive duration syntax accepted by the workload schema.
pub fn duration(mut text: &str) -> Result<Duration> {
    let mut seconds = 0.0;
    while !text.is_empty() {
        let length = text
            .bytes()
            .take_while(|c| c.is_ascii_digit() || *c == b'.')
            .count();
        ensure!(length > 0, "Invalid duration");
        let amount: f64 = text[..length].parse()?;
        text = &text[length..];
        let (suffix, scale) = [("ms", 0.001), ("s", 1.0), ("m", 60.0), ("h", 3600.0)]
            .into_iter()
            .find(|(suffix, _)| text.starts_with(suffix))
            .context("Unknown duration unit")?;
        seconds += amount * scale;
        text = &text[suffix.len()..];
    }
    ensure!(
        seconds.is_finite() && seconds > 0.0,
        "Duration must be finite and positive"
    );
    Duration::try_from_secs_f64(seconds).context("Duration exceeds supported range")
}

/// Check dependencies and service reachability before creating any synthetic voters.
pub fn check(settings: &Settings) -> Result<()> {
    settings.validate()?;
    ensure!(
        Command::new(&settings.runtime.k6)
            .arg("version")
            .stdout(Stdio::null())
            .status()?
            .success(),
        "Cannot start configured k6"
    );
    if matches!(settings.workload.engine, Engine::Chromium) {
        browser_check(settings)?;
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(duration(&settings.workload.request_timeout)?)
        .build()?;
    for endpoint in [
        &settings.target.portal_url,
        &settings.target.keycloak_url,
        &settings.target.graphql_url,
    ]
    .into_iter()
    .chain(settings.target.storage_origins.iter())
    {
        let response = client
            .get(endpoint)
            .send()
            .with_context(|| format!("Cannot reach {endpoint}; check DNS, TLS and networking"))?;
        // Authentication failures still prove that a protected API or bucket is reachable.
        ensure!(
            !response.status().is_server_error(),
            "Target service unavailable: {endpoint} ({})",
            response.status()
        );
    }
    println!("Configuration, target connectivity and runtime dependencies are valid.");
    Ok(())
}

/// Launch the actual configured browser, rather than just checking its package exists.
fn browser_check(settings: &Settings) -> Result<()> {
    let status = Command::new(&settings.runtime.node).args(["-e", "const {chromium}=require('@playwright/test');chromium.launch({headless:true,executablePath:process.argv[1]||undefined}).then(b=>b.close()).catch(e=>{console.error(e);process.exit(1)})"])
        .arg(settings.runtime.chromium.as_deref().unwrap_or(Path::new("")))
        .current_dir(&settings.runtime.playwright_dir).status()?;
    ensure!(status.success(), "Configured Chromium could not launch");
    Ok(())
}

/// Obtain public encryption inputs by authenticating one synthetic voter without casting.
fn bootstrap(input: &Input, output: &Path, assets: &Path) -> Result<Value> {
    let mut config = input.wire()?;
    if let Some(steps) = config["profile"]["steps"].as_array_mut() {
        steps.retain(|step| step["kind"] != "cast");
    }
    let path = output.join("bootstrap.json");
    files::save(&path, &config)?;
    let log_path = output.join("bootstrap.log");
    let log = files::create(&log_path)?;
    let mut command = Command::new(&input.settings.runtime.k6);
    command
        .args(["run", "--log-format", "raw"])
        .arg(assets.join("packages/voting-load/bootstrap.k6.js"));
    worker::environment(&mut command, input, &path)?;
    ensure!(
        command
            .stdout(Stdio::from(log.try_clone()?))
            .stderr(Stdio::from(log))
            .status()?
            .success(),
        "Publication bootstrap failed; inspect private bootstrap.log"
    );
    for line in BufReader::new(fs::File::open(&log_path)?).lines() {
        if let Some(result) = line?.strip_prefix("PUBLICATION ") {
            return Ok(serde_json::from_str(result)?);
        }
    }
    bail!("Bootstrap returned no publication; inspect private bootstrap.log")
}

/// Choose the minimum valid plurality selection. Other rules use an explicit choices file.
pub fn default_choices(style: &Value) -> Result<Value> {
    let mut decoded = Vec::new();
    for contest in style["contests"]
        .as_array()
        .context("Ballot needs contests")?
    {
        if contest["is_acclaimed"].as_bool() == Some(true) {
            continue;
        }
        let selected = contest["min_votes"]
            .as_u64()
            .context("Contest minimum missing")?
            .max(1);
        let candidates = contest["candidates"]
            .as_array()
            .context("Contest candidates missing")?;
        ensure!(
            selected
                <= contest["max_votes"]
                    .as_u64()
                    .context("Contest maximum missing")?
                && selected <= candidates.len() as u64,
            "Cannot choose a valid default ballot; configure preparation.choices"
        );
        decoded.push(json!({"contest_id":contest["id"],"is_explicit_invalid":false,"is_decline_to_vote":false,
            "is_blank_ballot":false,"invalid_errors":[],"invalid_alerts":[],"choices":candidates.iter().enumerate()
                .map(|(index, candidate)| json!({"id":candidate["id"],"selected":if (index as u64) < selected {0} else {-1},"write_in_text":null})).collect::<Vec<_>>()}));
    }
    Ok(json!(decoded))
}

/// Freeze one fresh run, with bounded parallel native encryption and a final ready marker.
pub fn prepare(settings: &Settings, source: &Path, directory: &Path, assets: &Path) -> Result<()> {
    check(settings)?;
    ensure!(
        !std::env::var(&settings.workload.password_env)
            .context("Set the synthetic voter password")?
            .is_empty(),
        "Synthetic password cannot be empty"
    );
    ensure!(
        !directory.exists(),
        "Run directory already exists; choose a fresh output"
    );
    if let Some(parent) = directory
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
    {
        files::directory(parent)?;
    }
    files::claim_directory(directory)?;
    let directory = directory.canonicalize()?;
    settings.create(&directory.join("settings.yaml"))?;
    let setup = directory.join("setup");
    files::directory(&setup)?;
    let log = files::create(&setup.join("setup.log"))?;
    let base = source
        .canonicalize()?
        .parent()
        .context("Configuration parent missing")?
        .to_path_buf();
    let status = Command::new(std::env::current_exe()?)
        .args(["load", "setup"])
        .arg(directory.join("settings.yaml"))
        .arg(&base)
        .arg(&setup)
        .stdout(Stdio::from(log.try_clone()?))
        .stderr(Stdio::from(log))
        .status()?;
    ensure!(
        status.success(),
        "Provisioning failed; inspect {}/setup.log",
        setup.display()
    );
    let mut input: Input = files::read(&setup.join("config.json"))?;
    let inputs = directory.join("inputs");
    files::directory(&inputs)?;
    let started = Instant::now();
    let publication = bootstrap(&input, &inputs, assets)?;
    input.style_id = publication["file"]["id"].clone();
    input.publication_version = publication["file"]["version"].clone();
    input.save(&inputs.join("config.json"))?;
    if matches!(settings.workload.engine, Engine::K6) && settings.workload.mode == "vote" {
        let wire = &publication["publications"]["style_url"];
        let eml = if let Some(prefix) = wire["ballot_eml_prefix"].as_str() {
            Value::String(format!(
                "{prefix}{}{}",
                publication["publications"]["event_url"]["ballot_eml_presentation"]
                    .as_str()
                    .context("Event presentation missing")?,
                wire["ballot_eml_suffix"]
                    .as_str()
                    .context("Ballot suffix missing")?
            ))
        } else {
            wire["ballot_eml"].clone()
        };
        let style: Value = if let Some(eml) = eml.as_str() {
            serde_json::from_str(eml)?
        } else {
            eml
        };
        let style_path = inputs.join("style.json");
        files::save(&style_path, &style)?;
        let choices =
            if let Some(choices) = provision::resolve(&settings.preparation.choices, &base) {
                choices
            } else {
                let path = inputs.join("choices.json");
                files::save(&path, &default_choices(&style)?)?;
                path
            };
        parallel(settings.execution.workers, |index| {
            for shard in (index..input.shards()).step_by(settings.execution.workers) {
                let (_, count) = input.bounds(shard)?;
                let destination = inputs.join(format!("{shard:06}.jsonl"));
                let partial = destination.with_extension("partial");
                encryption::encrypt_to(&style_path, &choices, count, files::create(&partial)?)?;
                fs::rename(partial, &destination)?;
                use std::io::Write;
                files::create(&destination.with_extension("sha256"))?
                    .write_all(files::digest(&destination)?.as_bytes())?;
            }
            Ok(())
        })?;
    }
    files::save(
        &inputs.join("ready.json"),
        &json!({"shards":input.shards(), "count":settings.workload.count,
        "config_sha256":files::digest(&inputs.join("config.json"))?, "elapsed_seconds":started.elapsed().as_secs_f64()}),
    )?;
    println!(
        "Prepared {} journeys. Next: step-cli load run {}",
        settings.workload.count,
        directory.display()
    );
    Ok(())
}

/// Run at most the requested number of tasks and join every worker before reporting.
pub fn parallel(workers: usize, task: impl Fn(usize) -> Result<()> + Send + Sync) -> Result<()> {
    ensure!(workers > 0, "workers must be positive");
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(workers)
        .build()?;
    let results: Vec<_> = pool.install(|| (0..workers).into_par_iter().map(task).collect());
    for result in results {
        result?;
    }
    Ok(())
}

/// Screenshot the actual standalone HTML with embedded fonts fully loaded.
pub fn screenshot(settings: &Settings, report: &Path, destination: &Path) -> Result<()> {
    if let Some(parent) = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        files::directory(parent)?;
    }
    let script = "const{createRequire}=require('module');const{chromium}=createRequire(process.argv[1]+'/package.json')('@playwright/test');(async()=>{const b=await chromium.launch({headless:true,executablePath:process.argv[4]||undefined});try{const p=await b.newPage({viewport:{width:Number(process.argv[5]),height:Number(process.argv[6])}});await p.goto(require('url').pathToFileURL(process.argv[2]).href);await p.evaluate(()=>document.fonts.ready);await p.screenshot({path:process.argv[3],fullPage:true});}finally{await b.close();}})().catch(e=>{console.error(e);process.exit(1)});";
    ensure!(
        Command::new(&settings.runtime.node)
            .args(["-e", script])
            .arg(settings.runtime.playwright_dir.canonicalize()?)
            .arg(report.canonicalize()?)
            .arg(std::path::absolute(destination)?)
            .arg(
                settings
                    .runtime
                    .chromium
                    .as_deref()
                    .unwrap_or(Path::new(""))
            )
            .arg(settings.reporting.screenshot_width.to_string())
            .arg(settings.reporting.screenshot_height.to_string())
            .status()?
            .success(),
        "Report screenshot failed"
    );
    Ok(())
}
