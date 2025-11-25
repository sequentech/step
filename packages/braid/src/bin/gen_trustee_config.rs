// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use b3::messages::protocol_manager::{ProtocolManager, ProtocolManagerConfig};
use base64::engine::general_purpose;
use base64::Engine;
use braid::protocol::trustee2::TrusteeConfig;
use clap::Parser;
use std::marker::PhantomData;

use strand::backend::ristretto::RistrettoCtx;
use strand::context::Ctx;
use strand::signature::{StrandSignaturePk, StrandSignatureSk};
use strand::symm;

#[derive(clap::ValueEnum, Clone)]
enum Command {
    Trustee,
    ProtocolManager,
}

/// This utility generates a trustee or protocol manager configuration printed to stdout.
#[derive(Parser)]
struct Cli {
    /// Whether to generate a trustee or protocol configuration file.
    #[arg(value_enum, default_value_t = Command::Trustee)]
    command: Command,
}

/// This utility generates a trustee or protocol manager configuration printed to stdout.
///
/// Trustee configuration contains
///
/// * signing_key_sk: base64 encoding of a der encoded pkcs#8 v1 encoding
/// * signing_key_pk: base64 encoding of corresponding StrandSignaturePk serialization
/// * encryption_key: base64 encoding of a sign::SymmetricKey
///
/// Protocol manager configuration contains
///
///  * signing_key_sk: base64 encoding of a der encoded pkcs#8 v1 encoding.
///
/// The randomness is provided by strand, see the strand::rand module.
fn main() {
    let args = Cli::parse();

    match &args.command {
        Command::Trustee => gen_trustee_config::<RistrettoCtx>(),
        Command::ProtocolManager => gen_protocol_manager_config::<RistrettoCtx>(),
    }
}

/// Generates a trustee configuration with cryptographic secrets.
///
/// Prints configuration to standard out.
fn gen_trustee_config<C: Ctx>() {
    let sk = StrandSignatureSk::gen().unwrap();
    let pk = StrandSignaturePk::from_sk(&sk).unwrap();
    let encryption_key: symm::SymmetricKey = symm::gen_key();

    let ek_bytes = encryption_key.as_slice();

    let sk_string: String = sk.to_der_b64_string().unwrap();
    let pk_string: String = pk.to_der_b64_string().unwrap();
    let ek_string: String = general_purpose::STANDARD_NO_PAD.encode(ek_bytes);

    let tc = TrusteeConfig {
        signing_key_sk: sk_string,
        signing_key_pk: pk_string,
        encryption_key: ek_string,
    };

    let toml = toml::to_string(&tc).unwrap();
    println!("{toml}");
}

/// Generates a protocol manager configuration with cryptographic secrets.
///
/// The protocol manager is the entity responsible for posting the protocol
/// configuration and ballots. Those messages must be signed by the entity
/// designated as protocol manager in the configuration.
///
/// Prints configuration to standard out.
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
