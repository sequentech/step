// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/*
cargo run --bin demo_tool -- gen-configs
cargo run --bin demo_tool -- init-protocol
cd demo/1
cargo run --manifest-path ../../Cargo.toml --target-dir ../../rust-local-target --bin main  -- --server-url http://immudb:3322 --board-index demoboardindex --trustee-config trustee1.toml
cargo run --bin demo_tool -- post-ballots
*/

use anyhow::{anyhow, Result};
use clap::Parser;
use rayon::prelude::*;
use std::fs;
use std::fs::File;
use std::marker::PhantomData;
use std::io::Write;
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
use strand::backend::ristretto::RistrettoCtx;
use strand::context::Ctx;
use strand::elgamal::Ciphertext;
use strand::serialization::StrandDeserialize;
use strand::serialization::StrandSerialize;
use strand::signature::{StrandSignaturePk, StrandSignatureSk};
use strand::symm;
use braid::protocol::trustee::Trustee;
use braid::protocol::trustee::TrusteeConfig;


const PROTOCOL_MANAGER: &str = "pm.toml";
const CONFIG: &str = "config.bin";
const IMMUDB_USER: &str = "immudb";
const IMMUDB_PW: &str = "immudb";
const IMMUDB_URL: &str = "http://immudb:3322";
const INDEXDB: &str = "demoboardindex";
const DBNAME: &str = "demoboard";
const DEMO_DIR: &str = "./demo";

#[derive(Parser)]
struct Cli {
    #[arg(long, default_value_t = IMMUDB_URL.to_string())]
    server_url: String,

    #[arg(short, long, default_value_t = DBNAME.to_string())]
    dbname: String,

    #[arg(short, long, default_value_t = INDEXDB.to_string())]
    indexdb: String,

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
            create_boards(&mut board, &args.indexdb, &args.dbname).await?;
            init(&mut board, &args.dbname, configuration).await?;
        }
        Command::PostBallots => {
            let mut board = BoardClient::new(&args.server_url, IMMUDB_USER, IMMUDB_PW).await?;
            post_ballots(&mut board, &args.dbname, ctx).await?;
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

#[instrument(skip(board))]
async fn create_boards(board: &mut BoardClient, indexdb: &str, dbname: &str) -> Result<()> {
    board.delete_database(indexdb).await?;
    board.delete_database(dbname).await?;

    board.upsert_index_db(indexdb).await?;
    board.create_board(indexdb, dbname).await?;

    Ok(())
}

#[instrument(skip(board))]
async fn post_ballots<C: Ctx>(board: &mut BoardClient, board_name: &str, ctx: C) -> Result<()> {
    
    let pm = get_pm(PhantomData::<RistrettoCtx>)?;
    let sender_pk = StrandSignaturePk::from_sk(&pm.signing_key)?;
    let sender_pk = sender_pk.to_der_b64_string()?;
    let ballots = board.get_messages_filtered(&board_name, &StatementType::Ballots.to_string(), &sender_pk, None, None).await?;
    if ballots.len() > 0 {
        return Err(anyhow!("Ballots already present"));
    }

    let path = Path::new(DEMO_DIR).join(CONFIG);
    let contents =
        fs::read(&path).expect("Should have been able to read session configuration file at '{path}'");

    let configuration = Configuration::<C>::strand_deserialize(&contents)
        .map_err(|e| anyhow!("Could not read configuration {}", e))?;
    
    let sender_pk = configuration.trustees.get(0).unwrap();
    let sender_pk = sender_pk.to_der_b64_string()?;
    let pk = board.get_messages_filtered(&board_name, &StatementType::PublicKey.to_string(), &sender_pk, None, None).await?;
    
    let mut rng = ctx.get_rng();
    if let Some(pk) = pk.get(0) {
        let message = Message::strand_deserialize(&pk.message)?;
        let bytes = message.artifact.unwrap();
        let dkgpk = DkgPublicKey::<C>::strand_deserialize(&bytes).unwrap();
        let pk_bytes = dkgpk.strand_serialize()?;
        let pk_h = strand::hash::hash_to_array(&pk_bytes)?;
        let pk_element = dkgpk.pk;
        let pk = strand::elgamal::PublicKey::from_element(&pk_element, &ctx);

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
    }
    else {
        return Err(anyhow!("Could not find public key or configuration artifact(s)"));
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
