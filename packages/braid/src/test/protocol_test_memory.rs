// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
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

use crate::protocol::trustee2::Trustee;
use crate::test::vector_board::VectorBoard;
use crate::test::vector_session::VectorSession;

pub fn run<C: Ctx + 'static>(ciphertexts: u32, batches: usize, ctx: C) {
    let n_trustees = rand::thread_rng().gen_range(2..13);
    let n_threshold = rand::thread_rng().gen_range(2..=n_trustees);
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

#[cfg(test)]
mod tests {
    use super::*;
    use b3::messages::artifact::DecryptionFactors;
    use b3::messages::statement::Statement;
    use strand::backend::ristretto::RistrettoCtx;
    use strand::serialization::StrandDeserialize;

    type EditFactors = fn(&mut DecryptionFactors<RistrettoCtx>);

    const N_TRUSTEES: usize = 3;
    const SELECTED: [usize; 2] = [1, 2];
    const EDITING_TRUSTEE: usize = 1;
    const BATCH: usize = 1;
    const CIPHERTEXTS: usize = 6;
    const MAX_CYCLES: usize = 30;

    struct DecryptionRun {
        plaintexts_in: Vec<<RistrettoCtx as Ctx>::P>,
        plaintexts_out: Option<Plaintexts<RistrettoCtx>>,
        step_errors: Vec<String>,
    }

    fn resign_decryption_factors(
        message: Message,
        cfg: &Configuration<RistrettoCtx>,
        signer: &Trustee<RistrettoCtx>,
        edit: EditFactors,
    ) -> Message {
        let Statement::DecryptionFactors(_, _, batch, _, mix_h, shares_hs) =
            message.statement.clone()
        else {
            return message;
        };
        let artifact = message
            .artifact
            .as_ref()
            .expect("decryption factors message has an artifact");
        let mut dfactors = DecryptionFactors::<RistrettoCtx>::strand_deserialize(artifact).unwrap();
        edit(&mut dfactors);

        Message::decryption_factors_msg(cfg, batch, dfactors, mix_h, shares_hs, signer).unwrap()
    }

    fn step_all(
        trustees: &mut [Trustee<RistrettoCtx>],
        last_ids: &mut [i64],
        board: &mut VectorBoard,
        cfg: &Configuration<RistrettoCtx>,
        edit: Option<EditFactors>,
        step_errors: &mut Vec<String>,
    ) {
        for (position, trustee) in trustees.iter_mut().enumerate() {
            let messages = board.get(last_ids[position]);
            last_ids[position] += messages.len() as i64;
            match trustee.step(&messages) {
                Ok(result) => {
                    for message in result.messages {
                        let message = match edit {
                            Some(edit) if position == EDITING_TRUSTEE => {
                                resign_decryption_factors(message, cfg, trustee, edit)
                            }
                            _ => message,
                        };
                        board.add(message);
                    }
                }
                Err(error) => step_errors.push(error.to_string()),
            }
        }
    }

    fn run_decryption(edit: Option<EditFactors>) -> DecryptionRun {
        let ctx = RistrettoCtx;
        let test = create_protocol_test(N_TRUSTEES, &SELECTED, ctx.clone()).unwrap();
        let cfg = test.cfg;
        let mut board = test.remote;
        let mut trustees = test.trustees;
        let mut last_ids = vec![-1i64; trustees.len()];
        let mut step_errors = vec![];

        let mut dkg_pk = None;
        for _ in 0..MAX_CYCLES {
            step_all(
                &mut trustees,
                &mut last_ids,
                &mut board,
                &cfg,
                edit,
                &mut step_errors,
            );
            dkg_pk = trustees[0]._get_dkg_public_key_nohash();
            if dkg_pk.is_some() {
                break;
            }
        }
        let dkg_pk = dkg_pk.expect("the key ceremony completes");
        let pk_h = strand::hash::hash_to_array(&dkg_pk.strand_serialize().unwrap()).unwrap();
        let pk = strand::elgamal::PublicKey::from_element(&dkg_pk.pk, &ctx);

        let mut rng = ctx.get_rng();
        let plaintexts_in: Vec<<RistrettoCtx as Ctx>::P> = (0..CIPHERTEXTS)
            .map(|_| ctx.rnd_plaintext(&mut rng))
            .collect();
        let ballots: Vec<Ciphertext<RistrettoCtx>> = plaintexts_in
            .iter()
            .map(|p| pk.encrypt(&ctx.encode(p).unwrap()))
            .collect();
        let mut selected = [NULL_TRUSTEE; MAX_TRUSTEES];
        selected[0..SELECTED.len()].copy_from_slice(&SELECTED);
        let message = Message::ballots_msg(
            &cfg,
            BATCH,
            &Ballots::new(ballots),
            selected,
            PublicKeyHash(crate::util::hash_from_vec(&pk_h).unwrap()),
            &test.protocol_manager,
        )
        .unwrap();
        board.add(message);

        let decryptor = SELECTED[0] - 1;
        let mut plaintexts_out = None;
        for _ in 0..MAX_CYCLES {
            step_all(
                &mut trustees,
                &mut last_ids,
                &mut board,
                &cfg,
                edit,
                &mut step_errors,
            );
            plaintexts_out = trustees[decryptor]._get_plaintexts_nohash(BATCH, decryptor);
            if plaintexts_out.is_some() {
                break;
            }
        }

        DecryptionRun {
            plaintexts_in,
            plaintexts_out,
            step_errors,
        }
    }

    #[test]
    fn decrypts_with_one_factor_and_proof_per_ciphertext() {
        let run = run_decryption(None);

        let plaintexts_out = run.plaintexts_out.expect("plaintexts are computed");
        let expected: HashSet<<RistrettoCtx as Ctx>::P> = HashSet::from_iter(run.plaintexts_in);
        let actual: HashSet<<RistrettoCtx as Ctx>::P> = HashSet::from_iter(plaintexts_out.0 .0);
        assert_eq!(expected, actual);
    }

    #[test]
    fn rejects_decryption_factors_with_fewer_proofs_than_ciphertexts() {
        let run = run_decryption(Some(|dfactors| {
            let half = dfactors.proofs.0.len() / 2;
            dfactors.proofs.0.truncate(half);
        }));

        assert!(run.plaintexts_out.is_none());
        assert!(run
            .step_errors
            .iter()
            .any(|e| e.contains("Decryption factors from trustee 1 have 3 proofs, expected 6")));
    }

    #[test]
    fn rejects_decryption_factors_without_proofs() {
        let run = run_decryption(Some(|dfactors| dfactors.proofs.0.clear()));

        assert!(run.plaintexts_out.is_none());
        assert!(run
            .step_errors
            .iter()
            .any(|e| e.contains("Decryption factors from trustee 1 have 0 proofs, expected 6")));
    }

    #[test]
    fn rejects_decryption_factors_with_fewer_factors_than_ciphertexts() {
        let run = run_decryption(Some(|dfactors| {
            dfactors.factors.0.pop();
        }));

        assert!(run.plaintexts_out.is_none());
        assert!(run
            .step_errors
            .iter()
            .any(|e| e.contains("Decryption factors from trustee 1 have 5 factors, expected 6")));
    }
}
