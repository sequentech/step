// cargo run --bin bb_client -- --server-url https://localhost:3000 --cache-dir /tmp init

cfg_if::cfg_if! {
    if #[cfg(feature = "bb-test")] {
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};
use clap::Parser;
use rayon::prelude::*;
use std::fs;
use std::marker::PhantomData;
use std::path::PathBuf;
use tracing::{info, instrument};
use uuid::Uuid;

use bulletin_board::client::CacheStore;
use bulletin_board::client::{Client, FileCache};
use bulletin_board::util::init_log;
use bulletin_board::{
    board_entry::Kind, AddEntriesRequest, NewDataEntry, BoardEntryData, CreateBoardRequest, ListBoardItem,
    ListBoardsRequest, ListEntriesRequest, Permissions, Role, User, UserRole
};
use bulletin_board::signature::Signable;
 
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
use strand::signature::StrandSignaturePk;

#[derive(Parser)]
struct Cli {
    #[arg(long)]
    cache_dir: PathBuf,

    #[arg(long)]
    server_url: String,

    #[arg(value_enum)]
    post: Post,
}

#[derive(clap::ValueEnum, Clone)]
enum Post {
    Init,
    Ballots,
    List,
}

const BOARD_NAME: &str = "default-board";
const PROTOCOL_MANAGER: &str = "pm.toml";
const CONFIG: &str = "config.bin";

#[tokio::main]
#[instrument]
async fn main() -> Result<()> {
    let ctx = RistrettoCtx;
    init_log().map_err(|error| anyhow!(error))?;
    let args = Cli::parse();
    let cache = FileCache::new(&args.cache_dir)?;
    let mut client = Client::new(args.server_url, cache).await?;

    match &args.post {
        Post::Init => {
            let cfg_bytes = fs::read("config.bin")
                .expect("Should have been able to read session configuration file at 'config.bin'");
            let configuration = Configuration::<RistrettoCtx>::strand_deserialize(&cfg_bytes)
                .map_err(|e| anyhow!("Could not deserialize configuration {}", e))?;

            init(&mut client, configuration).await?;
        }
        Post::Ballots => {
            post_ballots(&mut client, ctx).await?;
        }
        Post::List => {
            list_entries(&mut client).await?;
        }
    }

    Ok(())
}

async fn init<C: Ctx, CS: CacheStore>(
    client: &mut Client<CS>,
    configuration: Configuration<C>,
) -> Result<()> {
    let board_name = BOARD_NAME.to_string();
    let board = get_board(client).await;
    assert!(board.is_err());

    let (sk,pk) = get_admin_keys();

    let board_uuid = Uuid::new_v4().to_string();
    let request = CreateBoardRequest {
        board_uuid: board_uuid.clone(),
        board_name: board_name.clone(),
        permissions: Some(Permissions {
            users: vec![User {
                name: "admin".into(),
                public_key: ADMIN_PK.into(),
                ..Default::default()
            }],
            roles: vec![Role {
                name: "admins".into(),
                permissions: vec!["AddEntries".into()],
                ..Default::default()
            }],
            user_roles: vec![UserRole {
                user_name: "admin".into(),
                role_names: vec!["admins".into()],
            }],
            ..Default::default()
        }),
        ..Default::default()
    }
    .sign(&sk)
    .unwrap();

    let _response = client.create_board(request).await?;
    info!("Board created with uuid={board_uuid}");

    let pm = get_pm(PhantomData);
    let message = Message::bootstrap_msg(&configuration, &pm)?;

    let request = AddEntriesRequest {
        board_uuid: board_uuid.clone(),
        entries: vec![NewDataEntry {
            data: message.strand_serialize()?,
            ..Default::default()
        }
        .sign(&sk)
        .unwrap()]
    };


    info!("Adding configuration to the board..");
    let response = client.add_entries(request, true).await?;

    let entry_sequence_id = response
        .get_ref()
        .entries[0]
        .sequence_id;

    info!("New entry added with entry_sequence_id={entry_sequence_id:?}");

    Ok(())
}

async fn list_entries<C: CacheStore>(client: &mut Client<C>) -> Result<()> {
    let id = get_board(client).await?;
    info!("Board id is '{id}'");

    let request = ListEntriesRequest {
        board_uuid: id,
        start_sequence_id: 0,
    };

    let response = client.list_entries(request).await?;
    let entries = response.get_ref().board_entries.clone();
    info!("List entries response contains {} entries", entries.len());
    info!("=========================================");
    for entry in entries {
        if let Some(Kind::EntryData(BoardEntryData {
            data: Some(entry_data),
        })) = &entry.kind
        {
            if entry.sequence_id > 0 {
                let message = Message::strand_deserialize(entry_data).unwrap();
                info!("* {:?}", message);
            }
        } else {
            info!("Skipping non entry data at {}", entry.sequence_id);
        }
    }
    info!("=========================================");

    Ok(())
}

async fn post_ballots<C: Ctx, CS: CacheStore>(client: &mut Client<CS>, ctx: C) -> Result<()> {
    let id = get_board(client).await?;
    info!("Board id is '{id}'");

    let (sk,pk) = get_admin_keys();

    let request = ListEntriesRequest {
        board_uuid: id.clone(),
        start_sequence_id: 1,
    };

    let response = client.list_entries(request).await?;
    let entries = response.get_ref().board_entries.clone();
    info!("List entries response contains {} entries", entries.len());

    for entry in entries {
        if let Some(Kind::EntryData(BoardEntryData {
            data: Some(entry_data),
        })) = &entry.kind
        {
            let message = Message::strand_deserialize(entry_data).unwrap();
            let kind = message.statement.get_kind();
            info!("Found message kind {}", kind);
            if kind == StatementType::PublicKey {
                let bytes = message.artifact.unwrap();
                let dkgpk = DkgPublicKey::<C>::strand_deserialize(&bytes).unwrap();
                let pk_bytes = dkgpk.strand_serialize()?;
                let pk_h = strand::util::hash_array(&pk_bytes);
                let pk_element = dkgpk.pk;
                let pk = strand::elgamal::PublicKey::from_element(&pk_element, &ctx);
                
                // let pk_h = strand::util::hash(&pk.strand_serialize().unwrap());

                let ps: Vec<C::P> = (0..1000).map(|_| ctx.rnd_plaintext()).collect();
                let ballots: Vec<Ciphertext<C>> = ps
                    .par_iter()
                    .map(|p| {
                        let encoded = ctx.encode(p).unwrap();
                        pk.encrypt(&encoded)
                    })
                    .collect();

                info!("Generated {} ballots", ballots.len());
                let contents = fs::read(CONFIG)
                    .expect("Should have been able to read session configuration file");

                let configuration = Configuration::<C>::strand_deserialize(&contents)
                    .map_err(|e| anyhow!("Could not read configuration {}", e))?;

                let threshold = [1, 2];
                let mut selected_slots =
                    [braid::protocol2::datalog::NULL_TRUSTEE; braid::protocol2::MAX_TRUSTEES];
                selected_slots[0..threshold.len()].copy_from_slice(&threshold);

                let ballot_batch = braid::protocol2::artifact::Ballots::new(
                    ballots,
                    selected_slots,
                    &configuration,
                );
                let pm = get_pm(PhantomData);
                let message = braid::protocol2::message::Message::ballots_msg(
                    &configuration,
                    1,
                    &ballot_batch,
                    PublicKeyHash(strand::util::to_u8_array(&pk_h).unwrap()),
                    &pm,
                )?;

                let request = AddEntriesRequest {
                    board_uuid: id.clone(),
                    entries: vec![ NewDataEntry {
                        data: message.strand_serialize()?,
                        ..Default::default()
                    }
                    .sign(&sk)
                    .unwrap()]
                };

                info!("Adding ballots to the board..");
                let response = client.add_entries(request, true).await?;

                let entry_sequence_id = response
                    .get_ref()
                    .entries[0]
                    .sequence_id;
                info!("New entry added with entry_sequence_id={entry_sequence_id:?}");

                break;
            }
        } else {
            info!("Skipping non entry data at {}", entry.sequence_id);
        }
    }

    Ok(())
}

async fn get_board<CS: CacheStore>(client: &mut Client<CS>) -> Result<String> {
    let board_name = BOARD_NAME.to_string();
    let request = ListBoardsRequest {
        board_name: Some(board_name.clone()),
        ..Default::default()
    };
    let response = client.list_boards(request).await?;
    let boards: &Vec<ListBoardItem> = &response.get_ref().boards;

    if boards.len() == 1 {
        let ListBoardItem { board, .. } = boards.get(0).ok_or(anyhow!("BoardRetrievalError"))?;
        let board_uuid = board.clone().unwrap().uuid;

        Ok(board_uuid)
    } else {
        Err(anyhow::Error::msg("Expected 1 result"))
    }
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

const ADMIN_SK: &str = "gZai7r2m5/9bAV2vmtxFOXoUL8UEMBnOPZ//0eoBX2g";
const ADMIN_PK: &str = "NbkLVEFH7IOz9MAwpp9o7VmegTum4t9YSRo367dQ8ok";

fn get_admin_keys() -> (StrandSignatureSk, StrandSignaturePk) {
    let bytes = general_purpose::STANDARD_NO_PAD
        .decode(ADMIN_SK)
        .map_err(|error| anyhow!(error))
        .unwrap();
    let sk = StrandSignatureSk::strand_deserialize(&bytes).unwrap();

    let bytes = general_purpose::STANDARD_NO_PAD
        .decode(ADMIN_PK)
        .unwrap();

    let pk = StrandSignaturePk::strand_deserialize(&bytes).unwrap();

    (sk, pk)
}
}
else {
    fn main() {
        println!("Requires the 'bb-test' feature");
    }
}
}
