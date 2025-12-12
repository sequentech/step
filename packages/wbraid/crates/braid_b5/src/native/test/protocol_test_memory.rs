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

use cryptography::context::Context;
use cryptography::traits::groups::CryptographicGroup;
use cryptography::cryptosystem::elgamal::PublicKey;
use cryptography::cryptosystem::elgamal::Ciphertext;
use cryptography::utils::serialization::variable::VSerializable;
use cryptography::context::RistrettoCtx;

use b5::messages::artifact::{Configuration, Plaintexts};
use b5::messages::message::Message;
use b5::messages::newtypes::PublicKeyHash;
use b5::messages::newtypes::MAX_TRUSTEES;
use b5::messages::newtypes::NULL_TRUSTEE;
use b5::messages::protocol_manager::ProtocolManager;

use crate::protocol::trustee::Trustee;
use crate::native::test::vector_board::VectorBoard;
use crate::native::test::vector_session::VectorSession;

pub fn run<C: Context + 'static>(ciphertexts: u32, batches: usize) {
    let n_trustees = rand::rng().random_range(2..=MAX_TRUSTEES);
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
    let test: ProtocolTest<RistrettoCtx> = create_protocol_test(n_trustees, &threshold).unwrap();
    run_protocol_test(test, ciphertexts, batches, &threshold).unwrap();

    let time = now.elapsed().as_millis() as f64 / 1000.0;
    info!(
        "batches = {}, time = {}, rate = {}",
        batches,
        time,
        ((ciphertexts as f64 * batches as f64) / time),
    );
}

fn run_protocol_test<C: Context + 'static>(
    test: ProtocolTest<C>,
    ciphertexts: u32,
    batches: usize,
    threshold: &[usize],
) -> Result<()> {
    let remote = test.remote.clone();
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

    let pk_bytes = dkgpk.ser();
    let pk_h = b5::hash_to_array(&pk_bytes)?;

    let pk_element = dkgpk.pk;
    let pk = PublicKey::<C>::new(pk_element);

    let mut plaintexts_in = vec![];
    let mut rng = C::get_rng();
    for i in 0..batches {
        info!("Generating {} plaintexts..", count);
        let next_p: Vec<[C::Element; 2]> = (0..count).map(|_| [C::G::random_element(&mut rng), C::G::random_element(&mut rng)]).collect();

        info!("Encrypting {} ciphertexts..", next_p.len());

        let ballots: Vec<Ciphertext<C, 2>> = next_p
            .par_iter()
            .map(|p| {
                pk.encrypt(p)
            })
            .collect();
        let ballot_batch = b5::messages::artifact::Ballots::new(ballots);

        let message = Message::ballots_msg(
            &test.cfg,
            (i + 1) as u64,
            &ballot_batch,
            selected_trustees,
            PublicKeyHash(crate::util::hash_from_vec(&pk_h).unwrap()),
            &test.protocol_manager,
        )?;
        plaintexts_in.push(next_p);
        data.lock().unwrap().add(message);
    }

    let mut plaintexts_out: Option<Vec<b5::messages::artifact::Plaintexts<C, 2>>> = None;
    for i in 0..30 {
        info!("Cycle {}", i);

        sessions.par_iter_mut().for_each(|t| {
            t.step();
        });

        let decryptor = selected_trustees[0] - 1;
        let plaintexts: Vec<b5::messages::artifact::Plaintexts<C, 2>> = (0..batches)
            .filter_map(|b| sessions[decryptor].get_plaintexts_nohash((b + 1) as u64, decryptor))
            .map(|p| Plaintexts::<C, 2>(p.0.clone()))
            .collect();

        if plaintexts.len() == batches {
            plaintexts_out = Some(plaintexts);
            break;
        }
    }

    if let Some(plaintexts) = plaintexts_out {
        for (i, p) in plaintexts.iter().enumerate() {
            let expected: HashSet<[C::Element; 2]> = HashSet::from_iter(plaintexts_in[i].clone());
            let actual: HashSet<[C::Element; 2]> = HashSet::from_iter(p.0.clone());
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

pub struct ProtocolTest<C: Context> {
    pub cfg: Configuration<C>,
    pub protocol_manager: ProtocolManager<C>,
    pub trustees: Vec<Trustee<C, crate::native::board::NoOpStorage>>,
    pub remote: VectorBoard,
}

pub fn create_protocol_test<C: Context>(
    n_trustees: usize,
    threshold: &[usize],
) -> Result<ProtocolTest<C>> {
    let session_id = 0;

    use cryptography::utils::signatures::SignatureScheme;
    let mut rng = C::get_rng();
    let pmkey = C::SignatureScheme::gen_signing_key(&mut rng);
    let pm: ProtocolManager<C> = ProtocolManager {
        signing_key: pmkey,
        phantom: PhantomData,
    };
    let (trustees, trustee_pks): (Vec<Trustee<C, crate::native::board::NoOpStorage>>, Vec<<C::SignatureScheme as SignatureScheme<C::Rng>>::Verifier>) = (0..n_trustees)
        .map(|i| {
            let sk = C::SignatureScheme::gen_signing_key(&mut rng);
            // let encryption_key = ChaCha20Poly1305::generate_key(&mut csprng);
            let encryption_key = cryptography::utils::symm::gen_key();
            let pk = C::SignatureScheme::verifying_key(&sk);
            (
                Trustee::new(
                    i.to_string(),
                    "foo".to_string(),
                    sk,
                    encryption_key,
                    crate::native::board::NoOpStorage::new(),
                    None,
                ),
                pk,
            )
        })
        .unzip();

    let cfg = Configuration::<C>::new(
        0,
        C::SignatureScheme::verifying_key(&pm.signing_key),
        trustee_pks,
        threshold.len(),
        2, // ciphertext_width
        PhantomData,
    );

    let mut remote = VectorBoard::new(session_id);
    let message = Message::bootstrap_msg(&cfg, &pm)?;
    remote.add(message);

    Ok(ProtocolTest {
        cfg,
        protocol_manager: pm,
        trustees,
        remote,
    })
}
