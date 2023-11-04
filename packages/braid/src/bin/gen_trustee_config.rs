// This utility generates a trustee configuration printed to stdout with
//
//  * signing_key_sk: base64 encoding of a StrandSignatureSk serialization
//  * signing_key_pk: base64 encoding of corresponding StrandSignaturePk serialization
//  * encryption_key: base64 encoding of a sign::SymmetricKey
//

use base64::engine::general_purpose;
use base64::Engine;
use braid::run::config::TrusteeConfig;
use braid_messages::protocol_manager::{ProtocolManager, ProtocolManagerConfig};
use clap::Parser;
use std::marker::PhantomData;
use sequent_core::serialization::base64::Base64Serialize;

use strand::backend::ristretto::RistrettoCtx;
use strand::context::Ctx;
use strand::signature::{StrandSignaturePk, StrandSignatureSk};
use strand::symm;

#[derive(clap::ValueEnum, Clone)]
enum Command {
    Trustee,
    ProtocolManager,
}

#[derive(Parser)]
struct Cli {
    #[arg(value_enum, default_value_t = Command::Trustee)]
    command: Command,
}

fn main() {
    let args = Cli::parse();

    match &args.command {
        Command::Trustee => gen_trustee_config::<RistrettoCtx>(),
        Command::ProtocolManager => gen_protocol_manager_config::<RistrettoCtx>(),
    }
}

fn gen_trustee_config<C: Ctx>() {
    let sk = StrandSignatureSk::gen().unwrap();
    let pk = StrandSignaturePk::from(&sk).unwrap();
    let encryption_key: symm::SymmetricKey = symm::gen_key();

    let ek_bytes = encryption_key.as_slice();

    let sk_string: String = sk.serialize().unwrap();
    let pk_string: String = pk.serialize().unwrap();
    let ek_string: String = general_purpose::STANDARD_NO_PAD.encode(ek_bytes);

    let tc = TrusteeConfig {
        signing_key_sk: sk_string,
        signing_key_pk: pk_string,
        encryption_key: ek_string,
    };

    let toml = toml::to_string(&tc).unwrap();
    println!("{toml}");
}

fn gen_protocol_manager_config<C: Ctx>() {
    let pmkey: StrandSignatureSk = StrandSignatureSk::gen().unwrap();
    let pm: ProtocolManager<C> = ProtocolManager {
        signing_key: pmkey,
        phantom: PhantomData,
    };
    let pm = ProtocolManagerConfig::from(&pm);
    let toml = toml::to_string(&pm).unwrap();
    println!("{toml}");
}
