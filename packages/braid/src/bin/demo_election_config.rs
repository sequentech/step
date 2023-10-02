// This demo utility generates all the configuration information
// necessary to create a demo election, as files in the working directory:
//
// * Generate .toml config for each trustee, containing:
//      * signing_key_sk: base64 encoding of a StrandSignatureSk serialization
//      * signing_key_pk: base64 encoding of corresponding StrandSignaturePk serialization
//      * encryption_key: base64 encoding of a sign::SymmetricKey
// * Generate .toml config for the protocol manager:
//      signing_key: base64 encoding of a StrandSignatureSk serialization
// * Generate a .bin config for a session, a serialized Configuration artifact
//

use std::fs::File;
use std::io::prelude::*;
use std::marker::PhantomData;

use strand::backend::ristretto::RistrettoCtx;
use strand::context::Ctx;
use strand::serialization::StrandSerialize;
use strand::signature::{StrandSignaturePk, StrandSignatureSk};
use strand::symm;

use braid_messages::artifact::Configuration;
use braid::protocol2::trustee::ProtocolManager;
use braid::protocol2::trustee::Trustee;
use braid::run::config::{ProtocolManagerConfig, TrusteeConfig};

const CONFIG: &str = "config.bin";
const PROTOCOL_MANAGER: &str = "pm.toml";

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
    let (trustees, trustee_pks): (Vec<Trustee<C>>, Vec<StrandSignaturePk>) = (0..n_trustees)
        .map(|_| {
            let sk = StrandSignatureSk::gen().unwrap();
            let pk = StrandSignaturePk::from(&sk).unwrap();
            let encryption_key: symm::SymmetricKey = symm::gen_key();
            (Trustee::new(sk, encryption_key), pk)
        })
        .unzip();

    let cfg = Configuration::<C>::new(
        0,
        StrandSignaturePk::from(&pm.signing_key).unwrap(),
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
