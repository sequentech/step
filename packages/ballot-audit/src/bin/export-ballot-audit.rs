// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{bail, ensure, Context, Result};
use b3::messages::{message::Message, statement::StatementType};
use ballot_audit::{digest, incomplete, invalid_input, ExitStatus, MultiplicityPolicy, Scope};
use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine};
use clap::Parser;
use sequent_core::{
    ballot::{ContestEncryptionPolicy, DelegatedVotingPolicy, WeightedVotingPolicy},
    types::{
        hasura::core::TallySessionConfiguration,
        keycloak::{
            AREA_ID_ATTR_NAME, AUTHORIZED_ELECTION_IDS_NAME, DEFAULT_VOTE_WEIGHT, MAX_VOTE_WEIGHT,
            MIN_VOTE_WEIGHT, VOTE_WEIGHT_ATTR_NAME, VOTE_WEIGHT_BATCHES,
        },
    },
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    io::Write,
    net::IpAddr,
    path::{Path, PathBuf},
    time::Duration,
};
use strand::serialization::StrandDeserialize;
use tokio_postgres::{
    config::{Host, SslMode},
    types::ToSql,
    Client, Config, IsolationLevel, NoTls, Transaction,
};
use uuid::Uuid;

const DELEGATE_TO_ATTR_NAME: &str = "delegate-vote-to";
const SOURCE_REVISION: &str = "d6d05a1";
const SNAPSHOT_SQL: &str = "SELECT jsonb_build_object(
    'database', current_database(),
    'snapshot', txid_current_snapshot()::text,
    'transaction_started_at', transaction_timestamp(),
    'server_version', current_setting('server_version'),
    'max_identifier_length', current_setting('max_identifier_length'),
    'read_only', current_setting('transaction_read_only'),
    'row_security', current_setting('row_security'),
    'isolation', current_setting('transaction_isolation')) AS data";
const CAST_VOTES_SQL: &str = "SELECT to_jsonb(v) AS data
    FROM sequent_backend.cast_vote v
    WHERE tenant_id = $1::text::uuid AND election_event_id = $2::text::uuid
      AND election_id = $3::text::uuid AND area_id = $4::text::uuid
    ORDER BY created_at ASC NULLS FIRST, id ASC";
const CAST_VOTE_HISTORY_SQL: &str = "SELECT to_jsonb(v) AS data
    FROM sequent_backend.cast_vote v
    WHERE tenant_id = $1::text::uuid AND election_event_id = $2::text::uuid
      AND election_id = $3::text::uuid
    ORDER BY created_at ASC NULLS FIRST, id ASC";
// Only realm users assigned to the scope area can be eligible; others are not exported.
// Usernames are exported only when the delegated policy matches on them.
const USER_SOURCE_SQL: &str = "SELECT jsonb_build_object(
    'id', u.id, 'enabled', u.enabled,
    'service_account_client_link', u.service_account_client_link,
    'election_authorized', (
        NOT EXISTS (SELECT 1 FROM public.user_attribute a
            WHERE a.user_id = u.id AND a.name = $4)
        OR EXISTS (SELECT 1 FROM public.user_attribute a
            WHERE a.user_id = u.id AND a.name = $4
              AND (a.value = $5 OR a.value IS NULL))),
    'attributes', COALESCE((SELECT jsonb_agg(
        jsonb_build_object('id', a.id, 'name', a.name, 'value', a.value) ORDER BY a.id)
        FROM public.user_attribute a WHERE a.user_id = u.id AND a.name = ANY($6::text[])),
        '[]'::jsonb))
    || CASE WHEN $7 THEN jsonb_build_object('username', u.username) ELSE '{}'::jsonb END AS data
    FROM public.user_entity u WHERE u.realm_id = $1
      AND EXISTS (SELECT 1 FROM public.user_attribute a
          WHERE a.user_id = u.id AND a.name = $2 AND a.value = $3)
    ORDER BY u.id";
// This is the native tally predicate, including its global, per-attribute-row
// delegation count. Adding realm or enabled filters to delegators would change it.
const ELIGIBILITY_SQL: &str = "SELECT u.id,
    CASE WHEN $7 = 'weighted' THEN
        (SELECT COALESCE(MIN((NULLIF(a.value, ''))::bigint), $8::bigint)
         FROM public.user_attribute a WHERE a.user_id = u.id AND a.name = $9)
    WHEN $7 = 'delegated' THEN 1 +
        (SELECT COUNT(d.id) FROM public.user_entity d
         JOIN public.user_attribute a ON a.user_id = d.id
         WHERE d.service_account_client_link IS NULL AND a.name = $6
           AND a.value = u.username)
    ELSE 1 END AS multiplicity
    FROM public.user_entity u
    WHERE u.realm_id = $1 AND u.service_account_client_link IS NULL
      AND u.enabled IS TRUE
      AND EXISTS (SELECT 1 FROM public.user_attribute a
          WHERE a.user_id = u.id AND a.name = $2 AND a.value = $3)
      AND (NOT EXISTS (SELECT 1 FROM public.user_attribute a
              WHERE a.user_id = u.id AND a.name = $4)
          OR EXISTS (SELECT 1 FROM public.user_attribute a
              WHERE a.user_id = u.id AND a.name = $4
                AND (a.value = $5 OR a.value IS NULL)))
    ORDER BY u.id";
// Delegator rows of the scope realm that target a scope-area user.
const DELEGATION_SOURCE_SQL: &str = "SELECT jsonb_build_object(
    'user_id', d.id, 'enabled', d.enabled,
    'service_account_client_link', d.service_account_client_link,
    'attribute_id', a.id, 'delegate_to', a.value) AS data
    FROM public.user_entity d JOIN public.user_attribute a ON a.user_id = d.id
    WHERE d.realm_id = $1 AND a.name = $2 AND EXISTS (SELECT 1 FROM public.user_entity u
        WHERE u.realm_id = $1 AND u.username = a.value
          AND EXISTS (SELECT 1 FROM public.user_attribute ua
              WHERE ua.user_id = u.id AND ua.name = $3 AND ua.value = $4))
    ORDER BY d.id, a.id";
// Native counts delegators of every realm. Rows outside the scope realm are
// reported as per-delegate counts so that no foreign user ID is exported.
const CROSS_REALM_DELEGATION_SQL: &str = "SELECT jsonb_build_object(
    'delegate_user_id', u.id, 'counted_delegations', COUNT(a.id)) AS data
    FROM public.user_entity u
    JOIN public.user_attribute a ON a.name = $2 AND a.value = u.username
    JOIN public.user_entity d ON d.id = a.user_id
    WHERE u.realm_id = $1 AND d.realm_id IS DISTINCT FROM $1
      AND d.service_account_client_link IS NULL
      AND EXISTS (SELECT 1 FROM public.user_attribute ua
          WHERE ua.user_id = u.id AND ua.name = $3 AND ua.value = $4)
    GROUP BY u.id ORDER BY u.id";
// Prepare reads only the board configuration, public key statements and the
// scope's Ballots batches.
const BOARD_ROWS_SQL: &str = "SELECT id, sender_pk, statement_kind, batch, mix_number, message,
    version FROM {relation}
    WHERE statement_kind = ANY($1::text[]) OR (statement_kind = $2 AND batch = ANY($3::int4[]))
    ORDER BY id";
const BOARD_COUNT_SQL: &str = "SELECT count(*) AS total FROM {relation}";
const DEFAULT_STATEMENT_TIMEOUT_SECONDS: u32 = 120;

/// Export local restored PostgreSQL snapshots without application servers.
/// ImmuDB electoral-log acquisition is separate and remains required.
#[derive(Parser)]
struct Args {
    /// Saved scope envelope used by prepare-ballot-audit.
    #[arg(long)]
    scope: PathBuf,
    /// Independently authenticated SHA-512 of the exact saved scope file.
    #[arg(long)]
    scope_sha512: String,
    /// Private JSON connection file with per-database restore provenance.
    #[arg(long)]
    connections: PathBuf,
    /// Restored B3 blob root, required when a row has no message bytes.
    #[arg(long)]
    b3_blob_root: Option<PathBuf>,
    /// Per-statement PostgreSQL timeout in seconds for every snapshot query.
    #[arg(long, default_value_t = DEFAULT_STATEMENT_TIMEOUT_SECONDS, value_parser = clap::value_parser!(u32).range(1..))]
    statement_timeout_seconds: u32,
    /// New private directory. Existing paths are never overwritten.
    #[arg(long)]
    output: PathBuf,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedScope {
    format_version: u32,
    scope: Scope,
    batches: Vec<u32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Connections {
    format_version: u32,
    hasura: ConnectionSource,
    keycloak: ConnectionSource,
    b3: ConnectionSource,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConnectionSource {
    connection_string: String,
    restore_reference: String,
}

#[derive(Serialize)]
struct EligibleVoter {
    voter_id: String,
    multiplicity: u64,
}

struct HasuraExport {
    snapshot: Value,
    mapping: Value,
    cast_votes: Vec<Value>,
    cast_vote_history: Vec<Value>,
    election_alias: String,
}

struct KeycloakExport {
    snapshot: Value,
    source: Value,
    reasons: Value,
    voters: Vec<EligibleVoter>,
}

struct BoardExport {
    snapshot: Value,
    index: Value,
    messages: Vec<String>,
    rows: Vec<Value>,
    index_count: Value,
}

/// Keeps only the outer message: source chains can echo credentials or row values.
fn into_invalid_input(error: anyhow::Error) -> anyhow::Error {
    invalid_input(error.to_string())
}

fn local_config(source: &ConnectionSource) -> Result<Config> {
    ensure!(
        !source.restore_reference.trim().is_empty(),
        "restore reference is required"
    );
    let mut config: Config = source
        .connection_string
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid database connection configuration"))?;
    ensure!(
        config.get_hosts().len() == 1,
        "exactly one explicit local database host is required"
    );
    ensure!(
        config.get_hostaddrs().is_empty(),
        "hostaddr overrides are not allowed"
    );
    ensure!(
        config.get_ports().len() <= 1,
        "multiple database ports are not allowed"
    );
    ensure!(
        config.get_options().is_none(),
        "custom database startup options are not allowed"
    );
    ensure!(
        config.get_dbname().is_some_and(|v| !v.is_empty()),
        "explicit database name is required"
    );
    ensure!(
        config.get_user().is_some_and(|v| !v.is_empty()),
        "explicit database user is required"
    );
    ensure!(
        config.get_ssl_mode() != SslMode::Require,
        "this local-only exporter does not support TLS-required connections"
    );
    match &config.get_hosts()[0] {
        Host::Tcp(host) => {
            let ip: IpAddr = host.parse().context("use a loopback IP, not a hostname")?;
            ensure!(ip.is_loopback(), "remote database connections are refused");
        }
        #[cfg(unix)]
        Host::Unix(path) => ensure!(path.is_absolute(), "Unix socket directory must be absolute"),
        #[allow(unreachable_patterns)]
        _ => bail!("unsupported local database host"),
    }
    config.ssl_mode(SslMode::Disable);
    config.connect_timeout(Duration::from_secs(10));
    config.application_name("export-ballot-audit");
    config.options("-c default_transaction_read_only=on -c row_security=off");
    Ok(config)
}

async fn connect(config: Config) -> Result<Client> {
    let (client, connection) = config
        .connect(NoTls)
        .await
        .map_err(|_| invalid_input("local database connection failed"))?;
    tokio::spawn(async move {
        let _ = connection.await;
    });
    Ok(client)
}

fn snapshot_options(statement_timeout_seconds: u32) -> String {
    format!(
        "SET LOCAL statement_timeout = '{statement_timeout_seconds}s';
        SET LOCAL lock_timeout = '5s'; SET LOCAL row_security = off;
        SET LOCAL timezone = 'UTC'; SET LOCAL search_path = pg_catalog;"
    )
}

async fn start_snapshot(
    client: &mut Client,
    statement_timeout_seconds: u32,
) -> Result<Transaction<'_>> {
    let transaction = client
        .build_transaction()
        .isolation_level(IsolationLevel::RepeatableRead)
        .read_only(true)
        .start()
        .await
        .map_err(|_| invalid_input("cannot start read-only repeatable-read transaction"))?;
    transaction
        .batch_execute(&snapshot_options(statement_timeout_seconds))
        .await
        .map_err(|_| invalid_input("cannot set safe snapshot transaction options"))?;
    Ok(transaction)
}

async fn rows(
    transaction: &Transaction<'_>,
    sql: &str,
    params: &[&(dyn ToSql + Sync)],
) -> Result<Vec<Value>> {
    transaction
        .query(sql, params)
        .await
        .map_err(|_| {
            invalid_input("snapshot query failed; check restored schema and read permissions")
        })?
        .into_iter()
        .map(|row| {
            row.try_get("data")
                .map_err(|_| invalid_input("snapshot row has an unexpected schema"))
        })
        .collect()
}

fn single_row(mut values: Vec<Value>, what: &str) -> Result<Value> {
    match values.len() {
        1 => Ok(values.remove(0)),
        0 => Err(incomplete(format!(
            "expected exactly one {what} row, found none"
        ))),
        found => bail!("expected exactly one {what} row, found {found}"),
    }
}

async fn one(
    transaction: &Transaction<'_>,
    what: &str,
    sql: &str,
    params: &[&(dyn ToSql + Sync)],
) -> Result<Value> {
    single_row(rows(transaction, sql, params).await?, what)
}

async fn snapshot_metadata(
    transaction: &Transaction<'_>,
    source: &ConnectionSource,
) -> Result<Value> {
    let mut metadata = one(transaction, "snapshot metadata", SNAPSHOT_SQL, &[]).await?;
    ensure!(
        metadata["read_only"] == "on"
            && metadata["isolation"] == "repeatable read"
            && metadata["row_security"] == "off",
        invalid_input("database did not establish the required read-only repeatable-read snapshot")
    );
    metadata["restore_reference"] = json!(source.restore_reference);
    Ok(metadata)
}

fn validate_scope(saved: &SavedScope) -> Result<()> {
    ensure!(saved.format_version == 1, "unsupported scope version");
    let scope = &saved.scope;
    for id in [
        &scope.tenant_id,
        &scope.election_event_id,
        &scope.election_id,
        &scope.area_id,
        &scope.tally_session_id,
        &scope.tally_session_contest_id,
    ] {
        ensure!(
            Uuid::parse_str(id)
                .context("invalid scope UUID")?
                .to_string()
                == *id,
            "scope UUIDs must use canonical lowercase spelling"
        );
    }
    if let Some(id) = &scope.contest_id {
        ensure!(
            Uuid::parse_str(id)
                .context("invalid contest UUID")?
                .to_string()
                == *id,
            "contest UUID must use canonical lowercase spelling"
        );
    }
    board_identifier(&scope.board_name)?;
    // Windmill allocates batch 0 to the first contest-area of an event's first tally.
    ensure!(
        scope.base_batch <= i32::MAX as u32,
        "base batch exceeds the stored PostgreSQL integer range"
    );
    ensure!(!saved.batches.is_empty(), "saved batch set is empty");
    Ok(())
}

fn validate_mapping(saved: &SavedScope, mapping: &Value) -> Result<String> {
    let scope = &saved.scope;
    let contest = &mapping["tally_session_contest"];
    for (field, expected) in [
        ("id", &scope.tally_session_contest_id),
        ("tenant_id", &scope.tenant_id),
        ("election_event_id", &scope.election_event_id),
        ("election_id", &scope.election_id),
        ("area_id", &scope.area_id),
        ("tally_session_id", &scope.tally_session_id),
    ] {
        ensure!(
            contest[field].as_str() == Some(expected),
            "saved tally mapping differs from pinned scope"
        );
    }
    ensure!(
        contest["contest_id"] == json!(scope.contest_id),
        "saved contest ID differs from pinned scope"
    );
    ensure!(
        contest["session_id"].as_u64() == Some(scope.base_batch as u64),
        "saved base batch differs from pinned scope"
    );
    let ceremony = &mapping["keys_ceremony"];
    ensure!(
        ceremony["id"] == mapping["tally_session"]["keys_ceremony_id"]
            && ceremony["id"].is_string()
            && ceremony["tenant_id"].as_str() == Some(scope.tenant_id.as_str())
            && ceremony["election_event_id"].as_str() == Some(scope.election_event_id.as_str()),
        "saved keys ceremony differs from pinned session"
    );
    let default_ceremony = match ceremony.get("is_default") {
        Some(Value::Null) | Some(Value::Bool(true)) => true,
        Some(Value::Bool(false)) => false,
        _ => bail!(invalid_input(
            "saved keys ceremony lacks its default-board predicate"
        )),
    };
    ensure!(default_ceremony, invalid_input(
        "non-default keys-ceremony boards require separately authenticated environment-slug mapping; this exporter refuses to infer it"));
    ensure!(
        mapping["election_event"]["bulletin_board_reference"]["database_name"].as_str()
            == Some(scope.board_name.as_str()),
        "saved event board differs from pinned scope"
    );
    for (key, expected) in [
        ("election_event", &scope.election_event_id),
        ("election", &scope.election_id),
        ("tally_session", &scope.tally_session_id),
    ] {
        ensure!(
            mapping[key]["id"].as_str() == Some(expected)
                && mapping[key]["tenant_id"].as_str() == Some(scope.tenant_id.as_str()),
            "saved scope owner differs from pinned scope"
        );
    }
    for key in ["election", "tally_session"] {
        ensure!(
            mapping[key]["election_event_id"].as_str() == Some(scope.election_event_id.as_str()),
            "saved event ownership differs from pinned scope"
        );
    }
    let configuration = mapping["tally_session"]
        .get("configuration")
        .ok_or_else(|| invalid_input("saved session has no configuration column"))?;
    let configuration: Option<TallySessionConfiguration> =
        serde_json::from_value(configuration.clone())
            .map_err(|_| invalid_input("unsupported saved tally configuration"))?;
    let configuration = configuration.unwrap_or_default();
    let weighted =
        configuration.get_weighted_voting_policy() == WeightedVotingPolicy::VOTERS_WEIGHTED_VOTING;
    let delegated = configuration.get_delegated_voting_policy() == DelegatedVotingPolicy::ENABLED;
    ensure!(
        !(weighted && delegated),
        invalid_input("combined weighted and delegated voting is unsupported")
    );
    ensure!(
        matches!(
            (&scope.policy, weighted, delegated),
            (MultiplicityPolicy::Weighted, true, false)
                | (MultiplicityPolicy::Delegated, false, true)
                | (MultiplicityPolicy::Unweighted, false, false)
        ),
        "saved multiplicity policy differs from pinned scope"
    );
    let single =
        configuration.get_contest_encryption_policy() == ContestEncryptionPolicy::SINGLE_CONTEST;
    ensure!(
        single == scope.contest_id.is_some(),
        "saved contest encryption policy differs from pinned scope"
    );
    let expected_batches = if weighted {
        let mask = contest["annotations"]["weight_bit_mask"]
            .as_u64()
            .ok_or_else(|| incomplete("weighted saved mapping lacks a weight bit mask"))?;
        ensure!(
            mask > 0 && mask < (1u64 << VOTE_WEIGHT_BATCHES),
            invalid_input("invalid saved weight bit mask")
        );
        (0..VOTE_WEIGHT_BATCHES)
            .filter(|bit| mask & (1u64 << bit) != 0)
            .map(|bit| {
                scope
                    .base_batch
                    .checked_add(bit)
                    .context("saved batch overflow")
            })
            .collect::<Result<Vec<_>>>()?
    } else {
        vec![scope.base_batch]
    };
    let mut supplied_batches = saved.batches.clone();
    supplied_batches.sort_unstable();
    ensure!(
        supplied_batches == expected_batches,
        "saved batch set differs from pinned scope"
    );
    match &mapping["election"]["external_id"] {
        Value::Null => Ok(String::new()),
        Value::String(alias) => Ok(alias.clone()),
        _ => bail!(invalid_input("saved election alias has an unexpected type")),
    }
}

async fn export_hasura(
    config: Config,
    source: &ConnectionSource,
    saved: &SavedScope,
    statement_timeout_seconds: u32,
) -> Result<HasuraExport> {
    let mut client = connect(config).await?;
    let transaction = start_snapshot(&mut client, statement_timeout_seconds).await?;
    let snapshot = snapshot_metadata(&transaction, source).await?;
    let s = &saved.scope;
    let event = one(
        &transaction,
        "election event",
        "SELECT to_jsonb(e) AS data FROM sequent_backend.election_event e
        WHERE tenant_id = $1::text::uuid AND id = $2::text::uuid",
        &[&s.tenant_id, &s.election_event_id],
    )
    .await?;
    let election = one(&transaction, "election", "SELECT to_jsonb(e) AS data FROM sequent_backend.election e
        WHERE tenant_id = $1::text::uuid AND election_event_id = $2::text::uuid AND id = $3::text::uuid",
        &[&s.tenant_id, &s.election_event_id, &s.election_id]).await?;
    let session = one(&transaction, "tally session", "SELECT to_jsonb(t) AS data FROM sequent_backend.tally_session t
        WHERE tenant_id = $1::text::uuid AND election_event_id = $2::text::uuid AND id = $3::text::uuid",
        &[&s.tenant_id, &s.election_event_id, &s.tally_session_id]).await?;
    let contest = one(&transaction, "tally session contest", "SELECT to_jsonb(t) AS data FROM sequent_backend.tally_session_contest t
        WHERE tenant_id = $1::text::uuid AND election_event_id = $2::text::uuid AND id = $3::text::uuid",
        &[&s.tenant_id, &s.election_event_id, &s.tally_session_contest_id]).await?;
    let executions = rows(
        &transaction,
        "SELECT to_jsonb(t) AS data FROM sequent_backend.tally_session_execution t
        WHERE tenant_id = $1::text::uuid AND election_event_id = $2::text::uuid
          AND tally_session_id = $3::text::uuid ORDER BY created_at, id",
        &[&s.tenant_id, &s.election_event_id, &s.tally_session_id],
    )
    .await?;
    let ceremony_id = session["keys_ceremony_id"]
        .as_str()
        .ok_or_else(|| incomplete("saved session lacks a keys ceremony"))?;
    let ceremony = one(
        &transaction,
        "keys ceremony",
        "SELECT jsonb_build_object('id', id, 'tenant_id', tenant_id,
        'election_event_id', election_event_id, 'is_default', is_default) AS data
        FROM sequent_backend.keys_ceremony WHERE tenant_id = $1::text::uuid
          AND election_event_id = $2::text::uuid AND id = $3::text::uuid",
        &[&s.tenant_id, &s.election_event_id, &ceremony_id],
    )
    .await?;
    let mapping = json!({"election_event":event, "election":election, "tally_session":session,
        "tally_session_contest":contest, "tally_session_executions":executions, "keys_ceremony":ceremony});
    let election_alias = validate_mapping(saved, &mapping)?;
    let cast_votes = rows(
        &transaction,
        CAST_VOTES_SQL,
        &[
            &s.tenant_id,
            &s.election_event_id,
            &s.election_id,
            &s.area_id,
        ],
    )
    .await?;
    let cast_vote_history = rows(
        &transaction,
        CAST_VOTE_HISTORY_SQL,
        &[&s.tenant_id, &s.election_event_id, &s.election_id],
    )
    .await?;
    transaction
        .commit()
        .await
        .map_err(|_| invalid_input("cannot finish Hasura snapshot"))?;
    Ok(HasuraExport {
        snapshot,
        mapping,
        cast_votes,
        cast_vote_history,
        election_alias,
    })
}

fn eligibility_reasons(users: &[Value], voters: &[EligibleVoter]) -> Result<Value> {
    let mut selected: BTreeMap<&str, u64> = BTreeMap::new();
    for voter in voters {
        ensure!(
            selected
                .insert(&voter.voter_id, voter.multiplicity)
                .is_none(),
            "duplicate eligible voter"
        );
    }
    let mut seen = BTreeSet::new();
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    let mut decisions = Vec::with_capacity(users.len());
    for user in users {
        let id = user["id"]
            .as_str()
            .context("frozen source user lacks an ID")?;
        ensure!(seen.insert(id), "duplicate frozen source user");
        let mut reasons = Vec::new();
        if !user
            .get("service_account_client_link")
            .context("missing service-account source predicate")?
            .is_null()
        {
            reasons.push("service_account");
        }
        if user
            .get("enabled")
            .context("missing enabled source predicate")?
            != &Value::Bool(true)
        {
            reasons.push("not_enabled");
        }
        if !user["election_authorized"]
            .as_bool()
            .context("missing authorization source predicate")?
        {
            reasons.push("election_not_authorized");
        }
        let eligible = reasons.is_empty();
        ensure!(
            selected.contains_key(id) == eligible,
            "frozen eligibility contradicts source predicates"
        );
        if eligible {
            reasons.push("eligible");
        }
        for reason in &reasons {
            *counts.entry(reason).or_default() += 1;
        }
        decisions.push(
            json!({"voter_id":id, "eligible":eligible, "reasons":reasons,
            "multiplicity":selected.get(id)}),
        );
    }
    ensure!(
        selected.keys().all(|id| seen.contains(id)),
        "eligible voter missing from frozen source"
    );
    Ok(
        json!({"format_version":1, "source_users":users.len(), "eligible_users":voters.len(),
        "source_filter":"scope realm users assigned to the scope area",
        "reason_counts":counts, "counts_overlap_for_multiple_exclusions":true, "decisions":decisions}),
    )
}

/// Reconciles each native delegated multiplicity with the exported delegation
/// sources and reports counted delegations that the voter-facing rules would
/// not expect: delegators outside the scope realm or not enabled.
fn delegation_diagnostics(
    users: &[Value],
    voters: &[EligibleVoter],
    delegations: &[Value],
    cross_realm: &[Value],
) -> Result<Value> {
    let mut usernames = BTreeMap::new();
    for user in users {
        let id = user["id"]
            .as_str()
            .context("frozen source user lacks an ID")?;
        let username = user["username"]
            .as_str()
            .context("delegated source user lacks a username")?;
        usernames.insert(id, username);
    }
    let mut in_realm: BTreeMap<&str, u64> = BTreeMap::new();
    let mut not_enabled: BTreeMap<&str, u64> = BTreeMap::new();
    for delegation in delegations {
        if !delegation
            .get("service_account_client_link")
            .context("missing delegator service-account predicate")?
            .is_null()
        {
            continue;
        }
        let target = delegation["delegate_to"]
            .as_str()
            .context("delegation lacks its delegate username")?;
        *in_realm.entry(target).or_default() += 1;
        if delegation.get("enabled") != Some(&Value::Bool(true)) {
            *not_enabled.entry(target).or_default() += 1;
        }
    }
    let mut foreign: BTreeMap<&str, u64> = BTreeMap::new();
    for entry in cross_realm {
        let id = entry["delegate_user_id"]
            .as_str()
            .context("cross-realm delegation lacks its delegate ID")?;
        let count = entry["counted_delegations"]
            .as_u64()
            .context("cross-realm delegation lacks its count")?;
        ensure!(
            foreign.insert(id, count).is_none(),
            "duplicate cross-realm delegation count"
        );
    }
    let mut flagged = Vec::new();
    let (mut cross_realm_total, mut not_enabled_total) = (0, 0);
    for voter in voters {
        let username = usernames
            .get(voter.voter_id.as_str())
            .context("eligible voter missing from frozen source")?;
        let local = in_realm.get(username).copied().unwrap_or(0);
        let outside = foreign.get(voter.voter_id.as_str()).copied().unwrap_or(0);
        let disabled = not_enabled.get(username).copied().unwrap_or(0);
        ensure!(
            voter.multiplicity == 1 + local + outside,
            "native delegated multiplicity is not explained by the exported delegation source"
        );
        cross_realm_total += outside;
        not_enabled_total += disabled;
        if outside > 0 || disabled > 0 {
            flagged.push(
                json!({"voter_id":voter.voter_id, "cross_realm_delegations":outside,
                "not_enabled_delegations":disabled}),
            );
        }
    }
    Ok(json!({"cross_realm_delegations":cross_realm_total,
        "not_enabled_delegations":not_enabled_total,
        "flagged_voters":flagged}))
}

async fn export_keycloak(
    config: Config,
    connection: &ConnectionSource,
    scope: &Scope,
    alias: &str,
    statement_timeout_seconds: u32,
) -> Result<KeycloakExport> {
    let mut client = connect(config).await?;
    let transaction = start_snapshot(&mut client, statement_timeout_seconds).await?;
    let snapshot = snapshot_metadata(&transaction, connection).await?;
    let realm_name = format!(
        "tenant-{}-event-{}",
        scope.tenant_id, scope.election_event_id
    );
    let realm = one(
        &transaction,
        "realm",
        "SELECT jsonb_build_object('id', id, 'name', name) AS data
        FROM public.realm WHERE name = $1",
        &[&realm_name],
    )
    .await?;
    let realm_id = realm["id"].as_str().context("restored realm lacks an ID")?;
    let delegated = matches!(scope.policy, MultiplicityPolicy::Delegated);
    let policy_attribute = match scope.policy {
        MultiplicityPolicy::Unweighted => None,
        MultiplicityPolicy::Weighted => Some(VOTE_WEIGHT_ATTR_NAME),
        MultiplicityPolicy::Delegated => Some(DELEGATE_TO_ATTR_NAME),
    };
    let attrs: Vec<&str> = [AREA_ID_ATTR_NAME, AUTHORIZED_ELECTION_IDS_NAME]
        .into_iter()
        .chain(policy_attribute)
        .collect();
    let users = rows(
        &transaction,
        USER_SOURCE_SQL,
        &[
            &realm_id,
            &AREA_ID_ATTR_NAME,
            &scope.area_id,
            &AUTHORIZED_ELECTION_IDS_NAME,
            &alias,
            &attrs,
            &delegated,
        ],
    )
    .await?;
    let policy = scope.policy.as_str();
    let default_weight = DEFAULT_VOTE_WEIGHT as i64;
    let eligible_rows = transaction
        .query(
            ELIGIBILITY_SQL,
            &[
                &realm_id,
                &AREA_ID_ATTR_NAME,
                &scope.area_id,
                &AUTHORIZED_ELECTION_IDS_NAME,
                &alias,
                &DELEGATE_TO_ATTR_NAME,
                &policy,
                &default_weight,
                &VOTE_WEIGHT_ATTR_NAME,
            ],
        )
        .await
        .map_err(|_| invalid_input("native eligibility projection failed"))?;
    let voters = eligible_rows
        .into_iter()
        .map(|row| {
            let voter_id: String = row.try_get("id").context("invalid eligible voter ID")?;
            let value: i64 = row
                .try_get("multiplicity")
                .context("invalid native voter multiplicity")?;
            let multiplicity = u64::try_from(value).context("negative voter multiplicity")?;
            ensure!(multiplicity > 0, "zero voter multiplicity");
            if matches!(scope.policy, MultiplicityPolicy::Weighted) {
                ensure!(
                    (MIN_VOTE_WEIGHT..=MAX_VOTE_WEIGHT).contains(&multiplicity),
                    invalid_input("voter weight outside native range")
                );
            }
            Ok(EligibleVoter {
                voter_id,
                multiplicity,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let mut reasons = eligibility_reasons(&users, &voters)?;
    let (delegations, cross_realm) = if delegated {
        let params: [&(dyn ToSql + Sync); 4] = [
            &realm_id,
            &DELEGATE_TO_ATTR_NAME,
            &AREA_ID_ATTR_NAME,
            &scope.area_id,
        ];
        let delegations = rows(&transaction, DELEGATION_SOURCE_SQL, &params).await?;
        let cross_realm = rows(&transaction, CROSS_REALM_DELEGATION_SQL, &params).await?;
        reasons["delegation_diagnostics"] =
            delegation_diagnostics(&users, &voters, &delegations, &cross_realm)?;
        (delegations, cross_realm)
    } else {
        (Vec::new(), Vec::new())
    };
    let source = json!({"format_version":1, "realm":realm, "area_id":scope.area_id,
        "election_alias":alias, "policy":scope.policy, "users":users, "delegations":delegations,
        "cross_realm_delegation_counts":cross_realm,
        "native_predicate_revision":SOURCE_REVISION,
        "delegation_semantics":"Native counts matching delegate-vote-to attribute rows across all realms; only service accounts excluded; no enabled, area or election filter. Scope-realm delegator rows are listed; other realms are reported only as per-delegate counts."});
    transaction
        .commit()
        .await
        .map_err(|_| invalid_input("cannot finish Keycloak snapshot"))?;
    Ok(KeycloakExport {
        snapshot,
        source,
        reasons,
        voters,
    })
}

fn board_context_kinds() -> Vec<String> {
    [
        StatementType::Configuration,
        StatementType::PublicKey,
        StatementType::PublicKeySigned,
    ]
    .iter()
    .map(ToString::to_string)
    .collect()
}

fn board_relation_name(board: &str) -> Result<String> {
    ensure!(
        !board.is_empty()
            && board
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_')
            && board
                .bytes()
                .next()
                .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_'),
        "invalid B3 board SQL identifier"
    );
    // Native CREATE uses an unquoted name: PostgreSQL folds ASCII case and
    // truncates it to 63 bytes. The index and blob directory keep the full name.
    Ok(board[..board.len().min(63)].to_ascii_lowercase())
}

fn board_identifier(board: &str) -> Result<String> {
    Ok(format!("public.\"{}\"", board_relation_name(board)?))
}

fn message_bytes(
    database: Option<Vec<u8>>,
    root: Option<&Path>,
    board: &str,
    kind: &str,
    sender: &str,
    batch: i32,
    mix_number: i32,
) -> Result<(Vec<u8>, &'static str)> {
    let database = database.filter(|bytes| !bytes.is_empty());
    let blob = if let Some(root) = root {
        let root = root
            .canonicalize()
            .map_err(|_| invalid_input("cannot open restored B3 blob root"))?;
        let board_dir = root.join(board);
        let name = format!("{kind}-{sender}-{batch}-{mix_number}").replace('/', ":");
        let path = board_dir.join(name);
        match fs::symlink_metadata(&path) {
            Ok(_) => {
                let canonical = path
                    .canonicalize()
                    .map_err(|_| invalid_input("cannot resolve restored B3 blob"))?;
                let board_dir = board_dir
                    .canonicalize()
                    .map_err(|_| invalid_input("cannot resolve restored B3 board directory"))?;
                ensure!(
                    canonical.starts_with(&root) && canonical.starts_with(board_dir),
                    invalid_input("B3 blob escapes restored root")
                );
                Some(
                    fs::read(canonical)
                        .map_err(|_| invalid_input("cannot read restored B3 blob"))?,
                )
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(_) => bail!(invalid_input("cannot inspect restored B3 blob")),
        }
    } else {
        None
    };
    match (database, blob) {
        (Some(database), Some(blob)) => {
            ensure!(
                database == blob,
                "database and restored blob message bytes disagree"
            );
            Ok((database, "database_and_blob"))
        }
        (Some(database), None) => Ok((database, "database")),
        (None, Some(blob)) => {
            ensure!(!blob.is_empty(), incomplete("restored B3 blob is empty"));
            Ok((blob, "blob"))
        }
        (None, None) => bail!(incomplete(
            "B3 message bytes missing; restore the matching blob root"
        )),
    }
}

async fn export_board(
    config: Config,
    connection: &ConnectionSource,
    saved: &SavedScope,
    blob_root: Option<&Path>,
    statement_timeout_seconds: u32,
) -> Result<BoardExport> {
    let scope = &saved.scope;
    let relation = board_identifier(&scope.board_name).map_err(into_invalid_input)?;
    let mut client = connect(config).await?;
    let transaction = start_snapshot(&mut client, statement_timeout_seconds).await?;
    let snapshot = snapshot_metadata(&transaction, connection).await?;
    ensure!(snapshot["max_identifier_length"] == "63", invalid_input(
        "unsupported PostgreSQL max_identifier_length; native board relation mapping requires 63 bytes"));
    let index = one(
        &transaction,
        "board index",
        "SELECT to_jsonb(i) AS data FROM public.index i WHERE board_name = $1",
        &[&scope.board_name],
    )
    .await?;
    let relation_name = board_relation_name(&scope.board_name).map_err(into_invalid_input)?;
    let aliases = rows(
        &transaction,
        "SELECT to_jsonb(board_name) AS data FROM public.index
        WHERE translate(left(board_name, 63), 'ABCDEFGHIJKLMNOPQRSTUVWXYZ',
            'abcdefghijklmnopqrstuvwxyz') = $1",
        &[&relation_name],
    )
    .await?;
    ensure!(
        aliases == vec![json!(scope.board_name)],
        invalid_input("multiple indexed board names resolve to the same PostgreSQL relation")
    );
    let catalog = one(
        &transaction,
        "board catalog",
        "SELECT jsonb_build_object('kind', c.relkind::text,
        'row_security', c.relrowsecurity) AS data FROM pg_catalog.pg_class c
        JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
        WHERE n.nspname = 'public' AND c.relname = $1",
        &[&relation_name],
    )
    .await?;
    ensure!(
        catalog["kind"] == "r" && catalog["row_security"] == false,
        invalid_input("B3 board must be a regular table without row-level security")
    );
    let batches = saved
        .batches
        .iter()
        .map(|batch| i32::try_from(*batch))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| invalid_input("saved batch exceeds the stored PostgreSQL integer range"))?;
    // PostgreSQL cannot bind a relation name; this sole identifier is validated,
    // quoted, schema-qualified and checked against both the board index and catalog.
    let total: i64 = transaction
        .query_one(&BOARD_COUNT_SQL.replace("{relation}", &relation), &[])
        .await
        .and_then(|row| row.try_get("total"))
        .map_err(|_| invalid_input("cannot count B3 board rows"))?;
    let records = transaction
        .query(
            &BOARD_ROWS_SQL.replace("{relation}", &relation),
            &[
                &board_context_kinds(),
                &StatementType::Ballots.to_string(),
                &batches,
            ],
        )
        .await
        .map_err(|_| invalid_input("cannot read scoped B3 board rows"))?;
    ensure!(
        !records.is_empty(),
        incomplete("restored B3 board has no configuration or scoped ballots")
    );
    let mut messages = Vec::with_capacity(records.len());
    let mut exported_rows = Vec::with_capacity(records.len());
    for row in records {
        let id: i64 = row.try_get("id").context("invalid B3 row ID")?;
        let sender: String = row.try_get("sender_pk").context("missing B3 sender")?;
        let kind: String = row
            .try_get("statement_kind")
            .context("missing B3 statement kind")?;
        let batch: i32 = row.try_get("batch").context("missing B3 batch")?;
        let mix_number: i32 = row.try_get("mix_number").context("missing B3 mix number")?;
        let database: Option<Vec<u8>> = row
            .try_get("message")
            .context("invalid B3 message column")?;
        let version: Option<String> = row
            .try_get("version")
            .context("invalid B3 version column")?;
        let (bytes, storage) = message_bytes(
            database,
            blob_root,
            &scope.board_name,
            &kind,
            &sender,
            batch,
            mix_number,
        )?;
        let decoded = Message::strand_deserialize(&bytes)
            .map_err(|_| invalid_input("invalid serialized B3 message"))?;
        ensure!(
            decoded.sender.pk.to_der_b64_string()? == sender
                && decoded.statement.get_kind().to_string() == kind
                && i32::try_from(decoded.statement.get_batch_number())? == batch
                && i32::try_from(decoded.statement.get_mix_number())? == mix_number,
            "B3 signed bytes disagree with row index columns"
        );
        exported_rows.push(json!({"id":id, "sender_pk":sender, "statement_kind":kind,
            "batch":batch, "mix_number":mix_number, "version":version,
            "message_sha512":digest(&bytes)?, "storage":storage}));
        messages.push(STANDARD_NO_PAD.encode(bytes));
    }
    let stored_count = index["message_count"].as_u64();
    let index_count = json!({"stored":index["message_count"], "observed":total,
        "exported":messages.len(),
        "matches":stored_count.map(|count| i64::try_from(count).ok() == Some(total)),
        "authoritative":false,
        "note":"Native message_count is best-effort post-commit statistics; mismatch does not establish omitted rows. Only configuration, public key and the scope's Ballots statements are exported."});
    transaction
        .commit()
        .await
        .map_err(|_| invalid_input("cannot finish B3 snapshot"))?;
    Ok(BoardExport {
        snapshot,
        index,
        messages,
        rows: exported_rows,
        index_count,
    })
}

fn read_scope(path: &Path, pin: &str) -> Result<Vec<u8>> {
    ensure!(
        pin.len() == 128
            && pin
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        invalid_input("scope SHA-512 must be 128 lowercase hexadecimal characters")
    );
    let bytes = fs::read(path).map_err(|_| invalid_input("cannot read pinned scope"))?;
    ensure!(digest(&bytes)? == pin, "scope digest mismatch");
    Ok(bytes)
}

fn read_connections(path: &Path) -> Result<Connections> {
    let metadata = fs::symlink_metadata(path).context("cannot inspect private connections file")?;
    ensure!(
        metadata.is_file() && !metadata.file_type().is_symlink(),
        "connections must be a regular private file"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        ensure!(
            metadata.permissions().mode() & 0o077 == 0,
            "connections file must not grant group or other access; use mode 0600"
        );
    }
    #[cfg(not(unix))]
    bail!("private connection-file permissions require a Unix platform");
    let bytes = fs::read(path).context("cannot read private connections file")?;
    let connections: Connections = serde_json::from_slice(&bytes)
        .map_err(|_| anyhow::anyhow!("invalid private connections JSON"))?;
    ensure!(
        connections.format_version == 1,
        "unsupported connections format version"
    );
    Ok(connections)
}

fn new_private_directory(path: &Path) -> Result<()> {
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    #[cfg(not(unix))]
    bail!("private output permissions require a Unix platform");
    builder.create(path).context(
        "cannot create new private output directory; existing paths are never overwritten",
    )?;
    Ok(())
}

fn save_bytes(directory: &Path, name: &str, bytes: &[u8]) -> Result<Value> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(directory.join(name))
        .map_err(|_| invalid_input("cannot create new private export file"))?;
    file.write_all(bytes)
        .map_err(|_| invalid_input("cannot write export file"))?;
    file.sync_all()
        .map_err(|_| invalid_input("cannot sync export file"))?;
    Ok(json!({"sha512":digest(bytes)?, "bytes":bytes.len()}))
}

fn save_json<T: Serialize>(directory: &Path, name: &str, value: &T) -> Result<Value> {
    save_bytes(directory, name, &serde_json::to_vec_pretty(value)?)
}

async fn run(args: Args) -> Result<()> {
    let scope_bytes = read_scope(&args.scope, &args.scope_sha512)?;
    let saved: SavedScope = serde_json::from_slice(&scope_bytes)
        .map_err(|_| invalid_input("invalid saved scope envelope"))?;
    validate_scope(&saved).map_err(into_invalid_input)?;
    let connections = read_connections(&args.connections).map_err(into_invalid_input)?;
    let hasura_config = local_config(&connections.hasura).map_err(into_invalid_input)?;
    let keycloak_config = local_config(&connections.keycloak).map_err(into_invalid_input)?;
    let board_config = local_config(&connections.b3).map_err(into_invalid_input)?;
    new_private_directory(&args.output).map_err(into_invalid_input)?;
    let timeout = args.statement_timeout_seconds;
    let hasura = export_hasura(hasura_config, &connections.hasura, &saved, timeout).await?;
    let keycloak = export_keycloak(
        keycloak_config,
        &connections.keycloak,
        &saved.scope,
        &hasura.election_alias,
        timeout,
    )
    .await?;
    let board = export_board(
        board_config,
        &connections.b3,
        &saved,
        args.b3_blob_root.as_deref(),
        timeout,
    )
    .await?;
    let mut files = BTreeMap::new();
    files.insert(
        "scope.json",
        save_bytes(&args.output, "scope.json", &scope_bytes)?,
    );
    files.insert(
        "cast-votes.json",
        save_json(&args.output, "cast-votes.json", &hasura.cast_votes)?,
    );
    files.insert(
        "cast-vote-history.json",
        save_json(
            &args.output,
            "cast-vote-history.json",
            &hasura.cast_vote_history,
        )?,
    );
    files.insert(
        "saved-mapping.json",
        save_json(&args.output, "saved-mapping.json", &hasura.mapping)?,
    );
    files.insert(
        "keycloak-source.json",
        save_json(&args.output, "keycloak-source.json", &keycloak.source)?,
    );
    files.insert(
        "eligibility-reasons.json",
        save_json(&args.output, "eligibility-reasons.json", &keycloak.reasons)?,
    );
    files.insert(
        "eligible-voters.json",
        save_json(
            &args.output,
            "eligible-voters.json",
            &json!({"format_version":1,"scope_sha512":args.scope_sha512,"voters":keycloak.voters}),
        )?,
    );
    files.insert(
        "board-messages.json",
        save_json(&args.output, "board-messages.json", &board.messages)?,
    );
    files.insert(
        "board-source.json",
        save_json(
            &args.output,
            "board-source.json",
            &json!({"index":board.index,"rows":board.rows,"index_count":board.index_count}),
        )?,
    );
    let acquisition = json!({"format_version":1, "source_revision":SOURCE_REVISION,
    "status":"postgres_exports_complete_electoral_log_required", "files":files,
    "board_index_count":board.index_count,
    "snapshots":{"hasura":hasura.snapshot,"keycloak":keycloak.snapshot,"b3":board.snapshot},
    "counts":{"cast_vote_rows_including_revotes":hasura.cast_votes.len(),
        "cast_vote_history_rows_all_areas":hasura.cast_vote_history.len(),
        "eligible_voters":keycloak.voters.len(),"board_messages":board.messages.len()},
    "statement_timeout_seconds":timeout,
    "cast_vote_history_scope":{"tenant_id":saved.scope.tenant_id,
        "election_event_id":saved.scope.election_event_id,"election_id":saved.scope.election_id,
        "area_filter":null,"statuses":"all","revotes":"all",
        "snapshot":"same Hasura transaction as scoped cast-votes.json"},
    "limitations":[
        "Each PostgreSQL export has its own read-only repeatable-read snapshot; there is no cross-database atomic snapshot.",
        "Restored databases and blob files must represent the intended tally-time sources. Restore references are operator claims, not cryptographic proof of origin or historical completeness.",
        "B3 blob files are read without rewriting. Their filesystem snapshot is not atomic with PostgreSQL.",
        "SHA-512 values pin these captured files; authenticate the acquisition manifest independently before using its pins.",
        "ImmuDB is not PostgreSQL. Export the native signed electoral-log messages separately before running prepare-ballot-audit.",
        "This command acquires evidence; it does not verify electoral-log signatures, ballot inclusion, mixing or decryption."
    ]});
    let manifest = save_json(&args.output, "acquisition.json", &acquisition)?;
    if board.index_count["matches"] == false {
        eprintln!("Warning: B3 index message_count differs from the observed board row count.");
    }
    let delegation = &keycloak.reasons["delegation_diagnostics"];
    if delegation["cross_realm_delegations"].as_u64() > Some(0)
        || delegation["not_enabled_delegations"].as_u64() > Some(0)
    {
        eprintln!("Warning: native delegated multiplicities count delegators outside the event realm or not enabled; see eligibility-reasons.json.");
    }
    println!("PostgreSQL exports complete. Native electoral-log export is still required.");
    println!(
        "acquisition.json SHA-512: {}",
        manifest["sha512"]
            .as_str()
            .context("missing acquisition digest")?
    );
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run(Args::parse()).await {
        // Only the outer contextual message is printed; connection details and
        // database error chains can contain credentials or private row values.
        eprintln!("export-ballot-audit: {error}");
        std::process::exit(ExitStatus::of(&error).code());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn source(dsn: &str) -> ConnectionSource {
        ConnectionSource {
            connection_string: dsn.into(),
            restore_reference: "synthetic-restoration".into(),
        }
    }

    fn temp_path() -> PathBuf {
        std::env::temp_dir().join(format!(
            "ballot-export-unit-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn only_explicit_local_connections_are_accepted() {
        for dsn in [
            "host=127.0.0.1 user=test dbname=test",
            "host=::1 user=test dbname=test",
            "host=/tmp/local-socket user=test dbname=test",
        ] {
            let config = local_config(&source(dsn)).unwrap();
            assert_eq!(
                config.get_options(),
                Some("-c default_transaction_read_only=on -c row_security=off")
            );
        }
        for dsn in [
            "host=192.0.2.1 user=test dbname=test",
            "host=database.example user=test dbname=test",
            "host=localhost user=test dbname=test",
            "host=127.0.0.1,192.0.2.1 user=test dbname=test",
            "host=127.0.0.1 hostaddr=192.0.2.1 user=test dbname=test",
            "user=test dbname=test",
            "host=127.0.0.1 user=test dbname=test options='-c default_transaction_read_only=off'",
            "host=relative/socket user=test dbname=test",
        ] {
            assert!(local_config(&source(dsn)).is_err(), "accepted unsafe DSN");
        }
    }

    #[test]
    fn connection_error_does_not_echo_secrets() {
        let error =
            local_config(&source("password=synthetic-secret invalid-option=value")).unwrap_err();
        assert!(!error.to_string().contains("synthetic-secret"));
    }

    #[test]
    fn board_identifier_is_not_sql_input() {
        assert_eq!(
            board_identifier("Audit_Board").unwrap(),
            "public.\"audit_board\""
        );
        for name in [
            "",
            "board; DROP TABLE users",
            "public.board",
            "../board",
            "board\"",
            "123board",
        ] {
            assert!(board_identifier(name).is_err());
        }
        assert_eq!(
            board_relation_name(&"A".repeat(70)).unwrap(),
            "a".repeat(63)
        );
        assert!(board_identifier(&format!("{};DROP", "a".repeat(63))).is_err());
    }

    #[test]
    fn eligibility_reasons_require_complete_consistent_source() {
        let users = vec![
            json!({"id":"yes","enabled":true,"service_account_client_link":null,
            "election_authorized":true}),
            json!({"id":"no","enabled":false,"service_account_client_link":"client",
            "election_authorized":false}),
        ];
        let voters = vec![EligibleVoter {
            voter_id: "yes".into(),
            multiplicity: 1,
        }];
        let reasons = eligibility_reasons(&users, &voters).unwrap();
        assert_eq!(reasons["reason_counts"]["eligible"], 1);
        assert_eq!(reasons["reason_counts"]["not_enabled"], 1);
        assert_eq!(reasons["reason_counts"]["service_account"], 1);
        assert_eq!(reasons["reason_counts"]["election_not_authorized"], 1);
        assert!(eligibility_reasons(&users, &[]).is_err());
        assert!(eligibility_reasons(&[], &voters).is_err());
        assert!(eligibility_reasons(&[json!({"id":"missing-predicates"})], &[]).is_err());
    }

    const TENANT: &str = "00000000-0000-4000-8000-000000000010";
    const EVENT: &str = "00000000-0000-4000-8000-000000000020";
    const ELECTION: &str = "00000000-0000-4000-8000-000000000030";
    const AREA: &str = "00000000-0000-4000-8000-000000000040";
    const SESSION: &str = "00000000-0000-4000-8000-000000000050";
    const MAPPING: &str = "00000000-0000-4000-8000-000000000060";
    const CEREMONY: &str = "00000000-0000-4000-8000-000000000070";

    fn saved(base_batch: u32, policy: MultiplicityPolicy, batches: Vec<u32>) -> SavedScope {
        SavedScope {
            format_version: 1,
            scope: Scope {
                tenant_id: TENANT.into(),
                election_event_id: EVENT.into(),
                election_id: ELECTION.into(),
                area_id: AREA.into(),
                tally_session_id: SESSION.into(),
                tally_session_contest_id: MAPPING.into(),
                board_name: "audit_board".into(),
                public_key_sha512: String::new(),
                electoral_log_system_public_key: String::new(),
                contest_id: None,
                base_batch,
                policy,
            },
            batches,
        }
    }

    fn mapping(base_batch: u32, configuration: Value, annotations: Value) -> Value {
        json!({
            "election_event": {"id": EVENT, "tenant_id": TENANT,
                "bulletin_board_reference": {"database_name": "audit_board"}},
            "election": {"id": ELECTION, "tenant_id": TENANT, "election_event_id": EVENT,
                "external_id": "alias"},
            "tally_session": {"id": SESSION, "tenant_id": TENANT, "election_event_id": EVENT,
                "keys_ceremony_id": CEREMONY, "configuration": configuration},
            "tally_session_contest": {"id": MAPPING, "tenant_id": TENANT,
                "election_event_id": EVENT, "election_id": ELECTION, "area_id": AREA,
                "tally_session_id": SESSION, "contest_id": null, "session_id": base_batch,
                "annotations": annotations},
            "keys_ceremony": {"id": CEREMONY, "tenant_id": TENANT,
                "election_event_id": EVENT, "is_default": true},
        })
    }

    fn multiple_contests() -> Value {
        json!({"contest_encryption_policy": "multiple-contests"})
    }

    #[test]
    fn first_tally_batch_zero_is_a_valid_scope() {
        assert!(validate_scope(&saved(0, MultiplicityPolicy::Unweighted, vec![0])).is_ok());
        assert!(validate_scope(&saved(
            i32::MAX as u32,
            MultiplicityPolicy::Unweighted,
            vec![i32::MAX as u32]
        ))
        .is_ok());
        let error = validate_scope(&saved(
            i32::MAX as u32 + 1,
            MultiplicityPolicy::Unweighted,
            vec![0],
        ))
        .unwrap_err();
        assert!(error.to_string().contains("integer range"));
        assert!(validate_scope(&saved(0, MultiplicityPolicy::Unweighted, vec![])).is_err());
    }

    #[test]
    fn mapping_accepts_batch_zero_for_unweighted_and_weighted_scopes() {
        let unweighted = saved(0, MultiplicityPolicy::Unweighted, vec![0]);
        assert_eq!(
            validate_mapping(&unweighted, &mapping(0, multiple_contests(), json!({}))).unwrap(),
            "alias"
        );
        let weighted_configuration = json!({"contest_encryption_policy": "multiple-contests",
            "weighted_voting_policy": "voters-weighted-voting"});
        let weighted = saved(0, MultiplicityPolicy::Weighted, vec![1, 0]);
        assert!(validate_mapping(
            &weighted,
            &mapping(
                0,
                weighted_configuration.clone(),
                json!({"weight_bit_mask": 3})
            )
        )
        .is_ok());
        let wrong = saved(0, MultiplicityPolicy::Weighted, vec![1, 2]);
        let error = validate_mapping(
            &wrong,
            &mapping(
                0,
                weighted_configuration.clone(),
                json!({"weight_bit_mask": 3}),
            ),
        )
        .unwrap_err();
        assert_eq!(ExitStatus::of(&error), ExitStatus::VerificationFailed);
        let error = validate_mapping(&weighted, &mapping(0, weighted_configuration, json!({})))
            .unwrap_err();
        assert_eq!(ExitStatus::of(&error), ExitStatus::Incomplete);
    }

    #[test]
    fn mapping_errors_separate_contradictions_from_unsupported_input() {
        let scope = saved(0, MultiplicityPolicy::Unweighted, vec![0]);
        let error =
            validate_mapping(&scope, &mapping(1, multiple_contests(), json!({}))).unwrap_err();
        assert_eq!(ExitStatus::of(&error), ExitStatus::VerificationFailed);
        let mut non_default = mapping(0, multiple_contests(), json!({}));
        non_default["keys_ceremony"]["is_default"] = json!(false);
        let error = validate_mapping(&scope, &non_default).unwrap_err();
        assert_eq!(ExitStatus::of(&error), ExitStatus::InvalidInput);
        let combined = json!({"weighted_voting_policy": "voters-weighted-voting",
            "delegated_voting_policy": "enabled"});
        let error = validate_mapping(&scope, &mapping(0, combined, json!({}))).unwrap_err();
        assert_eq!(ExitStatus::of(&error), ExitStatus::InvalidInput);
    }

    #[test]
    fn single_row_names_the_expected_record() {
        assert_eq!(single_row(vec![json!(1)], "realm").unwrap(), json!(1));
        let error = single_row(vec![], "realm").unwrap_err();
        assert_eq!(ExitStatus::of(&error), ExitStatus::Incomplete);
        assert_eq!(
            error.to_string(),
            "expected exactly one realm row, found none"
        );
        let error = single_row(vec![json!(1), json!(2)], "keys ceremony").unwrap_err();
        assert_eq!(ExitStatus::of(&error), ExitStatus::VerificationFailed);
        assert!(error.to_string().contains("keys ceremony"));
    }

    #[test]
    fn acquisition_errors_are_invalid_input_without_source_chain() {
        let error = into_invalid_input(
            anyhow::anyhow!("password=synthetic-secret").context("invalid connection"),
        );
        assert_eq!(ExitStatus::of(&error), ExitStatus::InvalidInput);
        assert!(!format!("{error:#}").contains("synthetic-secret"));
        let path = temp_path();
        let error = read_scope(&path, "not-a-digest").unwrap_err();
        assert_eq!(ExitStatus::of(&error), ExitStatus::InvalidInput);
        fs::write(&path, b"{}").unwrap();
        let error = read_scope(&path, &"0".repeat(128)).unwrap_err();
        assert_eq!(ExitStatus::of(&error), ExitStatus::VerificationFailed);
        fs::remove_file(path).unwrap();
        let error = message_bytes(None, None, "board", "Ballots", "pk", 0, 0).unwrap_err();
        assert_eq!(ExitStatus::of(&error), ExitStatus::Incomplete);
    }

    #[test]
    fn statement_timeout_is_configurable() {
        assert!(snapshot_options(DEFAULT_STATEMENT_TIMEOUT_SECONDS)
            .contains("statement_timeout = '120s'"));
        assert!(snapshot_options(900).contains("statement_timeout = '900s'"));
        assert!(Args::try_parse_from([
            "export-ballot-audit",
            "--scope",
            "s",
            "--scope-sha512",
            "x",
            "--connections",
            "c",
            "--output",
            "o",
            "--statement-timeout-seconds",
            "0",
        ])
        .is_err());
    }

    #[test]
    fn source_queries_are_scoped() {
        assert!(USER_SOURCE_SQL.contains("WHERE u.realm_id = $1\n      AND EXISTS"));
        assert!(USER_SOURCE_SQL.contains("CASE WHEN $7 THEN jsonb_build_object('username'"));
        assert!(DELEGATION_SOURCE_SQL.contains("WHERE d.realm_id = $1 AND a.name = $2"));
        assert!(!CROSS_REALM_DELEGATION_SQL.contains("d.id,"));
        assert!(CROSS_REALM_DELEGATION_SQL.contains("d.realm_id IS DISTINCT FROM $1"));
        assert!(BOARD_ROWS_SQL.contains("batch = ANY($3::int4[])"));
        assert_eq!(
            board_context_kinds(),
            vec!["Configuration", "PublicKey", "PublicKeySigned"]
        );
        assert_eq!(StatementType::Ballots.to_string(), "Ballots");
    }

    #[test]
    fn delegation_diagnostics_explain_native_counts_and_flag_unexpected_delegators() {
        let users = vec![
            json!({"id":"alice-id","username":"alice"}),
            json!({"id":"bob-id","username":"bob"}),
        ];
        let delegations = vec![
            json!({"user_id":"bob-id","enabled":true,"service_account_client_link":null,
                "attribute_id":"a1","delegate_to":"alice"}),
            json!({"user_id":"carol-id","enabled":false,"service_account_client_link":null,
                "attribute_id":"a2","delegate_to":"alice"}),
            json!({"user_id":"svc-id","enabled":true,"service_account_client_link":"client",
                "attribute_id":"a3","delegate_to":"alice"}),
        ];
        let cross_realm = vec![json!({"delegate_user_id":"alice-id","counted_delegations":2})];
        let voters = vec![
            EligibleVoter {
                voter_id: "alice-id".into(),
                multiplicity: 5,
            },
            EligibleVoter {
                voter_id: "bob-id".into(),
                multiplicity: 1,
            },
        ];
        let diagnostics =
            delegation_diagnostics(&users, &voters, &delegations, &cross_realm).unwrap();
        assert_eq!(diagnostics["cross_realm_delegations"], 2);
        assert_eq!(diagnostics["not_enabled_delegations"], 1);
        assert_eq!(
            diagnostics["flagged_voters"],
            json!([{"voter_id":"alice-id","cross_realm_delegations":2,"not_enabled_delegations":1}])
        );
        let unexplained = vec![EligibleVoter {
            voter_id: "alice-id".into(),
            multiplicity: 6,
        }];
        let error =
            delegation_diagnostics(&users, &unexplained, &delegations, &cross_realm).unwrap_err();
        assert_eq!(ExitStatus::of(&error), ExitStatus::VerificationFailed);
        assert!(delegation_diagnostics(
            &[json!({"id":"alice-id"})],
            &voters[..1],
            &delegations,
            &cross_realm
        )
        .is_err());
    }

    #[test]
    fn private_files_preserve_content_and_refuse_overwrite() {
        use std::os::unix::fs::PermissionsExt;
        let path = temp_path();
        new_private_directory(&path).unwrap();
        let content = " \t{\"raw\" : \"α \\u0061\"}\r\n";
        let data = json!([{"content":content}]);
        save_json(&path, "rows.json", &data).unwrap();
        let decoded: Value =
            serde_json::from_slice(&fs::read(path.join("rows.json")).unwrap()).unwrap();
        assert_eq!(decoded[0]["content"].as_str(), Some(content));
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(path.join("rows.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        assert!(save_json(&path, "rows.json", &json!("overwrite")).is_err());
        assert!(new_private_directory(&path).is_err());
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn blob_fallback_is_exact_and_conflicts_fail_closed() {
        let root = temp_path();
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("board")).unwrap();
        let path = root.join("board").join("Ballots-pk:part-1-0");
        let bytes = vec![0, 1, 2, 255];
        fs::write(&path, &bytes).unwrap();
        assert_eq!(
            message_bytes(
                Some(vec![]),
                Some(&root),
                "board",
                "Ballots",
                "pk/part",
                1,
                0
            )
            .unwrap(),
            (bytes.clone(), "blob")
        );
        assert_eq!(
            message_bytes(
                Some(bytes.clone()),
                Some(&root),
                "board",
                "Ballots",
                "pk/part",
                1,
                0
            )
            .unwrap(),
            (bytes, "database_and_blob")
        );
        assert!(message_bytes(
            Some(vec![9]),
            Some(&root),
            "board",
            "Ballots",
            "pk/part",
            1,
            0
        )
        .is_err());
        assert!(message_bytes(None, None, "board", "Ballots", "pk/part", 1, 0).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn blob_symlink_cannot_escape_the_restored_root() {
        use std::os::unix::fs::symlink;
        let root = temp_path();
        let outside = temp_path();
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("board")).unwrap();
        fs::write(&outside, b"outside").unwrap();
        symlink(&outside, root.join("board/Ballots-pk-1-0")).unwrap();
        assert!(message_bytes(None, Some(&root), "board", "Ballots", "pk", 1, 0).is_err());
        fs::remove_dir_all(root).unwrap();
        fs::remove_file(outside).unwrap();
    }
}
