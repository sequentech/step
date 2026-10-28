// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Windmill's Vitrine code against a running `vitrine-log` sidecar and its
//! witnesses, with the development deployment configuration and keys of
//! `.devcontainer/vitrine`. Ignored by default; it needs:
//!
//! - `WINDMILL_VITRINE_TEST_INTERNAL_URL`, the sidecar's internal listener;
//! - `WINDMILL_VITRINE_TEST_ADMIN_TOKEN_FILE`, its admin token;
//! - `WINDMILL_VITRINE_TEST_DATABASE_URL`, an election event's electoral-log
//!   database as the application's role, which the sidecar's configuration
//!   lists under the database's name;
//! - `ELECTORAL_LOG_PG_VITRINE_QUEUE_ROLE`, the role the sidecar grants the
//!   `ballots` log's queue to, which the application's role is a member of;
//! - `WINDMILL_VITRINE_TEST_WITNESS_ADMINS`, the witnesses' admin listeners,
//!   as `HARVEST_VITRINE_DEV_WITNESS_ADMINS` lists them.
//!
//! - `WINDMILL_VITRINE_TEST_CONFIG_FILE`, optional explicit deployment TOML
//!   naming the actual running infrastructure while retaining the Step fixture
//!   Board/operator authorities (defaults to `.devcontainer/vitrine/deployment.toml`).
//!
//! Each run makes the logs of a new event, and freezes them at the end.

use anyhow::{bail, ensure, Context, Result};
use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD};
use base64::Engine;
use chrono::Utc;
use electoral_log::adapters::ballot_box::{
    AcceptBallot, AcceptOutcome, BallotStatus, QueueState, VitrineQueue,
};
use electoral_log::adapters::postgres::{PostgresConnection, PostgresStore};
use electoral_log::vitrine::{MilestoneKind, MilestoneStatement, NoteSigner, Release, Submitted};
use sequent_core::ballot::TransparencyMode;
use serde_json::json;
use std::collections::BTreeMap;
use std::future::Future;
use std::time::{Duration, Instant};
use strum::IntoEnumIterator;
use uuid::Uuid;
use windmill::services::vitrine::client::{
    post_enrolment, read_token_file, Answer, CreatedLog, InstalledPolicy, SidecarClient,
    MAX_WAIT_MS,
};
use windmill::services::vitrine::config::DeploymentConfig;
use windmill::services::vitrine::policy::{
    draft, policy_name, pseudo_origin, DraftEvent, DraftLog, PolicyFacts,
};
use windmill::services::vitrine::statements::{
    check_authority, config_statement, election_config, milestone_statement, statement_chain,
    PORTAL_PACKAGE, VOTING_OPENED_KEY,
};
use windmill::services::vitrine::transparency::{dev_witness_admins, log_request};
use windmill::services::vitrine::voter::{ballot_entries, check_forward};
use windmill::services::vitrine::window::VotingWindow;
use windmill::services::vitrine::{sidecar_name, LogName};

const INTERNAL_URL_ENV: &str = "WINDMILL_VITRINE_TEST_INTERNAL_URL";
const ADMIN_TOKEN_FILE_ENV: &str = "WINDMILL_VITRINE_TEST_ADMIN_TOKEN_FILE";
const DATABASE_URL_ENV: &str = "WINDMILL_VITRINE_TEST_DATABASE_URL";
const WITNESS_ADMINS_ENV: &str = "WINDMILL_VITRINE_TEST_WITNESS_ADMINS";
const DEV_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../.devcontainer/vitrine");
/// The authority in the development configuration that seals and signs.
const BOARD: &str = "board";
/// How long the log and its witnesses get to receipt an entry.
const RECEIPT_TIMEOUT: Duration = Duration::from_secs(60);
const POLL: Duration = Duration::from_millis(500);
const VOTER: &str = "voter";

fn env(name: &str) -> Result<String> {
    std::env::var(name).with_context(|| format!("{name} isn't set"))
}

fn dev_file(path: &str) -> Result<String> {
    let path = format!("{DEV_DIR}/{path}");
    std::fs::read_to_string(&path).with_context(|| format!("Error reading {path}"))
}

fn deployment_config() -> Result<DeploymentConfig> {
    DeploymentConfig::load(
        &std::env::var("WINDMILL_VITRINE_TEST_CONFIG_FILE")
            .unwrap_or_else(|_| format!("{DEV_DIR}/deployment.toml")),
    )
}

/// Seals a draft as the Board's tool does (specs/policy.md, section 1):
/// without its empty and comment lines, and with a `seal` line of 16 random
/// bytes.
fn seal(draft: &str, authority: &str) -> String {
    let mut text: String = draft
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| format!("{line}\n"))
        .collect();
    let bytes: [u8; 16] = rand::random();
    text.push_str(&format!("seal {authority} {}\n", STANDARD.encode(bytes)));
    text
}

/// Calls `attempt` until it gives a value, for up to `RECEIPT_TIMEOUT`.
async fn eventually<T, F, Fut>(what: &str, mut attempt: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<Option<T>>>,
{
    let deadline = Instant::now() + RECEIPT_TIMEOUT;
    loop {
        if let Some(value) = attempt().await? {
            return Ok(value);
        }
        ensure!(
            Instant::now() < deadline,
            "no {what} after {RECEIPT_TIMEOUT:?}"
        );
        tokio::time::sleep(POLL).await;
    }
}

fn receipt_for(receipt: &str, entry: &[u8]) -> Result<()> {
    match Submitted::parse(receipt)? {
        Submitted::Receipt { entry: logged } if logged == entry => Ok(()),
        other => bail!("the receipt isn't for the entry: {other:?}"),
    }
}

/// Adds a statement's entry to the `events` log, as windmill does, and waits
/// for its receipt against a checkpoint that meets the quorum.
async fn log_statement(
    sidecar: &SidecarClient,
    name: &str,
    token: &str,
    entry_base64: &str,
    leaf_hex: &str,
) -> Result<String> {
    let entry = STANDARD.decode(entry_base64)?;
    match sidecar.add(name, token, &entry, MAX_WAIT_MS).await? {
        Answer::Receipt(receipt) => receipt_for(&receipt, &entry)?,
        Answer::Queued => {}
    }
    let receipt = eventually("receipt of the statement", || async {
        Ok(sidecar.receipt(name, leaf_hex).await?)
    })
    .await?;
    receipt_for(&receipt, &entry)?;
    Ok(receipt)
}

#[tokio::test]
#[ignore = "needs a vitrine-log sidecar and its witnesses: see the module's documentation"]
async fn a_public_event_logs_its_ballots_and_statements_on_the_sidecar() -> Result<()> {
    let config = deployment_config()?;
    let admin_token = read_token_file(&env(ADMIN_TOKEN_FILE_ENV)?)?;
    let sidecar = SidecarClient::new(&env(INTERNAL_URL_ENV)?, Some(admin_token))?;
    let database_config: tokio_postgres::Config = env(DATABASE_URL_ENV)?.parse()?;
    let database = database_config
        .get_dbname()
        .context("the database URL names no database")?
        .to_string();
    let queue_role = PostgresConnection::vitrine_queue_role();
    let witnesses = dev_witness_admins(&env(WITNESS_ADMINS_ENV)?)?;
    let tenant = Uuid::new_v4().to_string();
    let event = Uuid::new_v4().to_string();
    let election = Uuid::new_v4().to_string();
    let mode = TransparencyMode::PUBLIC;
    let visibility = mode.to_string();

    // The event's logs, as enable_transparency asks for them.
    let mut logs: BTreeMap<LogName, (String, String, CreatedLog)> = BTreeMap::new();
    for name in LogName::iter() {
        let path = sidecar_name(&tenant, &event, name)?;
        let origin = config.origin(&tenant, &event, name)?;
        let request = log_request(
            name,
            &path,
            &origin,
            &database,
            &visibility,
            queue_role.as_deref(),
        )?;
        let created = sidecar.create_log(&request).await?;
        logs.insert(name, (path, origin, created));
    }
    let draft_log = |name: LogName| {
        let (path, origin, created) = &logs[&name];
        DraftLog {
            origin: origin.clone(),
            vkey: created.vkey.clone(),
            log_time_vkey: created.log_time_vkey.clone(),
            blinded_vkey: None,
            url: created.url.clone(),
            path: path.clone(),
        }
    };
    let ballots_log = logs[&LogName::Ballots].0.clone();
    let events_log = logs[&LogName::Events].0.clone();
    let events_token = logs[&LogName::Events]
        .2
        .token
        .clone()
        .context("the events log has no submission token")?;

    // The policy: drafted from the configuration and the event, then sealed
    // and signed by the Board, and installed in both logs.
    let now = Utc::now().timestamp();
    let draft_text = draft(
        &config,
        &DraftEvent {
            name: policy_name(None, &tenant, &event)?,
            mode,
            election_event_id: event.clone(),
            election_ids: vec![election.clone()],
            window: VotingWindow {
                opens: now - 60,
                closes: now + 86_400,
            },
            language: None,
            policy_salt: None,
            ballots: draft_log(LogName::Ballots),
            events: draft_log(LogName::Events),
        },
    )?;
    let board = NoteSigner::from_private_key(&dev_file("secrets/board.key")?)?;
    let policy = board.sign(&seal(&draft_text, BOARD))?;
    let facts = PolicyFacts::read(&policy)?;
    let mut installed: BTreeMap<LogName, InstalledPolicy> = BTreeMap::new();
    for name in LogName::iter() {
        installed.insert(name, sidecar.put_policy(&logs[&name].0, &policy).await?);
    }
    let policy_id = installed[&LogName::Events].policy_id.clone();
    assert_eq!(installed[&LogName::Ballots].policy_id, policy_id);

    // The witnesses' operators post the logs' enrolment notes.
    for name in LogName::iter() {
        let note = sidecar
            .enrolment(&logs[&name].0)
            .await?
            .with_context(|| format!("the {name} log has no enrolment note"))?;
        for (url, token_file) in &witnesses {
            post_enrolment(url, &read_token_file(token_file)?, &note).await?;
        }
    }

    // A ballot: the ballot box queues its entry with it, the log receipts
    // it, and the receipt is one that Harvest forwards for this voter only.
    let store = PostgresStore::new(database_config)?;
    store.create_ballot_box(&event).await?;
    let ballot_id = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let accepted = store
        .accept_ballot(&AcceptBallot {
            election_event_id: &event,
            election_id: &election,
            area_id: &Uuid::new_v4().to_string(),
            voter_id: VOTER,
            ballot_id: &ballot_id,
            format: "test",
            content: "ciphertext",
            voter_signature: None,
            pseudonym_hash: &[1; 64],
            ballot_hash: &[2; 64],
            voting_channel: "ONLINE",
            status: BallotStatus::Valid,
            voter_ip: None,
            voter_country: None,
            username: Some(VOTER),
            allowed_votes: 0,
            vitrine: Some(VitrineQueue {
                log_id: logs[&LogName::Ballots].2.id,
                accepted_at: now,
            }),
        })
        .await?;
    let queued = match accepted {
        AcceptOutcome::Accepted {
            vitrine: Some(queued),
            ..
        } => queued,
        other => bail!("the ballot wasn't accepted with its entry: {other:?}"),
    };
    assert_eq!(queued.queue, QueueState::Queued);
    let leaf_hex = hex::encode(queued.leaf_hash);
    let receipt = eventually("receipt of the ballot", || async {
        match sidecar.wait(&ballots_log, &leaf_hex, MAX_WAIT_MS).await? {
            Answer::Receipt(receipt) => Ok(Some(receipt)),
            Answer::Queued => Ok(None),
        }
    })
    .await?;
    receipt_for(&receipt, &queued.entry)?;
    let own = ballot_entries(&event, &store.voter_ballots(&event, VOTER).await?)?;
    assert_eq!(own, vec![queued.entry.clone()]);
    assert!(check_forward("upgrade", &receipt, &own).is_ok());
    assert!(check_forward("upgrade", &receipt, &[]).is_err());

    // The election's config statement, then voting-opened with its leaf.
    let operator = NoteSigner::from_private_key(&dev_file("secrets/operator.key")?)?;
    check_authority(&operator, &policy)?;
    let style = json!({
        "public_key": { "public_key": STANDARD_NO_PAD.encode([7_u8; 32]), "is_demo": false }
    })
    .to_string();
    let config_row = config_statement(
        &operator,
        &event,
        &election_config(&election, &[style.as_str()])?,
        &facts,
        &policy_id,
        &Release {
            package: PORTAL_PACKAGE.to_string(),
            version: "0.0.0-test".to_string(),
        },
        now as u64,
    )?;
    let config_receipt = log_statement(
        &sidecar,
        &events_log,
        &events_token,
        &config_row.entry,
        &config_row.leaf_hash,
    )
    .await?;
    let config_leaf = hex::decode(&config_row.leaf_hash)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("a leaf hash has 32 bytes"))?;
    let opened_row = milestone_statement(
        &operator,
        VOTING_OPENED_KEY.to_string(),
        &MilestoneStatement {
            event_id: event.clone(),
            kind: MilestoneKind::VotingOpened,
            time: Utc::now().timestamp() as u64,
            configs: vec![(election.clone(), config_leaf)],
            counts: vec![],
        },
    )?;
    let opened_receipt = log_statement(
        &sidecar,
        &events_log,
        &events_token,
        &opened_row.entry,
        &opened_row.leaf_hash,
    )
    .await?;
    let ack = installed[&LogName::Events].ack.as_str();
    for receipt in [&config_receipt, &opened_receipt] {
        statement_chain(&policy_id, Some(ack), receipt)?;
    }

    for name in LogName::iter() {
        sidecar.freeze(&logs[&name].0).await?;
    }
    store.drop_ballot_box(&event).await?;
    Ok(())
}

#[tokio::test]
#[ignore = "needs a vitrine-log sidecar: see the module's documentation"]
async fn a_private_event_provisions_both_blinded_keys_and_exposes_authenticated_policy_history(
) -> Result<()> {
    let config = deployment_config()?;
    let admin_token = read_token_file(&env(ADMIN_TOKEN_FILE_ENV)?)?;
    let sidecar = SidecarClient::new(&env(INTERNAL_URL_ENV)?, Some(admin_token))?;
    let database_config: tokio_postgres::Config = env(DATABASE_URL_ENV)?.parse()?;
    let database = database_config
        .get_dbname()
        .context("the database URL names no database")?;
    let tenant = Uuid::new_v4().to_string();
    let event = Uuid::new_v4().to_string();
    let election = Uuid::new_v4().to_string();
    let salt = hex::encode(rand::random::<[u8; 32]>());
    let queue_role = PostgresConnection::vitrine_queue_role();
    let mut logs = BTreeMap::new();
    for name in LogName::iter() {
        let path = sidecar_name(&tenant, &event, name)?;
        let origin = config.origin(&tenant, &event, name)?;
        let created = sidecar
            .create_log(&log_request(
                name,
                &path,
                &origin,
                database,
                "private",
                queue_role.as_deref(),
            )?)
            .await?;
        let pseudo = pseudo_origin(&origin, &salt)?;
        let blinded = sidecar.blinded_key(&path, &pseudo).await?;
        assert_eq!(
            sidecar.blinded_key(&path, &pseudo).await?,
            blinded,
            "provisioning is idempotent"
        );
        logs.insert(
            name,
            DraftLog {
                origin,
                vkey: created.vkey,
                log_time_vkey: created.log_time_vkey,
                blinded_vkey: Some(blinded),
                url: created.url,
                path,
            },
        );
    }
    assert_ne!(
        logs[&LogName::Ballots].blinded_vkey,
        logs[&LogName::Events].blinded_vkey
    );
    let now = Utc::now().timestamp();
    let draft_text = draft(
        &config,
        &DraftEvent {
            name: policy_name(None, &tenant, &event)?,
            mode: TransparencyMode::PRIVATE,
            election_event_id: event.clone(),
            election_ids: vec![election],
            window: VotingWindow {
                opens: now - 60,
                closes: now + 86_400,
            },
            language: None,
            policy_salt: Some(salt),
            ballots: logs[&LogName::Ballots].clone(),
            events: logs[&LogName::Events].clone(),
        },
    )?;
    assert_eq!(
        draft_text
            .lines()
            .filter(|l| l.starts_with("blinded "))
            .count(),
        2
    );
    let board = NoteSigner::from_private_key(&dev_file("secrets/board.key")?)?;
    let policy = board.sign(&seal(&draft_text, BOARD))?;
    for log in logs.values() {
        let installed = sidecar.put_policy(&log.path, &policy).await?;
        assert_eq!(
            installed.policy_id,
            electoral_log::vitrine::policy_note_id(&policy)?
        );
        assert!(!installed.ack.is_empty());
        let response = sidecar
            .voter(
                &log.path,
                "policies",
                "",
                None,
                &format!("{tenant}/{VOTER}"),
            )
            .await?;
        assert_eq!(response.status, 200);
        assert!(response.body.starts_with("vitrine/policy-proofs/v1\n"));
        sidecar.freeze(&log.path).await?;
    }
    Ok(())
}
