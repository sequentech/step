// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Result};
use clap::Parser;
use rayon::prelude::*;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::marker::PhantomData;
use std::path::Path;
use tracing::{info, instrument};

use immu_board::{Board, BoardClient, BoardMessage};
use sequent_core::util::init_log::init_log;

use board_messages::braid::artifact::Configuration;
use board_messages::braid::artifact::DkgPublicKey;
use board_messages::braid::message::Message;
use board_messages::braid::newtypes::PublicKeyHash;
use board_messages::braid::protocol_manager::{ProtocolManager, ProtocolManagerConfig};
use board_messages::braid::statement::StatementType;
use braid::protocol::trustee::Trustee;
use braid::protocol::trustee::TrusteeConfig;
use strand::backend::ristretto::RistrettoCtx;
use strand::context::Ctx;
use strand::elgamal::Ciphertext;
use strand::serialization::StrandDeserialize;
use strand::serialization::StrandSerialize;
use strand::signature::{StrandSignaturePk, StrandSignatureSk};
use strand::symm;

const IMMUDB_USER: &str = "immudb";
const IMMUDB_PW: &str = "immudb";
const IMMUDB_URL: &str = "http://immudb:3322";
const INDEXDB: &str = "demoboardindex";
const DBNAME: &str = "demoboard";
const DEMO_DIR: &str = "./demo";
const PROTOCOL_MANAGER: &str = "pm.toml";
const CONFIG: &str = "config.bin";
#[derive(Parser)]
struct Cli {
    #[arg(long, default_value_t = IMMUDB_URL.to_string())]
    server_url: String,

    #[arg(short, long, default_value_t = DBNAME.to_string())]
    dbname: String,

    #[arg(short, long, default_value_t = INDEXDB.to_string())]
    indexdb: String,

    #[arg(short, long, default_value_t = 1)]
    count: u32,

    #[arg(value_enum)]
    command: Command,
}

#[derive(clap::ValueEnum, Clone)]
enum Command {
    GenConfigs,
    InitProtocol,
    PostBallots,
    ListMessages,
    ListBoards,
}

/*
The demo tool can be used to run a demo election with a fixed set of parameters

Backend         Ristretto
Trustees        3
Threshold       2
Cast ballots    100

currently these cannot be changed, but it would be easy to add cli options for them.

The sequence of steps to run a demo election are

    1) Generate the election configuration data (at ./demo)

       cargo run --bin demo_tool -- gen-configs

    2) Initialize the protocol with said configuration data (from ./demo)

       cargo run --bin demo_tool -- init-protocol

    3) Launch each of the trustees (each in their own directory)

       cd demo/1
       cargo run --manifest-path ../../Cargo.toml --target-dir ../../rust-local-target --bin main  -- --server-url http://immudb:3322 --board-index demoboardindex --trustee-config trustee1.toml

       cd demo/2
       cargo run --manifest-path ../../Cargo.toml --target-dir ../../rust-local-target --bin main  -- --server-url http://immudb:3322 --board-index demoboardindex --trustee-config trustee2.toml

       cd demo/3
       cargo run --manifest-path ../../Cargo.toml --target-dir ../../rust-local-target --bin main  -- --server-url http://immudb:3322 --board-index demoboardindex --trustee-config trustee3.toml

    4) Wait until the distributed key generation process has finished. You can check that this process is complete
       by listing the messages in the protocol board and looking for "PublicKey".

       cargo run --bin demo_tool -- list-messages

       example output with statement=PublicKey

       INFO message: Message{ sender="Self" statement=PublicKey(1715226660, ConfigurationHash(5961c86066), PublicKeyHash(7fa5d0654f), SharesHashes(1045b3c1ae 825b49a0da 8dd943adb4 - - - - - - - - -)

    5) Wait until the protocol execution finishes.  You can check that this process is complete
       by listing the messages in the protocol board and looking for "Plaintexts".

       cargo run --bin demo_tool -- post-ballots

       example output with statement=Plaintexts

       INFO message: Message{ sender="Self" statement=Plaintexts(1715226699, ConfigurationHash(5961c86066), 2, PlaintextsHash(85b40fc230), DecryptionFactorsHashes(4e99c9bc7b 39bd723ffb - - - - - - - - - -), CiphertextsHash(c11d685b13), PublicKeyHash(7fa5d0654f)) artifact=true}

       Note that the trustee processes will not terminate, they will continue in an idle state.
*/
#[tokio::main]
#[instrument]
async fn main() -> Result<()> {
    let ctx = RistrettoCtx;
    init_log(true);
    let args = Cli::parse();

    match &args.command {
        Command::GenConfigs => {
            let threshold = [1, 2];
            gen_configs::<RistrettoCtx>(3, &threshold)?;
        }
        Command::InitProtocol => {
            let path = Path::new(DEMO_DIR).join(CONFIG);
            let cfg_bytes = fs::read(path)
                .expect("Should have been able to read session configuration file at '{path}'");
            let configuration = Configuration::<RistrettoCtx>::strand_deserialize(&cfg_bytes)
                .map_err(|e| anyhow!("Could not deserialize configuration {}", e))?;

            let mut board = BoardClient::new(&args.server_url, IMMUDB_USER, IMMUDB_PW).await?;
            create_boards(&args.server_url, IMMUDB_USER, IMMUDB_PW, &args.indexdb, &args.dbname, args.count).await?; 
            for i in 0..args.count {
                let mut board = BoardClient::new(&args.server_url, IMMUDB_USER, IMMUDB_PW).await?;
                let name = if i == 0 {
                    args.dbname.to_string()
                }
                else {
                    format!("{}_{}", &args.dbname, i + 1)
                };
                init(&mut board, &name, configuration.clone()).await?;
            }
        }
        Command::PostBallots => {
            let mut board = BoardClient::new(&args.server_url, IMMUDB_USER, IMMUDB_PW).await?;
            for i in 0..args.count {
                let name = if i == 0 {
                    args.dbname.to_string()
                }
                else {
                    format!("{}_{}", &args.dbname, i + 1)
                };
                post_ballots(&mut board, &name, &ctx).await?;
            }
        }
        Command::ListMessages => {
            let mut board = BoardClient::new(&args.server_url, IMMUDB_USER, IMMUDB_PW).await?;
            list_messages(&mut board, &args.dbname).await?;
        }
        Command::ListBoards => {
            let mut board = BoardClient::new(&args.server_url, IMMUDB_USER, IMMUDB_PW).await?;
            list_boards(&mut board, &args.indexdb).await?;
        }
    }

    Ok(())
}

/*
Generates all the configuration information necessary to create a demo election

    * Generate .toml config for each trustee, containing:
        * signing_key_sk: base64 encoding of a der encoded pkcs#8 v1
        * signing_key_pk: base64 encoding of a der encoded spki
        * encryption_key: base64 encoding of a sign::SymmetricKey
    * Generate .toml config for the protocol manager:
        signing_key: base64 encoding of a der encoded pkcs#8 v1
    * Generate a .bin config for a session, a serialized Configuration artifact
        This configuration artifact includes the protocol manager and trustee information
        of the previous items.

    These files are created in a demo directory with the following layout

    demo
    |
    └ config.bin
    └ pm.toml
    |
    └ 1
    | |
    | └ trustee1.toml
    └ 2
    | |
    | └ trustee2.toml
    └ 3
    |
    └ trustee3.toml
*/
fn gen_configs<C: Ctx>(n_trustees: usize, threshold: &[usize]) -> Result<()> {
    let pmkey: StrandSignatureSk = StrandSignatureSk::gen()?;
    let pm: ProtocolManager<C> = ProtocolManager {
        signing_key: pmkey,
        phantom: PhantomData,
    };
    let (trustees, trustee_pks): (Vec<Trustee<C>>, Vec<StrandSignaturePk>) = (0..n_trustees)
        .map(|i| {
            let sk = StrandSignatureSk::gen().unwrap();
            let pk = StrandSignaturePk::from_sk(&sk).unwrap();
            let encryption_key: symm::SymmetricKey = symm::gen_key();
            (Trustee::new(i.to_string(), sk, encryption_key), pk)
        })
        .unzip();

    let cfg = Configuration::<C>::new(
        0,
        StrandSignaturePk::from_sk(&pm.signing_key)?,
        trustee_pks,
        threshold.len(),
        PhantomData,
    );
    println!("Creating demo files at '{}'", DEMO_DIR);
    fs::create_dir_all(DEMO_DIR)?;

    let cfg_bytes = cfg.strand_serialize()?;
    let mut file = File::create(Path::new(DEMO_DIR).join(CONFIG))?;
    file.write_all(&cfg_bytes).unwrap();

    let pm = ProtocolManagerConfig::from(&pm);
    let toml = toml::to_string(&pm).unwrap();
    let mut file = File::create(Path::new(DEMO_DIR).join(PROTOCOL_MANAGER))?;
    file.write_all(toml.as_bytes()).unwrap();

    for (i, t) in trustees.iter().enumerate() {
        let tc = TrusteeConfig::from(t);
        let toml = toml::to_string(&tc)?;
        let path = Path::new(DEMO_DIR).join((i + 1).to_string());
        fs::create_dir_all(&path)?;
        let mut file = File::create(path.join(format!("trustee{}.toml", i + 1)))?;
        file.write_all(toml.as_bytes())?;
    }

    Ok(())
}

/*
Initializes the bulletin board with the necessary information to start a protocol run. This information will
be taken from the demo directory created in the step above. As part of this process the required bulletin board
tables will be created (and removed if they already existed). These are
    * demoboardindex    The index board used to query which protocols to run
    * demoboard         The specific artifact board for a protocol run
*/
#[instrument]
async fn init<C: Ctx>(
    board: &mut BoardClient,
    board_name: &str,
    configuration: Configuration<C>,
) -> Result<()> {
    let pm = get_pm(PhantomData::<RistrettoCtx>)?;
    let message: BoardMessage = Message::bootstrap_msg(&configuration, &pm)?.try_into()?;
    info!("Adding configuration to the board..");
    board.insert_messages(board_name, &vec![message]).await
}

/*
Posts randomly generated ballots on the bulletin board for the purposes of tallying. If a ballot artifact already exists the
operation will be aborted.

This operation can only be carried out once the distributed key generation phase has been completed such that the election
public key is present on the board and can be downloaded to allow the encryption of random ballots. The ballot plaintexts
are randomly generated.
*/
#[instrument(skip(board))]
async fn post_ballots<C: Ctx>(board: &mut BoardClient, board_name: &str, ctx: &C) -> Result<()> {
    let pm = get_pm(PhantomData::<RistrettoCtx>)?;
    let sender_pk = StrandSignaturePk::from_sk(&pm.signing_key)?;
    let sender_pk = sender_pk.to_der_b64_string()?;
    let ballots = board
        .get_messages_filtered(
            &board_name,
            &StatementType::Ballots.to_string(),
            &sender_pk,
            None,
            None,
        )
        .await?;
    if ballots.len() > 0 {
        return Err(anyhow!("Ballots already present"));
    }

    let path = Path::new(DEMO_DIR).join(CONFIG);
    let contents = fs::read(&path)
        .expect("Should have been able to read session configuration file at '{path}'");

    let configuration = Configuration::<C>::strand_deserialize(&contents)
        .map_err(|e| anyhow!("Could not read configuration {}", e))?;

    let sender_pk = configuration.trustees.get(0).unwrap();
    let sender_pk = sender_pk.to_der_b64_string()?;
    let pk = board
        .get_messages_filtered(
            &board_name,
            &StatementType::PublicKey.to_string(),
            &sender_pk,
            None,
            None,
        )
        .await?;

    let mut rng = ctx.get_rng();
    if let Some(pk) = pk.get(0) {
        let message = Message::strand_deserialize(&pk.message)?;
        let bytes = message.artifact.unwrap();
        let dkgpk = DkgPublicKey::<C>::strand_deserialize(&bytes).unwrap();
        let pk_bytes = dkgpk.strand_serialize()?;
        let pk_h = strand::hash::hash_to_array(&pk_bytes)?;
        let pk_element = dkgpk.pk;
        let pk = strand::elgamal::PublicKey::from_element(&pk_element, ctx);

        let ps: Vec<C::P> = (0..100).map(|_| ctx.rnd_plaintext(&mut rng)).collect();
        let ballots: Vec<Ciphertext<C>> = ps
            .par_iter()
            .map(|p| {
                let encoded = ctx.encode(p).unwrap();
                pk.encrypt(&encoded)
            })
            .collect();

        info!("Generated {} ballots", ballots.len());

        let threshold = [1, 2];
        let mut selected_trustees = [board_messages::braid::newtypes::NULL_TRUSTEE;
            board_messages::braid::newtypes::MAX_TRUSTEES];
        selected_trustees[0..threshold.len()].copy_from_slice(&threshold);

        let ballot_batch = board_messages::braid::artifact::Ballots::new(ballots);
        let pm = get_pm(PhantomData::<RistrettoCtx>)?;
        let message = board_messages::braid::message::Message::ballots_msg(
            &configuration,
            2,
            &ballot_batch,
            selected_trustees,
            PublicKeyHash(strand::util::to_u8_array(&pk_h).unwrap()),
            &pm,
        )?;

        info!("Adding ballots to the board..");
        let bm: BoardMessage = message.try_into()?;
        board.insert_messages(board_name, &vec![bm]).await?;
    } else {
        return Err(anyhow!(
            "Could not find public key or configuration artifact(s)"
        ));
    }

    Ok(())
}

#[instrument(skip(board))]
async fn list_messages(board: &mut BoardClient, board_name: &str) -> Result<()> {
    let messages: Result<Vec<Message>> = board
        .get_messages(board_name, 0)
        .await?
        .iter()
        .map(|board_message: &BoardMessage| {
            Ok(Message::strand_deserialize(&board_message.message)?)
        })
        .collect();

    for message in messages? {
        info!("message: {:?}", message);
    }
    Ok(())
}

#[instrument(skip(board))]
async fn list_boards(board: &mut BoardClient, indexdb: &str) -> Result<()> {
    let boards: Result<Vec<String>> = board
        .get_boards(&indexdb)
        .await?
        .iter()
        .map(|board: &Board| Ok(board.database_name.clone()))
        .collect();

    for board in boards? {
        info!("board: {}", board);
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

#[instrument()]
async fn create_boards(server_url: &str, immudb_user: &str, immudb_pw: &str, indexdb: &str, dbname: &str, count: u32) -> Result<()> {
    let mut board = BoardClient::new(server_url, immudb_user, immudb_pw).await?;
    board.delete_database(indexdb).await?;
    board.upsert_index_db(indexdb).await?;

    for i in 0..count {
        let mut board = BoardClient::new(server_url, immudb_user, immudb_pw).await?;
        let name = if i == 0 {
            dbname.to_string()
        }
        else {
            format!("{}_{}", dbname, i + 1)
        };
        board.delete_database(&name).await?;    
        board.create_board(indexdb, &name).await?;    
    }

    Ok(())
}
