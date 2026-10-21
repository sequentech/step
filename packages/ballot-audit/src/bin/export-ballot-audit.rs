// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{bail, ensure, Context, Result};
use b3::messages::message::Message;
use ballot_audit::{digest, MultiplicityPolicy, Scope};
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
const USER_SOURCE_SQL: &str = "SELECT jsonb_build_object(
    'id', u.id, 'username', u.username, 'enabled', u.enabled,
    'service_account_client_link', u.service_account_client_link,
    'area_matches', EXISTS (SELECT 1 FROM public.user_attribute a
        WHERE a.user_id = u.id AND a.name = $2 AND a.value = $3),
    'election_authorized', (
        NOT EXISTS (SELECT 1 FROM public.user_attribute a
            WHERE a.user_id = u.id AND a.name = $4)
        OR EXISTS (SELECT 1 FROM public.user_attribute a
            WHERE a.user_id = u.id AND a.name = $4
              AND (a.value = $5 OR a.value IS NULL))),
    'attributes', COALESCE((SELECT jsonb_agg(
        jsonb_build_object('id', a.id, 'name', a.name, 'value', a.value) ORDER BY a.id)
        FROM public.user_attribute a WHERE a.user_id = u.id AND a.name = ANY($6::text[])),
        '[]'::jsonb)) AS data
    FROM public.user_entity u WHERE u.realm_id = $1 ORDER BY u.id";
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
const DELEGATION_SOURCE_SQL: &str = "SELECT jsonb_build_object(
    'user_id', d.id, 'realm_id', d.realm_id, 'enabled', d.enabled,
    'service_account_client_link', d.service_account_client_link,
    'attribute_id', a.id, 'delegate_to', a.value) AS data
    FROM public.user_entity d JOIN public.user_attribute a ON a.user_id = d.id
    WHERE a.name = $2 AND EXISTS (SELECT 1 FROM public.user_entity u
        WHERE u.realm_id = $1 AND u.username = a.value)
    ORDER BY d.id, a.id";

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
        .context("local database connection failed")?;
    tokio::spawn(async move {
        let _ = connection.await;
    });
    Ok(client)
}

async fn start_snapshot(client: &mut Client) -> Result<Transaction<'_>> {
    let transaction = client
        .build_transaction()
        .isolation_level(IsolationLevel::RepeatableRead)
        .read_only(true)
        .start()
        .await
        .context("cannot start read-only repeatable-read transaction")?;
    transaction
        .batch_execute(
            "SET LOCAL statement_timeout = '120s';
        SET LOCAL lock_timeout = '5s'; SET LOCAL row_security = off;
        SET LOCAL timezone = 'UTC'; SET LOCAL search_path = pg_catalog;",
        )
        .await
        .context("cannot set safe snapshot transaction options")?;
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
        .context("snapshot query failed; check restored schema and read permissions")?
        .into_iter()
        .map(|row| {
            row.try_get("data")
                .context("snapshot row has an unexpected schema")
        })
        .collect()
}

async fn one(
    transaction: &Transaction<'_>,
    sql: &str,
    params: &[&(dyn ToSql + Sync)],
) -> Result<Value> {
    let mut values = rows(transaction, sql, params).await?;
    ensure!(values.len() == 1, "expected exactly one saved mapping row");
    Ok(values.remove(0))
}

async fn snapshot_metadata(
    transaction: &Transaction<'_>,
    source: &ConnectionSource,
) -> Result<Value> {
    let mut metadata = one(transaction, SNAPSHOT_SQL, &[]).await?;
    ensure!(
        metadata["read_only"] == "on"
            && metadata["isolation"] == "repeatable read"
            && metadata["row_security"] == "off",
        "database did not establish the required read-only repeatable-read snapshot"
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
    ensure!(
        scope.base_batch > 0 && scope.base_batch <= i32::MAX as u32,
        "base batch is outside the stored PostgreSQL range"
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
        _ => bail!("saved keys ceremony lacks its default-board predicate"),
    };
    ensure!(default_ceremony,
        "non-default keys-ceremony boards require separately authenticated environment-slug mapping; this exporter refuses to infer it");
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
        .context("saved session has no configuration column")?;
    let configuration: Option<TallySessionConfiguration> =
        serde_json::from_value(configuration.clone())
            .context("unsupported saved tally configuration")?;
    let configuration = configuration.unwrap_or_default();
    let weighted =
        configuration.get_weighted_voting_policy() == WeightedVotingPolicy::VOTERS_WEIGHTED_VOTING;
    let delegated = configuration.get_delegated_voting_policy() == DelegatedVotingPolicy::ENABLED;
    ensure!(
        !(weighted && delegated),
        "combined weighted and delegated voting is unsupported"
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
            .context("weighted saved mapping lacks a weight bit mask")?;
        ensure!(
            mask > 0 && mask < (1u64 << VOTE_WEIGHT_BATCHES),
            "invalid saved weight bit mask"
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
        _ => bail!("saved election alias has an unexpected type"),
    }
}

async fn export_hasura(
    config: Config,
    source: &ConnectionSource,
    saved: &SavedScope,
) -> Result<HasuraExport> {
    let mut client = connect(config).await?;
    let transaction = start_snapshot(&mut client).await?;
    let snapshot = snapshot_metadata(&transaction, source).await?;
    let s = &saved.scope;
    let event = one(
        &transaction,
        "SELECT to_jsonb(e) AS data FROM sequent_backend.election_event e
        WHERE tenant_id = $1::text::uuid AND id = $2::text::uuid",
        &[&s.tenant_id, &s.election_event_id],
    )
    .await?;
    let election = one(&transaction, "SELECT to_jsonb(e) AS data FROM sequent_backend.election e
        WHERE tenant_id = $1::text::uuid AND election_event_id = $2::text::uuid AND id = $3::text::uuid",
        &[&s.tenant_id, &s.election_event_id, &s.election_id]).await?;
    let session = one(&transaction, "SELECT to_jsonb(t) AS data FROM sequent_backend.tally_session t
        WHERE tenant_id = $1::text::uuid AND election_event_id = $2::text::uuid AND id = $3::text::uuid",
        &[&s.tenant_id, &s.election_event_id, &s.tally_session_id]).await?;
    let contest = one(&transaction, "SELECT to_jsonb(t) AS data FROM sequent_backend.tally_session_contest t
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
        .context("saved session lacks a keys ceremony")?;
    let ceremony = one(
        &transaction,
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
        .context("cannot finish Hasura snapshot")?;
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
        if !user["area_matches"]
            .as_bool()
            .context("missing area source predicate")?
        {
            reasons.push("area_not_matched");
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
        "reason_counts":counts, "counts_overlap_for_multiple_exclusions":true, "decisions":decisions}),
    )
}

async fn export_keycloak(
    config: Config,
    connection: &ConnectionSource,
    scope: &Scope,
    alias: &str,
) -> Result<KeycloakExport> {
    let mut client = connect(config).await?;
    let transaction = start_snapshot(&mut client).await?;
    let snapshot = snapshot_metadata(&transaction, connection).await?;
    let realm_name = format!(
        "tenant-{}-event-{}",
        scope.tenant_id, scope.election_event_id
    );
    let realm = one(
        &transaction,
        "SELECT jsonb_build_object('id', id, 'name', name) AS data
        FROM public.realm WHERE name = $1",
        &[&realm_name],
    )
    .await?;
    let realm_id = realm["id"].as_str().context("restored realm lacks an ID")?;
    let attrs = vec![
        AREA_ID_ATTR_NAME,
        AUTHORIZED_ELECTION_IDS_NAME,
        VOTE_WEIGHT_ATTR_NAME,
        DELEGATE_TO_ATTR_NAME,
    ];
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
        ],
    )
    .await?;
    let policy = match scope.policy {
        MultiplicityPolicy::Unweighted => "unweighted",
        MultiplicityPolicy::Weighted => "weighted",
        MultiplicityPolicy::Delegated => "delegated",
    };
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
        .context("native eligibility projection failed")?;
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
                    "voter weight outside native range"
                );
            }
            Ok(EligibleVoter {
                voter_id,
                multiplicity,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let delegations = if matches!(scope.policy, MultiplicityPolicy::Delegated) {
        rows(
            &transaction,
            DELEGATION_SOURCE_SQL,
            &[&realm_id, &DELEGATE_TO_ATTR_NAME],
        )
        .await?
    } else {
        Vec::new()
    };
    let reasons = eligibility_reasons(&users, &voters)?;
    let source = json!({"format_version":1, "realm":realm, "area_id":scope.area_id,
        "election_alias":alias, "policy":scope.policy, "users":users, "delegations":delegations,
        "native_predicate_revision":SOURCE_REVISION,
        "delegation_semantics":"COUNT of matching delegate-vote-to attribute rows across all realms; only service accounts excluded; no enabled, area or election filter"});
    transaction
        .commit()
        .await
        .context("cannot finish Keycloak snapshot")?;
    Ok(KeycloakExport {
        snapshot,
        source,
        reasons,
        voters,
    })
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
            .context("cannot open restored B3 blob root")?;
        let board_dir = root.join(board);
        let name = format!("{kind}-{sender}-{batch}-{mix_number}").replace('/', ":");
        let path = board_dir.join(name);
        match fs::symlink_metadata(&path) {
            Ok(_) => {
                let canonical = path
                    .canonicalize()
                    .context("cannot resolve restored B3 blob")?;
                ensure!(
                    canonical.starts_with(&root)
                        && canonical.starts_with(
                            board_dir
                                .canonicalize()
                                .context("cannot resolve restored B3 board directory")?
                        ),
                    "B3 blob escapes restored root"
                );
                Some(fs::read(canonical).context("cannot read restored B3 blob")?)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(_) => bail!("cannot inspect restored B3 blob"),
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
            ensure!(!blob.is_empty(), "restored B3 blob is empty");
            Ok((blob, "blob"))
        }
        (None, None) => bail!("B3 message bytes missing; restore the matching blob root"),
    }
}

async fn export_board(
    config: Config,
    connection: &ConnectionSource,
    scope: &Scope,
    blob_root: Option<&Path>,
) -> Result<BoardExport> {
    let relation = board_identifier(&scope.board_name)?;
    let mut client = connect(config).await?;
    let transaction = start_snapshot(&mut client).await?;
    let snapshot = snapshot_metadata(&transaction, connection).await?;
    ensure!(snapshot["max_identifier_length"] == "63",
        "unsupported PostgreSQL max_identifier_length; native board relation mapping requires 63 bytes");
    let index = one(
        &transaction,
        "SELECT to_jsonb(i) AS data FROM public.index i WHERE board_name = $1",
        &[&scope.board_name],
    )
    .await?;
    let relation_name = board_relation_name(&scope.board_name)?;
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
        "multiple indexed board names resolve to the same PostgreSQL relation"
    );
    let catalog = one(
        &transaction,
        "SELECT jsonb_build_object('kind', c.relkind::text,
        'row_security', c.relrowsecurity) AS data FROM pg_catalog.pg_class c
        JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
        WHERE n.nspname = 'public' AND c.relname = $1",
        &[&relation_name],
    )
    .await?;
    ensure!(
        catalog["kind"] == "r" && catalog["row_security"] == false,
        "B3 board must be a regular table without row-level security"
    );
    // PostgreSQL cannot bind a relation name; this sole identifier is validated,
    // quoted, schema-qualified and checked against both the board index and catalog.
    let sql = format!(
        "SELECT id, sender_pk, statement_kind, batch, mix_number, message,
        version FROM {relation} ORDER BY id"
    );
    let records = transaction
        .query(&sql, &[])
        .await
        .context("cannot read complete B3 board")?;
    ensure!(!records.is_empty(), "restored B3 board is empty");
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
        let decoded =
            Message::strand_deserialize(&bytes).context("invalid serialized B3 message")?;
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
    let index_count = json!({"stored":index["message_count"], "observed":messages.len(),
        "matches":stored_count.map(|count| count == messages.len() as u64),
        "authoritative":false,
        "note":"Native message_count is best-effort post-commit statistics; mismatch does not establish omitted rows."});
    transaction
        .commit()
        .await
        .context("cannot finish B3 snapshot")?;
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
        "scope SHA-512 must be 128 lowercase hexadecimal characters"
    );
    let bytes = fs::read(path).context("cannot read pinned scope")?;
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
        .context("cannot create new private export file")?;
    file.write_all(bytes).context("cannot write export file")?;
    file.sync_all().context("cannot sync export file")?;
    Ok(json!({"sha512":digest(bytes)?, "bytes":bytes.len()}))
}

fn save_json<T: Serialize>(directory: &Path, name: &str, value: &T) -> Result<Value> {
    save_bytes(directory, name, &serde_json::to_vec_pretty(value)?)
}

async fn run(args: Args) -> Result<()> {
    let scope_bytes = read_scope(&args.scope, &args.scope_sha512)?;
    let saved: SavedScope =
        serde_json::from_slice(&scope_bytes).context("invalid saved scope envelope")?;
    validate_scope(&saved)?;
    let connections = read_connections(&args.connections)?;
    let hasura_config = local_config(&connections.hasura)?;
    let keycloak_config = local_config(&connections.keycloak)?;
    let board_config = local_config(&connections.b3)?;
    new_private_directory(&args.output)?;
    let hasura = export_hasura(hasura_config, &connections.hasura, &saved).await?;
    let keycloak = export_keycloak(
        keycloak_config,
        &connections.keycloak,
        &saved.scope,
        &hasura.election_alias,
    )
    .await?;
    let board = export_board(
        board_config,
        &connections.b3,
        &saved.scope,
        args.b3_blob_root.as_deref(),
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
        eprintln!("Warning: B3 index message_count is stale or inconsistent; all observed table rows were exported.");
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
        std::process::exit(1);
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
            "area_matches":true,"election_authorized":true}),
            json!({"id":"no","enabled":false,"service_account_client_link":"client",
            "area_matches":false,"election_authorized":false}),
        ];
        let voters = vec![EligibleVoter {
            voter_id: "yes".into(),
            multiplicity: 1,
        }];
        let reasons = eligibility_reasons(&users, &voters).unwrap();
        assert_eq!(reasons["reason_counts"]["eligible"], 1);
        assert_eq!(reasons["reason_counts"]["not_enabled"], 1);
        assert_eq!(reasons["reason_counts"]["service_account"], 1);
        assert_eq!(reasons["reason_counts"]["area_not_matched"], 1);
        assert_eq!(reasons["reason_counts"]["election_not_authorized"], 1);
        assert!(eligibility_reasons(&users, &[]).is_err());
        assert!(eligibility_reasons(&[], &voters).is_err());
        assert!(eligibility_reasons(&[json!({"id":"missing-predicates"})], &[]).is_err());
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
