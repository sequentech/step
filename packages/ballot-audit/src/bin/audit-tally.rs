// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{ensure, Context, Result};
use b3::messages::statement::StatementType;
use ballot_audit::{
    digest, incomplete, invalid_input,
    source::{
        board_identifier, connect, into_invalid_input, local_config, new_private_directory, one,
        read_connections, rows, save_bytes, save_json, snapshot_metadata, start_snapshot,
        ConnectionSource,
    },
    tally::{tally_batches, tally_policies},
    validate_sha512_pin, ExitStatus, MultiplicityPolicy, Scope, SOURCE_REVISION,
};
use clap::{ArgGroup, Parser};
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use strand::signature::StrandSignaturePk;
use tokio_postgres::Config;
use uuid::Uuid;

const APPLICATION_NAME: &str = "audit-tally";
const DEFAULT_STATEMENT_TIMEOUT_SECONDS: u32 = 120;
const DEFAULT_PASSWORD_ENV: &str = "BALLOT_AUDIT_IMMUDB_PASSWORD";
const BRAID_VERIFY: &str = "verify";
const EXPORT_BALLOT_AUDIT: &str = "export-ballot-audit";
const EXPORT_ELECTORAL_LOG: &str = "export-electoral-log";
const PREPARE_BALLOT_AUDIT: &str = "prepare-ballot-audit";
const BALLOT_AUDIT: &str = "ballot-audit";
const AUDIT_CAST_HISTORY: &str = "audit-cast-history";
const ELECTORAL_LOG_FILE: &str = "electoral-log.json";
const POSTGRES_EXPORT_DIR: &str = "postgres-export";
const PREPARED_AUDIT_DIR: &str = "prepared-audit";
const IMMUDB_EXPORT_DIR: &str = "immudb-export";
const SCOPES_DIR: &str = "scopes";
const HISTORY_DIR: &str = "history";
const HISTORY_FOUND_BY: &str = "cast-vote history";
const BRAID_DIR: &str = "braid";
const TALLIED_VOTES_TO_AUDIT_FILE: &str = "tallied-votes-to-audit.json";
const HASURA_ROW_WITHOUT_SIGNED_LOG: &str = "hasura_row_without_signed_log";
/// Keys that the console output leaves out at any depth: voter IDs, ballot
/// contents, Keycloak users and the assurance, note and limitation texts of
/// the reports.
const NOT_PRINTED: [&str; 11] = [
    "voter_id",
    "voter_id_string",
    "voters",
    "decisions",
    "content",
    "users",
    "assurance",
    "note",
    "limitations",
    "limitation",
    "limits",
];
const RULE_WIDTH: usize = 78;
const ACCOUNTING_NOT_IN_SUMMARY: [&str; 3] = [
    "frozen_eligible_voters",
    "frozen_eligible_voters_without_valid_vote",
    "frozen_eligible_multiplicity_without_valid_vote",
];
const SESSION_SQL: &str = "SELECT to_jsonb(t) AS data FROM sequent_backend.tally_session t
    WHERE id = $1::text::uuid";
const EVENT_SQL: &str = "SELECT to_jsonb(e) AS data FROM sequent_backend.election_event e
    WHERE tenant_id = $1::text::uuid AND id = $2::text::uuid";
const CONTESTS_SQL: &str = "SELECT to_jsonb(t) AS data FROM sequent_backend.tally_session_contest t
    WHERE tenant_id = $1::text::uuid AND election_event_id = $2::text::uuid
      AND tally_session_id = $3::text::uuid
    ORDER BY election_id, area_id, contest_id NULLS FIRST, id";
const BOARD_BALLOT_BATCHES_SQL: &str =
    "SELECT DISTINCT batch FROM {relation} WHERE statement_kind = $1 ORDER BY batch";

/// Audit every area and contest of one tally with the ballot audit commands
/// and the Braid verifier, and write an audit record and a public summary.
#[derive(Parser)]
#[command(group(ArgGroup::new("electoral_log").required(true)
    .args(["immudb_server_url", "electoral_log_export"])))]
struct Args {
    /// Private JSON connection file of the restored Hasura, Keycloak and B3 copies.
    #[arg(long)]
    connections: PathBuf,
    /// The tally to audit, from the election authority.
    #[arg(long)]
    tally_session_id: String,
    /// The public key hash, from the election authority.
    #[arg(long)]
    public_key_sha512: String,
    /// The electoral log system public key, from the election authority.
    #[arg(long)]
    electoral_log_system_public_key: String,
    /// Loopback address of the restored ImmuDB.
    #[arg(long, requires = "immudb_username")]
    immudb_server_url: Option<String>,
    /// Read-only ImmuDB user.
    #[arg(long)]
    immudb_username: Option<String>,
    /// Environment variable that holds the ImmuDB password.
    #[arg(long, default_value = DEFAULT_PASSWORD_ENV)]
    immudb_password_env: String,
    /// Existing export-electoral-log directory of the full election event, used
    /// instead of reading ImmuDB.
    #[arg(long)]
    electoral_log_export: Option<PathBuf>,
    /// Address of the B3 server on the restored B3 copy. Without it the Braid
    /// verification does not run and the audit is incomplete.
    #[arg(long)]
    b3_server_url: Option<String>,
    /// The Braid verify program. Default: `verify` next to this program.
    #[arg(long)]
    braid_verify: Option<PathBuf>,
    /// Restored B3 blob root, when the board stores messages in files.
    #[arg(long)]
    b3_blob_root: Option<PathBuf>,
    /// Per-statement PostgreSQL timeout in seconds.
    #[arg(long, default_value_t = DEFAULT_STATEMENT_TIMEOUT_SECONDS, value_parser = clap::value_parser!(u32).range(1..))]
    statement_timeout_seconds: u32,
    /// New private directory. Existing paths are never overwritten.
    #[arg(long)]
    output: PathBuf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
enum StepStatus {
    Success,
    Incomplete,
    NotRun,
    InvalidInput,
    VerificationFailed,
}

impl StepStatus {
    fn from_exit_code(code: Option<i32>) -> Self {
        match code {
            Some(0) => StepStatus::Success,
            Some(3) => StepStatus::Incomplete,
            Some(2) | Some(4) => StepStatus::InvalidInput,
            _ => StepStatus::VerificationFailed,
        }
    }

    fn exit_status(self) -> ExitStatus {
        match self {
            StepStatus::Success => ExitStatus::Success,
            StepStatus::Incomplete | StepStatus::NotRun => ExitStatus::Incomplete,
            StepStatus::InvalidInput => ExitStatus::InvalidInput,
            StepStatus::VerificationFailed => ExitStatus::VerificationFailed,
        }
    }
}

/// A contradiction outranks invalid input, which outranks missing evidence.
fn overall_status<'a>(statuses: impl IntoIterator<Item = &'a StepStatus>) -> StepStatus {
    statuses
        .into_iter()
        .copied()
        .max()
        .unwrap_or(StepStatus::NotRun)
}

#[derive(Debug, Serialize)]
struct PlannedScope {
    directory: String,
    election_id: String,
    area_id: String,
    contest_id: Option<String>,
    tally_session_contest_id: String,
    base_batch: u32,
    batches: Vec<u32>,
    scope_sha512: String,
    #[serde(skip)]
    scope_bytes: Vec<u8>,
}

#[derive(Serialize)]
struct TallyPlan {
    format_version: u32,
    tenant_id: String,
    election_event_id: String,
    tally_session_id: String,
    board_name: String,
    policy: MultiplicityPolicy,
    scopes: Vec<PlannedScope>,
    tally_batches: Vec<u32>,
    board_ballot_batches: Vec<u32>,
    hasura_snapshot: Value,
    b3_snapshot: Value,
}

#[derive(Serialize)]
struct StepRecord {
    step: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    election_id: Option<String>,
    command: Vec<String>,
    exit_code: Option<i32>,
    status: StepStatus,
    outputs: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    message: String,
}

fn canonical_uuid(value: &str, label: &str) -> Result<()> {
    let parsed = Uuid::parse_str(value).map_err(|_| invalid_input(format!("invalid {label}")))?;
    ensure!(
        parsed.to_string() == value,
        invalid_input(format!("{label} must use canonical lowercase spelling"))
    );
    Ok(())
}

fn required_str(value: &Value, field: &str, what: &str) -> Result<String> {
    value[field]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| incomplete(format!("saved {what} lacks {field}")))
}

/// Builds one scope per saved tally contest row, with the same rules that
/// export-ballot-audit uses to check a scope against the saved tally.
fn plan_scopes(
    args: &Args,
    session: &Value,
    event: &Value,
    contests: &[Value],
) -> Result<(String, MultiplicityPolicy, Vec<PlannedScope>)> {
    let tenant_id = required_str(session, "tenant_id", "tally session")?;
    let election_event_id = required_str(session, "election_event_id", "tally session")?;
    let board_name = event["bulletin_board_reference"]["database_name"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| incomplete("saved election event lacks its board name"))?;
    board_identifier(&board_name).map_err(into_invalid_input)?;
    let configuration = session
        .get("configuration")
        .ok_or_else(|| invalid_input("saved session has no configuration column"))?;
    let (policy, _) = tally_policies(configuration)?;
    ensure!(
        !contests.is_empty(),
        incomplete("saved tally has no contest rows")
    );
    let mut scopes = Vec::with_capacity(contests.len());
    for (index, contest) in contests.iter().enumerate() {
        let base_batch = contest["session_id"]
            .as_u64()
            .and_then(|batch| u32::try_from(batch).ok())
            .ok_or_else(|| invalid_input("saved tally contest has an invalid batch"))?;
        let contest_id = match &contest["contest_id"] {
            Value::Null => None,
            Value::String(id) => Some(id.clone()),
            _ => {
                return Err(invalid_input(
                    "saved tally contest has an invalid contest ID",
                ))
            }
        };
        let scope = Scope {
            tenant_id: tenant_id.clone(),
            election_event_id: election_event_id.clone(),
            election_id: required_str(contest, "election_id", "tally contest")?,
            area_id: required_str(contest, "area_id", "tally contest")?,
            tally_session_id: args.tally_session_id.clone(),
            tally_session_contest_id: required_str(contest, "id", "tally contest")?,
            board_name: board_name.clone(),
            public_key_sha512: args.public_key_sha512.clone(),
            electoral_log_system_public_key: args.electoral_log_system_public_key.clone(),
            contest_id,
            base_batch,
            policy,
        };
        let batches = tally_batches(policy, base_batch, &contest["annotations"])?;
        let scope_bytes = serde_json::to_vec_pretty(
            &json!({"format_version":1, "scope":scope, "batches":batches}),
        )?;
        scopes.push(PlannedScope {
            directory: format!("{SCOPES_DIR}/{:03}", index + 1),
            election_id: scope.election_id,
            area_id: scope.area_id,
            contest_id: scope.contest_id,
            tally_session_contest_id: scope.tally_session_contest_id,
            base_batch,
            batches,
            scope_sha512: digest(&scope_bytes)?,
            scope_bytes,
        });
    }
    Ok((board_name, policy, scopes))
}

async fn read_hasura(
    config: Config,
    source: &ConnectionSource,
    tally_session_id: &str,
    timeout: u32,
) -> Result<(Value, Value, Vec<Value>, Value)> {
    let mut client = connect(config).await?;
    let transaction = start_snapshot(&mut client, timeout).await?;
    let snapshot = snapshot_metadata(&transaction, source).await?;
    let session = one(
        &transaction,
        "tally session",
        SESSION_SQL,
        &[&tally_session_id],
    )
    .await?;
    let tenant_id = required_str(&session, "tenant_id", "tally session")?;
    let event_id = required_str(&session, "election_event_id", "tally session")?;
    let event = one(
        &transaction,
        "election event",
        EVENT_SQL,
        &[&tenant_id, &event_id],
    )
    .await?;
    let contests = rows(
        &transaction,
        CONTESTS_SQL,
        &[&tenant_id, &event_id, &tally_session_id],
    )
    .await?;
    transaction
        .commit()
        .await
        .map_err(|_| invalid_input("cannot finish Hasura snapshot"))?;
    Ok((session, event, contests, snapshot))
}

async fn read_board_batches(
    config: Config,
    source: &ConnectionSource,
    board_name: &str,
    timeout: u32,
) -> Result<(Vec<u32>, Value)> {
    let relation = board_identifier(board_name).map_err(into_invalid_input)?;
    let mut client = connect(config).await?;
    let transaction = start_snapshot(&mut client, timeout).await?;
    let snapshot = snapshot_metadata(&transaction, source).await?;
    let records = transaction
        .query(
            &BOARD_BALLOT_BATCHES_SQL.replace("{relation}", &relation),
            &[&StatementType::Ballots.to_string()],
        )
        .await
        .map_err(|_| invalid_input("cannot read B3 board batches"))?;
    let batches = records
        .iter()
        .map(|row| {
            row.try_get::<_, i32>("batch")
                .ok()
                .and_then(|batch| u32::try_from(batch).ok())
                .ok_or_else(|| invalid_input("B3 board has an invalid batch"))
        })
        .collect::<Result<Vec<_>>>()?;
    transaction
        .commit()
        .await
        .map_err(|_| invalid_input("cannot finish B3 snapshot"))?;
    Ok((batches, snapshot))
}

struct Runner<'a> {
    output: &'a Path,
    tools: PathBuf,
    steps: Vec<StepRecord>,
}

impl Runner<'_> {
    fn tool(&self, name: &str) -> PathBuf {
        self.tools.join(name)
    }

    /// Shows paths inside the output directory relative to it.
    fn display(&self, arg: &Path) -> String {
        arg.strip_prefix(self.output)
            .map(|path| path.display().to_string())
            .unwrap_or_else(|_| arg.display().to_string())
    }

    fn digests(&self, paths: &[PathBuf]) -> BTreeMap<String, String> {
        let mut digests = BTreeMap::new();
        for path in paths {
            let files = if path.is_dir() {
                fs::read_dir(path)
                    .map(|entries| {
                        entries
                            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
                            .filter(|path| path.is_file())
                            .collect()
                    })
                    .unwrap_or_default()
            } else {
                vec![path.clone()]
            };
            for file in files {
                if let Some(sha) = fs::read(&file).ok().and_then(|bytes| digest(&bytes).ok()) {
                    digests.insert(self.display(&file), sha);
                }
            }
        }
        digests
    }

    /// Runs one command. With `report`, the standard output is the report and
    /// is saved as that file in the output directory.
    #[allow(clippy::too_many_arguments)]
    fn run(
        &mut self,
        step: &'static str,
        scope: Option<&str>,
        election_id: Option<&str>,
        program: &Path,
        args: Vec<std::ffi::OsString>,
        working_directory: Option<&Path>,
        report: Option<(&Path, &str)>,
        outputs: Vec<PathBuf>,
        status_of: fn(Option<i32>) -> StepStatus,
    ) -> StepStatus {
        let mut command = Command::new(program);
        command.args(&args);
        if let Some(directory) = working_directory {
            command.current_dir(directory);
        }
        let shown = std::iter::once(program.file_name().map_or_else(
            || program.display().to_string(),
            |name| name.to_string_lossy().into_owned(),
        ))
        .chain(args.iter().map(|arg| self.display(Path::new(arg))))
        .collect();
        let (exit_code, status, mut message, mut outputs) = match command.output() {
            Ok(result) => {
                let code = result.status.code();
                let mut message = String::from_utf8_lossy(&result.stderr).trim().to_string();
                let mut outputs = outputs;
                if let Some((directory, name)) = report {
                    if result.stdout.is_empty() {
                        message.push_str("\nno report on the standard output");
                    } else if save_bytes(directory, name, &result.stdout).is_ok() {
                        outputs.push(directory.join(name));
                    } else {
                        message.push_str("\ncannot save the report");
                    }
                }
                (code, status_of(code), message, outputs)
            }
            Err(_) => (
                None,
                StepStatus::InvalidInput,
                format!("cannot start {}", program.display()),
                vec![],
            ),
        };
        message = message.trim().to_string();
        outputs.retain(|path| path.exists());
        let outputs = self.digests(&outputs);
        self.push(StepRecord {
            step,
            scope: scope.map(str::to_string),
            election_id: election_id.map(str::to_string),
            command: shown,
            exit_code,
            status,
            outputs,
            message,
        });
        status
    }

    fn push(&mut self, record: StepRecord) {
        print_step(&record);
        self.steps.push(record);
    }

    fn not_run(
        &mut self,
        step: &'static str,
        scope: Option<&str>,
        election_id: Option<&str>,
        message: &str,
    ) {
        self.push(StepRecord {
            step,
            scope: scope.map(str::to_string),
            election_id: election_id.map(str::to_string),
            command: vec![],
            exit_code: None,
            status: StepStatus::NotRun,
            outputs: BTreeMap::new(),
            message: message.into(),
        });
    }
}

fn os(values: &[&dyn AsRef<std::ffi::OsStr>]) -> Vec<std::ffi::OsString> {
    values
        .iter()
        .map(|value| value.as_ref().to_owned())
        .collect()
}

fn file_pin(path: &Path) -> Option<String> {
    fs::read(path).ok().and_then(|bytes| digest(&bytes).ok())
}

fn read_json(path: &Path) -> Value {
    fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or(Value::Null)
}

fn scalar(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

/// Gives the title shown on the console for an output file, so that the
/// console output names no file.
fn file_title(path: &str) -> String {
    let path = Path::new(path);
    if path.parent().and_then(Path::file_name) == Some(OsStr::new(HISTORY_DIR)) {
        return "Cast-vote history report".to_string();
    }
    let name = path.file_stem().and_then(OsStr::to_str).unwrap_or_default();
    let mut title = match name {
        "verify" => "Braid verifier output".to_string(),
        _ => name.replace(['-', '_'], " "),
    };
    if let Some(first) = title.get_mut(..1) {
        first.make_ascii_uppercase();
    }
    title
}

/// Renders a JSON value as indented `key: value` lines, without the
/// `NOT_PRINTED` keys. Keys that name a file are shown by their title.
fn render(lines: &mut Vec<String>, indent: usize, key: &str, value: &Value) {
    let pad = " ".repeat(indent);
    match value {
        Value::Object(map) => {
            lines.push(format!("{pad}{key}:"));
            for (name, value) in map {
                if NOT_PRINTED.contains(&name.as_str()) {
                    continue;
                }
                if name.ends_with(".json") {
                    render(lines, indent + 2, &file_title(name), value);
                } else {
                    render(lines, indent + 2, name, value);
                }
            }
        }
        Value::Array(items) if items.is_empty() => lines.push(format!("{pad}{key}: none")),
        Value::Array(items)
            if items
                .iter()
                .all(|item| !item.is_object() && !item.is_array()) =>
        {
            let items: Vec<String> = items.iter().map(scalar).collect();
            if items.iter().any(|item| item.contains(' ')) {
                lines.push(format!("{pad}{key}:"));
                lines.extend(items.iter().map(|item| format!("{pad}  - {item}")));
            } else {
                lines.push(format!("{pad}{key}: {}", items.join(", ")));
            }
        }
        Value::Array(items) => {
            lines.push(format!("{pad}{key} ({}):", items.len()));
            for (index, item) in items.iter().enumerate() {
                render(lines, indent + 2, &format!("#{}", index + 1), item);
            }
        }
        _ => lines.push(format!("{pad}{key}: {}", scalar(value))),
    }
}

fn print_value(key: &str, value: &Value) {
    let mut lines = vec![];
    render(&mut lines, 0, key, value);
    for line in lines {
        println!("{line}");
    }
}

fn print_heading(title: &str) {
    println!();
    println!("{}", "=".repeat(RULE_WIDTH));
    println!("{title}");
    println!("{}", "=".repeat(RULE_WIDTH));
}

fn print_file(directory: &Path, name: &str) {
    let path = directory.join(name);
    let title = file_title(name);
    if path.is_file() {
        print_value(&title, &read_json(&path));
    } else {
        println!("{title}: not written");
    }
}

fn print_text_file(directory: &Path, name: &str) {
    let title = file_title(name);
    match fs::read(directory.join(name)) {
        Ok(bytes) => {
            println!("{title}:");
            for line in String::from_utf8_lossy(&bytes).lines() {
                println!("  {line}");
            }
        }
        Err(_) => println!("{title}: not written"),
    }
}

fn print_count(directory: &Path, name: &str, field: Option<&str>, what: &str) {
    let value = read_json(&directory.join(name));
    let items = field.map_or(&value, |field| &value[field]);
    match items.as_array() {
        Some(items) => println!("{what}: {}", items.len()),
        None => println!("{what}: not readable"),
    }
}

fn print_step(record: &StepRecord) {
    let target = record
        .scope
        .as_deref()
        .map(|scope| format!(" {scope}"))
        .or_else(|| {
            record
                .election_id
                .as_deref()
                .map(|election| format!(" election {election}"))
        })
        .unwrap_or_default();
    println!();
    println!("{}", "-".repeat(RULE_WIDTH));
    println!("Step {}{target}", record.step);
    println!("{}", "-".repeat(RULE_WIDTH));
    println!("Status: {:?}", record.status);
    println!(
        "Exit code: {}",
        record
            .exit_code
            .map_or_else(|| "none".to_string(), |code| code.to_string())
    );
    if !record.message.is_empty() {
        println!("Messages:");
        for line in record.message.lines() {
            println!("  {line}");
        }
    }
    if !record.outputs.is_empty() {
        println!("Output digests (SHA-512):");
        for (path, sha) in &record.outputs {
            println!("  {}: {sha}", file_title(path));
        }
    }
}

fn print_export(export: &Path) {
    print_file(export, "acquisition.json");
    print_file(export, "eligibility-reasons.json");
    print_count(
        export,
        "eligible-voters.json",
        Some("voters"),
        "Eligible voters",
    );
    print_file(export, "board-source.json");
    print_file(export, "saved-mapping.json");
    print_count(
        export,
        "cast-vote-history.json",
        None,
        "Stored votes of the election",
    );
    print_file(export, "cast-votes.json");
}

fn print_prepared(prepared: &Path) {
    print_file(prepared, "provenance.json");
    print_file(prepared, "anchors.json");
    print_file(prepared, "manifest.json");
    print_file(prepared, "selection.json");
}

/// Keeps only numbers and booleans, so that no ID, receipt or name can reach
/// the public summary.
fn counts_only(value: &Value) -> Value {
    match value {
        Value::Number(_) | Value::Bool(_) => value.clone(),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, value)| (key.clone(), counts_only(value)))
                .filter(|(_, value)| !value.is_null())
                .collect(),
        ),
        _ => Value::Null,
    }
}

fn summary_accounting(accounting: &Value) -> Value {
    let mut counts = counts_only(accounting);
    if let Some(map) = counts.as_object_mut() {
        map.retain(|key, _| !ACCOUNTING_NOT_IN_SUMMARY.contains(&key.as_str()));
    }
    counts
}

fn length(value: &Value) -> Value {
    value
        .as_array()
        .map_or(Value::Null, |items| json!(items.len()))
}

fn public_summary(
    plan: &TallyPlan,
    steps: &[StepRecord],
    output: &Path,
    histories: &BTreeMap<String, String>,
    status: StepStatus,
    tallied_votes_to_audit: usize,
) -> Value {
    let scope_status = |directory: &str| {
        overall_status(
            steps
                .iter()
                .filter(|step| step.scope.as_deref() == Some(directory))
                .map(|step| &step.status),
        )
    };
    let scopes: Vec<Value> = plan
        .scopes
        .iter()
        .map(|scope| {
            let report = read_json(&output.join(&scope.directory).join("audit-report.json"));
            let selection = read_json(
                &output
                    .join(&scope.directory)
                    .join(PREPARED_AUDIT_DIR)
                    .join("selection.json"),
            );
            json!({"election_id":scope.election_id, "area_id":scope.area_id,
                "contest_id":scope.contest_id, "batches":scope.batches,
                "status":scope_status(&scope.directory),
                "checked_cast_votes":report["checked_cast_votes"],
                "checked_batches":report["checked_batches"],
                "checked_electoral_log_messages":report["checked_electoral_log_messages"],
                "tallied_votes_to_audit":length(&report["votes_to_audit"]),
                "duplicate_selected_receipts":length(&report["duplicate_selected_receipts"]),
                "privacy_warnings":length(&report["privacy_warnings"]),
                "selection_accounting":summary_accounting(&selection["accounting"])})
        })
        .collect();
    let history: Vec<Value> = histories
        .iter()
        .map(|(election_id, file)| {
            let report = read_json(&output.join(file));
            let status = overall_status(
                steps
                    .iter()
                    .filter(|step| {
                        step.step == AUDIT_CAST_HISTORY
                            && step.election_id.as_deref() == Some(election_id.as_str())
                    })
                    .map(|step| &step.status),
            );
            json!({"election_id":election_id, "status":status,
                "outcome":report["outcome"].as_str(),
                "checked_hasura_rows":report["checked_hasura_rows"],
                "checked_signed_cast_vote_log_records":report["checked_signed_cast_vote_log_records"],
                "signed_log_records_without_hasura_row":report["signed_log_records_without_hasura_row"],
                "hasura_rows_without_signed_log":report["hasura_rows_without_signed_log"]})
        })
        .collect();
    let braid = overall_status(
        steps
            .iter()
            .filter(|step| step.step == BRAID_VERIFY)
            .map(|step| &step.status),
    );
    json!({"format_version":1, "source_revision":SOURCE_REVISION,
        "tenant_id":plan.tenant_id, "election_event_id":plan.election_event_id,
        "tally_session_id":plan.tally_session_id, "policy":plan.policy, "status":status,
        "tallied_votes_to_audit":tallied_votes_to_audit, "scopes":scopes, "cast_vote_history":history,
        "braid":{"status":braid, "board_name":plan.board_name,
            "tally_batches":plan.tally_batches,
            "board_ballot_batches":plan.board_ballot_batches}})
}

/// Collects the tallied votes with no signed CastVote record, one entry per
/// cast vote. A vote is tallied when a scope selected it; the cast-vote history
/// reports only add `HISTORY_FOUND_BY` to `found_by`. Entries carry voter IDs, so the
/// list is private.
fn tallied_votes_to_audit(
    scopes: &[PlannedScope],
    output: &Path,
    histories: &BTreeMap<String, String>,
    history_sources: &BTreeMap<String, (String, PathBuf)>,
) -> Vec<Value> {
    let mut found: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    for scope in scopes {
        let report = read_json(&output.join(&scope.directory).join("audit-report.json"));
        for vote in report["votes_to_audit"].as_array().into_iter().flatten() {
            if let Some(id) = vote["cast_vote_id"].as_str() {
                found
                    .entry((scope.election_id.clone(), id.to_string()))
                    .or_default()
                    .push(scope.directory.clone());
            }
        }
    }
    for (election_id, file) in histories {
        let report = read_json(&output.join(file));
        for receipt in report["receipts"].as_array().into_iter().flatten() {
            if receipt["classification"] != HASURA_ROW_WITHOUT_SIGNED_LOG {
                continue;
            }
            for id in receipt["cast_vote_ids"].as_array().into_iter().flatten() {
                if let Some(found_by) = id
                    .as_str()
                    .and_then(|id| found.get_mut(&(election_id.clone(), id.to_string())))
                {
                    found_by.push(HISTORY_FOUND_BY.to_string());
                }
            }
        }
    }
    let rows: BTreeMap<&String, Vec<Value>> = history_sources
        .iter()
        .map(|(election_id, (directory, _))| {
            let history = read_json(
                &output
                    .join(directory)
                    .join(POSTGRES_EXPORT_DIR)
                    .join("cast-vote-history.json"),
            );
            let rows = history.as_array().cloned().unwrap_or_default();
            (election_id, rows)
        })
        .collect();
    found
        .into_iter()
        .map(|((election_id, cast_vote_id), found_by)| {
            let row = rows
                .get(&election_id)
                .and_then(|rows| rows.iter().find(|row| row["id"] == cast_vote_id.as_str()))
                .cloned()
                .unwrap_or(Value::Null);
            json!({"election_id":election_id, "cast_vote_id":cast_vote_id,
                "voter_id":row["voter_id_string"], "ballot_id":row["ballot_id"],
                "area_id":row["area_id"], "status":row["status"],
                "created_at":row["created_at"], "found_by":found_by})
        })
        .collect()
}

fn tools_directory() -> Result<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .ok_or_else(|| invalid_input("cannot locate the ballot audit commands"))
}

async fn run(args: Args) -> Result<ExitStatus> {
    canonical_uuid(&args.tally_session_id, "tally session ID")?;
    validate_sha512_pin(&args.public_key_sha512, "public key")?;
    StrandSignaturePk::from_der_b64_string(&args.electoral_log_system_public_key)
        .map_err(|_| invalid_input("invalid electoral log system public key"))?;
    ensure!(
        !args.output.exists(),
        invalid_input("output directory already exists")
    );
    let tools = tools_directory()?;
    let connections = read_connections(&args.connections).map_err(into_invalid_input)?;
    let hasura_config =
        local_config(&connections.hasura, APPLICATION_NAME).map_err(into_invalid_input)?;
    let board_config =
        local_config(&connections.b3, APPLICATION_NAME).map_err(into_invalid_input)?;
    let timeout = args.statement_timeout_seconds;
    let (session, event, contests, hasura_snapshot) = read_hasura(
        hasura_config,
        &connections.hasura,
        &args.tally_session_id,
        timeout,
    )
    .await?;
    let (board_name, policy, scopes) = plan_scopes(&args, &session, &event, &contests)?;
    let (board_ballot_batches, b3_snapshot) =
        read_board_batches(board_config, &connections.b3, &board_name, timeout).await?;
    let tally_batches: Vec<u32> = scopes
        .iter()
        .flat_map(|scope| scope.batches.iter().copied())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let plan = TallyPlan {
        format_version: 1,
        tenant_id: required_str(&session, "tenant_id", "tally session")?,
        election_event_id: required_str(&session, "election_event_id", "tally session")?,
        tally_session_id: args.tally_session_id.clone(),
        board_name,
        policy,
        scopes,
        tally_batches,
        board_ballot_batches,
        hasura_snapshot,
        b3_snapshot,
    };

    let output = args.output.as_path();
    new_private_directory(output).map_err(into_invalid_input)?;
    new_private_directory(&output.join(SCOPES_DIR)).map_err(into_invalid_input)?;
    let started_at = chrono::Utc::now().to_rfc3339();
    let plan_file = save_json(output, "tally-plan.json", &plan)?;
    print_heading(&format!(
        "Ballot audit of tally session {}",
        args.tally_session_id
    ));
    println!("Source revision: {SOURCE_REVISION}");
    println!("Started at: {started_at}");
    print_value(
        "Inputs",
        &json!({"tally_session_id":args.tally_session_id,
            "public_key_sha512":args.public_key_sha512,
            "electoral_log_system_public_key":args.electoral_log_system_public_key,
            "restore_references":{"hasura":connections.hasura.restore_reference,
                "keycloak":connections.keycloak.restore_reference,
                "b3":connections.b3.restore_reference}}),
    );
    print_value("Tally plan", &read_json(&output.join("tally-plan.json")));
    let mut runner = Runner {
        output,
        tools,
        steps: vec![],
    };

    let electoral_log = match &args.electoral_log_export {
        Some(directory) => {
            let log = directory.join(ELECTORAL_LOG_FILE);
            let status = if log.is_file() {
                StepStatus::Success
            } else {
                StepStatus::InvalidInput
            };
            runner.push(StepRecord {
                step: EXPORT_ELECTORAL_LOG,
                scope: None,
                election_id: None,
                command: vec![],
                exit_code: None,
                status,
                outputs: runner.digests(std::slice::from_ref(&log)),
                message: "existing electoral log export".into(),
            });
            (status == StepStatus::Success).then_some(log)
        }
        None => {
            let directory = output.join(IMMUDB_EXPORT_DIR);
            let status = runner.run(
                EXPORT_ELECTORAL_LOG,
                None,
                None,
                &runner.tool(EXPORT_ELECTORAL_LOG),
                os(&[
                    &"--server-url",
                    &args.immudb_server_url.clone().unwrap_or_default(),
                    &"--database",
                    &plan.board_name,
                    &"--username",
                    &args.immudb_username.clone().unwrap_or_default(),
                    &"--password-env",
                    &args.immudb_password_env,
                    &"--election-event-id",
                    &plan.election_event_id,
                    &"--output",
                    &directory,
                ]),
                None,
                None,
                vec![directory.clone()],
                StepStatus::from_exit_code,
            );
            (status == StepStatus::Success).then(|| directory.join(ELECTORAL_LOG_FILE))
        }
    };
    if let Some(log) = &electoral_log {
        let directory = log.parent().unwrap_or(output);
        print_count(directory, ELECTORAL_LOG_FILE, None, "Electoral log records");
    }
    let log_pin = electoral_log.as_deref().and_then(file_pin);

    let mut history_sources: BTreeMap<String, (String, PathBuf)> = BTreeMap::new();
    for scope in &plan.scopes {
        let directory = output.join(&scope.directory);
        let label = Some(scope.directory.as_str());
        if new_private_directory(&directory).is_err()
            || save_bytes(&directory, "scope.json", &scope.scope_bytes).is_err()
        {
            runner.not_run(EXPORT_BALLOT_AUDIT, label, None, "cannot write scope.json");
            continue;
        }
        let scope_file = directory.join("scope.json");
        let export = directory.join(POSTGRES_EXPORT_DIR);
        let mut export_args = os(&[
            &"--scope",
            &scope_file,
            &"--scope-sha512",
            &scope.scope_sha512,
            &"--connections",
            &args.connections,
            &"--statement-timeout-seconds",
            &timeout.to_string(),
            &"--output",
            &export,
        ]);
        if let Some(root) = &args.b3_blob_root {
            export_args.extend(os(&[&"--b3-blob-root", root]));
        }
        let exported = runner.run(
            EXPORT_BALLOT_AUDIT,
            label,
            None,
            &runner.tool(EXPORT_BALLOT_AUDIT),
            export_args,
            None,
            None,
            vec![export.clone()],
            StepStatus::from_exit_code,
        );
        if exported != StepStatus::Success {
            runner.not_run(
                PREPARE_BALLOT_AUDIT,
                label,
                None,
                "the export did not succeed",
            );
            runner.not_run(BALLOT_AUDIT, label, None, "the export did not succeed");
            continue;
        }
        print_export(&export);
        let history = export.join("cast-vote-history.json");
        history_sources
            .entry(scope.election_id.clone())
            .or_insert_with(|| (scope.directory.clone(), scope_file.clone()));
        let (Some(log), Some(log_pin)) = (&electoral_log, &log_pin) else {
            runner.not_run(PREPARE_BALLOT_AUDIT, label, None, "no electoral log");
            runner.not_run(BALLOT_AUDIT, label, None, "no electoral log");
            continue;
        };
        let pinned = |name: &str| {
            let path = export.join(name);
            let pin = file_pin(&path).unwrap_or_default();
            (path, pin)
        };
        let (cast_votes, cast_votes_pin) = pinned("cast-votes.json");
        let (voters, voters_pin) = pinned("eligible-voters.json");
        let (board, board_pin) = pinned("board-messages.json");
        let (mapping, mapping_pin) = pinned("saved-mapping.json");
        let history_pin = file_pin(&history).unwrap_or_default();
        let prepared = directory.join(PREPARED_AUDIT_DIR);
        let status = runner.run(
            PREPARE_BALLOT_AUDIT,
            label,
            None,
            &runner.tool(PREPARE_BALLOT_AUDIT),
            os(&[
                &"--scope",
                &scope_file,
                &"--scope-sha512",
                &scope.scope_sha512,
                &"--cast-votes",
                &cast_votes,
                &"--cast-votes-sha512",
                &cast_votes_pin,
                &"--eligible-voters",
                &voters,
                &"--eligible-voters-sha512",
                &voters_pin,
                &"--board-messages",
                &board,
                &"--board-messages-sha512",
                &board_pin,
                &"--electoral-log",
                log,
                &"--electoral-log-sha512",
                log_pin,
                &"--cast-vote-history",
                &history,
                &"--cast-vote-history-sha512",
                &history_pin,
                &"--saved-mapping",
                &mapping,
                &"--saved-mapping-sha512",
                &mapping_pin,
                &"--output",
                &prepared,
            ]),
            None,
            None,
            vec![prepared.clone()],
            StepStatus::from_exit_code,
        );
        print_prepared(&prepared);
        if status != StepStatus::Success {
            runner.not_run(BALLOT_AUDIT, label, None, "the preparation did not succeed");
            continue;
        }
        let manifest = prepared.join("manifest.json");
        let configuration = prepared.join("configuration.borsh");
        runner.run(
            BALLOT_AUDIT,
            label,
            None,
            &runner.tool(BALLOT_AUDIT),
            os(&[
                &"--manifest",
                &manifest,
                &"--manifest-sha512",
                &file_pin(&manifest).unwrap_or_default(),
                &"--cast-votes",
                &prepared.join("cast-votes.json"),
                &"--configuration",
                &configuration,
                &"--configuration-sha512",
                &file_pin(&configuration).unwrap_or_default(),
                &"--messages",
                &prepared.join("messages.json"),
                &"--electoral-log",
                &prepared.join(ELECTORAL_LOG_FILE),
                &"--selection",
                &prepared.join("selection.json"),
            ]),
            None,
            Some((&directory, "audit-report.json")),
            vec![],
            StepStatus::from_exit_code,
        );
        print_file(&directory, "audit-report.json");
    }

    let elections: BTreeSet<&String> = plan.scopes.iter().map(|scope| &scope.election_id).collect();
    let history_directory = output.join(HISTORY_DIR);
    new_private_directory(&history_directory).map_err(into_invalid_input)?;
    let mut histories = BTreeMap::new();
    for election_id in elections {
        let election = Some(election_id.as_str());
        let Some((scope_directory, scope_file)) = history_sources.get(election_id) else {
            runner.not_run(AUDIT_CAST_HISTORY, None, election, "no successful export");
            continue;
        };
        let (Some(log), Some(log_pin)) = (&electoral_log, &log_pin) else {
            runner.not_run(AUDIT_CAST_HISTORY, None, election, "no electoral log");
            continue;
        };
        let history = output
            .join(scope_directory)
            .join(POSTGRES_EXPORT_DIR)
            .join("cast-vote-history.json");
        let name = format!("{election_id}.json");
        runner.run(
            AUDIT_CAST_HISTORY,
            None,
            election,
            &runner.tool(AUDIT_CAST_HISTORY),
            os(&[
                &"--scope",
                scope_file,
                &"--scope-sha512",
                &file_pin(scope_file).unwrap_or_default(),
                &"--cast-vote-history",
                &history,
                &"--cast-vote-history-sha512",
                &file_pin(&history).unwrap_or_default(),
                &"--electoral-log",
                log,
                &"--electoral-log-sha512",
                log_pin,
            ]),
            None,
            Some((&history_directory, &name)),
            vec![],
            StepStatus::from_exit_code,
        );
        print_file(&history_directory, &name);
        histories.insert(election_id.clone(), format!("{HISTORY_DIR}/{name}"));
    }

    let braid_batches: BTreeSet<u32> = plan
        .tally_batches
        .iter()
        .chain(&plan.board_ballot_batches)
        .copied()
        .collect();
    match &args.b3_server_url {
        Some(server_url) => {
            let braid_directory = output.join(BRAID_DIR);
            new_private_directory(&braid_directory).map_err(into_invalid_input)?;
            let program = args
                .braid_verify
                .clone()
                .unwrap_or_else(|| runner.tool(BRAID_VERIFY));
            let batches = braid_batches
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(",");
            runner.run(
                BRAID_VERIFY,
                None,
                None,
                &program,
                os(&[
                    &"--server-url",
                    server_url,
                    &"--board",
                    &plan.board_name,
                    &"--batches",
                    &batches,
                ]),
                Some(&braid_directory),
                Some((&braid_directory, "verify.log")),
                vec![],
                |code| {
                    if code == Some(0) {
                        StepStatus::Success
                    } else {
                        StepStatus::VerificationFailed
                    }
                },
            );
            print_text_file(&braid_directory, "verify.log");
        }
        None => runner.not_run(BRAID_VERIFY, None, None, "no B3 server address"),
    }

    let status = overall_status(runner.steps.iter().map(|step| &step.status));
    let votes = tallied_votes_to_audit(&plan.scopes, output, &histories, &history_sources);
    let votes_file = save_json(output, TALLIED_VOTES_TO_AUDIT_FILE, &votes)?;
    let summary = public_summary(
        &plan,
        &runner.steps,
        output,
        &histories,
        status,
        votes.len(),
    );
    let summary_file = save_json(output, "public-summary.json", &summary)?;
    let record = json!({"format_version":1, "source_revision":SOURCE_REVISION,
    "started_at":started_at, "finished_at":chrono::Utc::now().to_rfc3339(),
    "inputs":{"tally_session_id":args.tally_session_id,
        "public_key_sha512":args.public_key_sha512,
        "electoral_log_system_public_key":args.electoral_log_system_public_key,
        "restore_references":{"hasura":connections.hasura.restore_reference,
            "keycloak":connections.keycloak.restore_reference,
            "b3":connections.b3.restore_reference},
        "immudb_server_url":args.immudb_server_url,
        "electoral_log_export":args.electoral_log_export.as_ref().map(|p| p.display().to_string()),
        "b3_server_url":args.b3_server_url},
    "tally_plan":plan_file, "status":status, "steps":runner.steps,
    "braid_batches":braid_batches, "public_summary":summary_file,
    "tallied_votes_to_audit":votes_file,
    "limitations":[
        "The tally session ID, the public key hash, the electoral log system public key and the backup digests must come from the election authority, not from the restored copies.",
        "Each step keeps the limitations that its own command writes in its output files.",
        "Steps that did not run make the audit incomplete.",
        "Tallied votes to audit have no signed CastVote record and do not change the status: the audit is not finished until each listed vote is reviewed. Stored votes without a signed record that no scope selected are listed only in the cast-vote history reports."
    ]});
    let record_file = save_json(output, "audit-record.json", &record)?;
    print_heading("Results");
    print_value("Tallied votes to audit", &Value::Array(votes));
    print_value("Public summary", &summary);
    println!("Finished at: {}", scalar(&record["finished_at"]));
    println!();
    for scope in &plan.scopes {
        let scope_status = overall_status(
            runner
                .steps
                .iter()
                .filter(|step| step.scope.as_deref() == Some(scope.directory.as_str()))
                .map(|step| &step.status),
        );
        println!(
            "{}: election {} area {} contest {}: {:?}",
            scope.directory,
            scope.election_id,
            scope.area_id,
            scope.contest_id.as_deref().unwrap_or("all"),
            scope_status
        );
    }
    for (election_id, file) in &histories {
        let history_status = overall_status(
            runner
                .steps
                .iter()
                .filter(|step| step.election_id.as_deref() == Some(election_id.as_str()))
                .map(|step| &step.status),
        );
        let report = read_json(&output.join(file));
        println!(
            "Cast-vote history of election {election_id}: {history_status:?} (outcome {})",
            scalar(&report["outcome"])
        );
    }
    println!("Overall: {status:?}");
    println!(
        "Audit record SHA-512: {}",
        record_file["sha512"]
            .as_str()
            .context("missing audit record digest")?
    );
    Ok(status.exit_status())
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    match run(Args::parse()).await {
        Ok(status) => std::process::exit(status.code()),
        Err(error) => {
            eprintln!("audit-tally: {error}");
            std::process::exit(ExitStatus::of(&error).code());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TENANT: &str = "00000000-0000-4000-8000-000000000010";
    const EVENT: &str = "00000000-0000-4000-8000-000000000020";
    const ELECTION: &str = "00000000-0000-4000-8000-000000000030";
    const AREA: &str = "00000000-0000-4000-8000-000000000040";
    const SESSION: &str = "00000000-0000-4000-8000-000000000050";

    fn args() -> Args {
        Args::parse_from([
            "audit-tally",
            "--connections",
            "connections.json",
            "--tally-session-id",
            SESSION,
            "--public-key-sha512",
            &"7".repeat(128),
            "--electoral-log-system-public-key",
            "key",
            "--electoral-log-export",
            "immudb-export",
            "--output",
            "audit",
        ])
    }

    fn session(configuration: Value) -> Value {
        json!({"id":SESSION, "tenant_id":TENANT, "election_event_id":EVENT,
            "configuration":configuration})
    }

    fn event() -> Value {
        json!({"id":EVENT, "tenant_id":TENANT,
            "bulletin_board_reference":{"database_name":"audit_board"}})
    }

    fn contest(id: &str, area: &str, contest_id: Value, batch: u32, annotations: Value) -> Value {
        json!({"id":id, "election_id":ELECTION, "area_id":area, "contest_id":contest_id,
            "session_id":batch, "annotations":annotations})
    }

    #[test]
    fn plans_one_scope_per_saved_contest_row() {
        let contests = [
            contest(
                "00000000-0000-4000-8000-000000000061",
                AREA,
                Value::Null,
                10,
                json!({}),
            ),
            contest(
                "00000000-0000-4000-8000-000000000062",
                "00000000-0000-4000-8000-000000000041",
                Value::Null,
                11,
                json!({}),
            ),
        ];
        let (board, policy, scopes) = plan_scopes(
            &args(),
            &session(json!({"contest_encryption_policy":"multiple-contests"})),
            &event(),
            &contests,
        )
        .unwrap();
        assert_eq!(board, "audit_board");
        assert_eq!(policy, MultiplicityPolicy::Unweighted);
        assert_eq!(scopes.len(), 2);
        assert_eq!(scopes[0].directory, "scopes/001");
        assert_eq!(scopes[1].batches, vec![11]);
        let saved: Value = serde_json::from_slice(&scopes[0].scope_bytes).unwrap();
        assert_eq!(
            saved,
            json!({"format_version":1, "batches":[10], "scope":{"tenant_id":TENANT,
                "election_event_id":EVENT, "election_id":ELECTION, "area_id":AREA,
                "tally_session_id":SESSION,
                "tally_session_contest_id":"00000000-0000-4000-8000-000000000061",
                "board_name":"audit_board", "public_key_sha512":"7".repeat(128),
                "electoral_log_system_public_key":"key", "contest_id":null,
                "base_batch":10, "policy":"unweighted"}})
        );
        assert_eq!(
            scopes[0].scope_sha512,
            digest(&scopes[0].scope_bytes).unwrap()
        );
    }

    #[test]
    fn weighted_scopes_take_their_batches_from_the_weight_bit_mask() {
        let configuration = json!({"weighted_voting_policy":"voters-weighted-voting"});
        let contests = [contest(
            "00000000-0000-4000-8000-000000000061",
            AREA,
            json!("00000000-0000-4000-8000-000000000090"),
            20,
            json!({"weight_bit_mask": 6}),
        )];
        let (_, policy, scopes) =
            plan_scopes(&args(), &session(configuration), &event(), &contests).unwrap();
        assert_eq!(policy, MultiplicityPolicy::Weighted);
        assert_eq!(scopes[0].batches, vec![21, 22]);
        assert_eq!(
            scopes[0].contest_id.as_deref(),
            Some("00000000-0000-4000-8000-000000000090")
        );
    }

    #[test]
    fn plan_refuses_incomplete_or_unsupported_tallies() {
        let error = plan_scopes(&args(), &session(Value::Null), &event(), &[]).unwrap_err();
        assert_eq!(ExitStatus::of(&error), ExitStatus::Incomplete);
        let row = [contest(
            "00000000-0000-4000-8000-000000000061",
            AREA,
            Value::Null,
            0,
            json!({}),
        )];
        let mut no_board = event();
        no_board["bulletin_board_reference"] = Value::Null;
        let error = plan_scopes(&args(), &session(Value::Null), &no_board, &row).unwrap_err();
        assert_eq!(ExitStatus::of(&error), ExitStatus::Incomplete);
        let mut bad_board = event();
        bad_board["bulletin_board_reference"]["database_name"] = json!("board; DROP");
        let error = plan_scopes(&args(), &session(Value::Null), &bad_board, &row).unwrap_err();
        assert_eq!(ExitStatus::of(&error), ExitStatus::InvalidInput);
        let combined = json!({"weighted_voting_policy":"voters-weighted-voting",
            "delegated_voting_policy":"enabled"});
        let error = plan_scopes(&args(), &session(combined), &event(), &row).unwrap_err();
        assert_eq!(ExitStatus::of(&error), ExitStatus::InvalidInput);
        let mut negative = row.clone();
        negative[0]["session_id"] = json!(-1);
        let error = plan_scopes(&args(), &session(Value::Null), &event(), &negative).unwrap_err();
        assert_eq!(ExitStatus::of(&error), ExitStatus::InvalidInput);
    }

    #[test]
    fn overall_status_ranks_contradictions_first() {
        use StepStatus::*;
        assert_eq!(overall_status(&[Success, Success]), Success);
        assert_eq!(overall_status(&[Success, NotRun]), NotRun);
        assert_eq!(overall_status(&[Incomplete, InvalidInput]), InvalidInput);
        assert_eq!(
            overall_status(&[InvalidInput, VerificationFailed, Incomplete]),
            VerificationFailed
        );
        assert_eq!(overall_status(&[]), NotRun);
        assert_eq!(NotRun.exit_status(), ExitStatus::Incomplete);
        assert_eq!(StepStatus::from_exit_code(Some(0)), Success);
        assert_eq!(StepStatus::from_exit_code(Some(2)), InvalidInput);
        assert_eq!(StepStatus::from_exit_code(Some(3)), Incomplete);
        assert_eq!(StepStatus::from_exit_code(Some(101)), VerificationFailed);
        assert_eq!(StepStatus::from_exit_code(None), VerificationFailed);
    }

    #[test]
    fn tallied_votes_to_audit_leave_out_votes_no_scope_selected() {
        let output = std::env::temp_dir().join(format!(
            "audit-tally-votes-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        let scope = |directory: &str| PlannedScope {
            directory: directory.to_string(),
            election_id: ELECTION.to_string(),
            area_id: AREA.to_string(),
            contest_id: None,
            tally_session_contest_id: String::new(),
            base_batch: 10,
            batches: vec![10],
            scope_sha512: String::new(),
            scope_bytes: vec![],
        };
        let scopes = [scope("scopes/001"), scope("scopes/002")];
        let write = |path: PathBuf, value: Value| {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
        };
        write(
            output.join("scopes/001/audit-report.json"),
            json!({"votes_to_audit":[{"cast_vote_id":"cv-1","ballot_id":"b1",
                "reason":"no_signed_cast_vote_record"}]}),
        );
        write(
            output.join("scopes/001/postgres-export/cast-vote-history.json"),
            json!([{"id":"cv-1","voter_id_string":"voter-1","ballot_id":"b1",
                "area_id":AREA,"status":"valid","created_at":"2026-10-01T00:00:00Z"},
                {"id":"cv-2","voter_id_string":"voter-2","ballot_id":"b2"}]),
        );
        write(
            output.join("history/election.json"),
            json!({"receipts":[
                {"cast_vote_ids":["cv-1"],"classification":"hasura_row_without_signed_log"},
                {"cast_vote_ids":["cv-2"],"classification":"hasura_row_without_signed_log"},
                {"cast_vote_ids":["cv-3"],"classification":"matched"}]}),
        );
        let histories = BTreeMap::from([(ELECTION.to_string(), "history/election.json".into())]);
        let sources = BTreeMap::from([(
            ELECTION.to_string(),
            ("scopes/001".to_string(), PathBuf::new()),
        )]);
        let votes = tallied_votes_to_audit(&scopes, &output, &histories, &sources);
        fs::remove_dir_all(&output).unwrap();
        assert_eq!(votes.len(), 1);
        assert_eq!(votes[0]["cast_vote_id"], "cv-1");
        assert_eq!(votes[0]["voter_id"], "voter-1");
        assert_eq!(votes[0]["created_at"], "2026-10-01T00:00:00Z");
        assert_eq!(
            votes[0]["found_by"],
            json!(["scopes/001", "cast-vote history"])
        );
    }

    #[test]
    fn runner_saves_the_report_and_records_the_step() {
        let output = std::env::temp_dir().join(format!(
            "audit-tally-runner-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        new_private_directory(&output).unwrap();
        let mut runner = Runner {
            output: &output,
            tools: PathBuf::from("/nonexistent"),
            steps: vec![],
        };
        let status = runner.run(
            BALLOT_AUDIT,
            Some("scopes/001"),
            None,
            Path::new("/bin/sh"),
            os(&[
                &"-c",
                &"echo '{\"checked_cast_votes\":1}'; echo missing >&2; exit 3",
            ]),
            None,
            Some((&output, "audit-report.json")),
            vec![],
            StepStatus::from_exit_code,
        );
        assert_eq!(status, StepStatus::Incomplete);
        let step = &runner.steps[0];
        assert_eq!(step.exit_code, Some(3));
        assert_eq!(step.message, "missing");
        assert_eq!(step.command[0], "sh");
        let report = fs::read(output.join("audit-report.json")).unwrap();
        assert_eq!(step.outputs["audit-report.json"], digest(&report).unwrap());
        let status = runner.run(
            BRAID_VERIFY,
            None,
            None,
            &runner.tool(BRAID_VERIFY),
            vec![],
            None,
            None,
            vec![],
            StepStatus::from_exit_code,
        );
        assert_eq!(status, StepStatus::InvalidInput);
        runner.not_run(BALLOT_AUDIT, Some("scopes/002"), None, "no electoral log");
        assert_eq!(runner.steps[2].status, StepStatus::NotRun);
        fs::remove_dir_all(&output).unwrap();
    }

    #[test]
    fn console_output_leaves_out_voter_ids_and_ballot_contents() {
        let mut lines = vec![];
        render(
            &mut lines,
            0,
            "report",
            &json!({"checked":2, "batches":[10, 11], "empty":[], "cast-votes.json":"pin",
                "reasons":["First one.", "Second one."], "limitations":["Not proven."],
                "assurance":"What the check proves.", "note":"A note.",
                "rows":[{"cast_vote_id":"cv-1", "voter_id_string":"voter-1",
                    "content":"ballot"}],
                "voters":[{"voter_id":"voter-1"}]}),
        );
        assert_eq!(
            lines,
            [
                "report:",
                "  batches: 10, 11",
                "  Cast votes: pin",
                "  checked: 2",
                "  empty: none",
                "  reasons:",
                "    - First one.",
                "    - Second one.",
                "  rows (1):",
                "    #1:",
                "      cast_vote_id: cv-1",
            ]
        );
    }

    #[test]
    fn console_titles_name_no_file() {
        assert_eq!(
            file_title("scopes/001/postgres-export/cast-vote-history.json"),
            "Cast vote history"
        );
        assert_eq!(
            file_title("history/00000000-0000-4000-8000-000000000030.json"),
            "Cast-vote history report"
        );
        assert_eq!(file_title("braid/verify.log"), "Braid verifier output");
        assert_eq!(file_title("audit-report.json"), "Audit report");
        assert_eq!(file_title(""), "");
    }

    #[test]
    fn public_counts_drop_every_text_value() {
        let accounting = json!({"exported_rows":5, "voter":"alice",
            "rows_by_reason":{"selected":{"rows":1, "ballot_id":"receipt"}},
            "ids":["alice"], "complete":true});
        assert_eq!(
            counts_only(&accounting),
            json!({"exported_rows":5, "rows_by_reason":{"selected":{"rows":1}},
                "complete":true})
        );
        assert_eq!(
            summary_accounting(&json!({"exported_rows":5, "frozen_eligible_voters":3,
                "frozen_eligible_voters_without_valid_vote":1,
                "frozen_eligible_multiplicity_without_valid_vote":1})),
            json!({"exported_rows":5})
        );
        assert_eq!(length(&json!(["a", "b"])), json!(2));
        assert_eq!(length(&Value::Null), Value::Null);
    }
}
