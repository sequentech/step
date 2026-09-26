// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Disk-backed aggregation: exact global quantiles, unique ownership and receipt audits.
use super::{files, input::Input};
use anyhow::{ensure, Context, Result};
use rusqlite::{params, Connection, ErrorCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

/// Latency columns are an enum so SQL identifiers never come from operator input.
#[derive(Clone, Copy)]
pub enum Stage {
    Status,
    Cast,
    Journey,
}
impl Stage {
    pub const ALL: [Self; 3] = [Self::Status, Self::Cast, Self::Journey];
    pub fn column(self) -> &'static str {
        match self {
            Self::Status => "status_ms",
            Self::Cast => "cast_ms",
            Self::Journey => "journey_ms",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Status => "Voter status",
            Self::Cast => "Cast acceptance",
            Self::Journey => "Complete journey",
        }
    }
}

/// One engine result. Latencies are optional because failed journeys can stop early.
#[derive(Deserialize)]
struct Sample {
    index: u64,
    passed: bool,
    start: f64,
    end: f64,
    cast_ms: Option<f64>,
    status_ms: Option<f64>,
    receipt: Option<String>,
}

/// Portable report data; request inventory stays in private JSON, outside the HTML.
#[derive(Serialize)]
pub struct Summary {
    pub engine: super::Engine,
    pub mode: String,
    pub planned: usize,
    pub completed: usize,
    pub passed: usize,
    pub accepted_casts: usize,
    pub elapsed_seconds: f64,
    pub casts_per_second: f64,
    pub latency: BTreeMap<String, BTreeMap<String, Option<f64>>>,
    pub traffic: BTreeMap<String, u64>,
    pub errors: Vec<String>,
    pub persistence_verification: String,
}

/// Interpolate several quantiles in one ordered scan, with memory bounded by point count.
/// Fractions must be ascending; SQLite's stage index supplies the sorted samples.
pub fn quantiles(db: &Connection, stage: Stage, fractions: &[f64]) -> Result<Vec<Option<f64>>> {
    ensure!(
        fractions.iter().all(|value| (0.0..=1.0).contains(value))
            && fractions.windows(2).all(|pair| pair[0] <= pair[1]),
        "Quantiles must be ascending fractions"
    );
    let column = stage.column();
    let count: usize =
        db.query_row(&format!("SELECT count({column}) FROM samples"), [], |row| {
            row.get(0)
        })?;
    if count == 0 {
        return Ok(vec![None; fractions.len()]);
    }
    let positions: Vec<_> = fractions
        .iter()
        .map(|fraction| (count - 1) as f64 * fraction)
        .collect();
    let mut output = Vec::with_capacity(fractions.len());
    let mut statement = db.prepare(&format!(
        "SELECT {column} FROM samples WHERE {column} IS NOT NULL ORDER BY {column}"
    ))?;
    let mut rows = statement.query([])?;
    let mut index = 0;
    let mut previous = 0.0;
    while let Some(row) = rows.next()? {
        let value: f64 = row.get(0)?;
        while let Some(position) = positions
            .get(output.len())
            .filter(|position| position.ceil() as usize == index)
        {
            output.push(Some(if position.fract() == 0.0 {
                value
            } else {
                previous + (value - previous) * position.fract()
            }));
        }
        if output.len() == positions.len() {
            break;
        }
        previous = value;
        index += 1;
    }
    Ok(output)
}

/// Interpolate one global percentile; never average per-worker percentiles.
#[cfg(test)]
pub fn percentile(db: &Connection, stage: Stage, fraction: f64) -> Result<Option<f64>> {
    Ok(quantiles(db, stage, &[fraction])?[0])
}

/// Bound diagnostic memory even when every shard fails, preserving the total count.
struct Failures {
    messages: Vec<String>,
    count: usize,
    limit: usize,
}
impl Failures {
    fn add(&mut self, message: impl Into<String>) {
        self.count += 1;
        if self.messages.len() < self.limit {
            self.messages.push(message.into());
        }
    }
    fn finish(mut self) -> Vec<String> {
        if self.count > self.messages.len() {
            self.messages.push(format!(
                "{} additional failures; inspect private worker logs",
                self.count - self.messages.len()
            ));
        }
        self.messages
    }
}

/// Audit accepted receipt IDs in bounded read-only PostgreSQL batches using the DSN-selected transport.
fn audit(db: &Connection, input: &Input, dsn_env: &str) -> Result<usize> {
    let reporting = &input.settings.reporting;
    let mut config: tokio_postgres::Config = std::env::var(dsn_env)
        .context("Set the audit DSN environment variable")?
        .parse()
        .context("Invalid audit DSN")?;
    config.connect_timeout(std::time::Duration::from_secs(
        reporting.audit_connect_timeout_seconds,
    ));
    let connector = postgres_native_tls::MakeTlsConnector::new(native_tls::TlsConnector::new()?);
    tokio::runtime::Runtime::new()?.block_on(async {
        let (client, connection) = config.connect(connector).await.context("Cannot connect to the audit database")?;
        let connection = tokio::spawn(connection);
        client.batch_execute("SET default_transaction_read_only=on").await?;
        client.query_one("SELECT set_config('statement_timeout', $1, false)", &[&reporting.audit_timeout_ms.to_string()]).await?;
        let mut statement = db.prepare("SELECT receipt FROM samples WHERE receipt IS NOT NULL")?;
        let mut rows = statement.query([])?;
        let mut verified = 0;
        loop {
            let mut batch = Vec::<String>::with_capacity(reporting.audit_batch_size);
            while batch.len() < reporting.audit_batch_size {
                let Some(row) = rows.next()? else { break };
                batch.push(row.get(0)?);
            }
            if batch.is_empty() { break; }
            let row = client.query_one("SELECT count(*) FROM sequent_backend.cast_vote WHERE id=ANY($1::text[]::uuid[]) AND tenant_id=$2::text::uuid AND election_event_id=$3::text::uuid", &[&batch, &input.settings.target.tenant_id, &input.event.election_event_id]).await?;
            verified += row.get::<_, i64>(0) as usize;
        }
        drop(client);
        connection.await??;
        Ok(verified)
    })
}

/// Merge shards on disk and write aggregate artifacts even when workload goals fail.
pub fn generate(directory: &Path, dsn_env: Option<&str>) -> Result<()> {
    let input: Input = files::read(&directory.join("config.json"))?;
    input.validate()?;
    let database = directory.join("measurements.sqlite");
    match std::fs::remove_file(&database) {
        Ok(()) => (),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
        Err(error) => return Err(error.into()),
    }
    files::create(&database)?;
    let mut db = Connection::open(&database)?;
    db.pragma_update(
        None,
        "cache_size",
        -(input.settings.reporting.sqlite_cache_kib as i64),
    )?;
    db.pragma_update(None, "temp_store", "FILE")?;
    db.execute_batch("CREATE TABLE samples(voter INTEGER PRIMARY KEY, passed INTEGER, start REAL, end REAL, cast_ms REAL, status_ms REAL, journey_ms REAL, receipt TEXT UNIQUE)")?;
    let mut failures = Failures {
        messages: vec![],
        count: 0,
        limit: input.settings.reporting.max_errors,
    };
    let mut traffic = BTreeMap::<String, u64>::new();
    for shard in 0..input.shards() {
        let result = directory.join("results").join(format!("{shard:06}"));
        let exit: Result<Value> = files::read(&result.join("exit.json"));
        if !exit.is_ok_and(|value| value["code"] == 0) {
            failures.add(format!("Shard {shard} did not finish successfully"));
        }
        let samples = result.join("samples.jsonl");
        if !samples.exists() {
            continue;
        }
        let (first, count) = input.bounds(shard)?;
        let transaction = db.transaction()?;
        {
            let mut insert = transaction.prepare("INSERT INTO samples VALUES(?,?,?,?,?,?,?,?)")?;
            for line in BufReader::new(File::open(samples)?).lines() {
                let sample: Sample = match serde_json::from_str(&line?) {
                    Ok(sample) => sample,
                    Err(_) => {
                        failures.add(format!("Malformed result in shard {shard}"));
                        continue;
                    }
                };
                if sample.index < first || sample.index >= first + count as u64 {
                    failures.add(format!("Voter outside shard {shard}"));
                    continue;
                }
                if !sample.start.is_finite()
                    || !sample.end.is_finite()
                    || sample.end < sample.start
                    || [sample.status_ms, sample.cast_ms]
                        .into_iter()
                        .flatten()
                        .any(|value| !value.is_finite() || value < 0.0)
                {
                    failures.add(format!("Invalid timing in shard {shard}"));
                    continue;
                }
                match insert.execute(params![
                    sample.index,
                    sample.passed,
                    sample.start,
                    sample.end,
                    sample.cast_ms,
                    sample.status_ms,
                    sample.end - sample.start,
                    sample.receipt
                ]) {
                    Ok(_) => (),
                    Err(error)
                        if error.sqlite_error_code() == Some(ErrorCode::ConstraintViolation) =>
                    {
                        failures.add("Duplicate voter or API receipt in worker results")
                    }
                    Err(error) => return Err(error.into()),
                }
            }
        }
        transaction.commit()?;
        if let Ok(summary) = files::read::<Value>(&result.join("summary.json")) {
            if let Some(metrics) = summary["metrics"].as_object() {
                for (key, metric) in metrics {
                    if let Some(name) = key
                        .strip_prefix("http_reqs{name:")
                        .and_then(|name| name.strip_suffix('}'))
                    {
                        *traffic.entry(name.into()).or_default() +=
                            metric["values"]["count"].as_u64().unwrap_or(0);
                    }
                }
            }
        }
        if let Ok(browser) = files::read::<BTreeMap<String, u64>>(&result.join("traffic.json")) {
            for (name, count) in browser {
                *traffic.entry(name).or_default() += count;
            }
        }
    }
    for stage in Stage::ALL {
        let column = stage.column();
        db.execute_batch(&format!("CREATE INDEX {column}_order ON samples({column})"))?;
    }
    let (completed, passed, receipts, start, end): (usize,usize,usize,Option<f64>,Option<f64>) = db.query_row("SELECT count(*),coalesce(sum(passed),0),count(receipt),min(start),max(end) FROM samples", [], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?)))?;
    let elapsed = start
        .zip(end)
        .map(|(start, end)| (end - start) / 1000.0)
        .unwrap_or(0.0);
    let cps = if elapsed > 0.0 {
        receipts as f64 / elapsed
    } else {
        0.0
    };
    let mut latency = BTreeMap::new();
    for stage in Stage::ALL {
        let values = quantiles(&db, stage, &[0.5, 0.99])?;
        latency.insert(
            stage.column().into(),
            BTreeMap::from([("p50".into(), values[0]), ("p99".into(), values[1])]),
        );
    }

    if completed != input.settings.workload.count || passed != input.settings.workload.count {
        failures.add("Missing or failed voter journeys");
    }
    if input.settings.workload.mode == "vote" && receipts != input.settings.workload.count {
        failures.add("Missing unique API receipts");
    }
    if cps < input.settings.min_casts_per_second {
        failures.add(format!(
            "Throughput {cps:.2} below goal {}",
            input.settings.min_casts_per_second
        ));
    }
    for (stage, limits) in &input.settings.goals {
        for (quantile, limit) in limits {
            if latency[stage][quantile].is_none_or(|actual| actual > *limit) {
                failures.add(format!(
                    "{stage} {quantile} exceeds {limit} ms or is missing"
                ));
            }
        }
    }
    let mut verification = "API receipts; no independent database audit".to_owned();
    if let Some(dsn) = dsn_env {
        match audit(&db, &input, dsn) {
            Ok(verified) => {
                verification =
                    format!("{verified}/{receipts} API receipts matched PostgreSQL in batches");
                if verified != receipts {
                    failures.add("Database receipt audit is incomplete");
                }
            }
            Err(_) => {
                verification = "Independent database audit failed".into();
                failures.add("Database audit failed; check DSN, TLS trust and read permissions");
            }
        }
    }
    let summary = Summary {
        engine: input.settings.workload.engine,
        mode: input.settings.workload.mode.clone(),
        planned: input.settings.workload.count,
        completed,
        passed,
        accepted_casts: receipts,
        elapsed_seconds: elapsed,
        casts_per_second: cps,
        latency,
        traffic,
        errors: failures.finish(),
        persistence_verification: verification,
    };
    files::save(&directory.join("results.json"), &summary)?;
    super::presentation::render(directory, &db, &input, &summary)?;
    ensure!(
        summary.errors.is_empty(),
        "Load goals failed; inspect report.html"
    );
    Ok(())
}
