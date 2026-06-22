// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Result};
use aws_sdk_s3::Client as S3Client;
use clap::Parser;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::env;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
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
use cryptography::context::RistrettoCtx;
use cryptography::context::Context;
use cryptography::cryptosystem::elgamal::PublicKey;
use cryptography::utils::serialization::variable::{VSerializable, VDeserializable};
use cryptography::utils::signatures::SignatureScheme;
use cryptography::utils::symm;

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
    /// The SQLite database path (or use DATABASE_URL env var).
    #[arg(long)]
    database_url: Option<String>,

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

    /// The ciphertext width (number of group element pairs per ciphertext).
    ///
    /// Used when generating configuration data and posting ballots.
    /// When posting ballots, you must supply the same value
    /// as the one used during configuration generation.
    /// Valid values: 1-4
    #[arg(long, default_value_t = 2)]
    ciphertext_width: usize,

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
/// DropDb: Drops the entire SQLite database file.
///
/// All database operations execute on the database specified by the DATABASE_URL env var
/// or the --database-url argument (defaults to ./b4.db).
#[derive(clap::ValueEnum, Clone)]
enum Command {
    GenConfigs,
    InitProtocol,
    PostBallots,
    ListMessages,
    ListBoards,
    DropDb,
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
///       cargo run --manifest-path ../../Cargo.toml --target-dir ../../rust-local-target --release --bin main  -- --b3-url http://localhost:3000 --trustee-config trustee.toml
///
///       cd demo/2
///       cargo run --manifest-path ../../Cargo.toml --target-dir ../../rust-local-target --release --bin main  -- --b3-url http://localhost:3000 --trustee-config trustee.toml
///
///       cd demo/3
///       cargo run --manifest-path ../../Cargo.toml --target-dir ../../rust-local-target --release --bin main  -- --b3-url http://localhost:3000 --trustee-config trustee.toml
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
    braid::native::logging::init_log(true);
    let args = Cli::parse();

    match &args.command {
        Command::GenConfigs => {
            gen_configs::<RistrettoCtx>(args.num_trustees, args.threshold, args.ciphertext_width)?;
        }
        Command::InitProtocol => {
            let path = Path::new(DEMO_DIR).join(CONFIG);
            let cfg_bytes = fs::read(&path).expect(&format!(
                "Should have been able to read session configuration file at '{:?}'",
                path
            ));
            let configuration = Configuration::<RistrettoCtx>::deser(&cfg_bytes)
                .map_err(|e| anyhow!("Could not deserialize configuration {}", e))?;

            let (pool, _s3_client, _bucket) = init_clients(&args.database_url).await?;
            
            // Clear existing data
            clear_database(&pool).await?;

            for i in 0..args.board_count {
                let name = if i == 0 {
                    &args.board_name
                } else {
                    &format!("{}_{}", args.board_name, i + 1)
                };
                create_board(&pool, name).await?;
                init::<RistrettoCtx>(&pool, name, configuration.clone()).await?;
            }

            info!(
                "Initialized {} boards, don't forget to clear trustee message stores",
                args.board_count
            )
        }
        Command::PostBallots => {
            let (pool, s3_client, bucket) = init_clients(&args.database_url).await?;
            for i in 0..args.board_count {
                let name = if i == 0 {
                    args.board_name.to_string()
                } else {
                    format!("{}_{}", &args.board_name, i + 1)
                };
                post_ballots::<RistrettoCtx>(
                    &pool,
                    &s3_client,
                    &bucket,
                    &name,
                    args.ciphertexts,
                    args.batches,
                    args.num_trustees,
                    args.threshold,
                )
                .await?;
            }
        }
        Command::ListMessages => {
            let (pool, _s3_client, _bucket) = init_clients(&args.database_url).await?;
            list_messages(&pool, &args.board_name).await?;
        }
        Command::ListBoards => {
            let (pool, _s3_client, _bucket) = init_clients(&args.database_url).await?;
            list_boards(&pool).await?;
        }
        Command::DropDb => {
            drop_database(&args.database_url).await?;
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
fn gen_configs<C: Context>(n_trustees: usize, threshold: usize, ciphertext_width: usize) -> Result<()> {
    let mut rng = C::get_rng();
    let pmkey = <C::SignatureScheme as SignatureScheme<C::Rng>>::gen_signing_key(&mut rng);
    let pm: ProtocolManager<C> = ProtocolManager {
        signing_key: pmkey,
        phantom: PhantomData,
    };
    let (trustees, trustee_pks): (Vec<TrusteeConfig>, Vec<<<C as Context>::SignatureScheme as SignatureScheme<<C as Context>::Rng>>::Verifier>) = (0..n_trustees)
        .map(|_| {
            let sk = <C::SignatureScheme as SignatureScheme<C::Rng>>::gen_signing_key(&mut rng);
            let pk = <C::SignatureScheme as SignatureScheme<C::Rng>>::verifying_key(&sk);
            let encryption_key: symm::SymmetricKey = symm::gen_key().unwrap();
            let tc = TrusteeConfig::new_from_objects::<C>(sk, encryption_key);
            (tc, pk)
        })
        .unzip();

    let cfg = Configuration::<C>::new(
        0,
        <<C as Context>::SignatureScheme as SignatureScheme<_>>::verifying_key(&pm.signing_key),
        trustee_pks,
        threshold,
        ciphertext_width,
        PhantomData,
    );
    println!("Generated config: {:?}", cfg);
    println!("Creating demo files at '{}'", DEMO_DIR);
    fs::create_dir_all(DEMO_DIR)?;

    let cfg_bytes = cfg.ser();
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
            let run = "cargo run --manifest-path ../../Cargo.toml --release --bin main -- --b3-url http://localhost:3000 --trustee-config trustee.toml";
            file.write_all(run.as_bytes())?;
        }
    }

    Ok(())
}

/// Initializes the bulletin board with the necessary information to start a protocol run.
///
/// This information will be taken from the demo directory created in the gen-config step.
#[instrument(skip(pool))]
async fn init<C: Context>(
    pool: &SqlitePool,
    board_name: &str,
    configuration: Configuration<RistrettoCtx>,
) -> Result<()> {
    let pm = get_pm::<C>()?;
    let message = Message::bootstrap_msg(&configuration, &pm)?;
    info!("Adding configuration to the board..");
    
    // Serialize the message and store inline
    let message_bytes = message.ser();
    let timestamp = chrono::Utc::now().timestamp();
    let version = "1";
    
    // Extract metadata for database
    let sender_pk = <<RistrettoCtx as Context>::SignatureScheme as SignatureScheme<_>>::verifier_to_base64_string(&message.sender.pk)
        .map_err(|e| anyhow!("Failed to encode verifying key: {}", e))?;
    let statement_kind = format!("{:?}", message.statement.get_kind());
    
    // Store inline in SQLite (no S3 for demo_tool)
    sqlx::query(
        r#"INSERT INTO messages (board_name, timestamp, size, content_type, inline_data, s3_key, version, sender_pk, statement_kind, batch, mix_number)
           VALUES (?, ?, ?, 'inline', ?, NULL, ?, ?, ?, 0, 0)"#
    )
    .bind(board_name)
    .bind(timestamp)
    .bind(message_bytes.len() as i64)
    .bind(&message_bytes)
    .bind(version)
    .bind(&sender_pk)
    .bind(&statement_kind)
    .execute(pool)
    .await?;
    
    // Update board metadata with configuration info (similar to b3's update_index)
    sqlx::query(
        r#"UPDATE boards 
           SET cfg_id = ?, 
               threshold_no = ?, 
               trustees_no = ?,
               message_count = message_count + 1,
               last_message_kind = ?
           WHERE name = ?"#
    )
    .bind(configuration.id.to_string())
    .bind(configuration.threshold as i32)
    .bind(configuration.trustees.len() as i32)
    .bind(&statement_kind)
    .bind(board_name)
    .execute(pool)
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
async fn post_ballots<C: Context>(
    pool: &SqlitePool,
    s3_client: &S3Client,
    bucket: &str,
    board_name: &str,
    ciphertexts: usize,
    batches: u32,
    n_trustees: usize,
    threshold: usize,
) -> Result<()> {
    let pm = get_pm::<C>()?;

    let sender_pk_obj = <<RistrettoCtx as Context>::SignatureScheme as SignatureScheme<_>>::verifying_key(&pm.signing_key);
    let sender_pk_b64 = <<RistrettoCtx as Context>::SignatureScheme as SignatureScheme<_>>::verifier_to_base64_string(&sender_pk_obj)
        .map_err(|e| anyhow!("Failed to encode sender pk: {}", e))?;
    
    // Check if ballots already exist
    let existing: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM messages WHERE board_name = ? AND statement_kind = 'Ballots' AND sender_pk = ?"
    )
    .bind(board_name)
    .bind(&sender_pk_b64)
    .fetch_one(pool)
    .await?;
    
    if existing.0 > 0 {
        return Err(anyhow!("Ballots already present"));
    }

    let path = Path::new(DEMO_DIR).join(CONFIG);
    let contents = fs::read(&path)
        .expect("Should have been able to read session configuration file at '{path}'");

    let configuration = Configuration::<RistrettoCtx>::deser(&contents)
        .map_err(|e| anyhow!("Could not read configuration {e:?}"))?;

    let trustee_pk = configuration.trustees.get(0).unwrap();
    let trustee_pk_b64 = <<RistrettoCtx as Context>::SignatureScheme as SignatureScheme<_>>::verifier_to_base64_string(trustee_pk)
        .map_err(|e| anyhow!("Failed to encode trustee pk: {}", e))?;
    
    info!("Looking for PublicKey from trustee: {}", trustee_pk_b64);
    
    // Get the public key message
    let pk_row: Option<(String, Option<Vec<u8>>, Option<String>)> = sqlx::query_as(
        "SELECT content_type, inline_data, s3_key FROM messages WHERE board_name = ? AND statement_kind = 'PublicKey' AND sender_pk = ? LIMIT 1"
    )
    .bind(board_name)
    .bind(&trustee_pk_b64)
    .fetch_optional(pool)
    .await?;
    
    info!("PublicKey query result: {:?}", pk_row.is_some());

    if let Some((content_type, inline_data, s3_key)) = pk_row {
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
                return Err(anyhow!("Unknown content type for PublicKey: {}", content_type));
            }
        };
        
        let message: Message<RistrettoCtx> = Message::deser(&pk_data)?;
        let bytes = message.artifact.unwrap();
        let dkgpk = DkgPublicKey::<RistrettoCtx>::deser(&bytes).unwrap();
        let pk_bytes = dkgpk.ser();
        let pk_h = b4::hash_to_array(&pk_bytes)?;
        let pk_element = dkgpk.pk;
        let _pk = PublicKey::<RistrettoCtx>::new(pk_element.clone());

        let max: [usize; 12] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        let all = &max[0..n_trustees];
        let mut rng = &mut rand::rng();
        let threshold: Vec<usize> = all.choose_multiple(&mut rng, threshold).cloned().collect();

        let mut selected_trustees = [NULL_TRUSTEE; MAX_TRUSTEES];
        selected_trustees[0..threshold.len()].copy_from_slice(&threshold);

        let pm = get_pm::<RistrettoCtx>()?;

        // Use dispatch macro to generate ballots with the configured ciphertext width
        braid::dispatch_ciphertext_width!(configuration.ciphertext_width, {
            let ballots = b4::random_ciphertexts::<RistrettoCtx, W>(ciphertexts);
            info!("Generated {} ballots with width={}", ballots.len(), W);

            let ballot_batch = b4::messages::artifact::Ballots::new(ballots);

            for i in 0..batches {
                let message = b4::messages::message::Message::ballots_msg(
                &configuration,
                i as u64,
                &ballot_batch,
                selected_trustees,
                PublicKeyHash(pk_h),
                &pm,
            )?;

            info!("Adding ballots to the board..");
            
            // Serialize and store inline
            let message_bytes = message.ser();
            let timestamp = chrono::Utc::now().timestamp();
            let version = "1";
            let sender_pk = <<RistrettoCtx as Context>::SignatureScheme as SignatureScheme<_>>::verifier_to_base64_string(&message.sender.pk)
                .map_err(|e| anyhow!("Failed to encode sender pk: {}", e))?;
            let statement_kind = format!("{:?}", message.statement.get_kind());
            
            sqlx::query(
                r#"INSERT INTO messages (board_name, timestamp, size, content_type, inline_data, s3_key, version, sender_pk, statement_kind, batch, mix_number)
                   VALUES (?, ?, ?, 'inline', ?, NULL, ?, ?, ?, ?, 0)"#
            )
            .bind(board_name)
            .bind(timestamp)
            .bind(message_bytes.len() as i64)
            .bind(&message_bytes)
            .bind(version)
            .bind(&sender_pk)
            .bind(&statement_kind)
            .bind(i as i32)
            .execute(pool)
            .await?;
            }
            
            // Update board batch_count (similar to b3's approach)
            sqlx::query(
                r#"UPDATE boards 
                   SET batch_count = ?,
                       message_count = message_count + ?,
                       last_message_kind = ?
                   WHERE name = ?"#
            )
            .bind(batches as i32)
            .bind(batches as i32)  // Added one message per batch
            .bind("Ballots")
            .bind(board_name)
            .execute(pool)
            .await?;
        }); // Close dispatch_ciphertext_width macro
    } else {
        return Err(anyhow!(
            "Could not find public key or configuration artifact(s)"
        ));
    }

    Ok(())
}

#[instrument(skip(pool))]
async fn list_messages(pool: &SqlitePool, board_name: &str) -> Result<()> {
    let rows: Vec<(String, Option<Vec<u8>>, Option<String>)> = sqlx::query_as(
        "SELECT content_type, inline_data, s3_key FROM messages WHERE board_name = ? ORDER BY id ASC"
    )
    .bind(board_name)
    .fetch_all(pool)
    .await?;

    // Get S3 client for downloading S3-stored messages
    let s3_config = aws_sdk_s3::Config::builder()
        .behavior_version_latest()
        .region(aws_sdk_s3::config::Region::new(
            std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string())
        ))
        .endpoint_url(
            std::env::var("AWS_ENDPOINT_URL").unwrap_or_else(|_| "http://localhost:4566".to_string())
        )
        .force_path_style(true)
        .build();
    let s3_client = aws_sdk_s3::Client::from_conf(s3_config);
    let bucket_name = std::env::var("S3_BUCKET_NAME").unwrap_or_else(|_| "wbraid-messages".to_string());

    for (content_type, inline_data, s3_key) in rows {
        let message_data = match content_type.as_str() {
            "inline" => {
                inline_data.ok_or_else(|| anyhow!("Inline message has no data"))?
            }
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

        let message: Message<RistrettoCtx> = Message::deser(&message_data)?;
        info!("message: {:?}", message);
    }
    Ok(())
}

#[instrument(skip(pool))]
async fn list_boards(pool: &SqlitePool) -> Result<()> {
    let boards: Vec<(String, i64, String)> = sqlx::query_as(
        "SELECT name, created_at, status FROM boards ORDER BY created_at DESC"
    )
    .fetch_all(pool)
    .await?;

    for (name, created_at, status) in boards {
        // Count messages for this board
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM messages WHERE board_name = ?"
        )
        .bind(&name)
        .fetch_one(pool)
        .await?;
        
        info!(
            "board: '{}', created_at: {}, status: {}, message_count: {}",
            name, created_at, status, count.0
        );
    }
    Ok(())
}

fn get_pm<C: Context>() -> Result<ProtocolManager<RistrettoCtx>> {
    let path = Path::new(DEMO_DIR).join(PROTOCOL_MANAGER);
    let contents = fs::read_to_string(&path)
        .expect("Should have been able to read the protocol manager file at '{path}'");

    let pm_config: ProtocolManagerConfig = toml::from_str(&contents).unwrap();
    let sk = <<RistrettoCtx as Context>::SignatureScheme as SignatureScheme<_>>::signer_from_base64_string(&pm_config.signing_key)
        .map_err(|e| anyhow!("Could not deserialize configuration {}", e))?;
    let pm: ProtocolManager<RistrettoCtx> = ProtocolManager {
        signing_key: sk,
        phantom: PhantomData,
    };

    Ok(pm)
}

/// Initialize database connection and S3 client
async fn init_clients(database_url: &Option<String>) -> Result<(SqlitePool, S3Client, String)> {
    let db_url = database_url.clone().or_else(|| env::var("DATABASE_URL").ok())
        .unwrap_or_else(|| {
            let mut path = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            path.push("b4.db");
            format!("sqlite:{}?mode=rwc", path.display())
        });
    
    info!("Connecting to database: {}", db_url);
    
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    // Initialize tables
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS boards (
            name TEXT PRIMARY KEY,
            created_at INTEGER NOT NULL,
            status TEXT NOT NULL DEFAULT 'active'
        )"#
    ).execute(&pool).await?;

    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            board_name TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            sender_pk TEXT NOT NULL,
            statement_kind TEXT NOT NULL,
            batch INTEGER NOT NULL DEFAULT 0,
            mix_number INTEGER NOT NULL DEFAULT 0,
            size INTEGER NOT NULL,
            content_type TEXT NOT NULL,
            inline_data BLOB,
            s3_key TEXT,
            version TEXT NOT NULL,
            FOREIGN KEY (board_name) REFERENCES boards(name)
        )"#
    ).execute(&pool).await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_messages_board_id ON messages(board_name, id)"
    ).execute(&pool).await?;
    
    // Initialize S3 client
    let s3_config = aws_sdk_s3::Config::builder()
        .behavior_version_latest()
        .region(aws_sdk_s3::config::Region::new(
            env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string())
        ))
        .endpoint_url(
            env::var("AWS_ENDPOINT_URL").unwrap_or_else(|_| "http://localhost:4566".to_string())
        )
        .force_path_style(true)
        .build();
    let s3_client = S3Client::from_conf(s3_config);
    
    let bucket = env::var("S3_BUCKET_NAME").unwrap_or_else(|_| DEFAULT_BUCKET.to_string());
    
    Ok((pool, s3_client, bucket))
}

/// Clear all data from the database
async fn clear_database(pool: &SqlitePool) -> Result<()> {
    sqlx::query("DELETE FROM messages").execute(pool).await?;
    sqlx::query("DELETE FROM boards").execute(pool).await?;
    info!("Cleared database");
    Ok(())
}

/// Create a board
async fn create_board(pool: &SqlitePool, name: &str) -> Result<()> {
    let created_at = chrono::Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO boards (name, created_at, status) VALUES (?, ?, 'active')"
    )
    .bind(name)
    .bind(created_at)
    .execute(pool)
    .await?;
    info!("Created board: {}", name);
    Ok(())
}

/// Drops the entire database file.
#[instrument()]
async fn drop_database(database_url: &Option<String>) -> Result<()> {
    let db_path = database_url.clone().or_else(|| env::var("DATABASE_URL").ok())
        .unwrap_or_else(|| {
            let mut path = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            path.push("b4.db");
            path.display().to_string()
        });
    
    // Remove sqlite: prefix if present
    let file_path = db_path.trim_start_matches("sqlite:").split('?').next().unwrap();
    
    if Path::new(file_path).exists() {
        fs::remove_file(file_path)?;
        info!("Dropped database: {}", file_path);
    } else {
        info!("Database file not found: {}", file_path);
    }
    
    Ok(())
}
