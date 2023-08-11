// Need to
// * Generate .toml config for each trustee, containing:
//      signing_key: base64 encoding of a StrandSignatureSk serialization
//      encryption_key: base64 encoding of a GenericArray<u8, U32>,
// * Generate .toml config for the protocol manager:
//      signing_key: base64 encoding of a StrandSignatureSk serialization
// * Generate a .bin config for a session, a serialized Configuration artifact
//
// The pks present in the session config must match the
// sks in each trustee config

use chacha20poly1305::{aead::KeyInit, ChaCha20Poly1305};
use std::fs::File;
use std::io::prelude::*;
use std::marker::PhantomData;

use strand::backend::ristretto::RistrettoCtx;
use strand::context::Ctx;
use strand::rnd::StrandRng;
use strand::serialization::StrandSerialize;
use strand::signature::{StrandSignaturePk, StrandSignatureSk};

use braid::protocol2::artifact::Configuration;
use braid::protocol2::trustee::ProtocolManager;
use braid::protocol2::trustee::Trustee;
use braid::run::config::{ProtocolManagerConfig, TrusteeConfig};

const CONFIG: &str = "config.bin";
const PROTOCOL_MANAGER: &str = "pm.toml";

fn main() {
    let ctx = RistrettoCtx;
    let threshold = [1, 2];
    gen_config(3, &threshold, ctx);
}

fn gen_config<C: Ctx>(n_trustees: usize, threshold: &[usize], _ctx: C) {
    let mut csprng = StrandRng;

    let pmkey: StrandSignatureSk = StrandSignatureSk::new(&mut csprng);
    let pm: ProtocolManager<C> = ProtocolManager {
        signing_key: pmkey,
        phantom: PhantomData,
    };
    let (trustees, trustee_pks): (Vec<Trustee<C>>, Vec<StrandSignaturePk>) = (0..n_trustees)
        .map(|_| {
            let sk = StrandSignatureSk::new(&mut csprng);
            let encryption_key = ChaCha20Poly1305::generate_key(&mut chacha20poly1305::aead::OsRng);
            (
                Trustee::new(sk.clone(), encryption_key),
                StrandSignaturePk::from(&sk),
            )
        })
        .unzip();

    let cfg = Configuration::<C>::new(
        0,
        StrandSignaturePk::from(&pm.signing_key),
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

    for (i, t) in trustees.iter().enumerate() {
        let tc = TrusteeConfig::from(t);
        let toml = toml::to_string(&tc).unwrap();
        let mut file = File::create(format!("trustee{}.toml", i + 1)).unwrap();
        file.write_all(toml.as_bytes()).unwrap();
    }
}
