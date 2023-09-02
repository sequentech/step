// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

#[macro_use]
extern crate rocket;

use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};
use braid::util::init_log;
use clap::Parser;
use rocket::response::Debug;
use rocket::serde::json::Json;
use rocket::serde::json::Value;
use rocket::serde::{Deserialize, Serialize};
use std::format;
use std::fs;
use std::marker::PhantomData;
use std::path::PathBuf;
use tracing::{info, instrument};

use braid::protocol2::artifact::Configuration;
use braid::protocol2::artifact::DkgPublicKey;
use braid::protocol2::message::Message;
use braid::protocol2::predicate::PublicKeyHash;
use braid::protocol2::statement::StatementType;
use braid::protocol2::trustee::ProtocolManager;
use braid::run::config::ProtocolManagerConfig;
use immu_board::{Board, BoardClient, BoardMessage};
use strand::backend::ristretto::RistrettoCtx;
use strand::context::Ctx;
use strand::elgamal::Ciphertext;
use strand::serialization::StrandDeserialize;
use strand::serialization::StrandSerialize;
use strand::signature::StrandSignaturePk;
use strand::signature::StrandSignatureSk;

const IMMUDB_USER: &str = "immudb";
const IMMUDB_PW: &str = "immudb";
const PROTOCOL_MANAGER: &str = "pm.toml";
const BOARD_NAME: &str = "33f18502a67c48538333a58630663559";

#[derive(Parser)]
struct Cli {
    #[arg(short, long)]
    server_url: String,

    #[arg(short, long)]
    config: PathBuf,

    #[arg(short, long, default_value_t = IMMUDB_USER.to_string())]
    user: String,

    #[arg(short, long, default_value_t = IMMUDB_PW.to_string())]
    password: String,
}

#[launch]
#[instrument]
async fn rocket() -> _ {
    init_log(true);

    rocket::build().mount("/", routes![create_config])
}

#[derive(Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct CreateKeysBody {
    board_name: String,
    trustee_pks: Vec<String>,
    threshold: usize,
}

#[post("/create-keys", format = "json", data = "<body>")]
async fn create_config(body: Json<CreateKeysBody>) -> Result<(), Debug<anyhow::Error>> {
    let input = body.into_inner();

    call_method::<RistrettoCtx>(input).await?;
    Ok(())
}

async fn call_method<C: Ctx>(input: CreateKeysBody) -> Result<()> {
    let trustee_pks = input
        .trustee_pks
        .iter()
        .map(|public_key_string| {
            let public_key: StrandSignaturePk = public_key_string.try_into().unwrap();
            public_key
        })
        .collect();

    let args = Cli::parse();
    let pm = get_pm(PhantomData, args.config);
    let configuration = Configuration::<C>::new(
        0,
        StrandSignaturePk::from(&pm.signing_key),
        trustee_pks,
        input.threshold,
        PhantomData,
    );

    let mut board = BoardClient::new(&args.server_url, &args.user, &args.password).await?;

    init(&mut board, configuration, pm, input.board_name.as_str()).await
}

#[instrument]
async fn init<C: Ctx>(
    board: &mut BoardClient,
    configuration: Configuration<C>,
    pm: ProtocolManager<C>,
    board_name: &str,
) -> Result<()> {
    let message: BoardMessage = Message::bootstrap_msg(&configuration, &pm)?.try_into()?;
    info!("Adding configuration to the board..");
    board.insert_messages(board_name, &vec![message]).await
}

fn get_pm<C: Ctx>(ctxp: PhantomData<C>, config: PathBuf) -> ProtocolManager<C> {
    let contents = fs::read_to_string(config)
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
