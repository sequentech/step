// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use log::{info, warn};
use rand::seq::SliceRandom;
use rand::Rng;
use rayon::prelude::*;
use std::collections::HashSet;
use std::marker::PhantomData;
use std::time::Instant;

use strand::context::Ctx;
use strand::elgamal::Ciphertext;
use strand::serialization::StrandDeserialize;
use strand::signature::{StrandSignaturePk, StrandSignatureSk};

use board_messages::braid::artifact::{Ballots, Configuration, DkgPublicKey, Plaintexts};
use board_messages::braid::message::Message;
use board_messages::braid::newtypes::PublicKeyHash;
use board_messages::braid::newtypes::MAX_TRUSTEES;
use board_messages::braid::newtypes::NULL_TRUSTEE;
use board_messages::braid::protocol_manager::ProtocolManager;
use board_messages::braid::statement::StatementType;
use board_messages::grpc::pgsql;
use board_messages::grpc::pgsql::XPgsqlB3Client;
use board_messages::grpc::pgsql::{B3MessageRow, PgsqlConnectionParams};

use crate::protocol::board::pgsql::PgsqlBoard;
use crate::protocol::board::pgsql::PgsqlBoardParams;

use crate::protocol::session::Session;
use crate::protocol::trustee::Trustee;

const PG_HOST: &'static str = "postgres";
const PG_DATABASE: &'static str = "protocoldb";
const PG_USER: &'static str = "postgres";
const PG_PASSW: &'static str = "postgrespassword";
const PG_PORT: u32 = 5432;
const TEST_BOARD: &'static str = "testboard";

pub async fn run<C: Ctx + 'static>(ciphertexts: u32, batches: usize, ctx: C) {
    let n_trustees = rand::thread_rng().gen_range(2..13);
    let n_threshold = rand::thread_rng().gen_range(2..=n_trustees);
    // To test all trustees participating
    // let n_trustees = 2;
    // let n_threshold = n_trustees;
    let max: [usize; 12] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
    let all = &max[0..n_trustees];
    let mut rng = &mut rand::thread_rng();
    let threshold: Vec<usize> = all
        .choose_multiple(&mut rng, n_threshold)
        .cloned()
        .collect();

    let now = Instant::now();

    let test = create_protocol_test_pgsql(n_trustees, &threshold, ctx)
        .await
        .unwrap();

    run_protocol_test_pgsql(test, ciphertexts, batches, &threshold)
        .await
        .unwrap();

    let time = now.elapsed().as_millis() as f64 / 1000.0;
    info!(
        "batches = {}, time = {}, rate = {}",
        batches,
        time,
        ((ciphertexts as f64 * batches as f64) / time),
    );
}

pub struct ProtocolTestPgsql<C: Ctx> {
    pub ctx: C,
    pub cfg: Configuration<C>,
    pub protocol_manager: ProtocolManager<C>,
    pub trustees: Vec<Trustee<C>>,
}

async fn run_protocol_test_pgsql<C: Ctx + 'static>(
    test: ProtocolTestPgsql<C>,
    ciphertexts: u32,
    batches: usize,
    threshold: &[usize],
) -> Result<()> {
    info!("{}", strand::info_string());

    let ctx = test.ctx.clone();
    let mut sessions = vec![];

    let pk_strings: Vec<String> = test
        .trustees
        .iter()
        .map(|t| t.get_pk().unwrap().to_der_b64_string().unwrap())
        .collect();

    let c = PgsqlConnectionParams::new(PG_HOST, PG_PORT, PG_USER, PG_PASSW);
    let c = c.with_database(PG_DATABASE);

    for t in test.trustees.into_iter() {
        let board = PgsqlBoardParams::new(&c, TEST_BOARD.to_string(), None);
        let session: Session<C, PgsqlBoard> = Session::new(TEST_BOARD, t, board);
        sessions.push(session);
    }

    let mut b = XPgsqlB3Client::new(&c).await.unwrap();

    let mut dkg_pk_message: Vec<B3MessageRow> = vec![];
    let count = ciphertexts;

    let mut selected_trustees = [NULL_TRUSTEE; MAX_TRUSTEES];
    selected_trustees[0..threshold.len()].copy_from_slice(threshold);

    for i in 0..40 {
        info!("Cycle {}", i);

        // See https://stackoverflow.com/questions/63434977/how-can-i-spawn-asynchronous-methods-in-a-loop
        let handles: Vec<_> = sessions
            .into_iter()
            .map(|s| tokio::spawn(async { s.step().await }))
            .collect();

        sessions = vec![];
        for h in handles {
            let (session, result) = h.await.unwrap();
            if result.is_err() {
                warn!("Step returned err: {:?}", result);
            }
            sessions.push(session);
        }

        dkg_pk_message = b
            .get_with_kind(
                TEST_BOARD,
                &StatementType::PublicKey.to_string(),
                &pk_strings[0],
            )
            .await
            .unwrap();

        if dkg_pk_message.len() > 0 {
            break;
        }
    }

    assert!(dkg_pk_message.len() > 0);
    let message = Message::strand_deserialize(&dkg_pk_message[0].message).unwrap();
    let pk_bytes = message.artifact.unwrap();
    let pk_h = strand::hash::hash_to_array(&pk_bytes).unwrap();
    let dkg_pk = DkgPublicKey::<C>::strand_deserialize(&pk_bytes).unwrap();
    let pk = strand::elgamal::PublicKey::from_element(&dkg_pk.pk, &test.ctx);

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
        let messages = vec![message.try_into().unwrap()];
        b.insert_messages(TEST_BOARD, &messages).await.unwrap();
    }

    let mut plaintexts_out: Vec<B3MessageRow> = vec![];
    for i in 0..150 {
        info!("Cycle {}", i);

        let handles: Vec<_> = sessions
            .into_iter()
            .map(|s| tokio::spawn(async { s.step().await }))
            .collect();

        sessions = vec![];
        for h in handles {
            let (session, result) = h.await.unwrap();
            if result.is_err() {
                warn!("Step returned err: {:?}", result);
            }
            sessions.push(session);
        }

        plaintexts_out = b
            .get_with_kind(
                TEST_BOARD,
                &StatementType::Plaintexts.to_string(),
                &pk_strings[selected_trustees[0] - 1],
            )
            .await
            .unwrap();

        if plaintexts_out.len() == batches {
            break;
        }
    }

    assert!(plaintexts_out.len() == batches);
    for p in plaintexts_out {
        let message = Message::strand_deserialize(&p.message).unwrap();
        let batch = message.statement.get_batch_number();
        let plaintexts = Plaintexts::<C>::strand_deserialize(&message.artifact.unwrap()).unwrap();
        let expected: HashSet<C::P> = HashSet::from_iter(plaintexts_in[batch - 1].clone());
        let actual: HashSet<C::P> = HashSet::from_iter(plaintexts.0.clone().0);
        info!("expected {} actual {}", expected.len(), actual.len());

        assert!(expected == actual);
        info!("Match ok on plaintexts for batch {}", batch);
    }

    info!("***************************************************************");
    info!("* Completed");
    info!("* Trustees = {}", sessions.len());
    info!("* Threshold = {}", threshold.len());
    info!("* Ciphertexts = {}", count);
    info!("***************************************************************");

    Ok(())
}

pub async fn create_protocol_test_pgsql<C: Ctx>(
    n_trustees: usize,
    threshold: &[usize],
    ctx: C,
) -> Result<ProtocolTestPgsql<C>> {
    let pmkey: StrandSignatureSk = StrandSignatureSk::gen()?;
    let pm: ProtocolManager<C> = ProtocolManager {
        signing_key: pmkey,
        phantom: PhantomData,
    };
    let (trustees, trustee_pks): (Vec<Trustee<C>>, Vec<StrandSignaturePk>) = (0..n_trustees)
        .map(|i| {
            let sk = StrandSignatureSk::gen().unwrap();
            let encryption_key = strand::symm::gen_key();
            let pk = StrandSignaturePk::from_sk(&sk).unwrap();
            (Trustee::new(i.to_string(), sk, encryption_key), pk)
        })
        .unzip();

    let cfg = Configuration::<C>::new(
        0,
        StrandSignaturePk::from_sk(&pm.signing_key).unwrap(),
        trustee_pks,
        threshold.len(),
        PhantomData,
    );

    let c = PgsqlConnectionParams::new(PG_HOST, PG_PORT, PG_USER, PG_PASSW);
    pgsql::drop_database(&c, PG_DATABASE).await.unwrap();

    pgsql::create_database(&c, PG_DATABASE).await.unwrap();

    let c = c.with_database(PG_DATABASE);
    let mut b = XPgsqlB3Client::new(&c).await?;
    b.create_index_ine().await.unwrap();
    b.create_board_ine(TEST_BOARD).await.unwrap();

    let message = Message::bootstrap_msg(&cfg, &pm)?;
    let bm: Result<B3MessageRow> = message.try_into();
    let messages = vec![bm.unwrap()];
    b.insert_messages(TEST_BOARD, &messages).await.unwrap();

    Ok(ProtocolTestPgsql {
        ctx,
        cfg,
        protocol_manager: pm,
        trustees,
    })
}
