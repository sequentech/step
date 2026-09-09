// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use base64::prelude::*;
use log::{info, warn};
use rand::seq::IndexedRandom;
use rand::Rng;
use rayon::prelude::*;
use std::collections::HashSet;
use std::marker::PhantomData;
use std::time::Instant;

use strand::context::Ctx;
use strand::elgamal::Ciphertext;
use strand::serialization::StrandDeserialize;
use strand::signature::{StrandSignaturePk, StrandSignatureSk};

use b4::messages::artifact::{Ballots, Configuration, DkgPublicKey, Plaintexts};
use b4::messages::message::Message;
use b4::messages::newtypes::PublicKeyHash;
use b4::messages::newtypes::MAX_TRUSTEES;
use b4::messages::newtypes::NULL_TRUSTEE;

use crate::native::board::HttpB3;
use crate::native::board::HttpB3BoardParams;
use crate::protocol::board::Board;

use crate::native::session::Session;
use crate::protocol::trustee::Trustee;

const HTTP_URL: &str = "http://127.0.0.1:3000";
const TEST_BOARD: &str = "protocoltest";
const S3_ENDPOINT: &str = "http://127.0.0.1:4566";
const BUCKET_NAME: &str = "wbraid-messages";

pub async fn run<C: Ctx + 'static>(ciphertexts: u32, batches: usize, ctx: C) {
    let n_trustees = rand::rng().random_range(2..13);
    let n_threshold = rand::rng().random_range(2..=n_trustees);
    // To test all trustees participating
    // let n_trustees = 2;
    // let n_threshold = n_trustees;
    let max: [usize; 12] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
    let all = &max[0..n_trustees];
    let mut rng = &mut rand::rng();
    let threshold: Vec<usize> = all
        .choose_multiple(&mut rng, n_threshold)
        .cloned()
        .collect();

    let now = Instant::now();

    let test = create_protocol_test(n_trustees, &threshold, ctx)
        .await
        .unwrap();

    run_protocol_test_http(test, ciphertexts, batches, &threshold)
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
    pub protocol_manager: b4::messages::protocol_manager::ProtocolManager<C>,
    pub trustees: Vec<Trustee<C, crate::native::board::NoOpStorage>>,
}

async fn run_protocol_test_http<C: Ctx + 'static>(
    test: ProtocolTest<C>,
    ciphertexts: u32,
    batches: usize,
    threshold: &[usize],
) -> Result<()> {
    let ctx = test.ctx.clone();
    let mut sessions = vec![];

    let _pks: Vec<StrandSignaturePk> = test.trustees.iter().map(|t| t.get_pk().unwrap()).collect();

    for t in test.trustees.into_iter() {
        let board_params = HttpB3BoardParams::new(HTTP_URL).await;
        let session: Session<C, HttpB3, crate::native::board::NoOpStorage> =
            Session::new(TEST_BOARD, t, board_params);
        sessions.push(session);
    }

    // Create a separate HTTP client for verification queries
    let client = reqwest::Client::new();
    let mut dkg_pk_message_id: Option<i64> = None;
    let count = ciphertexts;

    let mut selected_trustees = [NULL_TRUSTEE; MAX_TRUSTEES];
    selected_trustees[0..threshold.len()].copy_from_slice(threshold);

    // Run protocol until we get a DKG public key
    for i in 0..40 {
        info!("DKG Cycle {}", i);

        for s in sessions.iter_mut() {
            let result = s.step().await;
            if result.is_err() {
                warn!("Step returned err: {:?}", result);
            }
        }

        // Check for DKG public key message
        let response = client
            .get(format!("{}/boards/{}/messages", HTTP_URL, TEST_BOARD))
            .send()
            .await?;

        if !response.status().is_success() {
            continue;
        }

        let messages: serde_json::Value = response.json().await?;
        if let Some(msgs) = messages["messages"].as_array() {
            for msg in msgs {
                // Check statement_kind field directly instead of deserializing
                if let Some(kind) = msg["statement_kind"].as_str() {
                    if kind == "PublicKey" {
                        // Parse id as string then convert to i64
                        if let Some(id_str) = msg["id"].as_str() {
                            dkg_pk_message_id = id_str.parse::<i64>().ok();
                            break;
                        }
                    }
                }
            }
        }

        if dkg_pk_message_id.is_some() {
            break;
        }
    }

    assert!(dkg_pk_message_id.is_some(), "DKG public key not found");

    // Get the DKG public key
    let pk_response = client
        .get(format!(
            "{}/boards/{}/messages/{}",
            HTTP_URL,
            TEST_BOARD,
            dkg_pk_message_id.unwrap()
        ))
        .send()
        .await?;

    let pk_msg: serde_json::Value = pk_response.json().await?;

    // Response format is {"message": {...}, "download_url": ...}
    let message_obj = &pk_msg["message"];

    // Handle both inline (message) and S3 (key) formats
    let pk_bytes_encoded =
        if let Some(message_data) = message_obj["content_type"]["message"].as_str() {
            // Inline format
            BASE64_STANDARD.decode(message_data)?
        } else if let Some(s3_key) = message_obj["content_type"]["key"].as_str() {
            // S3 format - download from S3
            let s3_url = format!("{}/{}/{}", S3_ENDPOINT, BUCKET_NAME, s3_key);
            let s3_response = client.get(&s3_url).send().await?;
            s3_response.bytes().await?.to_vec()
        } else {
            panic!(
                "Unknown content_type format: {:?}",
                message_obj["content_type"]
            );
        };

    let pk_message = Message::strand_deserialize(&pk_bytes_encoded).unwrap();

    let pk_bytes = pk_message.artifact.unwrap();
    let pk_h = strand::hash::hash_to_array(&pk_bytes).unwrap();
    let dkg_pk = DkgPublicKey::<C>::strand_deserialize(&pk_bytes).unwrap();
    let pk = strand::elgamal::PublicKey::from_element(&dkg_pk.pk, &test.ctx);

    let mut plaintexts_in = vec![];
    let mut rng = ctx.get_rng();

    // Encrypt and submit ballots
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
            (i + 1) as u64,
            &ballot_batch,
            selected_trustees,
            PublicKeyHash(crate::util::hash_from_vec(&pk_h).unwrap()),
            &test.protocol_manager,
        )?;
        plaintexts_in.push(next_p);

        // Insert ballot message using a temporary board
        let board_params = HttpB3BoardParams::new(HTTP_URL).await;
        let mut temp_board = board_params.create_board(TEST_BOARD, None);
        temp_board
            .insert_messages(TEST_BOARD, vec![message])
            .await?;
    }

    // Wait for decryption
    let mut plaintexts_out: Vec<(i64, Message)> = vec![];
    for i in 0..150 {
        info!("Decryption Cycle {}", i);

        for s in sessions.iter_mut() {
            let result = s.step().await;
            if result.is_err() {
                warn!("Step returned err: {:?}", result);
            }
        }

        // Check for plaintext messages
        let response = client
            .get(format!("{}/boards/{}/messages", HTTP_URL, TEST_BOARD))
            .send()
            .await?;

        if !response.status().is_success() {
            continue;
        }

        let messages: serde_json::Value = response.json().await?;
        plaintexts_out.clear();

        if let Some(msgs) = messages["messages"].as_array() {
            for msg in msgs {
                // Check statement_kind field first to avoid unnecessary deserialization
                if let Some(kind) = msg["statement_kind"].as_str() {
                    if kind == "Plaintexts" {
                        // Download message bytes (handle both inline and S3)
                        let message_bytes =
                            if let Some(message_data) = msg["content_type"]["message"].as_str() {
                                // Inline format
                                BASE64_STANDARD.decode(message_data)?
                            } else if let Some(s3_key) = msg["content_type"]["key"].as_str() {
                                // S3 format - download from S3
                                let s3_url = format!("{}/{}/{}", S3_ENDPOINT, BUCKET_NAME, s3_key);
                                let s3_response = client.get(&s3_url).send().await?;
                                s3_response.bytes().await?.to_vec()
                            } else {
                                continue;
                            };

                        if let Ok(message) = Message::strand_deserialize(&message_bytes) {
                            if let Some(id_str) = msg["id"].as_str() {
                                if let Ok(id) = id_str.parse::<i64>() {
                                    plaintexts_out.push((id, message));
                                }
                            }
                        }
                    }
                }
            }
        }

        if plaintexts_out.len() == batches {
            break;
        }
    }

    assert!(
        plaintexts_out.len() == batches,
        "Expected {} plaintext messages, got {}",
        batches,
        plaintexts_out.len()
    );

    for (_, message) in plaintexts_out {
        let batch = message.statement.get_batch_number();
        let plaintexts = Plaintexts::<C>::strand_deserialize(&message.artifact.unwrap()).unwrap();
        let expected: HashSet<C::P> =
            HashSet::from_iter(plaintexts_in[(batch - 1) as usize].clone());
        let actual: HashSet<C::P> = HashSet::from_iter(plaintexts.0.clone().0);
        info!("expected {} actual {}", expected.len(), actual.len());

        assert!(expected == actual, "Plaintext mismatch for batch {}", batch);
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
    let pmkey: StrandSignatureSk = StrandSignatureSk::generate()?;
    let pm = b4::messages::protocol_manager::ProtocolManager {
        signing_key: pmkey,
        phantom: PhantomData,
    };
    let (trustees, trustee_pks): (
        Vec<Trustee<C, crate::native::board::NoOpStorage>>,
        Vec<StrandSignaturePk>,
    ) = (0..n_trustees)
        .map(|i| {
            let sk = StrandSignatureSk::generate().unwrap();
            let encryption_key = strand::symm::gen_key();
            let pk = StrandSignaturePk::from_sk(&sk).unwrap();
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
        StrandSignaturePk::from_sk(&pm.signing_key).unwrap(),
        trustee_pks,
        threshold.len(),
        PhantomData,
    );

    // Bootstrap message will be sent by first session
    let message = Message::bootstrap_msg(&cfg, &pm)?;

    // Create HTTP client to initialize board
    let client = reqwest::Client::new();

    // Create board (ignore error if already exists)
    let _ = client
        .post(format!("{}/boards", HTTP_URL))
        .json(&serde_json::json!({
            "name": TEST_BOARD
        }))
        .send()
        .await;

    // Send bootstrap message
    let board_params = HttpB3BoardParams::new(HTTP_URL).await;
    let mut temp_board = board_params.create_board(TEST_BOARD, None);
    temp_board
        .insert_messages(TEST_BOARD, vec![message])
        .await?;

    Ok(ProtocolTest {
        ctx,
        cfg,
        protocol_manager: pm,
        trustees,
    })
}
