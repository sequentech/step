// cargo run --bin bb_client --features=bb-test -- --server-url http://immudb:3322 --indexdb defaultboardindex --dbname defaultboard <init|ballots|list|boards>
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};
use clap::Parser;
use rayon::prelude::*;
use std::fs;
use std::marker::PhantomData;
use tracing::{info, instrument};

use braid::util::init_log;
use immu_board::{Board, BoardClient, BoardMessage};

use braid::protocol2::artifact::Configuration;
use braid::protocol2::artifact::DkgPublicKey;
use braid::protocol2::message::Message;
use braid::protocol2::predicate::PublicKeyHash;
use braid::protocol2::statement::StatementType;
use braid::protocol2::trustee::ProtocolManager;
use braid::run::config::ProtocolManagerConfig;
use strand::backend::ristretto::RistrettoCtx;
use strand::context::Ctx;
use strand::elgamal::Ciphertext;
use strand::serialization::StrandDeserialize;
use strand::serialization::StrandSerialize;
use strand::signature::StrandSignatureSk;

#[derive(Parser)]
struct Cli {
    #[arg(long)]
    server_url: String,
    
    #[arg(short, long)]
    dbname: String,
    
    #[arg(short, long)]
    indexdb: String,

    #[arg(value_enum)]
    command: Command,
}

#[derive(clap::ValueEnum, Clone)]
enum Command {
    Init,
    Ballots,
    Messages,
    Boards,
}

const PROTOCOL_MANAGER: &str = "pm.toml";
const CONFIG: &str = "config.bin";
const IMMUDB_USER: &str = "immudb";
const IMMUDB_PW: &str = "immudb";

#[tokio::main]
#[instrument]
async fn main() -> Result<()> {
    let ctx = RistrettoCtx;
    init_log(true);
    let args = Cli::parse();

    let mut board = BoardClient::new(&args.server_url, IMMUDB_USER, IMMUDB_PW).await?;

    match &args.command {
        Command::Init => {
            let cfg_bytes = fs::read("config.bin")
                .expect("Should have been able to read session configuration file at 'config.bin'");
            let configuration = Configuration::<RistrettoCtx>::strand_deserialize(&cfg_bytes)
                .map_err(|e| anyhow!("Could not deserialize configuration {}", e))?;

            init(&mut board, &args.dbname, configuration).await?;
        }
        Command::Ballots => {
            post_ballots(&mut board, &args.dbname, ctx).await?;
        }
        Command::Messages => {
            list_messages(&mut board, &args.dbname,).await?;
        }
        Command::Boards => {
            list_boards(&mut board, &args.indexdb).await?;
        }
    }

    Ok(())
}

#[instrument]
async fn init<C: Ctx>(board: &mut BoardClient, board_name: &str, configuration: Configuration<C>) -> Result<()> {
    let pm = get_pm(PhantomData);
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

#[instrument]
async fn list_boards(index: &mut BoardClient, indexdb: &str) -> Result<()> {
    let boards: Result<Vec<String>> = index
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

#[instrument]
async fn post_ballots<C: Ctx>(board: &mut BoardClient, board_name: &str, ctx: C) -> Result<()> {
    let messages: Vec<Message> = board
        .get_messages(&board_name, 0)
        .await?
        .iter()
        .map(|board_message: &BoardMessage| {
            Ok(Message::strand_deserialize(&board_message.message)?)
        })
        .collect::<Result<Vec<Message>>>()?;
    let mut rng = ctx.get_rng();

    for message in messages {
        let kind = message.statement.get_kind();
        info!("Found message kind {}", kind);
        if kind == StatementType::PublicKey {
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
            let contents =
                fs::read(CONFIG).expect("Should have been able to read session configuration file");

            let configuration = Configuration::<C>::strand_deserialize(&contents)
                .map_err(|e| anyhow!("Could not read configuration {}", e))?;

            let threshold = [1, 2];
            let mut selected_trustees =
                [braid::protocol2::datalog::NULL_TRUSTEE; braid::protocol2::MAX_TRUSTEES];
            selected_trustees[0..threshold.len()].copy_from_slice(&threshold);

            let ballot_batch = braid::protocol2::artifact::Ballots::new(ballots);
            let pm = get_pm(PhantomData);
            let message = braid::protocol2::message::Message::ballots_msg(
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

            break;
        }
    }

    Ok(())
}

fn get_pm<C: Ctx>(ctxp: PhantomData<C>) -> ProtocolManager<C> {
    let contents = fs::read_to_string(PROTOCOL_MANAGER)
        .expect("Should have been able to read the protocol manager file");

    let pm_config: ProtocolManagerConfig = toml::from_str(&contents).unwrap();
    let bytes = general_purpose::STANDARD_NO_PAD
        .decode(pm_config.signing_key)
        .unwrap();
    let sk = StrandSignatureSk::strand_deserialize(&bytes).unwrap();
    let pm: ProtocolManager<C> = ProtocolManager {
        signing_key: sk,
        phantom: ctxp,
    };

    pm
}
