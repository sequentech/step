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

use b3::messages::artifact::{Ballots, Configuration, DkgPublicKey, Plaintexts};
use b3::messages::message::Message;
use b3::messages::newtypes::PublicKeyHash;
use b3::messages::newtypes::MAX_TRUSTEES;
use b3::messages::newtypes::NULL_TRUSTEE;
use b3::messages::protocol_manager::ProtocolManager;
use b3::messages::statement::StatementType;

use b3::client::pgsql::B3MessageRow;
use b3::client::pgsql::PgsqlConnectionParams;
use b3::client::pgsql::{self, PgsqlB3Client};

use crate::protocol::board::grpc_m::GrpcB3;
use crate::protocol::board::grpc_m::GrpcB3BoardParams;

use crate::protocol::session::Session;
use crate::protocol::trustee2::Trustee;

const B3_URL: &'static str = "http://127.0.0.1:50051";
const TEST_BOARD: &'static str = "protocoltest";
const PG_HOST: &'static str = "localhost";
const PG_DATABASE: &'static str = "protocoldb";
const PG_USER: &'static str = "postgres";
const PG_PASSW: &'static str = "password";
const PG_PORT: u32 = 5432;

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

    let test = create_protocol_test(n_trustees, &threshold, ctx)
        .await
        .unwrap();

    run_protocol_test_grpc(test, ciphertexts, batches, &threshold)
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

pub struct ProtocolTest<C: Ctx> {
    pub ctx: C,
    pub cfg: Configuration<C>,
    pub protocol_manager: ProtocolManager<C>,
    pub trustees: Vec<Trustee<C>>,
}

async fn run_protocol_test_grpc<C: Ctx + 'static>(
    test: ProtocolTest<C>,
    ciphertexts: u32,
    batches: usize,
    threshold: &[usize],
) -> Result<()> {
    info!("{}", strand::info_string());

    let ctx = test.ctx.clone();
    let mut sessions = vec![];

    let pks: Vec<StrandSignaturePk> = test.trustees.iter().map(|t| t.get_pk().unwrap()).collect();

    for t in test.trustees.into_iter() {
        let board_params = GrpcB3BoardParams::new(B3_URL);
        let session: Session<C, GrpcB3> = Session::new(TEST_BOARD, t, board_params);
        sessions.push(session);
    }

    let c = PgsqlConnectionParams::new(PG_HOST, PG_PORT, PG_USER, PG_PASSW);
    let c = c.with_database(PG_DATABASE);
    let mut b = PgsqlB3Client::new(&c).await?;

    let mut dkg_pk_message: Vec<B3MessageRow> = vec![];
    let count = ciphertexts;

    let mut selected_trustees = [NULL_TRUSTEE; MAX_TRUSTEES];
    selected_trustees[0..threshold.len()].copy_from_slice(threshold);

    for i in 0..40 {
        info!("Cycle {}", i);

        for s in sessions.iter_mut() {
            let result = s.step().await;
            if result.is_err() {
                warn!("Step returned err: {:?}", result);
            }
        }

        dkg_pk_message = b
            .get_with_kind(TEST_BOARD, StatementType::PublicKey, &pks[0])
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

        for s in sessions.iter_mut() {
            let result = s.step().await;
            if result.is_err() {
                warn!("Step returned err: {:?}", result);
            }
        }

        plaintexts_out = b
            .get_with_kind(
                TEST_BOARD,
                StatementType::Plaintexts,
                &pks[selected_trustees[0] - 1],
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

pub async fn create_protocol_test<C: Ctx>(
    n_trustees: usize,
    threshold: &[usize],
    ctx: C,
) -> Result<ProtocolTest<C>> {
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

    let c = PgsqlConnectionParams::new(PG_HOST, PG_PORT, PG_USER, PG_PASSW);
    // swallow database already exists errors
    let _ = pgsql::create_database(&c, PG_DATABASE).await;

    let c = c.with_database(PG_DATABASE);
    let mut client = PgsqlB3Client::new(&c).await.unwrap();
    client.clear_database().await.unwrap();
    client.create_index_ine().await.unwrap();
    client.create_board_ine(TEST_BOARD).await.unwrap();

    let message = Message::bootstrap_msg(&cfg, &pm)?;
    let bm: Result<B3MessageRow> = message.try_into();
    let messages = vec![bm.unwrap()];
    client.insert_messages(TEST_BOARD, &messages).await.unwrap();

    Ok(ProtocolTest {
        ctx,
        cfg,
        protocol_manager: pm,
        trustees,
    })
}
