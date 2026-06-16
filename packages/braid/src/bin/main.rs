// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use braid::native::board::{HttpB3, HttpB3BoardParams};
use braid::util::{ensure_directory, get_access_token, ProtocolError};
use clap::Parser;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use tokio::time::{sleep, Duration};
use tracing::instrument;
use tracing::{error, info};

use braid::native::session::Session;
use braid::protocol::trustee::Trustee;
use braid::protocol::trustee::TrusteeConfig;
use sequent_core::types::ceremonies::TrusteeModePolicy;
use sequent_core::types::env_vars as ev;
use sequent_core::util::init_log::init_log;
use strand::backend::ristretto::RistrettoCtx;
use strand::signature::{StrandSignaturePk, StrandSignatureSk};
use strand::symm;

cfg_if::cfg_if! {
    if #[cfg(feature = "jemalloc")] {
        use tikv_jemalloc_ctl::{stats, epoch};

        #[global_allocator]
        static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
    }
}

#[derive(Parser)]
struct Cli {
    #[arg(short, long)]
    b3_url: String,

    #[arg(short, long)]
    trustee_config: PathBuf,

    #[arg(long, default_value_t = false)]
    strict: bool,
}

// How often the session map (which contains trustee's memory board) is cleared
const SESSION_RESET_PERIOD: i64 = 20 * 60;

/// A single active keys ceremony returned by Harvest's discovery endpoint.
#[derive(serde::Deserialize, Debug)]
struct ActiveCeremony {
    keys_ceremony_id: String,
    election_event_id: String,
    board_name: String,
    execution_status: String,
}

/// Response wrapper from Harvest's `POST /active-ceremonies` discovery endpoint.
#[derive(serde::Deserialize, Debug)]
struct ActiveCeremonies {
    ceremonies: Vec<ActiveCeremony>,
}

/// Discover every active keys ceremony this trustee should participate in via
/// the Harvest `POST /active-ceremonies` endpoint — one per election event. Each
/// board name is resolved server-side from the election event's bulletin board
/// reference, so it is returned verbatim and never reconstructed locally.
#[instrument(err, skip(access_token))]
async fn discover_active_ceremonies(
    harvest_url: &str,
    access_token: &str,
) -> Result<Vec<ActiveCeremony>> {
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{harvest_url}/active-ceremonies"))
        .bearer_auth(access_token)
        .json(&serde_json::json!({}))
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        return Err(anyhow!("Discovery failed: {status}"));
    }

    let discovered: ActiveCeremonies = response.json().await?;

    info!(
        "Discovered {} active ceremonies",
        discovered.ceremonies.len()
    );

    Ok(discovered.ceremonies)
}

/// Register this trustee's public key for the active ceremony via Harvest's
/// `POST /register-trustee-key` endpoint — the same path browser-based trustees
/// use.
#[instrument(err, skip(access_token))]
async fn register_trustee_key(
    harvest_url: &str,
    access_token: &str,
    election_event_id: &str,
    keys_ceremony_id: &str,
    public_key: &str,
) -> Result<()> {
    let client = reqwest::Client::new();

    let response = client
        .post(format!("{harvest_url}/register-trustee-key"))
        .bearer_auth(access_token)
        .json(&serde_json::json!({
            "election_event_id": election_event_id,
            "keys_ceremony_id": keys_ceremony_id,
            "public_key": public_key,
        }))
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        return Err(anyhow!("Key registration failed: {status}"));
    }

    info!("Successfully registered public key for ceremony {keys_ceremony_id}");
    Ok(())
}

/*
Entry point for a braid-native trustee.

Example run command

cargo run --release --bin main  -- --b3-url http://127.0.0.1:50051 --trustee-config trustee.toml

A native trustee will:

    1) Authenticate to Keycloak and obtain JWT
    2) Discover active keys ceremony via Harvest /active-ceremony
    3) Register its public key via Harvest /register-trustee-key (unified with BBT flow)
    4) For each heartbeat cycle:
        a) Poll the protocol board for new messages
        b) Update the local store with new messages
        c) Execute the protocol with the existing messages in the local store
        d) Send heartbeat to B4

The process will loop indefinitely unless an error is encountered and the 'strict'
command line option is set to true.
*/
#[tokio::main]
#[instrument]
async fn main() -> Result<()> {
    init_log(true);

    cfg_if::cfg_if! {
        if #[cfg(feature = "jemalloc")] {
            let e = epoch::mib().unwrap();
            let allocated = stats::allocated::mib().unwrap();
            let resident = stats::resident::mib().unwrap();
        }
    }

    let args = Cli::parse();

    let contents = fs::read_to_string(args.trustee_config)
        .expect("Should have been able to read the trustee configuration file");

    let tc: TrusteeConfig = toml::from_str(&contents).unwrap();
    let sk: StrandSignatureSk = StrandSignatureSk::from_der_b64_string(&tc.signing_key_sk)?;

    let bytes = braid::util::decode_base64(&tc.encryption_key)?;
    let ek = symm::sk_from_bytes(&bytes)?;

    // Get trustee name and password for Keycloak authentication
    let trustee_name =
        std::env::var(ev::TRUSTEE_NAME).map_err(|_| anyhow!("TRUSTEE_NAME must be set"))?;

    let trustee_password =
        std::env::var(ev::TRUSTEE_PSW).map_err(|_| anyhow!("TRUSTEE_PSW must be set"))?;

    let heartbeat_secs: u64 = std::env::var(ev::BRAID_B4_HEARTBEAT)
        .map_err(|_| anyhow!("BRAID_B4_HEARTBEAT must be set"))?
        .parse::<u64>()
        .map_err(|_| anyhow!("BRAID_B4_HEARTBEAT must be a positive integer"))?;
    if heartbeat_secs == 0 {
        return Err(anyhow!("BRAID_B4_HEARTBEAT must be greater than 0"));
    }

    let sender_pk = { StrandSignaturePk::from_sk(&sk)?.to_der_b64_string()? };

    let ignored_boards = get_ignored_boards();
    info!("ignored boards {:?}", ignored_boards);

    let store_root = std::env::current_dir().unwrap().join("message_store");
    ensure_directory(store_root.clone())?;

    // Fetch initial access token for B4 authentication
    let initial_access_token = get_access_token(&trustee_name, &trustee_password).await?;
    let board_params = HttpB3BoardParams::new(&args.b3_url, initial_access_token.clone()).await;

    // Harvest base URL used for ceremony discovery and key registration.
    let harvest_url =
        std::env::var(ev::HARVEST_URL).map_err(|_| anyhow!("HARVEST_URL must be set"))?;

    let mut session_map: HashMap<
        String,
        Session<RistrettoCtx, HttpB3, braid::native::board::SqliteStorage>,
    > = HashMap::new();
    // Ceremonies whose key we have already registered, to keep registration a
    // one-shot per ceremony (the Harvest endpoint is idempotent regardless).
    let mut registered_ceremonies: HashSet<String> = HashSet::new();
    let mut loop_count: i64 = 0;
    loop {
        info!("{loop_count} >");

        // Fetch access token for B4 authentication using trustee credentials
        let access_token = match get_access_token(&trustee_name, &trustee_password).await {
            Ok(token) => token,
            Err(e) => {
                error!("Failed to get access token: {e:?}");
                sleep(Duration::from_millis(1000)).await;
                continue;
            }
        };
        // Update the shared token so all existing sessions see the refresh
        board_params.set_access_token(access_token.clone());

        // Discover the active ceremonies this trustee should participate in
        // (one per election event). Drives which boards we run DKG on.
        let ceremonies = match discover_active_ceremonies(&harvest_url, &access_token).await {
            Ok(ceremonies) => ceremonies,
            Err(e) => {
                error!("Failed to discover active ceremonies: {e:?}");
                sleep(Duration::from_millis(1000)).await;
                continue;
            }
        };

        // Register our public key for any ceremony not yet registered (unified
        // path with BBT trustees). Idempotent server-side; tracked locally to
        // avoid redundant calls every loop.
        for ceremony in &ceremonies {
            if registered_ceremonies.contains(&ceremony.keys_ceremony_id) {
                continue;
            }
            match register_trustee_key(
                &harvest_url,
                &access_token,
                &ceremony.election_event_id,
                &ceremony.keys_ceremony_id,
                &sender_pk,
            )
            .await
            {
                Ok(()) => {
                    registered_ceremonies.insert(ceremony.keys_ceremony_id.clone());
                }
                Err(e) => error!(
                    "Failed to register key for ceremony {}: {e:?}",
                    ceremony.keys_ceremony_id
                ),
            }
        }

        if loop_count % SESSION_RESET_PERIOD == 0 {
            info!("* Session memory reset");
            session_map = HashMap::new();
        }

        let mut step_error = false;

        // Create sessions for the discovered ceremony boards only.
        for ceremony in &ceremonies {
            let board_name = &ceremony.board_name;
            if ignored_boards.contains(board_name) {
                info!("Ignoring board '{board_name}'..");
                continue;
            }
            if session_map.contains_key(board_name) {
                continue;
            }

            info!("* Creating new session for discovered board '{board_name}'..");

            let storage =
                braid::native::board::SqliteStorage::new(store_root.join(board_name), None);
            let trustee = Trustee::new(
                trustee_name.clone(),
                board_name.clone(),
                sk.clone(),
                ek.clone(),
                storage,
                None,
            );
            let session = Session::new(board_name, trustee, board_params.clone());
            session_map.insert(board_name.clone(), session);
        }

        // This code is sequential, see main_concurrent for an alternative implementation
        for s in session_map.values_mut() {
            let board_name = s.board_name.clone();

            let result = s.step().await;
            match result {
                Ok(_) => (),
                Err(error) => {
                    let mut show_error = true;
                    let error_msg = format!("{:?}", error);
                    if let ProtocolError::BootstrapError(msg) = error {
                        show_error = !msg.starts_with("Zero messages received");
                    }
                    if show_error {
                        error!(
                            "Error executing step for board '{}': '{}'",
                            board_name.clone(),
                            error_msg
                        );
                    }
                    step_error = true;
                }
            };
        }

        // Send heartbeat for every active session every heartbeat_secs iterations
        if loop_count % heartbeat_secs as i64 == 0 {
            for board_name in session_map.keys() {
                if let Err(e) = board_params
                    .send_heartbeat(
                        board_name,
                        &sender_pk,
                        &trustee_name,
                        TrusteeModePolicy::SERVER_BASED,
                    )
                    .await
                {
                    tracing::warn!("Heartbeat failed for board '{board_name}': {e}");
                }
            }
        }

        if args.strict && step_error {
            break;
        }

        cfg_if::cfg_if! {
            if #[cfg(feature = "jemalloc")] {
                // Many statistics are cached and only updated
                // when the epoch is advanced:
                let e_ = e.advance();
                let alloc = allocated.read();
                let res = resident.read();
                let mb = 1024 * 1024;

                if let(Ok(_), Ok(alloc), Ok(res)) = (e_, alloc, res) {
                    info!("{} MB allocated / {} MB resident ({} boards)", (alloc / mb), (res / mb), boards.len());
                }
            }
        }

        loop_count = (loop_count + 1) % i64::MAX;
        println!("");
        sleep(Duration::from_millis(1000)).await;
    }

    Ok(())
}

fn get_ignored_boards() -> Vec<String> {
    let boards_str: String = std::env::var(ev::IGNORE_BOARDS).unwrap_or_else(|_| "".into());
    boards_str.split(',').map(|s| s.to_string()).collect()
}
