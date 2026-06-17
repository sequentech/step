// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use aws_sdk_s3::Client as S3Client;
use bb8_postgres::{bb8::Pool, PostgresConnectionManager};
use clap::Parser;
use std::env;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::marker::PhantomData;
use std::path::Path;
use std::path::PathBuf;
use tokio_postgres::NoTls;
use tracing::{info, instrument};

use b4::messages::artifact::Configuration;
use b4::messages::artifact::DkgPublicKey;
use b4::messages::message::Message;
use b4::messages::newtypes::PublicKeyHash;
use b4::messages::newtypes::MAX_TRUSTEES;
use b4::messages::newtypes::NULL_TRUSTEE;
use b4::messages::protocol_manager::{ProtocolManager, ProtocolManagerConfig};

use braid::protocol::trustee::TrusteeConfig;
use rand::seq::IndexedRandom;
use strand::backend::ristretto::RistrettoCtx;
use strand::context::Ctx;
use strand::serialization::StrandDeserialize;
use strand::serialization::StrandSerialize;
use strand::signature::{StrandSignaturePk, StrandSignatureSk};
use strand::symm;

/// PostgreSQL connection pool type alias
type DbPool = Pool<PostgresConnectionManager<NoTls>>;

/// The default board if none specified.
const TEST_BOARD: &'static str = "test";
/// The root directory from which the demo directories will be created.
const DEMO_DIR: &str = "./demo";
const PROTOCOL_MANAGER: &str = "pm.toml";
/// File with the serialized bytes of a Configuration object.
const CONFIG: &str = "config.bin";
/// S3 bucket name (can be overridden with S3_BUCKET_NAME env var)
const DEFAULT_BUCKET: &str = "wbraid-messages";

/// Runs a demo protocol.
#[derive(Parser)]
struct Cli {
    /// PostgreSQL host (or use B4_PG_HOST env var)
    #[arg(long)]
    pg_host: Option<String>,

    /// PostgreSQL port (or use B4_PG_PORT env var)
    #[arg(long)]
    pg_port: Option<u16>,

    /// PostgreSQL username (or use B4_PG_USER env var)
    #[arg(long)]
    pg_user: Option<String>,

    /// PostgreSQL password (or use B4_PG_PASSWORD env var)
    #[arg(long)]
    pg_password: Option<String>,

    /// PostgreSQL database name (or use B4_PG_DATABASE env var)
    #[arg(long)]
    pg_database: Option<String>,

    /// The board on which the requested operations will take place.
    ///
    /// Used when initializing the protocol, posting ballots and
    /// listing messages.
    #[arg(long, default_value_t = TEST_BOARD.to_string())]
    board_name: String,

    /// The number of boards to operate on, using the board_name as a prefix.
    ///
    /// Used when initializing the protocol and posting ballots.
    /// For example, if using board_name = test, and setting this parameter
    /// to 3 will use test, test_1, and test_3.
    #[arg(long, default_value_t = 1)]
    board_count: u32,

    /// The number of ciphertexts to generate when posting ballots.
    #[arg(long, default_value_t = 100)]
    ciphertexts: usize,

    /// The number of batches to generate when posting ballots.
    #[arg(long, default_value_t = 1)]
    batches: u32,

    /// The number of of trustees to use
    ///
    /// Used when generating configuration files and posting ballots.
    /// When posting ballots, you must supply the same value
    /// as the one used during configuration generation.
    #[arg(long, default_value_t = 3)]
    num_trustees: usize,

    /// The number of threshold trustees to use.
    ///
    /// Used when generating configuration data and posting ballots.
    /// When posting ballots, you must supply the same values
    /// as the one used during configuration generation.
    #[arg(long, default_value_t = 2)]
    threshold: usize,

    /// The operation to execute.
    #[arg(value_enum)]
    command: Command,
}

/// The requested operation for this tool.
///
/// GenConfigs: generate the trustee and protocol configuration files, creating
/// the required directory structure. Also generates a default launch script for
/// each trustee.
///
/// InitProtocol: Initializes the protocol by posting the protocol Configuration
/// to the requested board or set of boards. These boards are also added to the
/// database and set as active. This is done directly through the database and S3.
/// If the required database tables do not exist they are created. Any existing
/// data is dropped.
///
/// PostBallots: Posts randomly generated ciphertexts to the requested board or boards.
/// This is done directly through the database and S3.
///
/// ListMessages: Lists the messages from the requested board. This is done directly through
/// the database.
///
/// ListBoards: Lists the active boards in the index. This is done directly through
/// the database.
///
/// ClearDb: Clears all data from the PostgreSQL database tables.
///
/// All database operations execute on the database specified by B4_PG_* env vars
/// or the corresponding --pg-* arguments.
#[derive(clap::ValueEnum, Clone)]
enum Command {
    GenConfigs,
    InitProtocol,
    PostBallots,
    ListMessages,
    ListBoards,
    ClearDb,
}

///
/// The demo tool can be used to run a demo election, with backend fixed to Ristretto.
///
/// The sequence of steps to run a demo election are
///
///    1) Generate the election configuration data (at Self::DEMO_DIR)
///
///       cargo run --bin demo_tool -- gen-configs
///
///    2) Initialize the protocol with said configuration data (from Self::DEMO_DIR)
///
///       cargo run --bin demo_tool -- init-protocol
///
///    2.5) Launch the b4 bulletin board server (using bb.ps1 or manually)
///
///    3) Launch each of the trustees (each in their own directory)
///
///       cd demo/1
///       cargo run --manifest-path ../../Cargo.toml --target-dir ../../rust-local-target --release --bin main  -- --b4-url http://localhost:3000 --trustee-config trustee.toml
///
///       cd demo/2
///       cargo run --manifest-path ../../Cargo.toml --target-dir ../../rust-local-target --release --bin main  -- --b4-url http://localhost:3000 --trustee-config trustee.toml
///
///       cd demo/3
///       cargo run --manifest-path ../../Cargo.toml --target-dir ../../rust-local-target --release --bin main  -- --b4-url http://localhost:3000 --trustee-config trustee.toml
///
///    4) Wait until the distributed key generation process has finished. You can check that this process is complete
///       by listing the messages in the protocol board and looking for "PublicKey".
///
///       cargo run --bin demo_tool -- list-messages
///
///       example output with statement=PublicKey
///
///       INFO message: Message{ sender="Self" statement=PublicKey(1715226660, ConfigurationHash(5961c86066), PublicKeyHash(7fa5d0654f), SharesHashes(1045b3c1ae 825b49a0da 8dd943adb4 - - - - - - - - -)
///
///    5) Post random ballots
///
///        cargo run --bin demo_tool -- post-ballots
///
///    6) Wait until the protocol execution finishes.  You can check that this process is complete
///       by listing the messages in the protocol board and looking for "Plaintexts".
///
///       cargo run --bin demo_tool -- list-messages
///
///       example output with statement=Plaintexts
///
///       INFO message: Message{ sender="Self" statement=Plaintexts(1715226699, ConfigurationHash(5961c86066), 2, PlaintextsHash(85b40fc230), DecryptionFactorsHashes(4e99c9bc7b 39bd723ffb - - - - - - - - - -), CiphertextsHash(c11d685b13), PublicKeyHash(7fa5d0654f)) artifact=true}
///
///       Note that the trustee processes will not terminate, they will continue to execute in an idle state.
#[tokio::main]
#[instrument]
async fn main() -> Result<()> {
    let ctx = RistrettoCtx;
    braid::native::logging::init_log(true);
    let args = Cli::parse();

    match &args.command {
        Command::GenConfigs => {
            gen_configs::<RistrettoCtx>(args.num_trustees, args.threshold)?;
        }
        Command::InitProtocol => {
            let path = Path::new(DEMO_DIR).join(CONFIG);
            let cfg_bytes = fs::read(&path).expect(&format!(
                "Should have been able to read session configuration file at '{:?}'",
                path
            ));
            let configuration = Configuration::<RistrettoCtx>::strand_deserialize(&cfg_bytes)
                .map_err(|e| anyhow!("Could not deserialize configuration {}", e))?;

            let (pool, s3_client, bucket) = init_clients(&args).await?;

            // Clear existing data
            clear_database(&pool).await?;

            for i in 0..args.board_count {
                let name = if i == 0 {
                    &args.board_name
                } else {
                    &format!("{}_{}", args.board_name, i + 1)
                };
                create_board(&pool, name).await?;
                init(&pool, &s3_client, &bucket, name, configuration.clone()).await?;
            }

            info!(
                "Initialized {} boards, don't forget to clear trustee message stores",
                args.board_count
            )
        }
        Command::PostBallots => {
            let (pool, s3_client, bucket) = init_clients(&args).await?;
            for i in 0..args.board_count {
                let name = if i == 0 {
                    args.board_name.to_string()
                } else {
                    format!("{}_{}", &args.board_name, i + 1)
                };
                post_ballots(
                    &pool,
                    &s3_client,
                    &bucket,
                    &name,
                    args.ciphertexts,
                    args.batches,
                    args.num_trustees,
                    args.threshold,
                    &ctx,
                )
                .await?;
            }
        }
        Command::ListMessages => {
            let (pool, _s3_client, _bucket) = init_clients(&args).await?;
            list_messages(&pool, &args.board_name).await?;
        }
        Command::ListBoards => {
            let (pool, _s3_client, _bucket) = init_clients(&args).await?;
            list_boards(&pool).await?;
        }
        Command::ClearDb => {
            let (pool, _s3_client, _bucket) = init_clients(&args).await?;
            clear_database(&pool).await?;
        }
    }

    Ok(())
}

///
/// Generates all the configuration information necessary to create a demo election
///
///    * Generate .toml config for each trustee, containing:
///        * signing_key_sk: base64 encoding of a der encoded pkcs#8 v1
///        * signing_key_pk: base64 encoding of a der encoded spki
///        * encryption_key: base64 encoding of a sign::SymmetricKey
///    * Generate .toml config for the protocol manager:
///        signing_key: base64 encoding of a der encoded pkcs#8 v1
///    * Generate a .bin config for a session, a serialized Configuration artifact
///        This configuration artifact includes the protocol manager and trustee information
///        of the previous items.
///    * Generates default a run script for each trustee.
///
///    These files are created in a demo directory with the following layout,
///    for example with num_trustees = 3:
///
///    demo
///    |
///    └ config.bin
///    └ pm.toml
///    |
///    └ 1
///    | |
///    | └ trustee.toml
///    └ 2
///    | |
///    | └ trustee.toml
///    └ 3
///    |
///   └ trustee.toml
fn gen_configs<C: Ctx>(n_trustees: usize, threshold: usize) -> Result<()> {
    let pmkey: StrandSignatureSk = StrandSignatureSk::generate()?;
    let pm: ProtocolManager<C> = ProtocolManager {
        signing_key: pmkey,
        phantom: PhantomData,
    };
    let (trustees, trustee_pks): (Vec<TrusteeConfig>, Vec<StrandSignaturePk>) = (0..n_trustees)
        .map(|_| {
            let sk = StrandSignatureSk::generate().unwrap();
            let pk = StrandSignaturePk::from_sk(&sk).unwrap();
            let encryption_key: symm::SymmetricKey = symm::gen_key();
            let tc = TrusteeConfig::new_from_objects(sk, encryption_key);
            (tc, pk)
        })
        .unzip();

    let cfg = Configuration::<C>::new(
        0,
        StrandSignaturePk::from_sk(&pm.signing_key)?,
        trustee_pks,
        threshold,
        PhantomData,
    );
    println!("Generated config: {:?}", cfg);
    println!("Creating demo files at '{}'", DEMO_DIR);
    fs::create_dir_all(DEMO_DIR)?;

    let cfg_bytes = cfg.strand_serialize()?;
    let mut file = File::create(Path::new(DEMO_DIR).join(CONFIG))?;
    file.write_all(&cfg_bytes).unwrap();

    let pm = ProtocolManagerConfig::from(&pm);
    let toml = toml::to_string(&pm).unwrap();
    let mut file = File::create(Path::new(DEMO_DIR).join(PROTOCOL_MANAGER))?;
    file.write_all(toml.as_bytes()).unwrap();

    for (i, tc) in trustees.iter().enumerate() {
        let toml = toml::to_string(&tc)?;
        let path = Path::new(DEMO_DIR).join((i + 1).to_string());
        fs::create_dir_all(&path)?;
        let mut file = File::create(path.join("trustee.toml"))?;
        file.write_all(toml.as_bytes())?;
        let path = path.join("run.sh");
        if !Path::exists(&path) {
            let mut file = File::create(path)?;
            let run = "cargo run --manifest-path ../../Cargo.toml --release --bin main -- --b4-url http://localhost:3000 --trustee-config trustee.toml";
            file.write_all(run.as_bytes())?;
        }
    }

    Ok(())
}

/// Initializes the bulletin board with the necessary information to start a protocol run.
///
/// This information will be taken from the demo directory created in the gen-config step.
#[instrument(skip(pool, s3_client))]
async fn init<C: Ctx>(
    pool: &DbPool,
    s3_client: &S3Client,
    bucket: &str,
    board_name: &str,
    configuration: Configuration<C>,
) -> Result<()> {
    let pm = get_pm(PhantomData::<C>)?;
    let message = Message::bootstrap_msg(&configuration, &pm)?;
    info!("Adding configuration to the board..");

    // Serialize the message and store inline
    let message_bytes = message.strand_serialize()?;
    let timestamp = chrono::Utc::now().timestamp();
    let version = "1";

    // Extract metadata for database
    let sender_pk = message.sender.pk.to_der_b64_string()?;
    let statement_kind = format!("{:?}", message.statement.get_kind());

    let conn = pool.get().await?;

    // Store inline in PostgreSQL
    conn.execute(
        r#"INSERT INTO messages (board_name, timestamp, size, content_type, inline_data, s3_key, version, sender_pk, statement_kind, batch, mix_number)
           VALUES ($1, $2, $3, 'inline', $4, NULL, $5, $6, $7, 0, 0)"#,
        &[
            &board_name,
            &timestamp,
            &(message_bytes.len() as i64),
            &message_bytes,
            &version,
            &sender_pk,
            &statement_kind,
        ],
    )
    .await?;

    // Update board metadata with configuration info
    conn.execute(
        r#"UPDATE boards 
           SET cfg_id = $1, 
               threshold_no = $2, 
               trustees_no = $3,
               message_count = message_count + 1,
               last_message_kind = $4
           WHERE name = $5"#,
        &[
            &configuration.id.to_string(),
            &(configuration.threshold as i32),
            &(configuration.trustees.len() as i32),
            &statement_kind,
            &board_name,
        ],
    )
    .await?;

    Ok(())
}

/// Posts randomly generated ballots on the bulletin board for the purposes of tallying.
///
/// This operation can only be carried out once the distributed key generation phase has
/// been completed such that the election public key is present on the board and can be
/// downloaded to allow the encryption of random ballots. If there are already ballots
/// present on the board, an error will be returned. A protocol run can always be reset
/// with the init-protocol command.
#[instrument(skip(pool, s3_client))]
async fn post_ballots<C: Ctx>(
    pool: &DbPool,
    s3_client: &S3Client,
    bucket: &str,
    board_name: &str,
    ciphertexts: usize,
    batches: u32,
    n_trustees: usize,
    threshold: usize,
    ctx: &C,
) -> Result<()> {
    let pm = get_pm(PhantomData::<C>)?;
    let sender_pk_obj = StrandSignaturePk::from_sk(&pm.signing_key)?;
    let sender_pk_b64 = sender_pk_obj.to_der_b64_string()?;

    let conn = pool.get().await?;

    // Check if ballots already exist
    let existing = conn
        .query_one(
            "SELECT COUNT(*) FROM messages WHERE board_name = $1 AND statement_kind = 'Ballots' AND sender_pk = $2",
            &[&board_name, &sender_pk_b64],
        )
        .await?;
    let count: i64 = existing.get(0);

    if count > 0 {
        return Err(anyhow!("Ballots already present"));
    }

    let path = Path::new(DEMO_DIR).join(CONFIG);
    let contents = fs::read(&path)
        .expect("Should have been able to read session configuration file at '{path}'");

    let configuration = Configuration::<C>::strand_deserialize(&contents)
        .map_err(|e| anyhow!("Could not read configuration {e:?}"))?;

    let trustee_pk = configuration.trustees.get(0).unwrap();
    let trustee_pk_b64 = trustee_pk.to_der_b64_string()?;

    info!("Looking for PublicKey from trustee: {}", trustee_pk_b64);

    // Get the public key message
    let pk_row = conn.query_opt(
        "SELECT content_type, inline_data, s3_key FROM messages WHERE board_name = $1 AND statement_kind = 'PublicKey' AND sender_pk = $2 LIMIT 1",
        &[&board_name, &trustee_pk_b64],
    )
    .await?;

    info!("PublicKey query result: {:?}", pk_row.is_some());

    if let Some(row) = pk_row {
        let content_type: String = row.get(0);
        let inline_data: Option<Vec<u8>> = row.get(1);
        let s3_key: Option<String> = row.get(2);

        let pk_data = match content_type.as_str() {
            "inline" => {
                inline_data.ok_or_else(|| anyhow!("Inline PublicKey message has no data"))?
            }
            "s3" => {
                let key = s3_key.ok_or_else(|| anyhow!("S3 PublicKey message has no key"))?;
                let obj = s3_client
                    .get_object()
                    .bucket(bucket)
                    .key(&key)
                    .send()
                    .await?;
                let bytes = obj.body.collect().await?;
                bytes.to_vec()
            }
            _ => {
                return Err(anyhow!(
                    "Unknown content type for PublicKey: {}",
                    content_type
                ));
            }
        };

        let message = Message::strand_deserialize(&pk_data)?;
        let bytes = message.artifact.unwrap();
        let dkgpk = DkgPublicKey::<C>::strand_deserialize(&bytes).unwrap();
        let pk_bytes = dkgpk.strand_serialize()?;
        let pk_h = strand::hash::hash_to_array(&pk_bytes)?;
        let pk_element = dkgpk.pk;
        let _pk = strand::elgamal::PublicKey::from_element(&pk_element, ctx);

        let ballots = strand::util::random_ciphertexts(ciphertexts, &C::default());
        info!("Generated {} ballots", ballots.len());

        let max: [usize; 12] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        let all = &max[0..n_trustees];
        let mut rng = &mut rand::rng();
        let threshold: Vec<usize> = all.choose_multiple(&mut rng, threshold).cloned().collect();

        let mut selected_trustees = [NULL_TRUSTEE; MAX_TRUSTEES];
        selected_trustees[0..threshold.len()].copy_from_slice(&threshold);

        let ballot_batch = b4::messages::artifact::Ballots::new(ballots);
        let pm = get_pm(PhantomData::<RistrettoCtx>)?;

        for i in 0..batches {
            let message = b4::messages::message::Message::ballots_msg(
                &configuration,
                i as u64,
                &ballot_batch,
                selected_trustees,
                PublicKeyHash(strand::util::to_u8_array(&pk_h).unwrap()),
                &pm,
            )?;

            info!("Adding ballots to the board..");

            // Serialize and store inline
            let message_bytes = message.strand_serialize()?;
            let timestamp = chrono::Utc::now().timestamp();
            let version = "1";
            let sender_pk = message.sender.pk.to_der_b64_string()?;
            let statement_kind = format!("{:?}", message.statement.get_kind());

            conn.execute(
                r#"INSERT INTO messages (board_name, timestamp, size, content_type, inline_data, s3_key, version, sender_pk, statement_kind, batch, mix_number)
                   VALUES ($1, $2, $3, 'inline', $4, NULL, $5, $6, $7, $8, 0)"#,
                &[
                    &board_name,
                    &timestamp,
                    &(message_bytes.len() as i64),
                    &message_bytes,
                    &version,
                    &sender_pk,
                    &statement_kind,
                    &(i as i32),
                ],
            )
            .await?;
        }

        // Update board batch_count
        conn.execute(
            r#"UPDATE boards 
               SET batch_count = $1,
                   message_count = message_count + $2,
                   last_message_kind = $3
               WHERE name = $4"#,
            &[
                &(batches as i32),
                &(batches as i32),
                &"Ballots".to_string(),
                &board_name,
            ],
        )
        .await?;
    } else {
        return Err(anyhow!(
            "Could not find public key or configuration artifact(s)"
        ));
    }

    Ok(())
}

#[instrument(skip(pool))]
async fn list_messages(pool: &DbPool, board_name: &str) -> Result<()> {
    let conn = pool.get().await?;
    let rows = conn.query(
        "SELECT content_type, inline_data, s3_key FROM messages WHERE board_name = $1 ORDER BY id ASC",
        &[&board_name],
    )
    .await?;

    // Get S3 client for downloading S3-stored messages
    let s3_config = aws_sdk_s3::Config::builder()
        .behavior_version_latest()
        .region(aws_sdk_s3::config::Region::new(
            std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string()),
        ))
        .endpoint_url(
            std::env::var("AWS_ENDPOINT_URL")
                .unwrap_or_else(|_| "http://localhost:4566".to_string()),
        )
        .force_path_style(true)
        .build();
    let s3_client = aws_sdk_s3::Client::from_conf(s3_config);
    let bucket_name =
        std::env::var("S3_BUCKET_NAME").unwrap_or_else(|_| "wbraid-messages".to_string());

    for row in rows {
        let content_type: String = row.get(0);
        let inline_data: Option<Vec<u8>> = row.get(1);
        let s3_key: Option<String> = row.get(2);

        let message_data = match content_type.as_str() {
            "inline" => inline_data.ok_or_else(|| anyhow!("Inline message has no data"))?,
            "s3" => {
                let key = s3_key.ok_or_else(|| anyhow!("S3 message has no key"))?;
                let obj = s3_client
                    .get_object()
                    .bucket(&bucket_name)
                    .key(&key)
                    .send()
                    .await?;
                let bytes = obj.body.collect().await?;
                bytes.to_vec()
            }
            _ => {
                return Err(anyhow!("Unknown content type: {}", content_type));
            }
        };

        let message = Message::strand_deserialize(&message_data)?;
        info!("message: {:?}", message);
    }
    Ok(())
}

#[instrument(skip(pool))]
async fn list_boards(pool: &DbPool) -> Result<()> {
    let conn = pool.get().await?;
    let boards = conn
        .query(
            "SELECT name, created_at, status FROM boards ORDER BY created_at DESC",
            &[],
        )
        .await?;

    for row in boards {
        let name: String = row.get(0);
        let created_at: i64 = row.get(1);
        let status: String = row.get(2);

        // Count messages for this board
        let count_row = conn
            .query_one(
                "SELECT COUNT(*) FROM messages WHERE board_name = $1",
                &[&name],
            )
            .await?;
        let count: i64 = count_row.get(0);

        info!(
            "board: '{}', created_at: {}, status: {}, message_count: {}",
            name, created_at, status, count
        );
    }
    Ok(())
}

fn get_pm<C: Ctx>(ctxp: PhantomData<C>) -> Result<ProtocolManager<C>> {
    let path = Path::new(DEMO_DIR).join(PROTOCOL_MANAGER);
    let contents = fs::read_to_string(&path)
        .expect("Should have been able to read the protocol manager file at '{path}'");

    let pm_config: ProtocolManagerConfig = toml::from_str(&contents).unwrap();
    let sk = StrandSignatureSk::from_der_b64_string(&pm_config.signing_key)
        .map_err(|e| anyhow!("Could not deserialize configuration {}", e))?;
    let pm: ProtocolManager<C> = ProtocolManager {
        signing_key: sk,
        phantom: ctxp,
    };

    Ok(pm)
}

/// Initialize database connection pool and S3 client
async fn init_clients(args: &Cli) -> Result<(DbPool, S3Client, String)> {
    let host = args
        .pg_host
        .clone()
        .or_else(|| env::var("B4_PG_HOST").ok())
        .context("B4_PG_HOST must be set (via env var or --pg-host)")?;
    let port: u16 = args
        .pg_port
        .or_else(|| env::var("B4_PG_PORT").ok().and_then(|p| p.parse().ok()))
        .context("B4_PG_PORT must be set (via env var or --pg-port)")?;
    let user = args
        .pg_user
        .clone()
        .or_else(|| env::var("B4_PG_USER").ok())
        .context("B4_PG_USER must be set (via env var or --pg-user)")?;
    let password = args
        .pg_password
        .clone()
        .or_else(|| env::var("B4_PG_PASSWORD").ok())
        .context("B4_PG_PASSWORD must be set (via env var or --pg-password)")?;
    let database = args
        .pg_database
        .clone()
        .or_else(|| env::var("B4_PG_DATABASE").ok())
        .context("B4_PG_DATABASE must be set (via env var or --pg-database)")?;

    let conn_string = format!(
        "host={} port={} user={} password={} dbname={}",
        host, port, user, password, database
    );

    info!("Connecting to PostgreSQL at {}:{}", host, port);

    let manager = PostgresConnectionManager::new_from_stringlike(&conn_string, NoTls)?;

    let pool = Pool::builder().max_size(5).build(manager).await?;

    // Initialize tables in a scoped block so connection is dropped before returning pool
    {
        let conn = pool.get().await?;
        conn.execute(
            r#"CREATE TABLE IF NOT EXISTS boards (
                name VARCHAR PRIMARY KEY,
                created_at BIGINT NOT NULL,
                status VARCHAR NOT NULL DEFAULT 'active',
                cfg_id VARCHAR,
                threshold_no INTEGER,
                trustees_no INTEGER,
                last_message_kind VARCHAR,
                message_count INTEGER DEFAULT 0,
                batch_count INTEGER DEFAULT 0
            )"#,
            &[],
        )
        .await?;

        conn.execute(
            r#"CREATE TABLE IF NOT EXISTS messages (
                id BIGSERIAL PRIMARY KEY,
                board_name VARCHAR NOT NULL,
                timestamp BIGINT NOT NULL,
                sender_pk VARCHAR NOT NULL,
                statement_kind VARCHAR NOT NULL,
                batch INTEGER NOT NULL DEFAULT 0,
                mix_number INTEGER NOT NULL DEFAULT 0,
                size BIGINT NOT NULL,
                content_type VARCHAR NOT NULL,
                inline_data BYTEA,
                s3_key VARCHAR,
                version VARCHAR NOT NULL,
                UNIQUE (board_name, sender_pk, statement_kind, batch, mix_number)
            )"#,
            &[],
        )
        .await?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_messages_board_id ON messages(board_name, id)",
            &[],
        )
        .await?;
    }

    // Initialize S3 client
    let s3_config = aws_sdk_s3::Config::builder()
        .behavior_version_latest()
        .region(aws_sdk_s3::config::Region::new(
            env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string()),
        ))
        .endpoint_url(
            env::var("AWS_ENDPOINT_URL").unwrap_or_else(|_| "http://localhost:4566".to_string()),
        )
        .force_path_style(true)
        .build();
    let s3_client = S3Client::from_conf(s3_config);

    let bucket = env::var("S3_BUCKET_NAME").unwrap_or_else(|_| DEFAULT_BUCKET.to_string());

    Ok((pool, s3_client, bucket))
}

/// Clear all data from the database
async fn clear_database(pool: &DbPool) -> Result<()> {
    let conn = pool.get().await?;
    conn.execute("DELETE FROM messages", &[]).await?;
    conn.execute("DELETE FROM boards", &[]).await?;
    info!("Cleared database");
    Ok(())
}

/// Create a board
async fn create_board(pool: &DbPool, name: &str) -> Result<()> {
    let created_at = chrono::Utc::now().timestamp();
    let conn = pool.get().await?;
    conn.execute(
        "INSERT INTO boards (name, created_at, status) VALUES ($1, $2, 'active')",
        &[&name, &created_at],
    )
    .await?;
    info!("Created board: {}", name);
    Ok(())
}

/// Drops the entire database file.
#[instrument()]
async fn drop_database(database_url: &Option<String>) -> Result<()> {
    let db_path = database_url
        .clone()
        .or_else(|| env::var("DATABASE_URL").ok())
        .unwrap_or_else(|| {
            let mut path = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            path.push("b4.db");
            path.display().to_string()
        });

    // Remove sqlite: prefix if present
    let file_path = db_path
        .trim_start_matches("sqlite:")
        .split('?')
        .next()
        .unwrap();

    if Path::new(file_path).exists() {
        fs::remove_file(file_path)?;
        info!("Dropped database: {}", file_path);
    } else {
        info!("Database file not found: {}", file_path);
    }

    Ok(())
}
