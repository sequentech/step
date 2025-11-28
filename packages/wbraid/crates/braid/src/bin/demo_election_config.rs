// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::fs::File;
use std::io::prelude::*;
use std::marker::PhantomData;

use strand::backend::ristretto::RistrettoCtx;
use strand::context::Ctx;
use strand::serialization::StrandSerialize;
use strand::signature::{StrandSignaturePk, StrandSignatureSk};
use strand::symm;

use b3::messages::artifact::Configuration;
use b3::messages::protocol_manager::{ProtocolManager, ProtocolManagerConfig};
use braid::protocol::trustee::TrusteeConfig;

const CONFIG: &str = "config.bin";
const PROTOCOL_MANAGER: &str = "pm.toml";

/// This demo utility generates all the configuration information
/// necessary to create a demo election, as files in the working directory:
///
/// * Generate .toml config for each trustee, containing:
///      * signing_key_sk: base64 encoding of a der encoded pkcs#8 v1
///      * signing_key_pk: base64 encoding of a der encoded spki
///      * encryption_key: base64 encoding of a sign::SymmetricKey
/// * Generate .toml config for the protocol manager:
///      signing_key: base64 encoding of a der encoded pkcs#8 v1
/// * Generate a .bin config for a session, a serialized Configuration artifact
///
/// FIXME: made obsolete by demo_tool.
fn main() {
    let threshold = [1, 2];
    gen_election_config::<RistrettoCtx>(3, &threshold);
}

fn gen_election_config<C: Ctx>(n_trustees: usize, threshold: &[usize]) {
    let pmkey: StrandSignatureSk = StrandSignatureSk::gen().unwrap();
    let pm: ProtocolManager<C> = ProtocolManager {
        signing_key: pmkey,
        phantom: PhantomData,
    };
    let (trustees, trustee_pks): (Vec<TrusteeConfig>, Vec<StrandSignaturePk>) = (0..n_trustees)
        .map(|_i| {
            let sk = StrandSignatureSk::gen().unwrap();
            let pk = StrandSignaturePk::from_sk(&sk).unwrap();
            let encryption_key: symm::SymmetricKey = symm::gen_key();
            let tc = TrusteeConfig::new_from_objects(sk, encryption_key);
            (tc, pk)
        })
        .unzip();

    let cfg = Configuration::<C>::new(
        0,
        StrandSignaturePk::from_sk(&pm.signing_key).unwrap(),
        trustee_pks,
        threshold.len(),
        PhantomData,
    );
    let cfg_bytes = cfg.strand_serialize().unwrap();
    let mut file = File::create(CONFIG).unwrap();
    file.write_all(&cfg_bytes).unwrap();

    let pm = ProtocolManagerConfig::from(&pm);
    let toml = toml::to_string(&pm).unwrap();
    let mut file = File::create(PROTOCOL_MANAGER).unwrap();
    file.write_all(toml.as_bytes()).unwrap();

    for (i, tc) in trustees.iter().enumerate() {
        let toml = toml::to_string(&tc).unwrap();
        let mut file = File::create(format!("trustee{}.toml", i + 1)).unwrap();
        file.write_all(toml.as_bytes()).unwrap();
    }
}
