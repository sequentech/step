// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Result};
use b3::client::pgsql;
use b3::client::pgsql::B3IndexRow;
use b3::client::pgsql::B3MessageRow;
use b3::client::pgsql::PgsqlB3Client;
use b3::client::pgsql::PgsqlConnectionParams;
use clap::Parser;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::marker::PhantomData;
use std::path::Path;
use tracing::{info, instrument};

use b3::messages::artifact::Configuration;
use b3::messages::artifact::DkgPublicKey;
use b3::messages::message::Message;
use b3::messages::newtypes::PublicKeyHash;
use b3::messages::newtypes::MAX_TRUSTEES;
use b3::messages::newtypes::NULL_TRUSTEE;
use b3::messages::protocol_manager::{ProtocolManager, ProtocolManagerConfig};
use b3::messages::statement::StatementType;

use braid::protocol::trustee2::TrusteeConfig;
use rand::prelude::SliceRandom;
use strand::backend::ristretto::RistrettoCtx;
use strand::context::Ctx;
use strand::serialization::StrandDeserialize;
use strand::serialization::StrandSerialize;
use strand::signature::{StrandSignaturePk, StrandSignatureSk};
use strand::symm;

const PG_HOST: &'static str = "localhost";
const PG_PORT: u32 = 5432;
const PG_USER: &'static str = "postgres";
const PG_PASSW: &'static str = "postgrespw";
/// The postgresql database which will host the bulletin board.
/// Note that an entire bulletin board exists on a single database; each
/// individual board is implemented with a table.
const PG_DATABASE: &'static str = "protocoldb";
/// The default board if none specified.
const TEST_BOARD: &'static str = "test";
/// The root directory from which the demo directories will be created.
const DEMO_DIR: &str = "./demo";
const PROTOCOL_MANAGER: &str = "pm.toml";
/// File with the serialized bytes of a Configuration object.
const CONFIG: &str = "config.bin";

/// Runs a demo protocol.
#[derive(Parser)]
struct Cli {
    /// The postgresql database host.
    #[arg(long, default_value_t = PG_HOST.to_string())]
    host: String,

    /// The postgresql database port.
    #[arg(long, default_value_t = PG_PORT)]
    port: u32,

    /// The username with which to authenticate to postgres.
    #[arg(short, long, default_value_t = PG_USER.to_string())]
    username: String,

    /// The password with which to authenticate to postgres.
    #[arg(long, default_value_t = PG_PASSW.to_string())]
    password: String,

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
/// index and set as active. This is done directly through the database and not the
/// grpc server. If the required database tables do not exist
/// they are created. Any existing data is dropped.
///
/// PostBallots: Posts randomly generated ciphertexts to the requested board or boards.
/// This is done directly through the database and not the grpc server.
///
/// ListMessages: Lists the messages from the requested board. This is done directly through
/// the database and not the grpc server.
///
/// ListBoards: Lists the active boards in the index. This is done directly through
/// the database and not the grpc server.
///
/// DropDb: Drops the entire database.
///
/// All database operations execute on the database specified by the PG_DATABASE constant.
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
///    2.5) Launch the braid bulletin board server
///
///    3) Launch each of the trustees (each in their own directory)
///
///       cd demo/1
///       cargo run --manifest-path ../../Cargo.toml --target-dir ../../rust-local-target --release --bin main  -- --b3-url http://[::1]:50051 --trustee-config trustee1.toml
///
///       cd demo/2
///       cargo run --manifest-path ../../Cargo.toml --target-dir ../../rust-local-target --release --bin main  -- --b3-url http://[::1]:50051 --trustee-config trustee2.toml
///
///       cd demo/3
///       cargo run --manifest-path ../../Cargo.toml --target-dir ../../rust-local-target --release --bin main  -- --b3-url http://[::1]:50051 --trustee-config trustee3.toml
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
    braid::util::init_log(true);
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

            let c =
                PgsqlConnectionParams::new(&args.host, args.port, &args.username, &args.password);
            info!("Using connection string '{}'", c.connection_string());
            // pgsql::drop_database(&c, PG_DATABASE).await?;

            // swallow database already exists errors
            let _ = pgsql::create_database(&c, PG_DATABASE).await;

            let c = c.with_database(PG_DATABASE);
            let mut client = PgsqlB3Client::new(&c).await?;
            client.clear_database().await?;
            client.create_index_ine().await?;

            for i in 0..args.board_count {
                let name = if i == 0 {
                    &args.board_name
                } else {
                    &format!("{}_{}", args.board_name, i + 1)
                };
                client.create_board_ine(name).await?;
                init(&mut client, &name, configuration.clone()).await?;
            }

            info!(
                "Initialized {} boards, don't forget to clear trustee message stores",
                args.board_count
            )
        }
        Command::PostBallots => {
            let mut board =
                get_client(&args.host, args.port, &args.username, &args.password).await?;
            for i in 0..args.board_count {
                let name = if i == 0 {
                    args.board_name.to_string()
                } else {
                    format!("{}_{}", &args.board_name, i + 1)
                };
                post_ballots(
                    &mut board,
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
            let mut client =
                get_client(&args.host, args.port, &args.username, &args.password).await?;
            list_messages(&mut client, &args.board_name).await?;
        }
        Command::ListBoards => {
            let mut client =
                get_client(&args.host, args.port, &args.username, &args.password).await?;
            list_boards(&mut client).await?;
        }
        Command::DropDb => {
            delete_boards(&args.host, args.port, &args.username, &args.password).await?;
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
    let pmkey: StrandSignatureSk = StrandSignatureSk::gen()?;
    let pm: ProtocolManager<C> = ProtocolManager {
        signing_key: pmkey,
        phantom: PhantomData,
    };
    let (trustees, trustee_pks): (Vec<TrusteeConfig>, Vec<StrandSignaturePk>) = (0..n_trustees)
        .map(|_| {
            let sk = StrandSignatureSk::gen().unwrap();
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
            let run = "cargo run --manifest-path ../../Cargo.toml --release --bin main -- --b3-url http://127.0.0.1:50051 --trustee-config trustee.toml";
            file.write_all(run.as_bytes())?;
        }
    }

    Ok(())
}

/// Initializes the bulletin board with the necessary information to start a protocol run.
///
/// This information will be taken from the demo directory created in the gen-config step.
#[instrument(skip(client))]
async fn init<C: Ctx>(
    client: &mut PgsqlB3Client,
    board_name: &str,
    configuration: Configuration<C>,
) -> Result<()> {
    let pm = get_pm(PhantomData::<C>)?;
    // let message: B3MessageRow = Message::bootstrap_msg(&configuration, &pm)?.try_into()?;
    let message = Message::bootstrap_msg(&configuration, &pm)?;
    info!("Adding configuration to the board..");
    // client.insert_messages(board_name, &vec![message]).await
    client.insert_configuration::<C>(board_name, message).await
}

/// Posts randomly generated ballots on the bulletin board for the purposes of tallying.
///
/// This operation can only be carried out once the distributed key generation phase has
/// been completed such that the election public key is present on the board and can be
/// downloaded to allow the encryption of random ballots. If there are already ballots
/// present on the board, an error will be returned. A protocol run can always be reset
/// with the init-protocol command.
#[instrument(skip(client))]
async fn post_ballots<C: Ctx>(
    client: &mut PgsqlB3Client,
    board_name: &str,
    ciphertexts: usize,
    batches: u32,
    n_trustees: usize,
    threshold: usize,
    ctx: &C,
) -> Result<()> {
    let pm = get_pm(PhantomData::<C>)?;
    let sender_pk = StrandSignaturePk::from_sk(&pm.signing_key)?;
    let ballots = client
        .get_with_kind(&board_name, StatementType::Ballots, &sender_pk)
        .await?;
    if ballots.len() > 0 {
        return Err(anyhow!("Ballots already present"));
    }

    let path = Path::new(DEMO_DIR).join(CONFIG);
    let contents = fs::read(&path)
        .expect("Should have been able to read session configuration file at '{path}'");

    let configuration = Configuration::<C>::strand_deserialize(&contents)
        .map_err(|e| anyhow!("Could not read configuration {e:?}"))?;

    let sender_pk = configuration.trustees.get(0).unwrap();
    let pk = client
        .get_with_kind(&board_name, StatementType::PublicKey, &sender_pk)
        .await?;

    if let Some(pk) = pk.get(0) {
        let message = Message::strand_deserialize(&pk.message)?;
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
        let mut rng = &mut rand::thread_rng();
        let threshold: Vec<usize> = all.choose_multiple(&mut rng, threshold).cloned().collect();

        let mut selected_trustees = [NULL_TRUSTEE; MAX_TRUSTEES];
        selected_trustees[0..threshold.len()].copy_from_slice(&threshold);

        let ballot_batch = b3::messages::artifact::Ballots::new(ballots);
        let pm = get_pm(PhantomData::<RistrettoCtx>)?;

        for i in 0..batches {
            let message = b3::messages::message::Message::ballots_msg(
                &configuration,
                i as usize,
                &ballot_batch,
                selected_trustees,
                PublicKeyHash(strand::util::to_u8_array(&pk_h).unwrap()),
                &pm,
            )?;

            info!("Adding ballots to the board..");
            let bm: B3MessageRow = message.try_into()?;
            client.insert_messages(board_name, &vec![bm]).await?;
        }
    } else {
        return Err(anyhow!(
            "Could not find public key or configuration artifact(s)"
        ));
    }

    Ok(())
}

#[instrument(skip(board))]
async fn list_messages(board: &mut PgsqlB3Client, board_name: &str) -> Result<()> {
    let messages: Result<Vec<Message>> = board
        .get_messages(board_name, 0)
        .await?
        .iter()
        .map(|board_message: &B3MessageRow| {
            Ok(Message::strand_deserialize(&board_message.message)?)
        })
        .collect();

    for message in messages? {
        info!("message: {:?}", message);
    }
    Ok(())
}

#[instrument(skip(board))]
async fn list_boards(board: &mut PgsqlB3Client) -> Result<()> {
    let boards: Result<Vec<B3IndexRow>> = board.get_boards().await;

    for board in boards? {
        info!(
            "board: '{}', cfg_id: {}, message_count: {}",
            board.board_name, board.cfg_id, board.message_count
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

/// Drops the entire database.
#[instrument()]
async fn delete_boards(host: &str, port: u32, username: &str, password: &str) -> Result<()> {
    let c = get_connection(host, port, username, password);
    pgsql::drop_database(&c, PG_DATABASE).await?;

    Ok(())
}

fn get_connection(host: &str, port: u32, username: &str, password: &str) -> PgsqlConnectionParams {
    PgsqlConnectionParams::new(host, port, username, password)
}
async fn get_client(
    host: &str,
    port: u32,
    username: &str,
    password: &str,
) -> Result<PgsqlB3Client> {
    let c = get_connection(host, port, username, password);
    let c = c.with_database(PG_DATABASE);
    info!("Using connection string '{}'", c.connection_string());
    PgsqlB3Client::new(&c).await
}
