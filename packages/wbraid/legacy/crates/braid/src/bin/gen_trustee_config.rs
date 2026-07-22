// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use b4::messages::protocol_manager::{ProtocolManager, ProtocolManagerConfig};
use braid::protocol::trustee::TrusteeConfig;
use clap::Parser;
use std::marker::PhantomData;

use cryptography::context::RistrettoCtx;
use cryptography::context::Context;
use cryptography::utils::signatures::SignatureScheme;
use cryptography::utils::symm;

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
/// * signing_key_pk: base64 encoding of corresponding VerifyingKey serialization
/// * encryption_key: base64 encoding of a sign::SymmetricKey
///
/// Protocol manager configuration contains
///
///  * signing_key_sk: base64 encoding of a der encoded pkcs#8 v1 encoding.
///
/// The randomness is provided by the cryptography crate.
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
fn gen_trustee_config<C: Context>() {
    let mut rng = C::get_rng();
    let sk = <C::SignatureScheme as SignatureScheme<C::Rng>>::gen_signing_key(&mut rng);
    let encryption_key: symm::SymmetricKey = symm::gen_key().unwrap();

    let tc = TrusteeConfig::new_from_objects::<C>(sk, encryption_key);

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
fn gen_protocol_manager_config<C: Context>() {
    let mut rng = C::get_rng();
    let pmkey = <C::SignatureScheme as SignatureScheme<C::Rng>>::gen_signing_key(&mut rng);
    let pm: ProtocolManager<C> = ProtocolManager {
        signing_key: pmkey,
        phantom: PhantomData,
    };
    let pm = ProtocolManagerConfig::from(&pm);
    let toml = toml::to_string(&pm).unwrap();
    println!("{toml}");
}
