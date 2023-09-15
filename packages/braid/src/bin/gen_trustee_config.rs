// This utility generates a trustee configuration printed to stdout with
//
//  * signing_key_sk: base64 encoding of a StrandSignatureSk serialization
//  * signing_key_pk: base64 encoding of corresponding StrandSignaturePk serialization
//  * encryption_key: base64 encoding of a GenericArray<u8, U32>,
//

use base64::engine::general_purpose;
use base64::Engine;
use braid::protocol2::trustee::ProtocolManager;
use braid::run::config::{ProtocolManagerConfig, TrusteeConfig};
use chacha20poly1305::{aead::KeyInit, ChaCha20Poly1305};
use clap::Parser;
use std::marker::PhantomData;

use strand::backend::ristretto::RistrettoCtx;
use strand::context::Ctx;
use strand::rng::StrandRng;
use strand::serialization::StrandSerialize;
use strand::signature::{StrandSignaturePk, StrandSignatureSk};

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
    let mut csprng = StrandRng;

    let sk = StrandSignatureSk::new(&mut csprng);
    let pk = StrandSignaturePk::from(&sk);
    let encryption_key = ChaCha20Poly1305::generate_key(&mut chacha20poly1305::aead::OsRng);

    let sk_bytes = sk.strand_serialize().unwrap();
    let pk_bytes = pk.strand_serialize().unwrap();
    let ek_bytes = encryption_key.as_slice();

    let sk_string: String = general_purpose::STANDARD_NO_PAD.encode(sk_bytes);
    let pk_string: String = general_purpose::STANDARD_NO_PAD.encode(pk_bytes);
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
    let mut csprng = StrandRng;

    let pmkey: StrandSignatureSk = StrandSignatureSk::new(&mut csprng);
    let pm: ProtocolManager<C> = ProtocolManager {
        signing_key: pmkey,
        phantom: PhantomData,
    };
    let pm = ProtocolManagerConfig::from(&pm);
    let toml = toml::to_string(&pm).unwrap();
    println!("{toml}");
}
