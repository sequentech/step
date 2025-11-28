// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use log::{error, info};
use rand::seq::IndexedRandom;
use rand::Rng;
use rayon::prelude::*;
use std::collections::HashSet;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use strand::context::Ctx;
use strand::elgamal::Ciphertext;
use strand::serialization::StrandSerialize;
use strand::signature::{StrandSignaturePk, StrandSignatureSk};

use b3::messages::artifact::{Ballots, Configuration, Plaintexts};
use b3::messages::message::Message;
use b3::messages::newtypes::PublicKeyHash;
use b3::messages::newtypes::MAX_TRUSTEES;
use b3::messages::newtypes::NULL_TRUSTEE;
use b3::messages::protocol_manager::ProtocolManager;

use crate::protocol::trustee::Trustee;
use crate::test::vector_board::VectorBoard;
use crate::test::vector_session::VectorSession;

pub fn run<C: Ctx + 'static>(ciphertexts: u32, batches: usize, ctx: C) {
    let n_trustees = rand::rng().random_range(2..13);
    let n_threshold = rand::rng().random_range(2..=n_trustees);
    // To test all trustees participating
    // let n_trustees = 12;
    // let n_threshold = n_trustees;
    let max: [usize; 12] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
    let all = &max[0..n_trustees];
    let mut rng = &mut rand::rng();
    let threshold: Vec<usize> = all
        .choose_multiple(&mut rng, n_threshold)
        .cloned()
        .collect();

    let now = Instant::now();
    let test = create_protocol_test(n_trustees, &threshold, ctx).unwrap();
    run_protocol_test(test, ciphertexts, batches, &threshold).unwrap();

    let time = now.elapsed().as_millis() as f64 / 1000.0;
    info!(
        "batches = {}, time = {}, rate = {}",
        batches,
        time,
        ((ciphertexts as f64 * batches as f64) / time),
    );
}

fn run_protocol_test<C: Ctx + 'static>(
    test: ProtocolTest<C>,
    ciphertexts: u32,
    batches: usize,
    threshold: &[usize],
) -> Result<()> {
    info!("{}", strand::info_string());

    let remote = test.remote.clone();
    let ctx = test.ctx.clone();
    let mut sessions = vec![];
    let data = Arc::new(Mutex::new(remote));

    for t in test.trustees.into_iter() {
        sessions.push(VectorSession::new(t, Arc::clone(&data)));
    }

    let mut dkg_pk = None;
    let count = ciphertexts;

    let mut selected_trustees = [NULL_TRUSTEE; MAX_TRUSTEES];
    selected_trustees[0..threshold.len()].copy_from_slice(threshold);

    for i in 0..30 {
        info!("Cycle {}", i);

        sessions.par_iter_mut().for_each(|t| {
            t.step();
        });
        let dkg_pk_ = sessions[0].get_dkg_public_key_nohash();
        if dkg_pk_.is_some() {
            dkg_pk = dkg_pk_;
            break;
        }
    }

    let dkgpk = dkg_pk.unwrap();

    let pk_bytes = dkgpk.strand_serialize()?;
    let pk_h = strand::hash::hash_to_array(&pk_bytes)?;

    let pk_element = dkgpk.pk;
    let pk = strand::elgamal::PublicKey::from_element(&pk_element, &test.ctx);

    let mut plaintexts_in = vec![];
    let mut rng = ctx.get_rng();
    for i in 0..batches {
        info!("Generating {} plaintexts..", count);
        let next_p: Vec<C::P> = (0..count).map(|_| ctx.rnd_plaintext(&mut rng)).collect();

        info!("Encrypting {} ciphertexts..", next_p.len());

        let ballots: Vec<Ciphertext<C>> = next_p
            .par_iter()
            .map(|p| {
                let encoded = ctx.encode(p).unwrap();
                pk.encrypt(&encoded)
            })
            .collect();
        let ballot_batch = Ballots::new(ballots);

        let message = Message::ballots_msg(
            &test.cfg,
            i + 1,
            &ballot_batch,
            selected_trustees,
            PublicKeyHash(crate::util::hash_from_vec(&pk_h).unwrap()),
            &test.protocol_manager,
        )?;
        plaintexts_in.push(next_p);
        data.lock().unwrap().add(message);
    }

    let mut plaintexts_out: Option<Vec<Plaintexts<C>>> = None;
    for i in 0..30 {
        info!("Cycle {}", i);

        sessions.par_iter_mut().for_each(|t| {
            t.step();
        });

        let decryptor = selected_trustees[0] - 1;
        let plaintexts: Vec<Plaintexts<C>> = (0..batches)
            .filter_map(|b| sessions[decryptor].get_plaintexts_nohash(b + 1, decryptor))
            .map(|p| Plaintexts::<C>(p.0.clone()))
            .collect();

        if plaintexts.len() == batches {
            plaintexts_out = Some(plaintexts);
            break;
        }
    }

    if let Some(plaintexts) = plaintexts_out {
        for (i, p) in plaintexts.iter().enumerate() {
            let expected: HashSet<C::P> = HashSet::from_iter(plaintexts_in[i].clone());
            let actual: HashSet<C::P> = HashSet::from_iter(p.0.clone().0);
            assert!(expected == actual);
            info!("Match ok on plaintexts for batch {}", i + 1);
        }
    } else {
        error!("No plaintexts found");
        panic!();
    }

    info!("***************************************************************");
    info!("* Completed");
    info!("* Trustees = {}", sessions.len());
    info!("* Threshold = {}", threshold.len());
    info!("* Ciphertexts = {}", count);
    info!("***************************************************************");

    Ok(())
}

pub struct ProtocolTest<C: Ctx> {
    pub ctx: C,
    pub cfg: Configuration<C>,
    pub protocol_manager: ProtocolManager<C>,
    pub trustees: Vec<Trustee<C>>,
    pub remote: VectorBoard,
}

pub fn create_protocol_test<C: Ctx>(
    n_trustees: usize,
    threshold: &[usize],
    ctx: C,
) -> Result<ProtocolTest<C>> {
    let session_id = 0;

    let pmkey: StrandSignatureSk = StrandSignatureSk::gen()?;
    let pm: ProtocolManager<C> = ProtocolManager {
        signing_key: pmkey,
        phantom: PhantomData,
    };
    let (trustees, trustee_pks): (Vec<Trustee<C>>, Vec<StrandSignaturePk>) = (0..n_trustees)
        .map(|i| {
            let sk = StrandSignatureSk::gen().unwrap();
            // let encryption_key = ChaCha20Poly1305::generate_key(&mut csprng);
            let encryption_key = strand::symm::gen_key();
            let pk = StrandSignaturePk::from_sk(&sk).unwrap();
            (
                Trustee::new(
                    i.to_string(),
                    "foo".to_string(),
                    sk,
                    encryption_key,
                    None,
                    None,
                ),
                pk,
            )
        })
        .unzip();

    let cfg = Configuration::<C>::new(
        0,
        StrandSignaturePk::from_sk(&pm.signing_key).unwrap(),
        trustee_pks,
        threshold.len(),
        PhantomData,
    );

    let mut remote = VectorBoard::new(session_id);
    let message = Message::bootstrap_msg(&cfg, &pm)?;
    remote.add(message);

    Ok(ProtocolTest {
        ctx,
        cfg,
        protocol_manager: pm,
        trustees,
        remote,
    })
}
